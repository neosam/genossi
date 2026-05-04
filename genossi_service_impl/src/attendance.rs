//! Service-layer implementation of the Attendance aggregate (Phase 3 Plan 05).
//!
//! Wires `AttendanceServiceImpl` with the central permission funnel
//! `check_assembly_access` and the four endpoint methods (`list_members`,
//! `mark_present`, `mark_absent`, `stats`).
//!
//! **D-08, ATTN-05** — there is intentionally NO audit logging in this
//! service. attendance toggles are not lifecycle events; the
//! Genossenschaftsverband requires only the count, not the per-toggle act.
//! Member/MemberAction/MemberDocument/Application audit semantics are
//! therefore NOT replicated here. Do NOT introduce `audited_*!` macros into
//! this file — that would couple the attendance aggregate to the audit
//! hash chain and reverse the explicit user decision.
//!
//! **D-17, D-18** — `check_assembly_access` is the single permission funnel
//! for ALL four endpoint methods. Each method calls it as the FIRST DAO-
//! touching step after `use_transaction`. The funnel implements three
//! branches:
//!   1. `Authentication::Full` → unconditional Ok.
//!   2. `Authentication::Context(ctx)` with `ctx.as_helper() == Some(aid)` →
//!      helper branch. Requires `helper_aid == endpoint_aid` AND
//!      `assembly.status == Open`. Mismatch / Closed-status → PermissionDenied.
//!   3. `Authentication::Context(ctx)` without helper claim →
//!      admin branch via `permission_service.check_permission("admin", ...)`.
//!      D-20: NO status check for admin — admin may post-close edit (ASSY-06).
//!
//! **D-27** — `mark_present`/`mark_absent` BOTH call `is_in_snapshot` before
//! their respective DAO mutation. A non-snapshot member yields
//! `ServiceError::EntityNotFound(member_id)` (mapped to HTTP 404 in Plan 06).

use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use genossi_dao::assembly::{AssemblyDao, AssemblyEntity, AssemblyStatus};
use genossi_dao::assembly_member_snapshot::AssemblyMemberSnapshotDao;
use genossi_dao::attendance::{AttendanceDao, AttendanceMemberRow};
use genossi_dao::member::MemberDao;
use genossi_dao::TransactionDao;

use genossi_service::attendance::{AttendanceService, AttendanceStats};
use genossi_service::claim_context::ClaimContext;
use genossi_service::permission::{Authentication, PermissionService};
use genossi_service::ServiceError;

use crate::gen_service_impl;

/// Privilege constant for the admin (Vorstand) branch (D-19). The same
/// string `"admin"` is used by the bestehende AssemblyServiceImpl — we
/// intentionally do NOT introduce a new `attendance.access` privilege per
/// D-19 (existing admin role suffices).
const ADMIN_PRIVILEGE: &str = "admin";

gen_service_impl! {
    struct AttendanceServiceImpl: AttendanceService = AttendanceServiceDeps {
        AttendanceDao: AttendanceDao<Transaction = Self::Transaction> = attendance_dao,
        AssemblyDao: AssemblyDao<Transaction = Self::Transaction> = assembly_dao,
        MemberDao: MemberDao<Transaction = Self::Transaction> = member_dao,
        AssemblyMemberSnapshotDao: AssemblyMemberSnapshotDao<Transaction = Self::Transaction> = assembly_member_snapshot_dao,
        PermissionService: PermissionService<Context = Self::Context> = permission_service,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
    }
}

impl<Deps: AttendanceServiceDeps> AttendanceServiceImpl<Deps> {
    /// Permission funnel for ALL 4 attendance endpoints (D-17 / D-18).
    ///
    /// Returns the loaded `AssemblyEntity` so the caller does not need to
    /// re-load it. Every endpoint method MUST call this as its first
    /// DAO-touching step after `use_transaction` — bypassing the funnel is
    /// an Information-Disclosure / Elevation-of-Privilege bug
    /// (T-03-05-01..03 mitigation).
    ///
    /// Error mapping:
    ///   * `EntityNotFound(assembly_id)` — assembly does not exist.
    ///   * `PermissionDenied` — helper-aid mismatch, status != Open for a
    ///     helper, or non-admin without helper-claim.
    async fn check_assembly_access(
        &self,
        assembly_id: Uuid,
        context: Authentication<Deps::Context>,
        tx: Deps::Transaction,
    ) -> Result<AssemblyEntity, ServiceError> {
        // Always load the assembly first — needed both for the helper
        // status check and for accurate EntityNotFound mapping.
        let assembly = self
            .assembly_dao
            .find_by_id(assembly_id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(assembly_id))?;

        match &context {
            Authentication::Full => Ok(assembly),
            Authentication::Context(ctx) => {
                // Helper discrimination via ClaimContext::as_helper (Plan 03).
                if let Some(helper_aid) = ctx.as_helper() {
                    // D-18: helper-branch — aid match + status==Open.
                    if helper_aid != assembly_id {
                        return Err(ServiceError::PermissionDenied);
                    }
                    if assembly.status != AssemblyStatus::Open {
                        return Err(ServiceError::PermissionDenied);
                    }
                    return Ok(assembly);
                }
                // Vorstand-branch via admin privilege.
                // D-20: NO status check — admin may post-close edit (ASSY-06).
                self.permission_service
                    .check_permission(ADMIN_PRIVILEGE, context)
                    .await?;
                Ok(assembly)
            }
        }
    }
}

