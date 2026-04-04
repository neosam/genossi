use async_trait::async_trait;
use genossi_dao::member::MemberDao;
use genossi_dao::member_action::{ActionType, MemberActionDao};
use genossi_dao::TransactionDao;
use genossi_service::member_action::MigrationState;
use genossi_service::permission::{Authentication, PermissionService};
use genossi_service::validation::{
    ActiveMemberNoShares, DuplicateMemberNumber, ExitDateMismatch, ExitedMemberWithShares,
    MigratedFlagMismatch, MissingEntryAction, SharesMismatch, UnmatchedTransfer,
    ValidationResult, ValidationService,
};
use genossi_service::ServiceError;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use uuid::Uuid;

use crate::gen_service_impl;
use crate::member_action::compute_migration_status;

const VIEW_MEMBERS_PRIVILEGE: &str = "view_members";

gen_service_impl! {
    struct ValidationServiceImpl: ValidationService = ValidationServiceDeps {
        MemberDao: MemberDao<Transaction = Self::Transaction> = member_dao,
        MemberActionDao: MemberActionDao<Transaction = Self::Transaction> = member_action_dao,
        PermissionService: PermissionService<Context = Self::Context> = permission_service,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
    }
}

fn find_member_number_gaps(
    members: &[genossi_dao::member::MemberEntity],
) -> Arc<[i64]> {
    if members.is_empty() {
        return Arc::from([]);
    }

    let numbers: HashSet<i64> = members.iter().map(|m| m.member_number).collect();
    let min = members.iter().map(|m| m.member_number).min().unwrap();
    let max = members.iter().map(|m| m.member_number).max().unwrap();

    let mut gaps = Vec::new();
    for n in min..=max {
        if !numbers.contains(&n) {
            gaps.push(n);
        }
    }
    gaps.into()
}

fn find_unmatched_transfers(
    actions: &[genossi_dao::member_action::MemberActionEntity],
    members: &[genossi_dao::member::MemberEntity],
) -> Arc<[UnmatchedTransfer]> {
    let member_number_map: HashMap<Uuid, i64> = members
        .iter()
        .map(|m| (m.id, m.member_number))
        .collect();

    let transfers: Vec<&genossi_dao::member_action::MemberActionEntity> = actions
        .iter()
        .filter(|a| {
            matches!(
                a.action_type,
                ActionType::UebertragungEmpfang | ActionType::UebertragungAbgabe
            )
        })
        .collect();

    let mut matched: HashSet<Uuid> = HashSet::new();

    for a in &transfers {
        if matched.contains(&a.id) {
            continue;
        }
        let Some(transfer_member_id) = a.transfer_member_id else {
            continue;
        };

        let counterpart = transfers.iter().find(|b| {
            b.id != a.id
                && !matched.contains(&b.id)
                && b.member_id == transfer_member_id
                && b.transfer_member_id == Some(a.member_id)
                && b.shares_change == -a.shares_change
                && b.date == a.date
                && is_complementary_type(&a.action_type, &b.action_type)
        });

        if let Some(b) = counterpart {
            matched.insert(a.id);
            matched.insert(b.id);
        }
    }

    transfers
        .iter()
        .filter(|a| !matched.contains(&a.id))
        .map(|a| {
            let transfer_member_id = a.transfer_member_id.unwrap_or(Uuid::nil());
            UnmatchedTransfer {
                action_id: a.id,
                member_id: a.member_id,
                member_number: *member_number_map.get(&a.member_id).unwrap_or(&0),
                action_type: a.action_type.clone(),
                transfer_member_id,
                transfer_member_number: *member_number_map
                    .get(&transfer_member_id)
                    .unwrap_or(&0),
                shares_change: a.shares_change,
                date: a.date,
            }
        })
        .collect()
}

fn is_complementary_type(a: &ActionType, b: &ActionType) -> bool {
    matches!(
        (a, b),
        (ActionType::UebertragungAbgabe, ActionType::UebertragungEmpfang)
            | (ActionType::UebertragungEmpfang, ActionType::UebertragungAbgabe)
    )
}

