//! Service-layer implementation of the Assembly aggregate (Plan 03).
//!
//! Lifecycle: Preparation → Open → Closed (D-07/D-08/D-09). All write
//! methods route through the audit macros (`audited_create!` /
//! `audited_update!`). `open_assembly` is the only multi-DAO method —
//! a single transaction covers the status update plus the snapshot
//! population (Pitfall 2). Snapshot inserts deliberately bypass the
//! audit macros (Pitfall 1) — the snapshot is data, not a lifecycle
//! event.
//!
//! WR-07: `AssemblyEntity` carries a `deleted: Option<PrimitiveDateTime>`
//! field per project convention, and `AssemblyDao::all` filters
//! `deleted IS NULL`. Phase 1 deliberately implements **no delete path**:
//! there is no REST endpoint, no service method, and no `audited_delete!`
//! call that ever sets the field. The schema column is reserved for a
//! future Phase 2/3 soft-delete that must:
//!   1. add an `audited_delete!` invocation here so the deletion is
//!      recorded in the audit hash chain, and
//!   2. expose a DELETE handler in `genossi_rest/src/assembly.rs` that
//!      enforces lifecycle constraints (e.g. only delete from
//!      Preparation, never from Closed).
//! Do NOT remove the `deleted` field "because it is unused" — that would
//! force a fresh migration in Phase 2/3.

