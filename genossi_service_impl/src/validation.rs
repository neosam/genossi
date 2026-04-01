use async_trait::async_trait;
use genossi_dao::member::MemberDao;
use genossi_dao::member_action::{ActionType, MemberActionDao};
use genossi_dao::TransactionDao;
use genossi_service::permission::{Authentication, PermissionService};
use genossi_service::validation::{UnmatchedTransfer, ValidationResult, ValidationService};
use genossi_service::ServiceError;
use std::collections::HashSet;
use std::sync::Arc;
use uuid::Uuid;

use crate::gen_service_impl;

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
    let member_number_map: std::collections::HashMap<Uuid, i64> = members
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

        Ok(ValidationResult {
            member_number_gaps,
            unmatched_transfers,
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
            make_transfer(
                Uuid::new_v4(),
                member_a,
                ActionType::UebertragungAbgabe,
                member_b,
                -3,
                date,
            ),
            make_transfer(
                Uuid::new_v4(),
                member_b,
                ActionType::UebertragungEmpfang,
                member_a,
                3,
                date,
            ),
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
            Uuid::new_v4(),
            member_a,
            ActionType::UebertragungAbgabe,
            member_b,
            -3,
            date,
        )];

        let unmatched = find_unmatched_transfers(&actions, &members);
        assert_eq!(unmatched.len(), 1);
        assert_eq!(unmatched[0].member_id, member_a);
    }

    #[test]
    fn test_shares_mismatch() {
        let member_a = Uuid::new_v4();
        let member_b = Uuid::new_v4();
        let date = Date::from_calendar_date(2024, Month::May, 1).unwrap();

        let members = vec![
            make_member(1, member_a, None),
            make_member(2, member_b, None),
        ];

        let actions = vec![
            make_transfer(
                Uuid::new_v4(),
                member_a,
                ActionType::UebertragungAbgabe,
                member_b,
                -3,
                date,
            ),
            make_transfer(
                Uuid::new_v4(),
                member_b,
                ActionType::UebertragungEmpfang,
                member_a,
                2, // mismatch: should be 3
                date,
            ),
        ];

        let unmatched = find_unmatched_transfers(&actions, &members);
        assert_eq!(unmatched.len(), 2);
    }

    #[test]
    fn test_date_mismatch() {
        let member_a = Uuid::new_v4();
        let member_b = Uuid::new_v4();
        let date1 = Date::from_calendar_date(2024, Month::May, 1).unwrap();
        let date2 = Date::from_calendar_date(2024, Month::June, 1).unwrap();

        let members = vec![
            make_member(1, member_a, None),
            make_member(2, member_b, None),
        ];

        let actions = vec![
            make_transfer(
                Uuid::new_v4(),
                member_a,
                ActionType::UebertragungAbgabe,
                member_b,
                -3,
                date1,
            ),
            make_transfer(
                Uuid::new_v4(),
                member_b,
                ActionType::UebertragungEmpfang,
                member_a,
                3,
                date2, // different date
            ),
        ];

        let unmatched = find_unmatched_transfers(&actions, &members);
        assert_eq!(unmatched.len(), 2);
    }

    #[test]
    fn test_soft_deleted_actions_ignored() {
        // find_unmatched_transfers expects only active actions (filtered by .all())
        // so soft-deleted actions should not be in the input
        let member_a = Uuid::new_v4();
        let member_b = Uuid::new_v4();
        let date = Date::from_calendar_date(2024, Month::May, 1).unwrap();

        let members = vec![
            make_member(1, member_a, None),
            make_member(2, member_b, None),
        ];

        // Only the active action (no counterpart)
        let actions = vec![make_transfer(
            Uuid::new_v4(),
            member_a,
            ActionType::UebertragungAbgabe,
            member_b,
            -3,
            date,
        )];

        let unmatched = find_unmatched_transfers(&actions, &members);
        assert_eq!(unmatched.len(), 1);
    }

    #[test]
    fn test_no_transfers() {
        let members = vec![make_member(1, Uuid::new_v4(), None)];
        let actions: Vec<MemberActionEntity> = vec![];
        let unmatched = find_unmatched_transfers(&actions, &members);
        assert!(unmatched.is_empty());
    }
}