fn find_shares_mismatches(
    members: &[genossi_dao::member::MemberEntity],
    actions: &[genossi_dao::member_action::MemberActionEntity],
) -> Arc<[SharesMismatch]> {
    let actions_by_member: HashMap<Uuid, Vec<&genossi_dao::member_action::MemberActionEntity>> =
        actions.iter().fold(HashMap::new(), |mut map, a| {
            map.entry(a.member_id).or_default().push(a);
            map
        });

    members
        .iter()
        .filter(|m| m.deleted.is_none())
        .filter_map(|m| {
            let actual: i32 = actions_by_member
                .get(&m.id)
                .map(|acts| acts.iter().map(|a| a.shares_change).sum())
                .unwrap_or(0);
            if m.current_shares != actual {
                Some(SharesMismatch {
                    member_id: m.id,
                    member_number: m.member_number,
                    expected: m.current_shares,
                    actual,
                })
            } else {
                None
            }
        })
        .collect()
}

fn find_missing_entry_actions(
    members: &[genossi_dao::member::MemberEntity],
    actions: &[genossi_dao::member_action::MemberActionEntity],
) -> Arc<[MissingEntryAction]> {
    let eintritt_counts: HashMap<Uuid, i32> =
        actions
            .iter()
            .filter(|a| a.action_type == ActionType::Eintritt)
            .fold(HashMap::new(), |mut map, a| {
                *map.entry(a.member_id).or_insert(0) += 1;
                map
            });

    members
        .iter()
        .filter(|m| m.deleted.is_none())
        .filter_map(|m| {
            let count = *eintritt_counts.get(&m.id).unwrap_or(&0);
            if count != 1 {
                Some(MissingEntryAction {
                    member_id: m.id,
                    member_number: m.member_number,
                    actual_count: count,
                })
            } else {
                None
            }
        })
        .collect()
}

fn find_exit_date_mismatches(
    members: &[genossi_dao::member::MemberEntity],
    actions: &[genossi_dao::member_action::MemberActionEntity],
) -> Arc<[ExitDateMismatch]> {
    let has_austritt: HashSet<Uuid> = actions
        .iter()
        .filter(|a| matches!(a.action_type, ActionType::Austritt | ActionType::Todesfall))
        .map(|a| a.member_id)
        .collect();

    members
        .iter()
        .filter(|m| m.deleted.is_none())
        .filter_map(|m| {
            let has_exit_date = m.exit_date.is_some();
            let has_austritt_action = has_austritt.contains(&m.id);
            if has_exit_date != has_austritt_action {
                Some(ExitDateMismatch {
                    member_id: m.id,
                    member_number: m.member_number,
                    has_exit_date,
                    has_austritt_action,
                })
            } else {
                None
            }
        })
        .collect()
}

fn find_active_members_no_shares(
    members: &[genossi_dao::member::MemberEntity],
) -> Arc<[ActiveMemberNoShares]> {
    members
        .iter()
        .filter(|m| m.deleted.is_none() && m.exit_date.is_none() && m.current_shares <= 0)
        .map(|m| ActiveMemberNoShares {
            member_id: m.id,
            member_number: m.member_number,
        })
        .collect()
}

fn find_duplicate_member_numbers(
    members: &[genossi_dao::member::MemberEntity],
) -> Arc<[DuplicateMemberNumber]> {
    let mut number_to_ids: HashMap<i64, Vec<Uuid>> = HashMap::new();
    for m in members.iter().filter(|m| m.deleted.is_none()) {
        number_to_ids.entry(m.member_number).or_default().push(m.id);
    }

    let mut duplicates: Vec<DuplicateMemberNumber> = number_to_ids
        .into_iter()
        .filter(|(_, ids)| ids.len() > 1)
        .map(|(number, ids)| DuplicateMemberNumber {
            member_number: number,
            member_ids: ids.into(),
        })
        .collect();
    duplicates.sort_by_key(|d| d.member_number);
    duplicates.into()
}

fn find_exited_members_with_shares(
    members: &[genossi_dao::member::MemberEntity],
) -> Arc<[ExitedMemberWithShares]> {
    members
        .iter()
        .filter(|m| m.deleted.is_none() && m.exit_date.is_some() && m.current_shares > 0)
        .map(|m| ExitedMemberWithShares {
            member_id: m.id,
            member_number: m.member_number,
            current_shares: m.current_shares,
        })
        .collect()
}

