use async_trait::async_trait;
use genossi_dao::member::MemberDao;
use genossi_dao::member_action::MemberActionDao;
use genossi_dao::TransactionDao;
use genossi_service::member::{Member, MemberService};
use genossi_service::member_action::MigrationState;
use genossi_service::permission::{Authentication, PermissionService};
use genossi_service::uuid_service::UuidService;
use genossi_service::{ServiceError, ValidationFailureItem};
use std::sync::Arc;
use uuid::Uuid;

use crate::gen_service_impl;
use crate::member_action::{compute_dates, compute_migration_status};

const MEMBER_SERVICE_PROCESS: &str = "member-service";
const VIEW_MEMBERS_PRIVILEGE: &str = "view_members";
const MANAGE_MEMBERS_PRIVILEGE: &str = "manage_members";

gen_service_impl! {
    struct MemberServiceImpl: MemberService = MemberServiceDeps {
        MemberDao: MemberDao<Transaction = Self::Transaction> = member_dao,
        MemberActionDao: MemberActionDao<Transaction = Self::Transaction> = member_action_dao,
        PermissionService: PermissionService<Context = Self::Context> = permission_service,
        UuidService: UuidService = uuid_service,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
    }
}

impl<Deps: MemberServiceDeps> MemberServiceImpl<Deps> {
    async fn recalc_dates(
        &self,
        member_id: Uuid,
        tx: Deps::Transaction,
    ) -> Result<(), ServiceError> {
        let member = self
            .member_dao
            .find_by_id(member_id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(member_id))?;

        let actions = self
            .member_action_dao
            .find_by_member_id(member_id, tx.clone())
            .await?;

        let (join_date, exit_date) = compute_dates(&member, &actions);

        self.member_dao
            .update_dates(member_id, join_date, exit_date, tx)
            .await?;

        Ok(())
    }

    async fn recalc_migrated(
        &self,
        member_id: Uuid,
        tx: Deps::Transaction,
    ) -> Result<(), ServiceError> {
        let member = self
            .member_dao
            .find_by_id(member_id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(member_id))?;

        let actions = self
            .member_action_dao
            .find_by_member_id(member_id, tx.clone())
            .await?;

        let status = compute_migration_status(&member, &actions);
        let migrated = status.status == MigrationState::Migrated;

        self.member_dao
            .update_migrated(member_id, migrated, tx)
            .await?;

        Ok(())
    }
}

#[async_trait]
impl<Deps: MemberServiceDeps> MemberService for MemberServiceImpl<Deps> {
    type Context = Deps::Context;
    type Transaction = Deps::Transaction;

    async fn get_all(
        &self,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[Member]>, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        self.permission_service
            .check_permission(VIEW_MEMBERS_PRIVILEGE, context)
            .await?;

        let members = self
            .member_dao
            .all(tx.clone())
            .await?
            .iter()
            .map(Member::from)
            .collect();

        self.transaction_dao.commit(tx).await?;
        Ok(members)
    }

    async fn get(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Member, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        self.permission_service
            .check_permission(VIEW_MEMBERS_PRIVILEGE, context)
            .await?;

        let member = self
            .member_dao
            .find_by_id(id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(id))?;

        self.transaction_dao.commit(tx).await?;
        Ok(Member::from(&member))
    }

