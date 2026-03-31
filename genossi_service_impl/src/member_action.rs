use async_trait::async_trait;
use genossi_dao::member::MemberDao;
use genossi_dao::member_action::{ActionType, MemberActionDao};
use genossi_dao::TransactionDao;
use genossi_service::member_action::{
    MemberAction, MemberActionService, MigrationState, MigrationStatus,
};
use genossi_service::permission::{Authentication, PermissionService};
use genossi_service::uuid_service::UuidService;
use genossi_service::{ServiceError, ValidationFailureItem};
use std::sync::Arc;
use uuid::Uuid;

use crate::gen_service_impl;

const MEMBER_ACTION_SERVICE_PROCESS: &str = "member-action-service";
const VIEW_MEMBERS_PRIVILEGE: &str = "view_members";
const MANAGE_MEMBERS_PRIVILEGE: &str = "manage_members";

gen_service_impl! {
    struct MemberActionServiceImpl: MemberActionService = MemberActionServiceDeps {
        MemberActionDao: MemberActionDao<Transaction = Self::Transaction> = member_action_dao,
        MemberDao: MemberDao<Transaction = Self::Transaction> = member_dao,
        PermissionService: PermissionService<Context = Self::Context> = permission_service,
        UuidService: UuidService = uuid_service,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
    }
}

fn validate_action(item: &MemberAction) -> Vec<ValidationFailureItem> {
    let mut errors = Vec::new();

    match item.action_type {
        ActionType::Eintritt | ActionType::Austritt | ActionType::Todesfall => {
            if item.shares_change != 0 {
                errors.push(ValidationFailureItem {
                    field: Arc::from("shares_change"),
                    message: Arc::from("Status actions must have shares_change = 0"),
                });
            }
        }
        ActionType::Aufstockung | ActionType::UebertragungEmpfang => {
            if item.shares_change <= 0 {
                errors.push(ValidationFailureItem {
                    field: Arc::from("shares_change"),
                    message: Arc::from("shares_change must be positive for this action type"),
                });
            }
        }
        ActionType::Verkauf | ActionType::UebertragungAbgabe => {
            if item.shares_change >= 0 {
                errors.push(ValidationFailureItem {
                    field: Arc::from("shares_change"),
                    message: Arc::from("shares_change must be negative for this action type"),
                });
            }
        }
    }

    match item.action_type {
        ActionType::UebertragungEmpfang | ActionType::UebertragungAbgabe => {
            if item.transfer_member_id.is_none() {
                errors.push(ValidationFailureItem {
                    field: Arc::from("transfer_member_id"),
                    message: Arc::from("transfer_member_id is required for transfer actions"),
                });
            }
        }
        _ => {}
    }

    if item.effective_date.is_some() && item.action_type != ActionType::Austritt {
        errors.push(ValidationFailureItem {
            field: Arc::from("effective_date"),
            message: Arc::from("effective_date is only allowed for Austritt actions"),
        });
    }

    errors
}

#[async_trait]
impl<Deps: MemberActionServiceDeps> MemberActionService for MemberActionServiceImpl<Deps> {
    type Context = Deps::Context;
    type Transaction = Deps::Transaction;

    async fn get_by_member(
        &self,
        member_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[MemberAction]>, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        self.permission_service
            .check_permission(VIEW_MEMBERS_PRIVILEGE, context)
            .await?;

        // Verify member exists
        self.member_dao
            .find_by_id(member_id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(member_id))?;

        let actions = self
            .member_action_dao
            .find_by_member_id(member_id, tx.clone())
            .await?
            .iter()
            .map(MemberAction::from)
            .collect();

        self.transaction_dao.commit(tx).await?;
        Ok(actions)
    }

    async fn get(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<MemberAction, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        self.permission_service
            .check_permission(VIEW_MEMBERS_PRIVILEGE, context)
            .await?;

        let action = self
            .member_action_dao
            .find_by_id(id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(id))?;

        self.transaction_dao.commit(tx).await?;
        Ok(MemberAction::from(&action))
    }