#[async_trait]
impl<Deps: AttendanceServiceDeps> AttendanceService for AttendanceServiceImpl<Deps> {
    type Context = Deps::Context;
    type Transaction = Deps::Transaction;

    async fn list_members(
        &self,
        assembly_id: Uuid,
        search: Option<String>,
        context: Authentication<Self::Context>,
    ) -> Result<Arc<[AttendanceMemberRow]>, ServiceError> {
        let tx = self.transaction_dao.use_transaction(None).await?;
        // D-17: permission funnel as first DAO-touching step.
        let _assembly = self
            .check_assembly_access(assembly_id, context, tx.clone())
            .await?;

        // D-25: Substring search is forwarded 1:1 to the DAO (no in-memory
        // filter in the service layer). DAO performs LIKE-COLLATE-NOCASE.
        let rows = self
            .attendance_dao
            .list_members_for_assembly(assembly_id, search, tx.clone())
            .await?;

        self.transaction_dao.commit(tx).await?;
        Ok(rows)
    }

    async fn mark_present(
        &self,
        assembly_id: Uuid,
        member_id: Uuid,
        context: Authentication<Self::Context>,
    ) -> Result<(), ServiceError> {
        let tx = self.transaction_dao.use_transaction(None).await?;
        let _assembly = self
            .check_assembly_access(assembly_id, context.clone(), tx.clone())
            .await?;

        // D-27: snapshot membership check — non-snapshot member is 404.
        if !self
            .attendance_dao
            .is_in_snapshot(assembly_id, member_id, tx.clone())
            .await?
        {
            return Err(ServiceError::EntityNotFound(member_id));
        }

        // marked_by_user_id format: `helper:<token_id>` for helpers,
        // OIDC user id for the board (Phase 2 D-17). Fallback "SYSTEM"
        // mirrors the convention from AssemblyServiceImpl::create_assembly.
        let user_id = self
            .permission_service
            .current_user_id(context)
            .await?
            .unwrap_or_else(|| "SYSTEM".to_string());

        let now_offset = time::OffsetDateTime::now_utc();
        let now = time::PrimitiveDateTime::new(now_offset.date(), now_offset.time());

        self.attendance_dao
            .upsert_present(assembly_id, member_id, now, &user_id, tx.clone())
            .await?;

        self.transaction_dao.commit(tx).await?;
        Ok(())
    }

    async fn mark_absent(
        &self,
        assembly_id: Uuid,
        member_id: Uuid,
        context: Authentication<Self::Context>,
    ) -> Result<(), ServiceError> {
        let tx = self.transaction_dao.use_transaction(None).await?;
        let _assembly = self
            .check_assembly_access(assembly_id, context, tx.clone())
            .await?;

        // D-27: snapshot membership check — even idempotent toggle-off
        // requires that the member is part of the GV snapshot.
        if !self
            .attendance_dao
            .is_in_snapshot(assembly_id, member_id, tx.clone())
            .await?
        {
            return Err(ServiceError::EntityNotFound(member_id));
        }

        let now_offset = time::OffsetDateTime::now_utc();
        let now = time::PrimitiveDateTime::new(now_offset.date(), now_offset.time());

        self.attendance_dao
            .soft_delete(assembly_id, member_id, now, tx.clone())
            .await?;

        self.transaction_dao.commit(tx).await?;
        Ok(())
    }