    async fn create(
        &self,
        item: &Member,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Member, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        self.permission_service
            .check_permission(MANAGE_MEMBERS_PRIVILEGE, context)
            .await?;

        let mut validation_errors = Vec::new();
        if item.first_name.is_empty() {
            validation_errors.push(ValidationFailureItem {
                field: Arc::from("first_name"),
                message: Arc::from("First name cannot be empty"),
            });
        }
        if item.last_name.is_empty() {
            validation_errors.push(ValidationFailureItem {
                field: Arc::from("last_name"),
                message: Arc::from("Last name cannot be empty"),
            });
        }
        if item.member_number < 0 {
            validation_errors.push(ValidationFailureItem {
                field: Arc::from("member_number"),
                message: Arc::from("Member number must not be negative"),
            });
        }

        if !validation_errors.is_empty() {
            return Err(ServiceError::ValidationError(validation_errors));
        }

        // Auto-assign member number if 0
        let member_number = if item.member_number == 0 {
            self.member_dao.next_member_number(tx.clone()).await?
        } else {
            // Check uniqueness of explicit member_number
            if self
                .member_dao
                .find_by_member_number(item.member_number, tx.clone())
                .await?
                .is_some()
            {
                return Err(ServiceError::ValidationError(vec![ValidationFailureItem {
                    field: Arc::from("member_number"),
                    message: Arc::from("Member number already exists"),
                }]));
            }
            item.member_number
        };

        let now = time::OffsetDateTime::now_utc();
        let created = time::PrimitiveDateTime::new(now.date(), now.time());
        let new_member = Member {
            id: self.uuid_service.new_v4().await,
            member_number,
            first_name: item.first_name.clone(),
            last_name: item.last_name.clone(),
            email: item.email.clone(),
            company: item.company.clone(),
            comment: item.comment.clone(),
            street: item.street.clone(),
            house_number: item.house_number.clone(),
            postal_code: item.postal_code.clone(),
            city: item.city.clone(),
            join_date: item.join_date,
            shares_at_joining: item.shares_at_joining,
            current_shares: item.shares_at_joining,
            current_balance: 0,
            action_count: 0,
            migrated: false,
            exit_date: item.exit_date,
            bank_account: item.bank_account.clone(),
            created,
            deleted: None,
            version: self.uuid_service.new_v4().await,
        };

        self.member_dao
            .create(&(&new_member).into(), MEMBER_SERVICE_PROCESS, tx.clone())
            .await?;

        // Create Eintritt action
        let eintritt = genossi_dao::member_action::MemberActionEntity {
            id: self.uuid_service.new_v4().await,
            member_id: new_member.id,
            action_type: genossi_dao::member_action::ActionType::Eintritt,
            date: item.join_date,
            shares_change: 0,
            transfer_member_id: None,
            effective_date: None,
            comment: None,
            created,
            deleted: None,
            version: self.uuid_service.new_v4().await,
        };
        self.member_action_dao
            .create(&eintritt, MEMBER_SERVICE_PROCESS, tx.clone())
            .await?;

        // Create Aufstockung action
        let aufstockung = genossi_dao::member_action::MemberActionEntity {
            id: self.uuid_service.new_v4().await,
            member_id: new_member.id,
            action_type: genossi_dao::member_action::ActionType::Aufstockung,
            date: item.join_date,
            shares_change: item.shares_at_joining,
            transfer_member_id: None,
            effective_date: None,
            comment: None,
            created,
            deleted: None,
            version: self.uuid_service.new_v4().await,
        };
        self.member_action_dao
            .create(&aufstockung, MEMBER_SERVICE_PROCESS, tx.clone())
            .await?;

        self.recalc_dates(new_member.id, tx.clone()).await?;
        self.recalc_migrated(new_member.id, tx.clone()).await?;

        self.transaction_dao.commit(tx).await?;
        Ok(new_member)
    }

    async fn update(
        &self,
        item: &Member,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Member, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        self.permission_service
            .check_permission(MANAGE_MEMBERS_PRIVILEGE, context)
            .await?;

        let mut validation_errors = Vec::new();
        if item.first_name.is_empty() {
            validation_errors.push(ValidationFailureItem {
                field: Arc::from("first_name"),
                message: Arc::from("First name cannot be empty"),
            });
        }
        if item.last_name.is_empty() {
            validation_errors.push(ValidationFailureItem {
                field: Arc::from("last_name"),
                message: Arc::from("Last name cannot be empty"),
            });
        }
        if !validation_errors.is_empty() {
            return Err(ServiceError::ValidationError(validation_errors));
        }

        self.member_dao
            .update(&item.into(), MEMBER_SERVICE_PROCESS, tx.clone())
            .await?;

        self.recalc_migrated(item.id, tx.clone()).await?;

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

        let existing = self.member_dao.find_by_id(id, tx.clone()).await?;

        match existing {
            Some(mut entity) => {
                let now = time::OffsetDateTime::now_utc();
                entity.deleted = Some(time::PrimitiveDateTime::new(now.date(), now.time()));
                self.member_dao
                    .update(&entity, MEMBER_SERVICE_PROCESS, tx.clone())
                    .await?;
                self.transaction_dao.commit(tx).await?;
                Ok(())
            }
            None => Err(ServiceError::EntityNotFound(id)),
        }
    }
}