    async fn create(
        &self,
        item: &MemberAction,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<MemberAction, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        self.permission_service
            .check_permission(MANAGE_MEMBERS_PRIVILEGE, context)
            .await?;

        let validation_errors = validate_action(item);
        if !validation_errors.is_empty() {
            return Err(ServiceError::ValidationError(validation_errors));
        }

        // Verify member exists
        self.member_dao
            .find_by_id(item.member_id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(item.member_id))?;

        // Verify transfer member exists if set
        if let Some(transfer_id) = item.transfer_member_id {
            self.member_dao
                .find_by_id(transfer_id, tx.clone())
                .await?
                .ok_or(ServiceError::EntityNotFound(transfer_id))?;
        }

        let now = time::OffsetDateTime::now_utc();
        let new_action = MemberAction {
            id: self.uuid_service.new_v4().await,
            member_id: item.member_id,
            action_type: item.action_type.clone(),
            date: item.date,
            shares_change: item.shares_change,
            transfer_member_id: item.transfer_member_id,
            effective_date: item.effective_date,
            comment: item.comment.clone(),
            created: time::PrimitiveDateTime::new(now.date(), now.time()),
            deleted: None,
            version: self.uuid_service.new_v4().await,
        };

        self.member_action_dao
            .create(
                &(&new_action).into(),
                MEMBER_ACTION_SERVICE_PROCESS,
                tx.clone(),
            )
            .await?;

        self.transaction_dao.commit(tx).await?;
        Ok(new_action)
    }

    async fn update(
        &self,
        item: &MemberAction,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<MemberAction, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        self.permission_service
            .check_permission(MANAGE_MEMBERS_PRIVILEGE, context)
            .await?;

        let validation_errors = validate_action(item);
        if !validation_errors.is_empty() {
            return Err(ServiceError::ValidationError(validation_errors));
        }

        self.member_action_dao
            .update(&item.into(), MEMBER_ACTION_SERVICE_PROCESS, tx.clone())
            .await?;

        self.transaction_dao.commit(tx).await?;
        Ok(item.clone())
    }

    async fn delete(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<(), ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        self.permission_service
            .check_permission(MANAGE_MEMBERS_PRIVILEGE, context)
            .await?;

        let existing = self.member_action_dao.find_by_id(id, tx.clone()).await?;

        match existing {
            Some(mut entity) => {
                let now = time::OffsetDateTime::now_utc();
                entity.deleted = Some(time::PrimitiveDateTime::new(now.date(), now.time()));
                self.member_action_dao
                    .update(&entity, MEMBER_ACTION_SERVICE_PROCESS, tx.clone())
                    .await?;
                self.transaction_dao.commit(tx).await?;
                Ok(())
            }
            None => Err(ServiceError::EntityNotFound(id)),
        }
    }

