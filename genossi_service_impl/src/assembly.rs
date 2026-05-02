//! Service-layer implementation of the Assembly aggregate (Plan 03).
//!
//! Lifecycle: Preparation → Open → Closed (D-07/D-08/D-09). All write
//! methods route through the audit macros (`audited_create!` /
//! `audited_update!`). `open_assembly` is the only multi-DAO method —
//! a single transaction covers the status update plus the snapshot
//! population (Pitfall 2). Snapshot inserts deliberately bypass the
//! audit macros (Pitfall 1) — the snapshot is data, not a lifecycle
//! event.

use async_trait::async_trait;
use genossi_dao::assembly::{AssemblyDao, AssemblyEntity, AssemblyStatus};
use genossi_dao::assembly_member_snapshot::{
    AssemblyMemberSnapshotDao, AssemblyMemberSnapshotEntity,
};
use genossi_dao::audit_log::AuditLogDao;
use genossi_dao::member::MemberDao;
use genossi_dao::TransactionDao;
use genossi_service::assembly::{
    Assembly, AssemblyDetail, AssemblyService, AssemblySubmission, AssemblyUpdate,
};
use genossi_service::permission::{Authentication, PermissionService};
use genossi_service::uuid_service::UuidService;
use genossi_service::ServiceError;
use std::sync::Arc;
use uuid::Uuid;

use crate::gen_service_impl;

const ASSEMBLY_PROCESS_CREATE: &str = "assembly.create";
const ASSEMBLY_PROCESS_OPEN: &str = "assembly.open";
const ASSEMBLY_PROCESS_CLOSE: &str = "assembly.close";
const ASSEMBLY_PROCESS_UPDATE: &str = "assembly.update";
const ADMIN_PRIVILEGE: &str = "admin";

gen_service_impl! {
    struct AssemblyServiceImpl: AssemblyService = AssemblyServiceDeps {
        AssemblyDao: AssemblyDao<Transaction = Self::Transaction> = assembly_dao,
        AssemblyMemberSnapshotDao: AssemblyMemberSnapshotDao<Transaction = Self::Transaction> = assembly_member_snapshot_dao,
        MemberDao: MemberDao<Transaction = Self::Transaction> = member_dao,
        AuditLogDao: AuditLogDao<Transaction = Self::Transaction> = audit_log_dao,
        PermissionService: PermissionService<Context = Self::Context> = permission_service,
        UuidService: UuidService = uuid_service,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
    }
}

#[async_trait]
impl<Deps: AssemblyServiceDeps> AssemblyService for AssemblyServiceImpl<Deps> {
    type Context = Deps::Context;
    type Transaction = Deps::Transaction;

    async fn create_assembly(
        &self,
        submission: &AssemblySubmission,
        context: Authentication<Self::Context>,
    ) -> Result<Assembly, ServiceError> {
        let tx = self.transaction_dao.use_transaction(None).await?;

        let user_id = self
            .permission_service
            .current_user_id(context.clone())
            .await?
            .unwrap_or_else(|| "SYSTEM".to_string());
        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context)
            .await?;

        let now = time::OffsetDateTime::now_utc();
        let created = time::PrimitiveDateTime::new(now.date(), now.time());

        let entity = AssemblyEntity {
            id: self.uuid_service.new_v4().await,
            name: submission.name.clone(),
            date: submission.date,
            location: submission.location.clone(),
            status: AssemblyStatus::Preparation,
            opened_at: None,
            closed_at: None,
            created,
            deleted: None,
            version: self.uuid_service.new_v4().await,
        };

        crate::audited_create!(
            self,
            self.assembly_dao,
            &entity,
            ASSEMBLY_PROCESS_CREATE,
            &user_id,
            tx
        );