    async fn stats(
        &self,
        assembly_id: Uuid,
        context: Authentication<Self::Context>,
    ) -> Result<AttendanceStats, ServiceError> {
        let tx = self.transaction_dao.use_transaction(None).await?;
        let _assembly = self
            .check_assembly_access(assembly_id, context, tx.clone())
            .await?;

        // ASSY-04: present from attendance, total from snapshot.
        let present = self
            .attendance_dao
            .count_present_by_assembly(assembly_id, tx.clone())
            .await?;
        let total = self
            .assembly_member_snapshot_dao
            .count_by_assembly_id(assembly_id, tx.clone())
            .await?;

        self.transaction_dao.commit(tx).await?;
        Ok(AttendanceStats { present, total })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use genossi_dao::member::MemberEntity;
    use genossi_dao::{DaoError, Transaction};
    use mockall::{mock, predicate::*};

    /// Test-local Transaction type. The genossi_dao::MockTransaction is
    /// hard-wired into the various #[automock]-generated mocks via
    /// `type Transaction = MockTransaction`, so we cannot retarget those
    /// mocks. We re-roll local mocks against TestTransaction (Pitfall 4).
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

    /// Test-local Context with configurable helper claim. The trait bound
    /// from gen_service_impl! requires `Context: ClaimContext`, so the
    /// helper-discrimination branch reads from this struct's `helper_claim`.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct TestContext {
        pub helper_claim: Option<Uuid>,
    }

    impl ClaimContext for TestContext {
        fn has_claims(&self) -> bool {
            self.helper_claim.is_some()
        }