    async fn migration_status(
        &self,
        member_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<MigrationStatus, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        self.permission_service
            .check_permission(VIEW_MEMBERS_PRIVILEGE, context)
            .await?;

        let member = self
            .member_dao
            .find_by_id(member_id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(member_id))?;

        let actions = self
            .member_action_dao
            .find_by_member_id(member_id, tx.clone())
            .await?;

        let actual_shares: i32 = actions.iter().map(|a| a.shares_change).sum();

        // Count non-Eintritt share actions (Aufstockung, Verkauf, Uebertragung*)
        let actual_action_count = actions
            .iter()
            .filter(|a| {
                !matches!(
                    a.action_type,
                    ActionType::Eintritt | ActionType::Austritt | ActionType::Todesfall
                )
            })
            .count() as i32;

        let expected_shares = member.current_shares;
        // action_count from Excel doesn't include the initial Aufstockung,
        // but our system counts all non-Eintritt share actions, so add 1.
        let expected_action_count = member.action_count + 1;

        let status = if actual_shares == expected_shares
            && actual_action_count == expected_action_count
        {
            MigrationState::Migrated
        } else {
            MigrationState::Pending
        };

        self.transaction_dao.commit(tx).await?;

        Ok(MigrationStatus {
            member_id,
            status,
            expected_shares,
            actual_shares,
            expected_action_count,
            actual_action_count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_eintritt_with_nonzero_shares() {
        let action = MemberAction {
            id: Uuid::new_v4(),
            member_id: Uuid::new_v4(),
            action_type: ActionType::Eintritt,
            date: time::Date::from_calendar_date(2024, time::Month::January, 1).unwrap(),
            shares_change: 5,
            transfer_member_id: None,
            effective_date: None,
            comment: None,
            created: time::PrimitiveDateTime::new(
                time::Date::from_calendar_date(2024, time::Month::January, 1).unwrap(),
                time::Time::MIDNIGHT,
            ),
            deleted: None,
            version: Uuid::new_v4(),
        };
        let errors = validate_action(&action);
        assert_eq!(errors.len(), 1);
        assert_eq!(&*errors[0].field, "shares_change");
    }

    #[test]
    fn test_validate_aufstockung_with_negative_shares() {
        let action = MemberAction {
            id: Uuid::new_v4(),
            member_id: Uuid::new_v4(),
            action_type: ActionType::Aufstockung,
            date: time::Date::from_calendar_date(2024, time::Month::January, 1).unwrap(),
            shares_change: -3,
            transfer_member_id: None,
            effective_date: None,
            comment: None,
            created: time::PrimitiveDateTime::new(
                time::Date::from_calendar_date(2024, time::Month::January, 1).unwrap(),
                time::Time::MIDNIGHT,
            ),
            deleted: None,
            version: Uuid::new_v4(),
        };
        let errors = validate_action(&action);
        assert_eq!(errors.len(), 1);
        assert_eq!(&*errors[0].field, "shares_change");
    }

    #[test]
    fn test_validate_verkauf_with_positive_shares() {
        let action = MemberAction {
            id: Uuid::new_v4(),
            member_id: Uuid::new_v4(),
            action_type: ActionType::Verkauf,
            date: time::Date::from_calendar_date(2024, time::Month::January, 1).unwrap(),
            shares_change: 3,
            transfer_member_id: None,
            effective_date: None,
            comment: None,
            created: time::PrimitiveDateTime::new(
                time::Date::from_calendar_date(2024, time::Month::January, 1).unwrap(),
                time::Time::MIDNIGHT,
            ),
            deleted: None,
            version: Uuid::new_v4(),
        };
        let errors = validate_action(&action);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn test_validate_uebertragung_without_transfer_member() {
        let action = MemberAction {
            id: Uuid::new_v4(),
            member_id: Uuid::new_v4(),
            action_type: ActionType::UebertragungEmpfang,
            date: time::Date::from_calendar_date(2024, time::Month::January, 1).unwrap(),
            shares_change: 2,
            transfer_member_id: None,
            effective_date: None,
            comment: None,
            created: time::PrimitiveDateTime::new(
                time::Date::from_calendar_date(2024, time::Month::January, 1).unwrap(),
                time::Time::MIDNIGHT,
            ),
            deleted: None,
            version: Uuid::new_v4(),
        };
        let errors = validate_action(&action);
        assert_eq!(errors.len(), 1);
        assert_eq!(&*errors[0].field, "transfer_member_id");
    }

    #[test]
    fn test_validate_effective_date_on_non_austritt() {
        let action = MemberAction {
            id: Uuid::new_v4(),
            member_id: Uuid::new_v4(),
            action_type: ActionType::Eintritt,
            date: time::Date::from_calendar_date(2024, time::Month::January, 1).unwrap(),
            shares_change: 0,
            transfer_member_id: None,
            effective_date: Some(
                time::Date::from_calendar_date(2024, time::Month::December, 31).unwrap(),
            ),
            comment: None,
            created: time::PrimitiveDateTime::new(
                time::Date::from_calendar_date(2024, time::Month::January, 1).unwrap(),
                time::Time::MIDNIGHT,
            ),
            deleted: None,
            version: Uuid::new_v4(),
        };
        let errors = validate_action(&action);
        assert_eq!(errors.len(), 1);
        assert_eq!(&*errors[0].field, "effective_date");
    }

    #[test]
    fn test_validate_valid_aufstockung() {
        let action = MemberAction {
            id: Uuid::new_v4(),
            member_id: Uuid::new_v4(),
            action_type: ActionType::Aufstockung,
            date: time::Date::from_calendar_date(2024, time::Month::January, 1).unwrap(),
            shares_change: 3,
            transfer_member_id: None,
            effective_date: None,
            comment: None,
            created: time::PrimitiveDateTime::new(
                time::Date::from_calendar_date(2024, time::Month::January, 1).unwrap(),
                time::Time::MIDNIGHT,
            ),
            deleted: None,
            version: Uuid::new_v4(),
        };
        let errors = validate_action(&action);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_validate_valid_uebertragung_abgabe() {
        let action = MemberAction {
            id: Uuid::new_v4(),
            member_id: Uuid::new_v4(),
            action_type: ActionType::UebertragungAbgabe,
            date: time::Date::from_calendar_date(2024, time::Month::January, 1).unwrap(),
            shares_change: -2,
            transfer_member_id: Some(Uuid::new_v4()),
            effective_date: None,
            comment: None,
            created: time::PrimitiveDateTime::new(
                time::Date::from_calendar_date(2024, time::Month::January, 1).unwrap(),
                time::Time::MIDNIGHT,
            ),
            deleted: None,
            version: Uuid::new_v4(),
        };
        let errors = validate_action(&action);
        assert!(errors.is_empty());
    }
}
