use async_trait::async_trait;
use genossi_dao::audit_log::AuditLogDao;
use genossi_dao::member::MemberDao;
use genossi_dao::member_action::MemberActionDao;
use genossi_dao::TransactionDao;
use genossi_service::member::{Member, MemberService};
use genossi_service::member_action::MigrationState;
use genossi_service::permission::{Authentication, PermissionService, ADMIN_PRIVILEGE};
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
        AuditLogDao: AuditLogDao<Transaction = Self::Transaction> = audit_log_dao,
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

    async fn list_transfer_recipients(
        &self,
        exclude_member_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[Member]>, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        // Permission funnel: ADMIN_PRIVILEGE (Vorstand-only, D-14-11).
        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context)
            .await?;

        let members: Arc<[Member]> = self
            .member_dao
            .all(tx.clone())
            .await?
            .iter()
            .filter(|e| e.exit_date.is_none() && e.id != exclude_member_id)
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

        let user_id = self
            .permission_service
            .current_user_id(context.clone())
            .await?
            .unwrap_or_else(|| "SYSTEM".to_string());

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
            salutation: item.salutation.clone(),
            title: item.title.clone(),
            email: item.email.clone(),
            company: item.company.clone(),
            comment: item.comment.clone(),
            street: item.street.clone(),
            house_number: item.house_number.clone(),
            postal_code: item.postal_code.clone(),
            city: item.city.clone(),
            join_date: item.join_date,
            shares_at_joining: item.shares_at_joining,
            current_shares: if item.status.is_normal() {
                item.shares_at_joining
            } else {
                0
            },
            current_balance: 0,
            action_count: 0,
            migrated: false,
            exit_date: item.exit_date,
            bank_account: item.bank_account.clone(),
            status: item.status.clone(),
            created,
            deleted: None,
            version: self.uuid_service.new_v4().await,
        };

        let member_entity: genossi_dao::member::MemberEntity = (&new_member).into();
        crate::audited_create!(
            self,
            self.member_dao,
            &member_entity,
            MEMBER_SERVICE_PROCESS,
            &user_id,
            tx
        );

        if item.status.is_normal() {
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
            crate::audited_create!(
                self,
                self.member_action_dao,
                &eintritt,
                MEMBER_SERVICE_PROCESS,
                &user_id,
                tx
            );

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
            crate::audited_create!(
                self,
                self.member_action_dao,
                &aufstockung,
                MEMBER_SERVICE_PROCESS,
                &user_id,
                tx
            );
        }

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

        let user_id = self
            .permission_service
            .current_user_id(context.clone())
            .await?
            .unwrap_or_else(|| "SYSTEM".to_string());

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

        let member_entity: genossi_dao::member::MemberEntity = item.into();
        crate::audited_update!(
            self,
            self.member_dao,
            item.id,
            &member_entity,
            MEMBER_SERVICE_PROCESS,
            &user_id,
            tx
        );

        self.recalc_migrated(item.id, tx.clone()).await?;

        // Re-read to get the new version UUID generated by the DAO
        let updated = self
            .member_dao
            .find_by_id(item.id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(item.id))?;

        self.transaction_dao.commit(tx).await?;
        Ok(Member::from(&updated))
    }

    async fn delete(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<(), ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        let user_id = self
            .permission_service
            .current_user_id(context.clone())
            .await?
            .unwrap_or_else(|| "SYSTEM".to_string());

        self.permission_service
            .check_permission(MANAGE_MEMBERS_PRIVILEGE, context)
            .await?;

        crate::audited_delete!(
            self,
            self.member_dao,
            id,
            MEMBER_SERVICE_PROCESS,
            &user_id,
            tx
        );

        self.transaction_dao.commit(tx).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    //! Service-level tests for `list_transfer_recipients` (TRSF-06).
    //!
    //! Mockall pitfall (Pitfall 2 RESEARCH.md): `#[automock]` overrides the
    //! `MemberDao::all` default-impl, so each test MUST explicitly call
    //! `.expect_all().returning(...)` — the default-impl via `dump_all` is
    //! IGNORED by the generated mock.
    use super::*;
    use async_trait::async_trait;
    use genossi_dao::audit_log::{AuditLogEntry, AuditQueryFilter};
    use genossi_dao::member::{MemberEntity, MemberStatus, Salutation};
    use genossi_dao::member_action::MemberActionEntity;
    use genossi_dao::{DaoError, Transaction};
    use genossi_service::permission::MockContext;
    use mockall::mock;

    /// Test-local Transaction with Debug — `MockTransaction` from genossi_dao
    /// does not implement Debug (gen_service_impl! requires it).
    #[derive(Clone, Debug)]
    pub struct TestTransaction;

    #[async_trait]
    impl Transaction for TestTransaction {
        async fn begin(&mut self) -> Result<(), DaoError> {
            Ok(())
        }
        async fn commit(self) -> Result<(), DaoError> {
            Ok(())
        }
        async fn rollback(self) -> Result<(), DaoError> {
            Ok(())
        }
    }

    mock! {
        pub TestTxDao {}
        #[async_trait]
        impl TransactionDao for TestTxDao {
            type Transaction = TestTransaction;
            async fn transaction(&self) -> Result<TestTransaction, DaoError>;
            async fn use_transaction(
                &self,
                tx: Option<TestTransaction>,
            ) -> Result<TestTransaction, DaoError>;
            async fn commit(&self, tx: TestTransaction) -> Result<(), DaoError>;
        }
    }

    mock! {
        pub TestMemberDao {}
        #[async_trait]
        impl MemberDao for TestMemberDao {
            type Transaction = TestTransaction;
            async fn dump_all(&self, tx: TestTransaction) -> Result<Arc<[MemberEntity]>, DaoError>;
            async fn create(&self, entity: &MemberEntity, process: &str, tx: TestTransaction) -> Result<(), DaoError>;
            async fn update(&self, entity: &MemberEntity, process: &str, tx: TestTransaction) -> Result<(), DaoError>;
            async fn all(&self, tx: TestTransaction) -> Result<Arc<[MemberEntity]>, DaoError>;
            async fn find_by_id(&self, id: Uuid, tx: TestTransaction) -> Result<Option<MemberEntity>, DaoError>;
            async fn update_migrated(&self, id: Uuid, migrated: bool, tx: TestTransaction) -> Result<(), DaoError>;
            async fn update_dates(
                &self,
                id: Uuid,
                join_date: time::Date,
                exit_date: Option<time::Date>,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn find_by_member_number(
                &self,
                member_number: i64,
                tx: TestTransaction,
            ) -> Result<Option<MemberEntity>, DaoError>;
            async fn count_active(&self, today: time::Date, tx: TestTransaction) -> Result<u64, DaoError>;
            async fn next_member_number(&self, tx: TestTransaction) -> Result<i64, DaoError>;
        }
    }

    mock! {
        pub TestMemberActionDao {}
        #[async_trait]
        impl MemberActionDao for TestMemberActionDao {
            type Transaction = TestTransaction;
            async fn dump_all(&self, tx: TestTransaction) -> Result<Arc<[MemberActionEntity]>, DaoError>;
            async fn create(&self, entity: &MemberActionEntity, process: &str, tx: TestTransaction) -> Result<(), DaoError>;
            async fn update(&self, entity: &MemberActionEntity, process: &str, tx: TestTransaction) -> Result<(), DaoError>;
            async fn all(&self, tx: TestTransaction) -> Result<Arc<[MemberActionEntity]>, DaoError>;
            async fn find_by_id(&self, id: Uuid, tx: TestTransaction) -> Result<Option<MemberActionEntity>, DaoError>;
            async fn find_by_member_id(
                &self,
                member_id: Uuid,
                tx: TestTransaction,
            ) -> Result<Arc<[MemberActionEntity]>, DaoError>;
        }
    }

    mock! {
        pub TestAuditLogDao {}
        #[async_trait]
        impl AuditLogDao for TestAuditLogDao {
            type Transaction = TestTransaction;
            async fn create_entries(
                &self,
                entries: &[AuditLogEntry],
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn get_latest_hash(&self, tx: TestTransaction) -> Result<Option<String>, DaoError>;
            async fn get_by_entity(
                &self,
                entity_type: &str,
                entity_id: Uuid,
                tx: TestTransaction,
            ) -> Result<Arc<[AuditLogEntry]>, DaoError>;
            async fn get_all_ordered(
                &self,
                tx: TestTransaction,
            ) -> Result<Arc<[AuditLogEntry]>, DaoError>;
            async fn query(
                &self,
                filter: AuditQueryFilter,
                limit: i64,
                offset: i64,
                tx: TestTransaction,
            ) -> Result<Arc<[AuditLogEntry]>, DaoError>;
            async fn count(
                &self,
                filter: AuditQueryFilter,
                tx: TestTransaction,
            ) -> Result<i64, DaoError>;
        }
    }

    mock! {
        pub TestPermissionService {}
        #[async_trait]
        impl PermissionService for TestPermissionService {
            type Context = MockContext;
            async fn check_permission(
                &self,
                privilege: &str,
                context: Authentication<MockContext>,
            ) -> Result<(), ServiceError>;
            async fn current_user_id(
                &self,
                context: Authentication<MockContext>,
            ) -> Result<Option<String>, ServiceError>;
            async fn get_all_users(
                &self,
                context: Authentication<MockContext>,
            ) -> Result<Arc<[genossi_service::auth_types::UserResponseTO]>, ServiceError>;
            async fn create_user(
                &self,
                user: genossi_service::auth_types::UserTO,
                context: Authentication<MockContext>,
            ) -> Result<(), ServiceError>;
            async fn delete_user(
                &self,
                username: String,
                context: Authentication<MockContext>,
            ) -> Result<(), ServiceError>;
            async fn get_all_roles(
                &self,
                context: Authentication<MockContext>,
            ) -> Result<Arc<[genossi_service::auth_types::RoleResponseTO]>, ServiceError>;
            async fn create_role(
                &self,
                role: genossi_service::auth_types::RoleTO,
                context: Authentication<MockContext>,
            ) -> Result<(), ServiceError>;
            async fn delete_role(
                &self,
                role_name: String,
                context: Authentication<MockContext>,
            ) -> Result<(), ServiceError>;
            async fn get_all_privileges(
                &self,
                context: Authentication<MockContext>,
            ) -> Result<Arc<[genossi_service::auth_types::PrivilegeResponseTO]>, ServiceError>;
            async fn create_privilege(
                &self,
                privilege: genossi_service::auth_types::PrivilegeTO,
                context: Authentication<MockContext>,
            ) -> Result<(), ServiceError>;
            async fn delete_privilege(
                &self,
                privilege_name: String,
                context: Authentication<MockContext>,
            ) -> Result<(), ServiceError>;
            async fn assign_user_role(
                &self,
                user_role: genossi_service::auth_types::UserRole,
                context: Authentication<MockContext>,
            ) -> Result<(), ServiceError>;
            async fn remove_user_role(
                &self,
                user_role: genossi_service::auth_types::UserRole,
                context: Authentication<MockContext>,
            ) -> Result<(), ServiceError>;
            async fn get_user_roles(
                &self,
                username: String,
                context: Authentication<MockContext>,
            ) -> Result<Arc<[genossi_service::auth_types::RoleResponseTO]>, ServiceError>;
            async fn assign_role_privilege(
                &self,
                role_privilege: genossi_service::auth_types::RolePrivilege,
                context: Authentication<MockContext>,
            ) -> Result<(), ServiceError>;
            async fn remove_role_privilege(
                &self,
                role_privilege: genossi_service::auth_types::RolePrivilege,
                context: Authentication<MockContext>,
            ) -> Result<(), ServiceError>;
            async fn get_role_privileges(
                &self,
                role_name: String,
                context: Authentication<MockContext>,
            ) -> Result<Arc<[genossi_service::auth_types::PrivilegeResponseTO]>, ServiceError>;
            async fn get_user_privileges(
                &self,
                username: String,
                context: Authentication<MockContext>,
            ) -> Result<Arc<[genossi_service::auth_types::PrivilegeResponseTO]>, ServiceError>;
            async fn has_claims(&self, context: &MockContext) -> Result<bool, ServiceError>;
        }
    }

    #[derive(Clone)]
    struct StaticUuidService;
    #[async_trait]
    impl UuidService for StaticUuidService {
        async fn new_v4(&self) -> Uuid {
            Uuid::new_v4()
        }
    }

    struct TestDeps;
    impl MemberServiceDeps for TestDeps {
        type Context = MockContext;
        type Transaction = TestTransaction;
        type MemberDao = MockTestMemberDao;
        type MemberActionDao = MockTestMemberActionDao;
        type AuditLogDao = MockTestAuditLogDao;
        type PermissionService = MockTestPermissionService;
        type UuidService = StaticUuidService;
        type TransactionDao = MockTestTxDao;
    }

    fn setup_mock_tx_dao() -> MockTestTxDao {
        let mut tx_dao = MockTestTxDao::new();
        tx_dao
            .expect_use_transaction()
            .returning(|_| Ok(TestTransaction));
        tx_dao.expect_commit().returning(|_| Ok(()));
        tx_dao
    }

    fn sample_member_entity(id: Uuid, exit_date: Option<time::Date>) -> MemberEntity {
        let join = time::Date::from_calendar_date(2020, time::Month::January, 1).unwrap();
        MemberEntity {
            id,
            member_number: 1,
            first_name: Arc::from("Test"),
            last_name: Arc::from("Member"),
            salutation: Some(Salutation::Herr),
            title: None,
            email: None,
            company: None,
            comment: None,
            street: None,
            house_number: None,
            postal_code: None,
            city: None,
            join_date: join,
            shares_at_joining: 1,
            current_shares: 1,
            current_balance: 0,
            action_count: 0,
            migrated: false,
            exit_date,
            bank_account: None,
            status: MemberStatus::Normal,
            account_holder: None,
            created: time::PrimitiveDateTime::new(join, time::Time::MIDNIGHT),
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    fn build_service(
        member_dao: MockTestMemberDao,
        permission_service: MockTestPermissionService,
    ) -> MemberServiceImpl<TestDeps> {
        MemberServiceImpl {
            member_dao: Arc::new(member_dao),
            member_action_dao: Arc::new(MockTestMemberActionDao::new()),
            audit_log_dao: Arc::new(MockTestAuditLogDao::new()),
            permission_service: Arc::new(permission_service),
            uuid_service: Arc::new(StaticUuidService),
            transaction_dao: Arc::new(setup_mock_tx_dao()),
        }
    }

    #[tokio::test]
    async fn test_list_transfer_recipients_happy_path_filters_self() {
        // Setup: 3 active members; exclude_self = m_a → returns [m_b, m_c].
        let m_a_id = Uuid::new_v4();
        let m_b_id = Uuid::new_v4();
        let m_c_id = Uuid::new_v4();
        let entities = vec![
            sample_member_entity(m_a_id, None),
            sample_member_entity(m_b_id, None),
            sample_member_entity(m_c_id, None),
        ];

        let mut member_dao = MockTestMemberDao::new();
        let entities_clone = entities.clone();
        // Mockall pitfall guard (Pitfall 2): default-impl `all()` is overridden;
        // explicit .expect_all() is mandatory.
        member_dao
            .expect_all()
            .returning(move |_| Ok(Arc::from(entities_clone.clone())));

        let mut permission_service = MockTestPermissionService::new();
        // Admin-gate witness: assert ADMIN_PRIVILEGE is the privilege passed.
        permission_service
            .expect_check_permission()
            .withf(|priv_, _ctx| priv_ == "admin")
            .returning(|_, _| Ok(()));

        let service = build_service(member_dao, permission_service);

        let result = service
            .list_transfer_recipients(m_a_id, Authentication::Full, None)
            .await
            .expect("list_transfer_recipients should succeed");

        assert_eq!(result.len(), 2, "self must be excluded");
        let ids: Vec<Uuid> = result.iter().map(|m| m.id).collect();
        assert!(ids.contains(&m_b_id), "m_b must be present");
        assert!(ids.contains(&m_c_id), "m_c must be present");
        assert!(!ids.contains(&m_a_id), "self (m_a) must be filtered out");
    }

    #[tokio::test]
    async fn test_list_transfer_recipients_all_cancelled_returns_empty() {
        // Setup: 3 members all with exit_date = Some(...) → returns [].
        let m_a_id = Uuid::new_v4();
        let m_b_id = Uuid::new_v4();
        let m_c_id = Uuid::new_v4();
        let exit = time::Date::from_calendar_date(2026, time::Month::June, 30).unwrap();
        let entities = vec![
            sample_member_entity(m_a_id, Some(exit)),
            sample_member_entity(m_b_id, Some(exit)),
            sample_member_entity(m_c_id, Some(exit)),
        ];

        let mut member_dao = MockTestMemberDao::new();
        let entities_clone = entities.clone();
        member_dao
            .expect_all()
            .returning(move |_| Ok(Arc::from(entities_clone.clone())));

        let mut permission_service = MockTestPermissionService::new();
        permission_service
            .expect_check_permission()
            .returning(|_, _| Ok(()));

        let service = build_service(member_dao, permission_service);

        let result = service
            .list_transfer_recipients(m_a_id, Authentication::Full, None)
            .await
            .expect("list_transfer_recipients should succeed");

        assert_eq!(
            result.len(),
            0,
            "all cancelled members must be filtered out"
        );
    }

    #[tokio::test]
    async fn test_list_transfer_recipients_only_self_returns_empty() {
        // Setup: 1 active member m_a; exclude_self = m_a → returns [].
        let m_a_id = Uuid::new_v4();
        let entities = vec![sample_member_entity(m_a_id, None)];

        let mut member_dao = MockTestMemberDao::new();
        let entities_clone = entities.clone();
        member_dao
            .expect_all()
            .returning(move |_| Ok(Arc::from(entities_clone.clone())));

        let mut permission_service = MockTestPermissionService::new();
        permission_service
            .expect_check_permission()
            .returning(|_, _| Ok(()));

        let service = build_service(member_dao, permission_service);

        let result = service
            .list_transfer_recipients(m_a_id, Authentication::Full, None)
            .await
            .expect("list_transfer_recipients should succeed");

        assert_eq!(result.len(), 0, "only-self setup must return empty list");
    }
}