fn find_migrated_flag_mismatches(
    members: &[genossi_dao::member::MemberEntity],
    actions: &[genossi_dao::member_action::MemberActionEntity],
) -> Arc<[MigratedFlagMismatch]> {
    let actions_by_member: HashMap<Uuid, Vec<genossi_dao::member_action::MemberActionEntity>> =
        actions.iter().fold(HashMap::new(), |mut map, a| {
            map.entry(a.member_id).or_default().push(a.clone());
            map
        });

    members
        .iter()
        .filter(|m| m.deleted.is_none())
        .filter_map(|m| {
            let member_actions = actions_by_member.get(&m.id);
            let empty = Vec::new();
            let acts = member_actions.unwrap_or(&empty);
            let status = compute_migration_status(m, acts);
            let computed_migrated = status.status == MigrationState::Migrated;
            if m.migrated != computed_migrated {
                Some(MigratedFlagMismatch {
                    member_id: m.id,
                    member_number: m.member_number,
                    flag_value: m.migrated,
                    computed_status: Arc::from(if computed_migrated {
                        "Migrated"
                    } else {
                        "Pending"
                    }),
                })
            } else {
                None
            }
        })
        .collect()
}

#[async_trait]
impl<Deps: ValidationServiceDeps> ValidationService for ValidationServiceImpl<Deps> {
    type Context = Deps::Context;