        fn as_helper(&self) -> Option<Uuid> {
            self.helper_claim
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
        pub TestAttendanceDao {}
        #[async_trait]
        impl AttendanceDao for TestAttendanceDao {
            type Transaction = TestTransaction;
            async fn upsert_present(
                &self,
                assembly_id: Uuid,
                member_id: Uuid,
                marked_at: time::PrimitiveDateTime,
                marked_by_user_id: &str,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn soft_delete(
                &self,
                assembly_id: Uuid,
                member_id: Uuid,
                deleted_at: time::PrimitiveDateTime,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn list_members_for_assembly(
                &self,
                assembly_id: Uuid,
                search: Option<String>,
                tx: TestTransaction,
            ) -> Result<Arc<[AttendanceMemberRow]>, DaoError>;
            async fn count_present_by_assembly(
                &self,
                assembly_id: Uuid,
                tx: TestTransaction,
            ) -> Result<u64, DaoError>;
            async fn is_in_snapshot(
                &self,
                assembly_id: Uuid,
                member_id: Uuid,
                tx: TestTransaction,
            ) -> Result<bool, DaoError>;
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
        pub TestSnapshotDao {}
        #[async_trait]
        impl AssemblyMemberSnapshotDao for TestSnapshotDao {
            type Transaction = TestTransaction;
            async fn create(
                &self,
                entity: &genossi_dao::assembly_member_snapshot::AssemblyMemberSnapshotEntity,
                process: &str,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn create_batch(
                &self,
                entities: &[genossi_dao::assembly_member_snapshot::AssemblyMemberSnapshotEntity],
                process: &str,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn find_by_assembly_id(
                &self,
                assembly_id: Uuid,
                tx: TestTransaction,
            ) -> Result<Arc<[genossi_dao::assembly_member_snapshot::AssemblyMemberSnapshotEntity]>, DaoError>;
            async fn count_by_assembly_id(
                &self,
                assembly_id: Uuid,
                tx: TestTransaction,
            ) -> Result<u64, DaoError>;
        }
    }

    mock! {
        pub TestPermissionService {}
        #[async_trait]
        impl PermissionService for TestPermissionService {
            type Context = TestContext;
            async fn check_permission(
                &self,
                privilege: &str,
                context: Authentication<TestContext>,
            ) -> Result<(), ServiceError>;
            async fn current_user_id(
                &self,
                context: Authentication<TestContext>,
            ) -> Result<Option<String>, ServiceError>;
            async fn get_all_users(
                &self,
                context: Authentication<TestContext>,
            ) -> Result<Arc<[genossi_service::auth_types::UserResponseTO]>, ServiceError>;
            async fn create_user(
                &self,
                user: genossi_service::auth_types::UserTO,
                context: Authentication<TestContext>,
            ) -> Result<(), ServiceError>;
            async fn delete_user(
                &self,
                username: String,
                context: Authentication<TestContext>,
            ) -> Result<(), ServiceError>;
            async fn get_all_roles(
                &self,
                context: Authentication<TestContext>,
            ) -> Result<Arc<[genossi_service::auth_types::RoleResponseTO]>, ServiceError>;
            async fn create_role(
                &self,
                role: genossi_service::auth_types::RoleTO,
                context: Authentication<TestContext>,
            ) -> Result<(), ServiceError>;
            async fn delete_role(
                &self,
                role_name: String,
                context: Authentication<TestContext>,
            ) -> Result<(), ServiceError>;
            async fn get_all_privileges(
                &self,
                context: Authentication<TestContext>,
            ) -> Result<Arc<[genossi_service::auth_types::PrivilegeResponseTO]>, ServiceError>;
            async fn create_privilege(
                &self,
                privilege: genossi_service::auth_types::PrivilegeTO,
                context: Authentication<TestContext>,
            ) -> Result<(), ServiceError>;
            async fn delete_privilege(
                &self,
                privilege_name: String,
                context: Authentication<TestContext>,
            ) -> Result<(), ServiceError>;
            async fn assign_user_role(
                &self,
                user_role: genossi_service::auth_types::UserRole,
                context: Authentication<TestContext>,
            ) -> Result<(), ServiceError>;
            async fn remove_user_role(
                &self,
                user_role: genossi_service::auth_types::UserRole,
                context: Authentication<TestContext>,
            ) -> Result<(), ServiceError>;
            async fn get_user_roles(
                &self,
                username: String,
                context: Authentication<TestContext>,
            ) -> Result<Arc<[genossi_service::auth_types::RoleResponseTO]>, ServiceError>;
            async fn assign_role_privilege(
                &self,
                role_privilege: genossi_service::auth_types::RolePrivilege,
                context: Authentication<TestContext>,
            ) -> Result<(), ServiceError>;
            async fn remove_role_privilege(
                &self,
                role_privilege: genossi_service::auth_types::RolePrivilege,
                context: Authentication<TestContext>,
            ) -> Result<(), ServiceError>;
            async fn get_role_privileges(
                &self,
                role_name: String,
                context: Authentication<TestContext>,
            ) -> Result<Arc<[genossi_service::auth_types::PrivilegeResponseTO]>, ServiceError>;
            async fn get_user_privileges(
                &self,
                username: String,
                context: Authentication<TestContext>,
            ) -> Result<Arc<[genossi_service::auth_types::PrivilegeResponseTO]>, ServiceError>;
            async fn has_claims(&self, context: &TestContext) -> Result<bool, ServiceError>;
        }
    }

    /// TestDeps wires the local mocks as associated types.
    pub struct TestDeps;

    impl AttendanceServiceDeps for TestDeps {
        type Context = TestContext;
        type Transaction = TestTransaction;
        type AttendanceDao = MockTestAttendanceDao;
        type AssemblyDao = MockTestAssemblyDao;
        type MemberDao = MockTestMemberDao;
        type AssemblyMemberSnapshotDao = MockTestSnapshotDao;
        type PermissionService = MockTestPermissionService;
        type TransactionDao = MockTestTxDao;
    }

    fn assembly_in_status(aid: Uuid, status: AssemblyStatus) -> AssemblyEntity {
        let date = time::Date::from_calendar_date(2026, time::Month::May, 15).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        AssemblyEntity {
            id: aid,
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

    fn build_service(
        attendance_dao: MockTestAttendanceDao,
        assembly_dao: MockTestAssemblyDao,
        snapshot_dao: MockTestSnapshotDao,
        permission_service: MockTestPermissionService,
        tx_dao: MockTestTxDao,
    ) -> AttendanceServiceImpl<TestDeps> {
        AttendanceServiceImpl {
            attendance_dao: Arc::new(attendance_dao),
            assembly_dao: Arc::new(assembly_dao),
            member_dao: Arc::new(MockTestMemberDao::new()),
            assembly_member_snapshot_dao: Arc::new(snapshot_dao),
            permission_service: Arc::new(permission_service),
            transaction_dao: Arc::new(tx_dao),
        }
    }

    fn tx_dao_with_commit() -> MockTestTxDao {
        let mut tx_dao = MockTestTxDao::new();
        tx_dao
            .expect_use_transaction()
            .returning(|_| Ok(TestTransaction));
        tx_dao.expect_commit().returning(|_| Ok(()));
        tx_dao
    }

    fn tx_dao_no_commit() -> MockTestTxDao {
        // For tests that error before reaching commit. We allow 0..=1
        // commits because the service short-circuits — `times(0)` would
        // panic on any unexpected call but `times(0..=1)` is more robust
        // against future implementation tweaks.
        let mut tx_dao = MockTestTxDao::new();
        tx_dao
            .expect_use_transaction()
            .returning(|_| Ok(TestTransaction));
        tx_dao.expect_commit().times(0..=1).returning(|_| Ok(()));
        tx_dao
    }

    // ------------------------------------------------------------------
    // check_assembly_access tests (Tests 1-7 from <behavior>)
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_check_assembly_access_full_authentication_returns_ok() {
        // Test 1: Authentication::Full bypasses permission checks entirely.
        let aid = Uuid::new_v4();

        let mut assembly_dao = MockTestAssemblyDao::new();
        let assembly = assembly_in_status(aid, AssemblyStatus::Closed); // even Closed is OK with Full
        assembly_dao
            .expect_find_by_id()
            .with(eq(aid), always())
            .times(1)
            .returning(move |_, _| Ok(Some(assembly.clone())));

        let mut attendance_dao = MockTestAttendanceDao::new();
        attendance_dao
            .expect_list_members_for_assembly()
            .returning(|_, _, _| Ok(Arc::from(Vec::<AttendanceMemberRow>::new())));

        let svc = build_service(
            attendance_dao,
            assembly_dao,
            MockTestSnapshotDao::new(),
            MockTestPermissionService::new(),
            tx_dao_with_commit(),
        );

        let result = svc.list_members(aid, None, Authentication::Full).await;
        assert!(
            result.is_ok(),
            "Authentication::Full must short-circuit, got {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_check_assembly_access_helper_matching_aid_open_returns_ok() {
        // Test 2: helper claim with matching aid and Open status passes.
        let aid = Uuid::new_v4();

        let mut assembly_dao = MockTestAssemblyDao::new();
        let assembly = assembly_in_status(aid, AssemblyStatus::Open);
        assembly_dao
            .expect_find_by_id()
            .with(eq(aid), always())
            .times(1)
            .returning(move |_, _| Ok(Some(assembly.clone())));

        let mut attendance_dao = MockTestAttendanceDao::new();
        attendance_dao
            .expect_list_members_for_assembly()
            .returning(|_, _, _| Ok(Arc::from(Vec::<AttendanceMemberRow>::new())));

        // Permission service must NOT be called — helper-branch wins.
        let perm = MockTestPermissionService::new();

        let svc = build_service(
            attendance_dao,
            assembly_dao,
            MockTestSnapshotDao::new(),
            perm,
            tx_dao_with_commit(),
        );

        let ctx = Authentication::Context(TestContext {
            helper_claim: Some(aid),
        });
        let result = svc.list_members(aid, None, ctx).await;
        assert!(
            result.is_ok(),
            "helper claim matching aid + Open status must pass, got {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_check_assembly_access_helper_wrong_aid_returns_permission_denied() {
        // Test 3: helper claim with mismatching aid → PermissionDenied.
        let endpoint_aid = Uuid::new_v4();
        let other_aid = Uuid::new_v4();
        assert_ne!(endpoint_aid, other_aid);

        let mut assembly_dao = MockTestAssemblyDao::new();
        let assembly = assembly_in_status(endpoint_aid, AssemblyStatus::Open);
        assembly_dao
            .expect_find_by_id()
            .with(eq(endpoint_aid), always())
            .times(1)
            .returning(move |_, _| Ok(Some(assembly.clone())));

        let svc = build_service(
            MockTestAttendanceDao::new(),
            assembly_dao,
            MockTestSnapshotDao::new(),
            MockTestPermissionService::new(),
            tx_dao_no_commit(),
        );

        let ctx = Authentication::Context(TestContext {
            helper_claim: Some(other_aid), // <- mismatch
        });
        let result = svc.list_members(endpoint_aid, None, ctx).await;
        assert!(
            matches!(result, Err(ServiceError::PermissionDenied)),
            "helper-aid mismatch must return PermissionDenied, got {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_check_assembly_access_helper_assembly_closed_returns_permission_denied() {
        // Test 4: helper claim matches aid but assembly.status==Closed →
        // PermissionDenied (status check applies to helper branch only).
        let aid = Uuid::new_v4();

        let mut assembly_dao = MockTestAssemblyDao::new();
        let assembly = assembly_in_status(aid, AssemblyStatus::Closed);
        assembly_dao
            .expect_find_by_id()
            .with(eq(aid), always())
            .times(1)
            .returning(move |_, _| Ok(Some(assembly.clone())));

        let svc = build_service(
            MockTestAttendanceDao::new(),
            assembly_dao,
            MockTestSnapshotDao::new(),
            MockTestPermissionService::new(),
            tx_dao_no_commit(),
        );

        let ctx = Authentication::Context(TestContext {
            helper_claim: Some(aid),
        });
        let result = svc.list_members(aid, None, ctx).await;
        assert!(
            matches!(result, Err(ServiceError::PermissionDenied)),
            "helper-aid match but Status=Closed must return PermissionDenied, got {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_check_assembly_access_admin_pass_through_no_status_check() {
        // Test 5: Admin context (no helper claim) bypasses status check —
        // D-20: admin may post-close edit (ASSY-06).
        let aid = Uuid::new_v4();

        let mut assembly_dao = MockTestAssemblyDao::new();
        let assembly = assembly_in_status(aid, AssemblyStatus::Closed); // <- D-20 key
        assembly_dao
            .expect_find_by_id()
            .with(eq(aid), always())
            .times(1)
            .returning(move |_, _| Ok(Some(assembly.clone())));

        let mut perm = MockTestPermissionService::new();
        perm.expect_check_permission()
            .withf(|priv_str, _ctx| priv_str == "admin")
            .times(1)
            .returning(|_, _| Ok(()));

        let mut attendance_dao = MockTestAttendanceDao::new();
        attendance_dao
            .expect_list_members_for_assembly()
            .returning(|_, _, _| Ok(Arc::from(Vec::<AttendanceMemberRow>::new())));

        let svc = build_service(
            attendance_dao,
            assembly_dao,
            MockTestSnapshotDao::new(),
            perm,
            tx_dao_with_commit(),
        );

        let ctx = Authentication::Context(TestContext { helper_claim: None });
        let result = svc.list_members(aid, None, ctx).await;
        assert!(
            result.is_ok(),
            "D-20: admin must reach list_members even with Status=Closed, got {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_check_assembly_access_admin_denied_returns_permission_denied() {
        // Test 6: Admin context but PermissionService returns
        // PermissionDenied → PermissionDenied bubbles up.
        let aid = Uuid::new_v4();

        let mut assembly_dao = MockTestAssemblyDao::new();
        let assembly = assembly_in_status(aid, AssemblyStatus::Open);
        assembly_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(assembly.clone())));

        let mut perm = MockTestPermissionService::new();
        perm.expect_check_permission()
            .returning(|_, _| Err(ServiceError::PermissionDenied));

        let svc = build_service(
            MockTestAttendanceDao::new(),
            assembly_dao,
            MockTestSnapshotDao::new(),
            perm,
            tx_dao_no_commit(),
        );

        let ctx = Authentication::Context(TestContext { helper_claim: None });
        let result = svc.list_members(aid, None, ctx).await;
        assert!(
            matches!(result, Err(ServiceError::PermissionDenied)),
            "admin denied must return PermissionDenied, got {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_check_assembly_access_unknown_assembly_returns_entity_not_found() {
        // Test 7: assembly_dao.find_by_id returns None → EntityNotFound.
        let aid = Uuid::new_v4();

        let mut assembly_dao = MockTestAssemblyDao::new();
        assembly_dao
            .expect_find_by_id()
            .with(eq(aid), always())
            .times(1)
            .returning(|_, _| Ok(None));

        let svc = build_service(
            MockTestAttendanceDao::new(),
            assembly_dao,
            MockTestSnapshotDao::new(),
            MockTestPermissionService::new(),
            tx_dao_no_commit(),
        );

        let result = svc.list_members(aid, None, Authentication::Full).await;
        assert!(
            matches!(result, Err(ServiceError::EntityNotFound(uid)) if uid == aid),
            "unknown assembly must return EntityNotFound(assembly_id), got {:?}",
            result
        );
    }

    // ------------------------------------------------------------------
    // mark_present tests (Tests 8-9)
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_mark_present_idempotent_calls_upsert_with_synthetic_user_id() {
        // Test 8: helper claim with matching aid + Open assembly →
        // upsert_present is called once with marked_by_user_id == "helper:abc-token".
        let aid = Uuid::new_v4();
        let mid = Uuid::new_v4();

        let mut assembly_dao = MockTestAssemblyDao::new();
        let assembly = assembly_in_status(aid, AssemblyStatus::Open);
        assembly_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(assembly.clone())));

        let mut perm = MockTestPermissionService::new();
        perm.expect_current_user_id()
            .returning(|_| Ok(Some("helper:abc-token".to_string())));
        // No expect_check_permission — helper branch wins; any call would panic.

        let mut attendance_dao = MockTestAttendanceDao::new();
        attendance_dao
            .expect_is_in_snapshot()
            .with(eq(aid), eq(mid), always())
            .times(1)
            .returning(|_, _, _| Ok(true));
        attendance_dao
            .expect_upsert_present()
            .withf(|_aid, _mid, _t, by, _tx| by == "helper:abc-token")
            .times(1)
            .returning(|_, _, _, _, _| Ok(()));

        let svc = build_service(
            attendance_dao,
            assembly_dao,
            MockTestSnapshotDao::new(),
            perm,
            tx_dao_with_commit(),
        );

        let ctx = Authentication::Context(TestContext {
            helper_claim: Some(aid),
        });
        let result = svc.mark_present(aid, mid, ctx).await;
        assert!(result.is_ok(), "expected Ok(()), got {:?}", result);
    }

    #[tokio::test]
    async fn test_mark_present_member_not_in_snapshot_returns_404() {
        // Test 9: D-27 — non-snapshot member returns EntityNotFound,
        // upsert_present is NOT called.
        let aid = Uuid::new_v4();
        let mid = Uuid::new_v4();

        let mut assembly_dao = MockTestAssemblyDao::new();
        let assembly = assembly_in_status(aid, AssemblyStatus::Open);
        assembly_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(assembly.clone())));

        let mut attendance_dao = MockTestAttendanceDao::new();
        attendance_dao
            .expect_is_in_snapshot()
            .with(eq(aid), eq(mid), always())
            .times(1)
            .returning(|_, _, _| Ok(false));
        // No expect_upsert_present — must not be invoked.

        let svc = build_service(
            attendance_dao,
            assembly_dao,
            MockTestSnapshotDao::new(),
            MockTestPermissionService::new(),
            tx_dao_no_commit(),
        );

        let result = svc
            .mark_present(aid, mid, Authentication::Full)
            .await;
        assert!(
            matches!(result, Err(ServiceError::EntityNotFound(uid)) if uid == mid),
            "non-snapshot member must return EntityNotFound(member_id), got {:?}",
            result
        );
    }

    // ------------------------------------------------------------------
    // mark_absent tests (Tests 10-11)
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_mark_absent_idempotent_no_error_on_no_op() {
        // Test 10: in-snapshot member, soft_delete returns Ok(()) →
        // result Ok(()) (idempotent toggle-off).
        let aid = Uuid::new_v4();
        let mid = Uuid::new_v4();

        let mut assembly_dao = MockTestAssemblyDao::new();
        let assembly = assembly_in_status(aid, AssemblyStatus::Open);
        assembly_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(assembly.clone())));

        let mut attendance_dao = MockTestAttendanceDao::new();
        attendance_dao
            .expect_is_in_snapshot()
            .with(eq(aid), eq(mid), always())
            .times(1)
            .returning(|_, _, _| Ok(true));
        attendance_dao
            .expect_soft_delete()
            .with(eq(aid), eq(mid), always(), always())
            .times(1)
            .returning(|_, _, _, _| Ok(()));

        let svc = build_service(
            attendance_dao,
            assembly_dao,
            MockTestSnapshotDao::new(),
            MockTestPermissionService::new(),
            tx_dao_with_commit(),
        );

        let result = svc
            .mark_absent(aid, mid, Authentication::Full)
            .await;
        assert!(result.is_ok(), "expected Ok(()), got {:?}", result);
    }

    #[tokio::test]
    async fn test_mark_absent_member_not_in_snapshot_returns_404() {
        // Test 11: D-27 — non-snapshot member returns EntityNotFound,
        // soft_delete is NOT called.
        let aid = Uuid::new_v4();
        let mid = Uuid::new_v4();

        let mut assembly_dao = MockTestAssemblyDao::new();
        let assembly = assembly_in_status(aid, AssemblyStatus::Open);
        assembly_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(assembly.clone())));

        let mut attendance_dao = MockTestAttendanceDao::new();
        attendance_dao
            .expect_is_in_snapshot()
            .returning(|_, _, _| Ok(false));
        // No expect_soft_delete — must not be invoked.

        let svc = build_service(
            attendance_dao,
            assembly_dao,
            MockTestSnapshotDao::new(),
            MockTestPermissionService::new(),
            tx_dao_no_commit(),
        );

        let result = svc.mark_absent(aid, mid, Authentication::Full).await;
        assert!(
            matches!(result, Err(ServiceError::EntityNotFound(uid)) if uid == mid),
            "non-snapshot member must return EntityNotFound(member_id), got {:?}",
            result
        );
    }

    // ------------------------------------------------------------------
    // list_members tests (Tests 12-13)
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_list_members_returns_dao_result_unmodified() {
        // Test 12: DAO returns 3 rows → service returns the same Arc.
        let aid = Uuid::new_v4();

        let mut assembly_dao = MockTestAssemblyDao::new();
        let assembly = assembly_in_status(aid, AssemblyStatus::Open);
        assembly_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(assembly.clone())));

        let rows = vec![
            AttendanceMemberRow {
                member_id: Uuid::new_v4(),
                member_number: 1,
                first_name: Arc::from("Alice"),
                last_name: Arc::from("A"),
                salutation: None,
                title: None,
                is_present: true,
            },
            AttendanceMemberRow {
                member_id: Uuid::new_v4(),
                member_number: 2,
                first_name: Arc::from("Bob"),
                last_name: Arc::from("B"),
                salutation: None,
                title: None,
                is_present: false,
            },
            AttendanceMemberRow {
                member_id: Uuid::new_v4(),
                member_number: 3,
                first_name: Arc::from("Carol"),
                last_name: Arc::from("C"),
                salutation: None,
                title: None,
                is_present: true,
            },
        ];
        let rows_arc: Arc<[AttendanceMemberRow]> = Arc::from(rows);
        let rows_clone = rows_arc.clone();

        let mut attendance_dao = MockTestAttendanceDao::new();
        attendance_dao
            .expect_list_members_for_assembly()
            .times(1)
            .returning(move |_, _, _| Ok(rows_clone.clone()));

        let svc = build_service(
            attendance_dao,
            assembly_dao,
            MockTestSnapshotDao::new(),
            MockTestPermissionService::new(),
            tx_dao_with_commit(),
        );

        let result = svc
            .list_members(aid, None, Authentication::Full)
            .await
            .expect("list_members should succeed");
        assert_eq!(result.len(), 3, "expected 3 rows, got {}", result.len());
    }

    #[tokio::test]
    async fn test_list_members_passes_search_string_to_dao() {
        // Test 13: D-25 — search String is forwarded to DAO 1:1.
        let aid = Uuid::new_v4();

        let mut assembly_dao = MockTestAssemblyDao::new();
        let assembly = assembly_in_status(aid, AssemblyStatus::Open);
        assembly_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(assembly.clone())));