use async_trait::async_trait;
use genossi_dao::assembly::{AssemblyDao, AssemblyEntity, AssemblyStatus};
use genossi_dao::assembly_member_snapshot::{
    AssemblyMemberSnapshotDao, AssemblyMemberSnapshotEntity,
};
use genossi_dao::audit_log::AuditLogDao;
use genossi_dao::helper_token::HelperTokenDao;
use genossi_dao::member::MemberDao;
use genossi_dao::permission::PermissionDao;
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
        // Phase 3 Plan 05 (D-12, D-16): cascade-discovery for close_assembly.
        HelperTokenDao: HelperTokenDao<Transaction = Self::Transaction> = helper_token_dao,
        // Phase 3 Plan 05: cascade calls delete_session on PermissionDao
        // (pool-based, NO tx — Conflict-2 resolution: commit BEFORE loop).
        PermissionDao: PermissionDao<Transaction = Self::Transaction> = permission_dao,
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

        // WR-04: this find_by_id duplicates the load that `audited_update!`
        // performs internally to compute the OLD entity for the audit diff.
        // We accept the duplicate read here on purpose: the service-level
        // load is required to enforce the state-transition guard (D-07) and
        // optimistic-locking version check BEFORE we mutate `entity`. Both
        // reads run inside the same transaction (`tx.clone()`), so they see
        // the same committed snapshot. Do NOT collapse this into a single
        // load that bypasses `audited_update!` -- that would break the
        // audit trail.
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

        // WR-04: see update_assembly comment above. The duplicate read against
        // `audited_update!` is intentional and required for the state-guard.
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

        // D-02: count_active filter — identical logic to genossi_dao/src/member.rs:172-185,
        // *plus* an additional `join_date <= opened_date` guard. The member_dao.count_active
        // helper does not filter on join_date, but for assembly snapshots the GV-protocol
        // semantics demand that members whose membership starts in the future (e.g. newly
        // captured with a join_date 6 months ahead) are excluded from the attendance
        // baseline -- they have no voting rights at the time the assembly opens
        // (Verbandskonformitaet, Phase 1 constraint).
        // member_dao.all() already filters deleted IS NULL.
        let all_members = self.member_dao.all(tx.clone()).await?;
        let snapshot_entities: Vec<AssemblyMemberSnapshotEntity> = all_members
            .iter()
            .filter(|m| m.status.is_normal())
            .filter(|m| m.join_date <= opened_date)
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

        // WR-04: see update_assembly comment above. The duplicate read against
        // `audited_update!` is intentional and required for the state-guard.
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

        // Phase 3 Plan 05 cascade extension (D-11, D-12, D-13, D-15).
        //
        // 1) Discover all bound helper-session ids INSIDE the still-open tx
        //    so we read the same snapshot as the audited_update! above.
        let session_ids = self
            .helper_token_dao
            .list_session_ids_for_assembly(id, tx.clone())
            .await?;

        // 2) RESEARCH §DECISION CONFLICT 2 — commit BEFORE the pool-based
        //    PermissionDao::delete_session calls. delete_session takes no
        //    `tx` argument (genossi_dao/src/permission.rs:90) and acquires
        //    its own pool connection; keeping an open BEGIN while a parallel
        //    pool acquire is requested deadlocks the sqlite pool. The same
        //    caveat is documented in helper_token.rs:316-325.
        self.transaction_dao.commit(tx).await?;

        // 3) D-13/D-14: Continue-on-Error. The status=Closed audit-entry is
        //    already committed; failed session-DELETEs are caught by the
        //    Phase-2-D-18 verify_user_session status-check (defense-in-depth
        //    — closed assemblies reject helper requests downstream). Each
        //    failure logs a WARN line for operator visibility.
        for sid in session_ids.iter() {
            if let Err(e) = self.permission_dao.delete_session(sid.as_ref()).await {
                tracing::warn!(
                    error = ?e,
                    session_id = %sid.as_ref(),
                    assembly_id = %id,
                    "cascade delete_session failed; defense-in-depth via verify_user_session-Status-Check active"
                );
            }
        }

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
        pub TestHelperTokenDao {}
        #[async_trait]
        impl HelperTokenDao for TestHelperTokenDao {
            type Transaction = TestTransaction;
            async fn dump_all(&self, tx: TestTransaction) -> Result<Arc<[genossi_dao::helper_token::HelperTokenEntity]>, DaoError>;
            async fn create(
                &self,
                entity: &genossi_dao::helper_token::HelperTokenEntity,
                process: &str,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn update(
                &self,
                entity: &genossi_dao::helper_token::HelperTokenEntity,
                process: &str,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn all(
                &self,
                tx: TestTransaction,
            ) -> Result<Arc<[genossi_dao::helper_token::HelperTokenEntity]>, DaoError>;
            async fn find_by_id(
                &self,
                id: Uuid,
                tx: TestTransaction,
            ) -> Result<Option<genossi_dao::helper_token::HelperTokenEntity>, DaoError>;
            async fn atomic_redeem(
                &self,
                token_hash: &str,
                used_at: time::PrimitiveDateTime,
                tx: TestTransaction,
            ) -> Result<Option<(Uuid, Uuid)>, DaoError>;
            async fn set_session_id(
                &self,
                token_id: Uuid,
                session_id: &str,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn lookup_status(
                &self,
                token_hash: &str,
                tx: TestTransaction,
            ) -> Result<
                Option<(Option<time::PrimitiveDateTime>, Option<time::PrimitiveDateTime>)>,
                DaoError,
            >;
            async fn all_for_assembly(
                &self,
                assembly_id: Uuid,
                tx: TestTransaction,
            ) -> Result<Arc<[genossi_dao::helper_token::HelperTokenEntity]>, DaoError>;
            async fn list_session_ids_for_assembly(
                &self,
                assembly_id: Uuid,
                tx: TestTransaction,
            ) -> Result<Vec<Arc<str>>, DaoError>;
        }
    }

    mock! {
        pub TestPermissionDao {}
        #[async_trait]
        impl PermissionDao for TestPermissionDao {
            type Transaction = TestTransaction;
            async fn has_privilege(&self, user: &str, privilege: &str) -> Result<bool, DaoError>;
            async fn all_users(&self) -> Result<Arc<[genossi_dao::permission::UserEntity]>, DaoError>;
            async fn get_user(&self, name: &str) -> Result<Option<genossi_dao::permission::UserEntity>, DaoError>;
            async fn create_user(&self, user: &genossi_dao::permission::UserEntity, process: &str) -> Result<(), DaoError>;
            async fn delete_user(&self, username: &str) -> Result<(), DaoError>;
            async fn ensure_user_exists(&self, username: &str, process: &str) -> Result<bool, DaoError>;
            async fn all_roles(&self) -> Result<Arc<[genossi_dao::permission::RoleEntity]>, DaoError>;
            async fn get_role(&self, name: &str) -> Result<Option<genossi_dao::permission::RoleEntity>, DaoError>;
            async fn create_role(&self, role: &genossi_dao::permission::RoleEntity, process: &str) -> Result<(), DaoError>;
            async fn delete_role(&self, role_name: &str) -> Result<(), DaoError>;
            async fn all_privileges(&self) -> Result<Arc<[genossi_dao::permission::PrivilegeEntity]>, DaoError>;
            async fn get_privilege(&self, name: &str) -> Result<Option<genossi_dao::permission::PrivilegeEntity>, DaoError>;
            async fn create_privilege(&self, privilege: &genossi_dao::permission::PrivilegeEntity, process: &str) -> Result<(), DaoError>;
            async fn delete_privilege(&self, privilege_name: &str) -> Result<(), DaoError>;
            async fn add_user_role(&self, username: &str, role: &str, process: &str) -> Result<(), DaoError>;
            async fn remove_user_role(&self, username: &str, role: &str) -> Result<(), DaoError>;
            async fn get_user_roles(&self, username: &str) -> Result<Arc<[genossi_dao::permission::RoleEntity]>, DaoError>;
            async fn add_role_privilege(&self, role_name: &str, privilege_name: &str, process: &str) -> Result<(), DaoError>;
            async fn remove_role_privilege(&self, role_name: &str, privilege_name: &str) -> Result<(), DaoError>;
            async fn get_role_privileges(&self, role_name: &str) -> Result<Arc<[genossi_dao::permission::PrivilegeEntity]>, DaoError>;
            async fn get_user_privileges(&self, username: &str) -> Result<Arc<[genossi_dao::permission::PrivilegeEntity]>, DaoError>;
            async fn create_session(&self, session: &genossi_dao::permission::SessionEntity) -> Result<(), DaoError>;
            async fn get_session(&self, session_id: &str) -> Result<Option<genossi_dao::permission::SessionEntity>, DaoError>;
            async fn delete_session(&self, session_id: &str) -> Result<(), DaoError>;
            async fn cleanup_expired_sessions(&self, before_timestamp: i64) -> Result<(), DaoError>;
            async fn touch_session(&self, session_id: &str, now: i64) -> Result<(), DaoError>;
            async fn delete_sessions_for_user(&self, user_id: &str) -> Result<u64, DaoError>;
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
        // Phase 3 Plan 05 cascade additions:
        type HelperTokenDao = MockTestHelperTokenDao;
        type PermissionDao = MockTestPermissionDao;
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
        // Phase 3 Plan 05: cascade Mocks default to no-op — existing
        // Phase-1 tests do not exercise close_assembly's cascade path
        // (they hit Conflict short-circuit before list_session_ids_for_assembly
        // is reached). Tests that DO exercise the cascade use
        // `build_service_with_cascade` below.
        AssemblyServiceImpl {
            assembly_dao: Arc::new(assembly_dao),
            assembly_member_snapshot_dao: Arc::new(snapshot_dao),
            member_dao: Arc::new(member_dao),
            audit_log_dao: Arc::new(make_audit_log_dao_quiet()),
            permission_service: Arc::new(make_permission_service_admin_ok()),
            uuid_service: Arc::new(StaticUuidService),
            transaction_dao: Arc::new(setup_mock_tx_dao()),
            helper_token_dao: Arc::new(MockTestHelperTokenDao::new()),
            permission_dao: Arc::new(MockTestPermissionDao::new()),
        }
    }

    /// Cascade-aware service builder for close_assembly tests (Phase 3
    /// Plan 05). Lets the caller wire HelperTokenDao and PermissionDao
    /// expectations explicitly while keeping all other deps at the
    /// build_service defaults.
    #[allow(clippy::too_many_arguments)]
    fn build_service_with_cascade(
        assembly_dao: MockTestAssemblyDao,
        snapshot_dao: MockTestSnapshotDao,
        member_dao: MockTestMemberDao,
        helper_token_dao: MockTestHelperTokenDao,
        permission_dao: MockTestPermissionDao,
    ) -> AssemblyServiceImpl<TestDeps> {
        AssemblyServiceImpl {
            assembly_dao: Arc::new(assembly_dao),
            assembly_member_snapshot_dao: Arc::new(snapshot_dao),
            member_dao: Arc::new(member_dao),
            audit_log_dao: Arc::new(make_audit_log_dao_quiet()),
            permission_service: Arc::new(make_permission_service_admin_ok()),
            uuid_service: Arc::new(StaticUuidService),
            transaction_dao: Arc::new(setup_mock_tx_dao()),
            helper_token_dao: Arc::new(helper_token_dao),
            permission_dao: Arc::new(permission_dao),
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

    #[tokio::test]
    async fn test_get_assembly_returns_snapshot_member_count() {
        // WR-06: get_assembly must surface the snapshot count from
        // count_by_assembly_id verbatim. The number is what downstream
        // consumers (Frontend, GV-Protokoll export) will trust as the
        // attendance baseline -- a quietly-zero return here would silently
        // break the protocol.
        let entity = assembly_in_status(AssemblyStatus::Open);
        let entity_id = entity.id;

        let mut assembly_dao = MockTestAssemblyDao::new();
        assembly_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(entity.clone())));

        let mut snapshot_dao = MockTestSnapshotDao::new();
        snapshot_dao
            .expect_count_by_assembly_id()
            .times(1)
            .returning(|_, _| Ok(7));

        let member_dao = MockTestMemberDao::new();
        let service = build_service(assembly_dao, snapshot_dao, member_dao);

        let detail = service
            .get_assembly(entity_id, Authentication::Full)
            .await
            .expect("get_assembly should succeed");

        assert_eq!(detail.snapshot_member_count, 7);
        assert_eq!(detail.assembly.id, entity_id);
    }

    #[tokio::test]
    async fn test_open_assembly_excludes_future_joiner_from_snapshot() {
        // WR-02: snapshot must NOT include members whose join_date is in the future
        // (e.g. newly captured members scheduled to join after the GV opens). They
        // have no voting rights at the moment of opening, so they must be excluded
        // from the attendance baseline used for the GV-Protokoll.
        let entity = assembly_in_status(AssemblyStatus::Preparation);
        let entity_id = entity.id;
        let entity_for_find = entity.clone();

        let mut assembly_dao = MockTestAssemblyDao::new();
        assembly_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(entity_for_find.clone())));
        assembly_dao.expect_update().returning(|_, _, _| Ok(()));

        // a) Past join_date → INCLUDED
        let active_normal = make_member(MemberStatus::Normal, true);
        // b) Future join_date → EXCLUDED (would otherwise pass status + exit_date filters)
        let now = time::OffsetDateTime::now_utc().date();
        let future_join = now
            .checked_add(time::Duration::days(180))
            .expect("future date should be representable");
        let future_joiner = MemberEntity {
            id: Uuid::new_v4(),
            member_number: 60,
            join_date: future_join,
            ..make_member(MemberStatus::Normal, true)
        };

        let mut member_dao = MockTestMemberDao::new();
        let members = vec![active_normal, future_joiner];
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

    // ------------------------------------------------------------------
    // Phase 3 Plan 05 — close_assembly cascade tests (D-11..D-15).
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_close_assembly_cascades_to_all_helper_sessions() {
        // D-11/D-12: cascade must call delete_session for every session id
        // returned by list_session_ids_for_assembly. The audited_update
        // (status=Closed) must still happen, and the assembly is returned
        // with status==Closed.
        let entity = assembly_in_status(AssemblyStatus::Open);
        let entity_id = entity.id;
        let entity_for_find = entity.clone();

        let mut assembly_dao = MockTestAssemblyDao::new();
        assembly_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(entity_for_find.clone())));
        assembly_dao
            .expect_update()
            .times(1)
            .returning(|_, _, _| Ok(()));

        let mut helper_token_dao = MockTestHelperTokenDao::new();
        helper_token_dao
            .expect_list_session_ids_for_assembly()
            .with(mockall::predicate::eq(entity_id), mockall::predicate::always())
            .times(1)
            .returning(|_, _| {
                Ok(vec![
                    Arc::from("s1"),
                    Arc::from("s2"),
                    Arc::from("s3"),
                ])
            });

        let mut permission_dao = MockTestPermissionDao::new();
        permission_dao
            .expect_delete_session()
            .with(mockall::predicate::eq("s1"))
            .times(1)
            .returning(|_| Ok(()));
        permission_dao
            .expect_delete_session()
            .with(mockall::predicate::eq("s2"))
            .times(1)
            .returning(|_| Ok(()));
        permission_dao
            .expect_delete_session()
            .with(mockall::predicate::eq("s3"))
            .times(1)
            .returning(|_| Ok(()));

        let service = build_service_with_cascade(
            assembly_dao,
            MockTestSnapshotDao::new(),
            MockTestMemberDao::new(),
            helper_token_dao,
            permission_dao,
        );

        let result = service
            .close_assembly(entity_id, Authentication::Full)
            .await
            .expect("close_assembly should succeed");
        assert_eq!(result.status, AssemblyStatus::Closed);
    }

    #[tokio::test]
    async fn test_close_assembly_continues_on_delete_session_error() {
        // Conflict-2 Resolution: cascade-loop is continue-on-error. A failure
        // on s1 must NOT short-circuit; s2 must still be invoked, and the
        // method must return Ok(_) (the close_assembly audit-entry is
        // already committed before the loop).
        let entity = assembly_in_status(AssemblyStatus::Open);
        let entity_id = entity.id;
        let entity_for_find = entity.clone();

        let mut assembly_dao = MockTestAssemblyDao::new();
        assembly_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(entity_for_find.clone())));
        assembly_dao
            .expect_update()
            .times(1)
            .returning(|_, _, _| Ok(()));

        let mut helper_token_dao = MockTestHelperTokenDao::new();
        helper_token_dao
            .expect_list_session_ids_for_assembly()
            .times(1)
            .returning(|_, _| Ok(vec![Arc::from("s1"), Arc::from("s2")]));

        let mut permission_dao = MockTestPermissionDao::new();
        permission_dao
            .expect_delete_session()
            .with(mockall::predicate::eq("s1"))
            .times(1)
            .returning(|_| Err(DaoError::DatabaseError(Arc::from("simulated"))));
        permission_dao
            .expect_delete_session()
            .with(mockall::predicate::eq("s2"))
            .times(1)
            .returning(|_| Ok(()));

        let service = build_service_with_cascade(
            assembly_dao,
            MockTestSnapshotDao::new(),
            MockTestMemberDao::new(),
            helper_token_dao,
            permission_dao,
        );

        let result = service
            .close_assembly(entity_id, Authentication::Full)
            .await;
        assert!(
            result.is_ok(),
            "cascade must continue on per-session error, got {:?}",
            result
        );
        assert_eq!(result.unwrap().status, AssemblyStatus::Closed);
    }

    #[tokio::test]
    async fn test_close_assembly_empty_session_list_succeeds() {
        // Empty session list → delete_session is never called; close_assembly
        // returns Ok(_) (no helpers attached to this assembly).
        let entity = assembly_in_status(AssemblyStatus::Open);
        let entity_id = entity.id;
        let entity_for_find = entity.clone();

        let mut assembly_dao = MockTestAssemblyDao::new();
        assembly_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(entity_for_find.clone())));
        assembly_dao
            .expect_update()
            .times(1)
            .returning(|_, _, _| Ok(()));

        let mut helper_token_dao = MockTestHelperTokenDao::new();
        helper_token_dao
            .expect_list_session_ids_for_assembly()
            .times(1)
            .returning(|_, _| Ok(Vec::new()));

        // No expect_delete_session — mockall panics on unexpected calls,
        // verifying the empty-list short-circuit.
        let permission_dao = MockTestPermissionDao::new();

        let service = build_service_with_cascade(
            assembly_dao,
            MockTestSnapshotDao::new(),
            MockTestMemberDao::new(),
            helper_token_dao,
            permission_dao,
        );

        let result = service
            .close_assembly(entity_id, Authentication::Full)
            .await
            .expect("close_assembly should succeed with no sessions");
        assert_eq!(result.status, AssemblyStatus::Closed);
    }

    #[tokio::test]
    async fn test_close_assembly_audited_update_runs_before_cascade_discovery() {
        // Sequencing guarantee (D-15): audited_update -> list_session_ids
        // -> delete_session. We assert this with a mockall::Sequence so
        // a future refactor that reverses the order fails the test.
        let entity = assembly_in_status(AssemblyStatus::Open);
        let entity_id = entity.id;
        let entity_for_find = entity.clone();

        let mut seq = mockall::Sequence::new();

        let mut assembly_dao = MockTestAssemblyDao::new();
        assembly_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(entity_for_find.clone())));
        assembly_dao
            .expect_update()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_, _, _| Ok(()));

        let mut helper_token_dao = MockTestHelperTokenDao::new();
        helper_token_dao
            .expect_list_session_ids_for_assembly()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_, _| Ok(vec![Arc::from("s1")]));

        let mut permission_dao = MockTestPermissionDao::new();
        permission_dao
            .expect_delete_session()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Ok(()));

        let service = build_service_with_cascade(
            assembly_dao,
            MockTestSnapshotDao::new(),
            MockTestMemberDao::new(),
            helper_token_dao,
            permission_dao,
        );

        let result = service
            .close_assembly(entity_id, Authentication::Full)
            .await
            .expect("close_assembly should succeed");
        assert_eq!(result.status, AssemblyStatus::Closed);
    }

    // Note: the existing `test_close_assembly_from_preparation_returns_conflict`
    // test (above) is the Phase-1 regression guard (Test 4 of <behavior>).
    // It uses `build_service` (cascade Mocks default to no-op) and short-
    // circuits before list_session_ids_for_assembly is reached — therefore
    // those Mocks are never exercised. Leaving the test untouched validates
    // the no-regression promise of Plan 05.
}