    async fn validate(
        &self,
        context: Authentication<Self::Context>,
    ) -> Result<ValidationResult, ServiceError> {
        self.permission_service
            .check_permission(VIEW_MEMBERS_PRIVILEGE, context)
            .await?;

        let tx = self.transaction_dao.use_transaction(None).await?;

        let all_members = self.member_dao.dump_all(tx.clone()).await?;
        let all_actions = self.member_action_dao.all(tx.clone()).await?;

        self.transaction_dao.commit(tx).await?;

        let member_number_gaps = find_member_number_gaps(&all_members);
        let unmatched_transfers = find_unmatched_transfers(&all_actions, &all_members);
        let shares_mismatches = find_shares_mismatches(&all_members, &all_actions);
        let missing_entry_actions = find_missing_entry_actions(&all_members, &all_actions);
        let exit_date_mismatches = find_exit_date_mismatches(&all_members, &all_actions);
        let active_members_no_shares = find_active_members_no_shares(&all_members);
        let duplicate_member_numbers = find_duplicate_member_numbers(&all_members);
        let exited_members_with_shares = find_exited_members_with_shares(&all_members);
        let migrated_flag_mismatches = find_migrated_flag_mismatches(&all_members, &all_actions);

        Ok(ValidationResult {
            member_number_gaps,
            unmatched_transfers,
            shares_mismatches,
            missing_entry_actions,
            exit_date_mismatches,
            active_members_no_shares,
            duplicate_member_numbers,
            exited_members_with_shares,
            migrated_flag_mismatches,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use genossi_dao::member::MemberEntity;
    use genossi_dao::member_action::MemberActionEntity;
    use time::{Date, Month, Time};

    fn make_member(member_number: i64, id: Uuid, deleted: Option<time::PrimitiveDateTime>) -> MemberEntity {
        let date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, Time::MIDNIGHT);
        MemberEntity {
            id,
            member_number,
            first_name: Arc::from("Test"),
            last_name: Arc::from("User"),
            salutation: None,
            title: None,
            email: None,
            company: None,
            comment: None,
            street: None,
            house_number: None,
            postal_code: None,
            city: None,
            join_date: date,
            shares_at_joining: 1,
            current_shares: 1,
            current_balance: 0,
            action_count: 0,
            migrated: false,
            exit_date: None,
            bank_account: None,
            created: datetime,
            deleted,
            version: Uuid::new_v4(),
        }
    }

    fn make_action(
        member_id: Uuid,
        action_type: ActionType,
        shares_change: i32,
    ) -> MemberActionEntity {
        let date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, Time::MIDNIGHT);
        MemberActionEntity {
            id: Uuid::new_v4(),
            member_id,
            action_type,
            date,
            shares_change,
            transfer_member_id: None,
            effective_date: None,
            comment: None,
            created: datetime,
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    fn make_transfer(
        id: Uuid,
        member_id: Uuid,
        action_type: ActionType,
        transfer_member_id: Uuid,
        shares_change: i32,
        date: Date,
    ) -> MemberActionEntity {
        let datetime = time::PrimitiveDateTime::new(date, Time::MIDNIGHT);
        MemberActionEntity {
            id,
            member_id,
            action_type,
            date,
            shares_change,
            transfer_member_id: Some(transfer_member_id),
            effective_date: None,
            comment: None,
            created: datetime,
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    // === Member number gap tests ===

    #[test]
    fn test_no_gaps() {
        let members = vec![
            make_member(1, Uuid::new_v4(), None),
            make_member(2, Uuid::new_v4(), None),
            make_member(3, Uuid::new_v4(), None),
        ];
        let gaps = find_member_number_gaps(&members);
        assert!(gaps.is_empty());
    }

    #[test]
    fn test_gaps_present() {
        let members = vec![
            make_member(1, Uuid::new_v4(), None),
            make_member(2, Uuid::new_v4(), None),
            make_member(5, Uuid::new_v4(), None),
            make_member(8, Uuid::new_v4(), None),
        ];
        let gaps = find_member_number_gaps(&members);
        assert_eq!(&*gaps, &[3, 4, 6, 7]);
    }

    #[test]
    fn test_gaps_not_starting_at_1() {
        let members = vec![
            make_member(100, Uuid::new_v4(), None),
            make_member(101, Uuid::new_v4(), None),
            make_member(103, Uuid::new_v4(), None),
        ];
        let gaps = find_member_number_gaps(&members);
        assert_eq!(&*gaps, &[102]);
    }

    #[test]
    fn test_soft_deleted_members_count() {
        let deleted_at = time::PrimitiveDateTime::new(
            Date::from_calendar_date(2025, Month::June, 1).unwrap(),
            Time::MIDNIGHT,
        );
        let members = vec![
            make_member(1, Uuid::new_v4(), None),
            make_member(2, Uuid::new_v4(), None),
            make_member(3, Uuid::new_v4(), Some(deleted_at)),
            make_member(4, Uuid::new_v4(), None),
        ];
        let gaps = find_member_number_gaps(&members);
        assert!(gaps.is_empty());
    }

    #[test]
    fn test_no_members() {
        let members: Vec<MemberEntity> = vec![];
        let gaps = find_member_number_gaps(&members);
        assert!(gaps.is_empty());
    }

    // === Transfer matching tests ===

    #[test]
    fn test_all_transfers_matched() {
        let member_a = Uuid::new_v4();
        let member_b = Uuid::new_v4();
        let date = Date::from_calendar_date(2024, Month::May, 1).unwrap();

        let members = vec![
            make_member(1, member_a, None),
            make_member(2, member_b, None),
        ];

        let actions = vec![
            make_transfer(Uuid::new_v4(), member_a, ActionType::UebertragungAbgabe, member_b, -3, date),
            make_transfer(Uuid::new_v4(), member_b, ActionType::UebertragungEmpfang, member_a, 3, date),
        ];

        let unmatched = find_unmatched_transfers(&actions, &members);
        assert!(unmatched.is_empty());
    }

    #[test]
    fn test_missing_counterpart() {
        let member_a = Uuid::new_v4();
        let member_b = Uuid::new_v4();
        let date = Date::from_calendar_date(2024, Month::May, 1).unwrap();

        let members = vec![
            make_member(1, member_a, None),
            make_member(2, member_b, None),
        ];

        let actions = vec![make_transfer(
            Uuid::new_v4(), member_a, ActionType::UebertragungAbgabe, member_b, -3, date,
        )];

        let unmatched = find_unmatched_transfers(&actions, &members);
        assert_eq!(unmatched.len(), 1);
        assert_eq!(unmatched[0].member_id, member_a);
    }

    #[test]
    fn test_transfer_shares_mismatch() {
        let member_a = Uuid::new_v4();
        let member_b = Uuid::new_v4();
        let date = Date::from_calendar_date(2024, Month::May, 1).unwrap();

        let members = vec![
            make_member(1, member_a, None),
            make_member(2, member_b, None),
        ];

        let actions = vec![
            make_transfer(Uuid::new_v4(), member_a, ActionType::UebertragungAbgabe, member_b, -3, date),
            make_transfer(Uuid::new_v4(), member_b, ActionType::UebertragungEmpfang, member_a, 2, date),
        ];

        let unmatched = find_unmatched_transfers(&actions, &members);
        assert_eq!(unmatched.len(), 2);
    }

    #[test]
    fn test_transfer_date_mismatch() {
        let member_a = Uuid::new_v4();
        let member_b = Uuid::new_v4();
        let date1 = Date::from_calendar_date(2024, Month::May, 1).unwrap();
        let date2 = Date::from_calendar_date(2024, Month::June, 1).unwrap();

        let members = vec![
            make_member(1, member_a, None),
            make_member(2, member_b, None),
        ];

        let actions = vec![
            make_transfer(Uuid::new_v4(), member_a, ActionType::UebertragungAbgabe, member_b, -3, date1),
            make_transfer(Uuid::new_v4(), member_b, ActionType::UebertragungEmpfang, member_a, 3, date2),
        ];

        let unmatched = find_unmatched_transfers(&actions, &members);
        assert_eq!(unmatched.len(), 2);
    }

    #[test]
    fn test_no_transfers() {
        let members = vec![make_member(1, Uuid::new_v4(), None)];
        let actions: Vec<MemberActionEntity> = vec![];
        let unmatched = find_unmatched_transfers(&actions, &members);
        assert!(unmatched.is_empty());
    }

    // === Shares consistency tests ===

    #[test]
    fn test_shares_match() {
        let id = Uuid::new_v4();
        let mut member = make_member(1, id, None);
        member.current_shares = 5;

        let actions = vec![
            make_action(id, ActionType::Eintritt, 0),
            make_action(id, ActionType::Aufstockung, 5),
        ];

        let mismatches = find_shares_mismatches(&[member], &actions);
        assert!(mismatches.is_empty());
    }

    #[test]
    fn test_shares_diverge() {
        let id = Uuid::new_v4();
        let mut member = make_member(1, id, None);
        member.current_shares = 5;

        let actions = vec![
            make_action(id, ActionType::Eintritt, 0),
            make_action(id, ActionType::Aufstockung, 3),
        ];

        let mismatches = find_shares_mismatches(&[member], &actions);
        assert_eq!(mismatches.len(), 1);
        assert_eq!(mismatches[0].expected, 5);
        assert_eq!(mismatches[0].actual, 3);
    }

    #[test]
    fn test_shares_no_actions() {
        let id = Uuid::new_v4();
        let mut member = make_member(1, id, None);
        member.current_shares = 3;

        let mismatches = find_shares_mismatches(&[member], &[]);
        assert_eq!(mismatches.len(), 1);
        assert_eq!(mismatches[0].expected, 3);
        assert_eq!(mismatches[0].actual, 0);
    }

    // === Entry action tests ===

    #[test]
    fn test_entry_action_present() {
        let id = Uuid::new_v4();
        let member = make_member(1, id, None);
        let actions = vec![make_action(id, ActionType::Eintritt, 0)];

        let missing = find_missing_entry_actions(&[member], &actions);
        assert!(missing.is_empty());
    }

    #[test]
    fn test_entry_action_missing() {
        let id = Uuid::new_v4();
        let member = make_member(1, id, None);
        let actions = vec![make_action(id, ActionType::Aufstockung, 3)];

        let missing = find_missing_entry_actions(&[member], &actions);
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].actual_count, 0);
    }

    #[test]
    fn test_entry_action_duplicate() {
        let id = Uuid::new_v4();
        let member = make_member(1, id, None);
        let actions = vec![
            make_action(id, ActionType::Eintritt, 0),
            make_action(id, ActionType::Eintritt, 0),
        ];

        let missing = find_missing_entry_actions(&[member], &actions);
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].actual_count, 2);
    }

    // === Exit date consistency tests ===

    #[test]
    fn test_exit_date_both_present() {
        let id = Uuid::new_v4();
        let mut member = make_member(1, id, None);
        member.exit_date = Some(Date::from_calendar_date(2025, Month::June, 1).unwrap());
        let actions = vec![make_action(id, ActionType::Austritt, 0)];

        let mismatches = find_exit_date_mismatches(&[member], &actions);
        assert!(mismatches.is_empty());
    }

    #[test]
    fn test_exit_date_both_absent() {
        let id = Uuid::new_v4();
        let member = make_member(1, id, None);
        let actions = vec![make_action(id, ActionType::Aufstockung, 3)];

        let mismatches = find_exit_date_mismatches(&[member], &actions);
        assert!(mismatches.is_empty());
    }

    #[test]
    fn test_exit_date_without_austritt() {
        let id = Uuid::new_v4();
        let mut member = make_member(1, id, None);
        member.exit_date = Some(Date::from_calendar_date(2025, Month::June, 1).unwrap());

        let mismatches = find_exit_date_mismatches(&[member], &[]);
        assert_eq!(mismatches.len(), 1);
        assert!(mismatches[0].has_exit_date);
        assert!(!mismatches[0].has_austritt_action);
    }

    #[test]
    fn test_austritt_without_exit_date() {
        let id = Uuid::new_v4();
        let member = make_member(1, id, None);
        let actions = vec![make_action(id, ActionType::Austritt, 0)];

        let mismatches = find_exit_date_mismatches(&[member], &actions);
        assert_eq!(mismatches.len(), 1);
        assert!(!mismatches[0].has_exit_date);
        assert!(mismatches[0].has_austritt_action);
    }

    // === Active members no shares tests ===

    #[test]
    fn test_active_member_with_shares() {
        let mut member = make_member(1, Uuid::new_v4(), None);
        member.current_shares = 3;

        let result = find_active_members_no_shares(&[member]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_active_member_no_shares() {
        let mut member = make_member(1, Uuid::new_v4(), None);
        member.current_shares = 0;

        let result = find_active_members_no_shares(&[member]);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_exited_member_no_shares_not_flagged() {
        let mut member = make_member(1, Uuid::new_v4(), None);
        member.current_shares = 0;
        member.exit_date = Some(Date::from_calendar_date(2025, Month::June, 1).unwrap());

        let result = find_active_members_no_shares(&[member]);
        assert!(result.is_empty());
    }

    // === Duplicate member numbers tests ===

    #[test]
    fn test_no_duplicates() {
        let members = vec![
            make_member(1, Uuid::new_v4(), None),
            make_member(2, Uuid::new_v4(), None),
        ];
        let result = find_duplicate_member_numbers(&members);
        assert!(result.is_empty());
    }

    #[test]
    fn test_duplicate_numbers() {
        let id_a = Uuid::new_v4();
        let id_b = Uuid::new_v4();
        let members = vec![
            make_member(42, id_a, None),
            make_member(42, id_b, None),
        ];
        let result = find_duplicate_member_numbers(&members);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].member_number, 42);
        assert_eq!(result[0].member_ids.len(), 2);
    }

    #[test]
    fn test_soft_deleted_duplicate_not_flagged() {
        let deleted_at = time::PrimitiveDateTime::new(
            Date::from_calendar_date(2025, Month::June, 1).unwrap(),
            Time::MIDNIGHT,
        );
        let members = vec![
            make_member(42, Uuid::new_v4(), None),
            make_member(42, Uuid::new_v4(), Some(deleted_at)),
        ];
        let result = find_duplicate_member_numbers(&members);
        assert!(result.is_empty());
    }

    // === Exited members with shares tests ===

    #[test]
    fn test_exited_no_shares() {
        let mut member = make_member(1, Uuid::new_v4(), None);
        member.exit_date = Some(Date::from_calendar_date(2025, Month::June, 1).unwrap());
        member.current_shares = 0;

        let result = find_exited_members_with_shares(&[member]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_exited_with_shares() {
        let mut member = make_member(1, Uuid::new_v4(), None);
        member.exit_date = Some(Date::from_calendar_date(2025, Month::June, 1).unwrap());
        member.current_shares = 3;

        let result = find_exited_members_with_shares(&[member]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].current_shares, 3);
    }

    #[test]
    fn test_active_with_shares_not_flagged() {
        let mut member = make_member(1, Uuid::new_v4(), None);
        member.current_shares = 3;

        let result = find_exited_members_with_shares(&[member]);
        assert!(result.is_empty());
    }

    // === Migrated flag mismatch tests ===

    #[test]
    fn test_migrated_flag_matches() {
        let id = Uuid::new_v4();
        let mut member = make_member(1, id, None);
        member.current_shares = 3;
        member.action_count = 0;
        member.migrated = true;

        let actions = vec![
            make_action(id, ActionType::Eintritt, 0),
            make_action(id, ActionType::Aufstockung, 3),
        ];

        let result = find_migrated_flag_mismatches(&[member], &actions);
        assert!(result.is_empty());
    }

    #[test]
    fn test_migrated_flag_true_but_pending() {
        let id = Uuid::new_v4();
        let mut member = make_member(1, id, None);
        member.current_shares = 5;
        member.action_count = 0;
        member.migrated = true;

        let actions = vec![
            make_action(id, ActionType::Eintritt, 0),
            make_action(id, ActionType::Aufstockung, 3),
        ];

        let result = find_migrated_flag_mismatches(&[member], &actions);
        assert_eq!(result.len(), 1);
        assert!(result[0].flag_value);
        assert_eq!(&*result[0].computed_status, "Pending");
    }

    #[test]
    fn test_migrated_flag_false_but_migrated() {
        let id = Uuid::new_v4();
        let mut member = make_member(1, id, None);
        member.current_shares = 3;
        member.action_count = 0;
        member.migrated = false;

        let actions = vec![
            make_action(id, ActionType::Eintritt, 0),
            make_action(id, ActionType::Aufstockung, 3),
        ];

        let result = find_migrated_flag_mismatches(&[member], &actions);
        assert_eq!(result.len(), 1);
        assert!(!result[0].flag_value);
        assert_eq!(&*result[0].computed_status, "Migrated");
    }
}