        let mut attendance_dao = MockTestAttendanceDao::new();
        attendance_dao
            .expect_list_members_for_assembly()
            .withf(|_aid, search, _tx| {
                search.as_deref() == Some("Müller")
            })
            .times(1)
            .returning(|_, _, _| Ok(Arc::from(Vec::<AttendanceMemberRow>::new())));

        let svc = build_service(
            attendance_dao,
            assembly_dao,
            MockTestSnapshotDao::new(),
            MockTestPermissionService::new(),
            tx_dao_with_commit(),
        );

        let result = svc
            .list_members(aid, Some("Müller".to_string()), Authentication::Full)
            .await;
        assert!(result.is_ok(), "expected Ok, got {:?}", result);
    }

    // ------------------------------------------------------------------
    // stats tests (Test 14)
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_stats_combines_present_and_total_counts() {
        // Test 14: present=3 from attendance, total=10 from snapshot →
        // AttendanceStats { present: 3, total: 10 }.
        let aid = Uuid::new_v4();

        let mut assembly_dao = MockTestAssemblyDao::new();
        let assembly = assembly_in_status(aid, AssemblyStatus::Open);
        assembly_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(assembly.clone())));

        let mut attendance_dao = MockTestAttendanceDao::new();
        attendance_dao
            .expect_count_present_by_assembly()
            .with(eq(aid), always())
            .times(1)
            .returning(|_, _| Ok(3));

        let mut snapshot_dao = MockTestSnapshotDao::new();
        snapshot_dao
            .expect_count_by_assembly_id()
            .with(eq(aid), always())
            .times(1)
            .returning(|_, _| Ok(10));

        let svc = build_service(
            attendance_dao,
            assembly_dao,
            snapshot_dao,
            MockTestPermissionService::new(),
            tx_dao_with_commit(),
        );

        let stats = svc
            .stats(aid, Authentication::Full)
            .await
            .expect("stats should succeed");
        assert_eq!(stats.present, 3);
        assert_eq!(stats.total, 10);
    }
}