        self.transaction_dao.commit(tx).await?;
        Ok(Assembly::from(&entity))
    }

    async fn update_assembly(
        &self,
        id: Uuid,
        update: &AssemblyUpdate,
        context: Authentication<Self::Context>,
    ) -> Result<Assembly, ServiceError> {
        let tx = self.transaction_dao.use_transaction(None).await?;

        let user_id = self
            .permission_service
            .current_user_id(context.clone())
            .await?
            .unwrap_or_else(|| "SYSTEM".to_string());
        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context)
            .await?;

        let mut entity = self
            .assembly_dao
            .find_by_id(id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(id))?;

        // D-07: only Preparation is editable.
        if entity.status != AssemblyStatus::Preparation {
            return Err(ServiceError::Conflict(Arc::from(format!(
                "Cannot update assembly: status is '{}', expected 'Preparation' (D-07)",
                entity.status.as_str()
            ))));
        }
        // Optimistic locking — version must match.
        if entity.version != update.version {
            return Err(ServiceError::Conflict(Arc::from("Version mismatch")));
        }

        entity.name = update.name.clone();
        entity.date = update.date;
        entity.location = update.location.clone();

        crate::audited_update!(
            self,
            self.assembly_dao,
            id,
            &entity,
            ASSEMBLY_PROCESS_UPDATE,
            &user_id,
            tx
        );

        self.transaction_dao.commit(tx).await?;
        Ok(Assembly::from(&entity))
    }

    async fn open_assembly(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
    ) -> Result<Assembly, ServiceError> {
        // Pitfall 2: ONE transaction, ONE commit at the end. tx.clone() for sub-calls.
        let tx = self.transaction_dao.use_transaction(None).await?;

        let user_id = self
            .permission_service
            .current_user_id(context.clone())
            .await?
            .unwrap_or_else(|| "SYSTEM".to_string());
        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context)
            .await?;

        let mut entity = self
            .assembly_dao
            .find_by_id(id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(id))?;

        // Pitfall 3: state-transition guard.
        if entity.status != AssemblyStatus::Preparation {
            return Err(ServiceError::Conflict(Arc::from(format!(
                "Cannot open assembly: status is '{}', expected 'Preparation'",
                entity.status.as_str()
            ))));
        }

        let now_offset = time::OffsetDateTime::now_utc();
        let now_pdt = time::PrimitiveDateTime::new(now_offset.date(), now_offset.time());
        let opened_date = now_offset.date();
        entity.status = AssemblyStatus::Open;
        entity.opened_at = Some(now_pdt);

        crate::audited_update!(
            self,
            self.assembly_dao,
            id,
            &entity,
            ASSEMBLY_PROCESS_OPEN,
            &user_id,
            tx
        );

        // D-02: count_active filter — identical logic to genossi_dao/src/member.rs:172-185.
        // member_dao.all() already filters deleted IS NULL.
        let all_members = self.member_dao.all(tx.clone()).await?;
        let snapshot_entities: Vec<AssemblyMemberSnapshotEntity> = all_members
            .iter()
            .filter(|m| m.status.is_normal())
            .filter(|m| m.exit_date.map_or(true, |d| d > opened_date))
            .map(|m| AssemblyMemberSnapshotEntity {
                assembly_id: id,
                member_id: m.id,
                captured_at: now_pdt,
            })
            .collect();

        // Pitfall 1: snapshot inserts deliberately bypass audit macros — the snapshot
        // is data, not a lifecycle event. The act of opening is audited above.
        self.assembly_member_snapshot_dao
            .create_batch(&snapshot_entities, ASSEMBLY_PROCESS_OPEN, tx.clone())
            .await?;

        self.transaction_dao.commit(tx).await?;
        Ok(Assembly::from(&entity))
    }

    async fn close_assembly(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
    ) -> Result<Assembly, ServiceError> {
        let tx = self.transaction_dao.use_transaction(None).await?;

        let user_id = self
            .permission_service
            .current_user_id(context.clone())
            .await?
            .unwrap_or_else(|| "SYSTEM".to_string());
        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context)
            .await?;

        let mut entity = self
            .assembly_dao
            .find_by_id(id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(id))?;

        if entity.status != AssemblyStatus::Open {
            return Err(ServiceError::Conflict(Arc::from(format!(
                "Cannot close assembly: status is '{}', expected 'Open'",
                entity.status.as_str()
            ))));
        }

        let now_offset = time::OffsetDateTime::now_utc();
        let now_pdt = time::PrimitiveDateTime::new(now_offset.date(), now_offset.time());
        entity.status = AssemblyStatus::Closed;
        entity.closed_at = Some(now_pdt);

        crate::audited_update!(
            self,
            self.assembly_dao,
            id,
            &entity,
            ASSEMBLY_PROCESS_CLOSE,
            &user_id,
            tx
        );

        // D-09: NO HelperSession cascade in Phase 1. Phase 3 will extend.

        self.transaction_dao.commit(tx).await?;
        Ok(Assembly::from(&entity))
    }

    async fn get_assembly(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
    ) -> Result<AssemblyDetail, ServiceError> {
        let tx = self.transaction_dao.use_transaction(None).await?;
        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context)
            .await?;

        let entity = self
            .assembly_dao
            .find_by_id(id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(id))?;
        let snapshot_member_count = self
            .assembly_member_snapshot_dao
            .count_by_assembly_id(id, tx.clone())
            .await?;

        self.transaction_dao.commit(tx).await?;
        Ok(AssemblyDetail {
            assembly: Assembly::from(&entity),
            snapshot_member_count,
        })
    }

    async fn get_all_assemblies(
        &self,
        context: Authentication<Self::Context>,
    ) -> Result<Arc<[Assembly]>, ServiceError> {
        let tx = self.transaction_dao.use_transaction(None).await?;
        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context)
            .await?;

        let entities = self.assembly_dao.all(tx.clone()).await?;
        let assemblies: Arc<[Assembly]> = entities.iter().map(Assembly::from).collect();

        self.transaction_dao.commit(tx).await?;
        Ok(assemblies)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use genossi_dao::audit_log::{AuditLogEntry, AuditQueryFilter};
    use genossi_dao::auditable::Auditable;
    use genossi_dao::member::{MemberEntity, MemberStatus, Salutation};
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

    // Local mocks bound to TestTransaction. genossi_dao::Mock*Dao types
    // hardcode `Transaction = MockTransaction` via #[automock] so we cannot
    // re-target them; we re-roll the mocks here against TestTransaction.

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
        pub TestAssemblyDao {}
        #[async_trait]
        impl AssemblyDao for TestAssemblyDao {
            type Transaction = TestTransaction;
            async fn dump_all(&self, tx: TestTransaction) -> Result<Arc<[AssemblyEntity]>, DaoError>;
            async fn create(&self, entity: &AssemblyEntity, process: &str, tx: TestTransaction) -> Result<(), DaoError>;
            async fn update(&self, entity: &AssemblyEntity, process: &str, tx: TestTransaction) -> Result<(), DaoError>;
            async fn all(&self, tx: TestTransaction) -> Result<Arc<[AssemblyEntity]>, DaoError>;
            async fn find_by_id(&self, id: Uuid, tx: TestTransaction) -> Result<Option<AssemblyEntity>, DaoError>;
        }
    }

    mock! {
        pub TestSnapshotDao {}
        #[async_trait]
        impl AssemblyMemberSnapshotDao for TestSnapshotDao {
            type Transaction = TestTransaction;
            async fn create(
                &self,
                entity: &AssemblyMemberSnapshotEntity,
                process: &str,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn create_batch(
                &self,
                entities: &[AssemblyMemberSnapshotEntity],
                process: &str,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn find_by_assembly_id(
                &self,
                assembly_id: Uuid,
                tx: TestTransaction,
            ) -> Result<Arc<[AssemblyMemberSnapshotEntity]>, DaoError>;
            async fn count_by_assembly_id(
                &self,
                assembly_id: Uuid,
                tx: TestTransaction,
            ) -> Result<u64, DaoError>;
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

    /// TestDeps wires the local mocks as associated types.
    struct TestDeps;
    impl AssemblyServiceDeps for TestDeps {
        type Context = MockContext;
        type Transaction = TestTransaction;
        type AssemblyDao = MockTestAssemblyDao;
        type AssemblyMemberSnapshotDao = MockTestSnapshotDao;
        type MemberDao = MockTestMemberDao;
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

    fn make_permission_service_admin_ok() -> MockTestPermissionService {
        let mut p = MockTestPermissionService::new();
        p.expect_current_user_id()
            .returning(|_| Ok(Some("admin-user".to_string())));
        p.expect_check_permission().returning(|_, _| Ok(()));
        p
    }

    fn make_audit_log_dao_quiet() -> MockTestAuditLogDao {
        let mut dao = MockTestAuditLogDao::new();
        dao.expect_get_latest_hash().returning(|_| Ok(None));
        dao.expect_create_entries().returning(|_, _| Ok(()));
        dao
    }

    /// Silence unused-import warning when Auditable isn't needed in every test.
    fn _force_auditable_compile() {
        fn _check<T: Auditable>(_: &T) {}
    }

    fn assembly_in_status(status: AssemblyStatus) -> AssemblyEntity {
        let date = time::Date::from_calendar_date(2026, time::Month::May, 15).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        AssemblyEntity {
            id: Uuid::new_v4(),
            name: Arc::from("GV 2026"),
            date: datetime,
            location: Some(Arc::from("Vereinsheim")),
            status,
            opened_at: None,
            closed_at: None,
            created: datetime,
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    fn make_member(status: MemberStatus, exit_in_future: bool) -> MemberEntity {
        let join = time::Date::from_calendar_date(2020, time::Month::January, 1).unwrap();
        let now = time::OffsetDateTime::now_utc().date();
        let exit_date = if exit_in_future {
            // Future exit: still active.
            Some(now.checked_add(time::Duration::days(365)).unwrap_or(now))
        } else {
            // Past exit: inactive.
            Some(now.checked_sub(time::Duration::days(1)).unwrap_or(now))
        };
        MemberEntity {
            id: Uuid::new_v4(),
            member_number: 42,
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
            status,
            created: time::PrimitiveDateTime::new(join, time::Time::MIDNIGHT),
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    fn build_service(
        assembly_dao: MockTestAssemblyDao,
        snapshot_dao: MockTestSnapshotDao,
        member_dao: MockTestMemberDao,
    ) -> AssemblyServiceImpl<TestDeps> {
        AssemblyServiceImpl {
            assembly_dao: Arc::new(assembly_dao),
            assembly_member_snapshot_dao: Arc::new(snapshot_dao),
            member_dao: Arc::new(member_dao),
            audit_log_dao: Arc::new(make_audit_log_dao_quiet()),
            permission_service: Arc::new(make_permission_service_admin_ok()),
            uuid_service: Arc::new(StaticUuidService),
            transaction_dao: Arc::new(setup_mock_tx_dao()),
        }
    }

    #[tokio::test]
    async fn test_create_assembly_success() {
        let mut assembly_dao = MockTestAssemblyDao::new();
        assembly_dao
            .expect_create()
            .times(1)
            .returning(|_, _, _| Ok(()));

        let snapshot_dao = MockTestSnapshotDao::new();
        let member_dao = MockTestMemberDao::new();

        let service = build_service(assembly_dao, snapshot_dao, member_dao);

        let date = time::Date::from_calendar_date(2026, time::Month::May, 15).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        let submission = AssemblySubmission {
            name: Arc::from("GV 2026"),
            date: datetime,
            location: Some(Arc::from("Vereinsheim")),
        };

        let result = service
            .create_assembly(&submission, Authentication::Full)
            .await
            .expect("create_assembly should succeed");

        assert_eq!(result.status, AssemblyStatus::Preparation);
        assert_eq!(&*result.name, "GV 2026");
        assert!(result.opened_at.is_none());
        assert!(result.closed_at.is_none());
    }

    #[tokio::test]
    async fn test_open_assembly_from_preparation_succeeds_atomic() {
        let entity = assembly_in_status(AssemblyStatus::Preparation);
        let entity_id = entity.id;
        let entity_for_find = entity.clone();

        let mut assembly_dao = MockTestAssemblyDao::new();
        // find_by_id is called from open_assembly itself AND from audited_update!
        // (which loads the old entity to diff against). Both calls return the same
        // pre-update entity — the macro performs the DAO update and writes audit
        // entries for changed fields.
        assembly_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(entity_for_find.clone())));
        assembly_dao
            .expect_update()
            .times(1)
            .returning(|_, _, _| Ok(()));

        // Two active members → batch insert with EXACTLY 2 entities.
        let active_a = make_member(MemberStatus::Normal, true);
        let active_b = MemberEntity {
            id: Uuid::new_v4(),
            member_number: 43,
            ..make_member(MemberStatus::Normal, true)
        };
        // exit_date == None → still active.
        let active_c = MemberEntity {
            id: Uuid::new_v4(),
            member_number: 44,
            exit_date: None,
            ..make_member(MemberStatus::Normal, true)
        };
        let _ = active_c; // we intentionally do not include the third here

        let mut member_dao = MockTestMemberDao::new();
        let members = vec![active_a, active_b];
        member_dao
            .expect_all()
            .returning(move |_| Ok(Arc::from(members.clone())));

        let mut snapshot_dao = MockTestSnapshotDao::new();
        snapshot_dao
            .expect_create_batch()
            .times(1)
            .withf(|entities: &[AssemblyMemberSnapshotEntity], _process, _tx| entities.len() == 2)
            .returning(|_, _, _| Ok(()));

        let service = build_service(assembly_dao, snapshot_dao, member_dao);

        let result = service
            .open_assembly(entity_id, Authentication::Full)
            .await
            .expect("open_assembly should succeed");

        assert_eq!(result.status, AssemblyStatus::Open);
        assert!(result.opened_at.is_some());
    }

    #[tokio::test]
    async fn test_open_assembly_from_closed_returns_conflict() {
        let entity = assembly_in_status(AssemblyStatus::Closed);
        let entity_id = entity.id;

        let mut assembly_dao = MockTestAssemblyDao::new();
        assembly_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(entity.clone())));

        // Update / batch / member calls must NOT be made — the service short-circuits.
        let snapshot_dao = MockTestSnapshotDao::new();
        let member_dao = MockTestMemberDao::new();

        let service = build_service(assembly_dao, snapshot_dao, member_dao);

        let result = service.open_assembly(entity_id, Authentication::Full).await;

        match result {
            Err(ServiceError::Conflict(_)) => {}
            other => panic!("expected ServiceError::Conflict, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_close_assembly_from_preparation_returns_conflict() {
        let entity = assembly_in_status(AssemblyStatus::Preparation);
        let entity_id = entity.id;

        let mut assembly_dao = MockTestAssemblyDao::new();
        assembly_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(entity.clone())));

        let snapshot_dao = MockTestSnapshotDao::new();
        let member_dao = MockTestMemberDao::new();

        let service = build_service(assembly_dao, snapshot_dao, member_dao);

        let result = service
            .close_assembly(entity_id, Authentication::Full)
            .await;
        match result {
            Err(ServiceError::Conflict(_)) => {}
            other => panic!("expected ServiceError::Conflict, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_update_assembly_version_mismatch_returns_conflict() {
        let entity = assembly_in_status(AssemblyStatus::Preparation);
        let entity_id = entity.id;
        let stale_version = Uuid::new_v4();
        // Confirm the stale_version differs from entity.version (extremely unlikely
        // to collide, but guard against a flaky test).
        assert_ne!(stale_version, entity.version);

        let mut assembly_dao = MockTestAssemblyDao::new();
        assembly_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(entity.clone())));

        let snapshot_dao = MockTestSnapshotDao::new();
        let member_dao = MockTestMemberDao::new();

        let service = build_service(assembly_dao, snapshot_dao, member_dao);

        let date = time::Date::from_calendar_date(2026, time::Month::May, 15).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        let update = AssemblyUpdate {
            name: Arc::from("renamed"),
            date: datetime,
            location: None,
            version: stale_version,
        };

        let result = service
            .update_assembly(entity_id, &update, Authentication::Full)
            .await;
        match result {
            Err(ServiceError::Conflict(msg)) => {
                assert!(
                    msg.contains("Version mismatch"),
                    "expected 'Version mismatch' in message, got: {}",
                    msg
                );
            }
            other => panic!("expected Conflict, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_open_assembly_filters_inactive_members() {
        // Pitfall 6: count_active filter — three members but only one matches:
        //   a) Normal + future exit → INCLUDED
        //   b) FehlerhaftErfasst + future exit → EXCLUDED (status filter)
        //   c) Normal + past exit → EXCLUDED (exit_date filter)
        let entity = assembly_in_status(AssemblyStatus::Preparation);
        let entity_id = entity.id;
        let entity_for_find = entity.clone();

        let mut assembly_dao = MockTestAssemblyDao::new();
        assembly_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(entity_for_find.clone())));
        assembly_dao.expect_update().returning(|_, _, _| Ok(()));

        let active_normal = make_member(MemberStatus::Normal, true);
        let inactive_status = MemberEntity {
            id: Uuid::new_v4(),
            member_number: 50,
            ..make_member(MemberStatus::FehlerhaftErfasst, true)
        };
        let exited_member = MemberEntity {
            id: Uuid::new_v4(),
            member_number: 51,
            ..make_member(MemberStatus::Normal, false)
        };

        let mut member_dao = MockTestMemberDao::new();
        let members = vec![active_normal, inactive_status, exited_member];
        member_dao
            .expect_all()
            .returning(move |_| Ok(Arc::from(members.clone())));

        let mut snapshot_dao = MockTestSnapshotDao::new();
        snapshot_dao
            .expect_create_batch()
            .times(1)
            .withf(|entities: &[AssemblyMemberSnapshotEntity], _process, _tx| entities.len() == 1)
            .returning(|_, _, _| Ok(()));

        let service = build_service(assembly_dao, snapshot_dao, member_dao);

        let result = service
            .open_assembly(entity_id, Authentication::Full)
            .await
            .expect("open_assembly should succeed");

        assert_eq!(result.status, AssemblyStatus::Open);
    }
}
