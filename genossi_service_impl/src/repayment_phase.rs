//! Service-layer implementation of the RepaymentPhase aggregate (Phase 7
//! Plan 03).
//!
//! Lifecycle: `Preparation → Open → Closed` (D-02, D-05, D-06: no
//! reverse-transition). All write methods route through the audit macros
//! (`audited_create!` / `audited_update!` / `audited_delete!`) — direct
//! `repayment_phase_dao.create(...)` / `.update(...)` calls outside the
//! macro expansion are forbidden (Threat T-07-03-01).
//!
//! Edit-Matrix (D-04):
//! - `Preparation`: alle Felder editierbar (fiscal_year + share_value)
//! - `Open`: nur share_value editierbar; fiscal_year-Mutation → 409 (D-07)
//! - `Closed`: kein Update (final)
//!
//! Field-level validation (D-11 fiscal_year in 2000..=2100, D-12
//! share_value > 0) wird vor jeder Mutation per `validate_phase_fields`
//! durchgesetzt (in `create_*` und `update_*`).
//!
//! Soft-Delete-Restriction (D-09): `delete_repayment_phase` ist NUR im
//! Status `Preparation` erlaubt; jeder andere Status → 409 Conflict.
//!
//! Phase-8-Erweiterungen (auskommentiert mit TODO-Anchor):
//! - `open_repayment_phase`: Auto-Befüllung der RepaymentEntries (PHAS-02)
//! - `close_repayment_phase`: Pending-Entry-Validation "alle Entries
//!   paid_out oder soft-deleted" (PHAS-03)

use async_trait::async_trait;
use genossi_dao::audit_log::AuditLogDao;
use genossi_dao::repayment_phase::{RepaymentPhaseDao, RepaymentPhaseEntity, RepaymentPhaseStatus};
use genossi_dao::TransactionDao;
use genossi_service::permission::{Authentication, PermissionService};
use genossi_service::repayment_phase::{
    RepaymentPhase, RepaymentPhaseService, RepaymentPhaseSubmission, RepaymentPhaseUpdate,
};
use genossi_service::uuid_service::UuidService;
use genossi_service::{ServiceError, ValidationFailureItem};
use std::sync::Arc;
use uuid::Uuid;

use crate::gen_service_impl;

const REPAYMENT_PHASE_PROCESS_CREATE: &str = "repayment-phase.create";
const REPAYMENT_PHASE_PROCESS_UPDATE: &str = "repayment-phase.update";
const REPAYMENT_PHASE_PROCESS_OPEN: &str = "repayment-phase.open";
const REPAYMENT_PHASE_PROCESS_CLOSE: &str = "repayment-phase.close";
const REPAYMENT_PHASE_PROCESS_DELETE: &str = "repayment-phase.delete";
const ADMIN_PRIVILEGE: &str = "admin";

gen_service_impl! {
    struct RepaymentPhaseServiceImpl: RepaymentPhaseService = RepaymentPhaseServiceDeps {
        RepaymentPhaseDao: RepaymentPhaseDao<Transaction = Self::Transaction> = repayment_phase_dao,
        AuditLogDao: AuditLogDao<Transaction = Self::Transaction> = audit_log_dao,
        PermissionService: PermissionService<Context = Self::Context> = permission_service,
        UuidService: UuidService = uuid_service,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
    }
}

/// Field-level validation per D-11 (fiscal_year range) and D-12 (share_value
/// strictly positive). Used in both `create_repayment_phase` and
/// `update_repayment_phase` because a value can be corrected at any point
/// while still in the editable window (Preparation always; Open only for
/// share_value).
fn validate_phase_fields(fiscal_year: i32, share_value: i64) -> Result<(), ServiceError> {
    let mut errors: Vec<ValidationFailureItem> = Vec::new();
    if !(2000..=2100).contains(&fiscal_year) {
        errors.push(ValidationFailureItem {
            field: Arc::from("fiscal_year"),
            message: Arc::from(format!(
                "must be in 2000..=2100, got {}",
                fiscal_year
            )),
        });
    }
    if share_value <= 0 {
        errors.push(ValidationFailureItem {
            field: Arc::from("share_value"),
            message: Arc::from("must be > 0 (Cent)"),
        });
    }
    if !errors.is_empty() {
        return Err(ServiceError::ValidationError(errors));
    }
    Ok(())
}

#[async_trait]
impl<Deps: RepaymentPhaseServiceDeps> RepaymentPhaseService for RepaymentPhaseServiceImpl<Deps> {
    type Context = Deps::Context;
    type Transaction = Deps::Transaction;

    async fn create_repayment_phase(
        &self,
        submission: &RepaymentPhaseSubmission,
        context: Authentication<Self::Context>,
    ) -> Result<RepaymentPhase, ServiceError> {
        let tx = self.transaction_dao.use_transaction(None).await?;

        let user_id = self
            .permission_service
            .current_user_id(context.clone())
            .await?
            .unwrap_or_else(|| "SYSTEM".to_string());
        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context)
            .await?;

        // D-11 / D-12: validate inputs BEFORE entity construction. On error,
        // the DAO-create call is never made (verified by Test 1/2/3 with
        // mockall .expect_create().times(0)).
        validate_phase_fields(submission.fiscal_year, submission.share_value)?;

        let now = time::OffsetDateTime::now_utc();
        let created = time::PrimitiveDateTime::new(now.date(), now.time());

        let entity = RepaymentPhaseEntity {
            id: self.uuid_service.new_v4().await,
            fiscal_year: submission.fiscal_year,
            share_value: submission.share_value,
            status: RepaymentPhaseStatus::Preparation,
            opened_at: None,
            closed_at: None,
            created,
            deleted: None,
            version: self.uuid_service.new_v4().await,
        };

        crate::audited_create!(
            self,
            self.repayment_phase_dao,
            &entity,
            REPAYMENT_PHASE_PROCESS_CREATE,
            &user_id,
            tx
        );

        self.transaction_dao.commit(tx).await?;
        Ok(RepaymentPhase::from(&entity))
    }

    async fn update_repayment_phase(
        &self,
        id: Uuid,
        update: &RepaymentPhaseUpdate,
        context: Authentication<Self::Context>,
    ) -> Result<RepaymentPhase, ServiceError> {
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
        // load is required to enforce the Edit-Matrix (D-04) and the
        // optimistic-locking version check BEFORE we mutate `entity`. Both
        // reads run inside the same transaction (`tx.clone()`), so they see
        // the same committed snapshot.
        let mut entity = self
            .repayment_phase_dao
            .find_by_id(id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(id))?;

        // D-04 / D-07: Edit-Matrix check BEFORE any other validation. The
        // order matters — a Closed phase rejects ALL mutations atomically;
        // Open rejects fiscal_year mutations atomically (D-07 "atomare
        // Ablehnung" — wenn ein verbotenes Feld berührt wird, wird die
        // GESAMTE Mutation abgelehnt, nicht selektiv durchgewunken).
        match entity.status {
            RepaymentPhaseStatus::Closed => {
                return Err(ServiceError::Conflict(Arc::from(
                    "Cannot update: phase is Closed (D-04)",
                )));
            }
            RepaymentPhaseStatus::Open => {
                if entity.fiscal_year != update.fiscal_year {
                    return Err(ServiceError::Conflict(Arc::from(
                        "Cannot change fiscal_year: phase is Open (D-04/D-07)",
                    )));
                }
                // share_value is the only editable field in Open — fall
                // through to version check and field-validation below.
            }
            RepaymentPhaseStatus::Preparation => {
                // All fields editable — fall through.
            }
        }

        // Optimistic locking — version must match the persisted snapshot.
        if entity.version != update.version {
            return Err(ServiceError::Conflict(Arc::from("Version mismatch")));
        }

        // D-11 / D-12: re-validate on update too, because the corrected
        // value must still satisfy the invariants (e.g. share_value > 0).
        validate_phase_fields(update.fiscal_year, update.share_value)?;

        entity.fiscal_year = update.fiscal_year;
        entity.share_value = update.share_value;

        crate::audited_update!(
            self,
            self.repayment_phase_dao,
            id,
            &entity,
            REPAYMENT_PHASE_PROCESS_UPDATE,
            &user_id,
            tx
        );

        self.transaction_dao.commit(tx).await?;
        Ok(RepaymentPhase::from(&entity))
    }

    async fn open_repayment_phase(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
    ) -> Result<RepaymentPhase, ServiceError> {
        let tx = self.transaction_dao.use_transaction(None).await?;

        let user_id = self
            .permission_service
            .current_user_id(context.clone())
            .await?
            .unwrap_or_else(|| "SYSTEM".to_string());
        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context)
            .await?;

        // WR-04: see update_repayment_phase. Duplicate read is intentional
        // for the state-transition guard.
        let mut entity = self
            .repayment_phase_dao
            .find_by_id(id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(id))?;

        // D-05 / D-06: state-transition guard. Reverse-transitions
        // (Open→Preparation, Closed→Open) are forbidden — every non-
        // Preparation state returns 409 Conflict. Doppel-Open ist auch
        // 409 (Open→Open ist die Concurrency-Defense für D-03 — open
        // ohne version-check würde sonst beliebig oft idempotent
        // wirken).
        if entity.status != RepaymentPhaseStatus::Preparation {
            return Err(ServiceError::Conflict(Arc::from(format!(
                "Cannot open repayment phase: status is '{}', expected 'Preparation'",
                entity.status.as_str()
            ))));
        }

        let now_offset = time::OffsetDateTime::now_utc();
        let now_pdt = time::PrimitiveDateTime::new(now_offset.date(), now_offset.time());
        entity.status = RepaymentPhaseStatus::Open;
        entity.opened_at = Some(now_pdt);

        crate::audited_update!(
            self,
            self.repayment_phase_dao,
            id,
            &entity,
            REPAYMENT_PHASE_PROCESS_OPEN,
            &user_id,
            tx
        );

        // PHAS-02 (Phase 8): hier wird die Auto-Befüllung der
        // RepaymentEntries ergänzt — pro Member mit aktiven Anteilen
        // wird ein Entry mit amount = share_value * shares angelegt.
        // Phase 7 lässt das skeleton-mäßig leer, weil RepaymentEntry
        // erst in Phase 8 als Entität existiert.

        self.transaction_dao.commit(tx).await?;
        Ok(RepaymentPhase::from(&entity))
    }

    async fn close_repayment_phase(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
    ) -> Result<RepaymentPhase, ServiceError> {
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
            .repayment_phase_dao
            .find_by_id(id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(id))?;

        // D-05 / D-06: only Open → Closed is allowed.
        if entity.status != RepaymentPhaseStatus::Open {
            return Err(ServiceError::Conflict(Arc::from(format!(
                "Cannot close repayment phase: status is '{}', expected 'Open'",
                entity.status.as_str()
            ))));
        }

        // PHAS-03 (Phase 8): hier wird die Validation "alle
        // RepaymentEntries paid_out oder soft-deleted" ergänzt — close
        // blockt mit 409, wenn noch pending Entries existieren. Phase 7
        // schließt skeleton-mäßig ohne diesen Check, weil RepaymentEntry
        // erst in Phase 8 als Entität existiert.

        let now_offset = time::OffsetDateTime::now_utc();
        let now_pdt = time::PrimitiveDateTime::new(now_offset.date(), now_offset.time());
        entity.status = RepaymentPhaseStatus::Closed;
        entity.closed_at = Some(now_pdt);

        crate::audited_update!(
            self,
            self.repayment_phase_dao,
            id,
            &entity,
            REPAYMENT_PHASE_PROCESS_CLOSE,
            &user_id,
            tx
        );

        self.transaction_dao.commit(tx).await?;
        Ok(RepaymentPhase::from(&entity))
    }

    async fn delete_repayment_phase(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
    ) -> Result<(), ServiceError> {
        let tx = self.transaction_dao.use_transaction(None).await?;

        let user_id = self
            .permission_service
            .current_user_id(context.clone())
            .await?
            .unwrap_or_else(|| "SYSTEM".to_string());
        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context)
            .await?;

        // D-09: soft-delete nur in Preparation. Sobald `open` geschah,
        // hängen Audit-Einträge dran (und ab Phase 8 RepaymentEntries) —
        // Löschung würde Audit-Konsistenz brechen.
        let entity = self
            .repayment_phase_dao
            .find_by_id(id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(id))?;
        if entity.status != RepaymentPhaseStatus::Preparation {
            return Err(ServiceError::Conflict(Arc::from(format!(
                "Cannot delete: status is '{}', expected 'Preparation' (D-09)",
                entity.status.as_str()
            ))));
        }

        crate::audited_delete!(
            self,
            self.repayment_phase_dao,
            id,
            REPAYMENT_PHASE_PROCESS_DELETE,
            &user_id,
            tx
        );

        self.transaction_dao.commit(tx).await?;
        Ok(())
    }

    async fn get_repayment_phase(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
    ) -> Result<RepaymentPhase, ServiceError> {
        let tx = self.transaction_dao.use_transaction(None).await?;
        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context)
            .await?;

        let entity = self
            .repayment_phase_dao
            .find_by_id(id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(id))?;

        self.transaction_dao.commit(tx).await?;
        Ok(RepaymentPhase::from(&entity))
    }

    async fn get_all_repayment_phases(
        &self,
        context: Authentication<Self::Context>,
    ) -> Result<Arc<[RepaymentPhase]>, ServiceError> {
        let tx = self.transaction_dao.use_transaction(None).await?;
        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context)
            .await?;

        // DAO-Default-Impl `all()` filtert `deleted IS NULL` per D-10.
        let entities = self.repayment_phase_dao.all(tx.clone()).await?;
        let phases: Arc<[RepaymentPhase]> = entities.iter().map(RepaymentPhase::from).collect();

        self.transaction_dao.commit(tx).await?;
        Ok(phases)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use genossi_dao::audit_log::{AuditLogEntry, AuditQueryFilter};
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
        pub TestRepaymentPhaseDao {}
        #[async_trait]
        impl RepaymentPhaseDao for TestRepaymentPhaseDao {
            type Transaction = TestTransaction;
            async fn dump_all(
                &self,
                tx: TestTransaction,
            ) -> Result<Arc<[RepaymentPhaseEntity]>, DaoError>;
            async fn create(
                &self,
                entity: &RepaymentPhaseEntity,
                process: &str,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn update(
                &self,
                entity: &RepaymentPhaseEntity,
                process: &str,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn all(
                &self,
                tx: TestTransaction,
            ) -> Result<Arc<[RepaymentPhaseEntity]>, DaoError>;
            async fn find_by_id(
                &self,
                id: Uuid,
                tx: TestTransaction,
            ) -> Result<Option<RepaymentPhaseEntity>, DaoError>;
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
    impl RepaymentPhaseServiceDeps for TestDeps {
        type Context = MockContext;
        type Transaction = TestTransaction;
        type RepaymentPhaseDao = MockTestRepaymentPhaseDao;
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

    fn phase_in_status(status: RepaymentPhaseStatus) -> RepaymentPhaseEntity {
        let date = time::Date::from_calendar_date(2026, time::Month::May, 29).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        RepaymentPhaseEntity {
            id: Uuid::new_v4(),
            fiscal_year: 2026,
            share_value: 12000,
            status,
            opened_at: None,
            closed_at: None,
            created: datetime,
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    fn build_service(dao: MockTestRepaymentPhaseDao) -> RepaymentPhaseServiceImpl<TestDeps> {
        RepaymentPhaseServiceImpl {
            repayment_phase_dao: Arc::new(dao),
            audit_log_dao: Arc::new(make_audit_log_dao_quiet()),
            permission_service: Arc::new(make_permission_service_admin_ok()),
            uuid_service: Arc::new(StaticUuidService),
            transaction_dao: Arc::new(setup_mock_tx_dao()),
        }
    }

    // ---------- Validation tests (Test 1-3) ----------

    #[tokio::test]
    async fn test_create_repayment_phase_validation_rejects_fiscal_year_out_of_range() {
        // D-11: fiscal_year=1999 is below the 2000..=2100 range.
        // The DAO-create call MUST NOT be made — validation runs before
        // entity construction.
        let mut dao = MockTestRepaymentPhaseDao::new();
        dao.expect_create().times(0).returning(|_, _, _| Ok(()));

        let service = build_service(dao);

        let submission = RepaymentPhaseSubmission {
            fiscal_year: 1999,
            share_value: 12000,
        };

        let result = service
            .create_repayment_phase(&submission, Authentication::Full)
            .await;

        match result {
            Err(ServiceError::ValidationError(items)) => {
                assert!(
                    items.iter().any(|i| i.field.as_ref() == "fiscal_year"),
                    "expected fiscal_year validation failure, got {:?}",
                    items
                );
            }
            other => panic!("expected ValidationError, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_create_repayment_phase_validation_rejects_share_value_zero() {
        // D-12: share_value=0 is not strictly positive.
        let mut dao = MockTestRepaymentPhaseDao::new();
        dao.expect_create().times(0).returning(|_, _, _| Ok(()));

        let service = build_service(dao);

        let submission = RepaymentPhaseSubmission {
            fiscal_year: 2026,
            share_value: 0,
        };

        let result = service
            .create_repayment_phase(&submission, Authentication::Full)
            .await;

        match result {
            Err(ServiceError::ValidationError(items)) => {
                assert!(
                    items.iter().any(|i| i.field.as_ref() == "share_value"),
                    "expected share_value validation failure, got {:?}",
                    items
                );
            }
            other => panic!("expected ValidationError, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_create_repayment_phase_validation_rejects_share_value_negative() {
        // D-12: share_value=-100 is not strictly positive.
        let mut dao = MockTestRepaymentPhaseDao::new();
        dao.expect_create().times(0).returning(|_, _, _| Ok(()));

        let service = build_service(dao);

        let submission = RepaymentPhaseSubmission {
            fiscal_year: 2026,
            share_value: -100,
        };

        let result = service
            .create_repayment_phase(&submission, Authentication::Full)
            .await;

        match result {
            Err(ServiceError::ValidationError(items)) => {
                assert!(
                    items.iter().any(|i| i.field.as_ref() == "share_value"),
                    "expected share_value validation failure, got {:?}",
                    items
                );
            }
            other => panic!("expected ValidationError, got {:?}", other),
        }
    }

    // ---------- Create happy-path (Test 4) ----------

    #[tokio::test]
    async fn test_create_repayment_phase_success() {
        // Happy-path: fiscal_year=2026 in range, share_value=12000 > 0.
        // DAO-create + AuditLogDao-create are both called exactly once;
        // result is Preparation status with no opened_at/closed_at.
        let mut dao = MockTestRepaymentPhaseDao::new();
        dao.expect_create()
            .times(1)
            .returning(|_, _, _| Ok(()));

        let service = build_service(dao);

        let submission = RepaymentPhaseSubmission {
            fiscal_year: 2026,
            share_value: 12000,
        };

        let result = service
            .create_repayment_phase(&submission, Authentication::Full)
            .await
            .expect("create_repayment_phase should succeed");

        assert_eq!(result.status, RepaymentPhaseStatus::Preparation);
        assert_eq!(result.fiscal_year, 2026);
        assert_eq!(result.share_value, 12000);
        assert!(result.opened_at.is_none());
        assert!(result.closed_at.is_none());
        assert!(result.deleted.is_none());
    }

    // ---------- Update Edit-Matrix tests (Test 5-8) ----------

    #[tokio::test]
    async fn test_update_repayment_phase_in_closed_returns_conflict() {
        // D-04: Closed phase rejects ALL updates atomically.
        let entity = phase_in_status(RepaymentPhaseStatus::Closed);
        let entity_id = entity.id;
        let stored_version = entity.version;

        let mut dao = MockTestRepaymentPhaseDao::new();
        dao.expect_find_by_id()
            .returning(move |_, _| Ok(Some(entity.clone())));
        // No update or create — service short-circuits at Closed-check.
        dao.expect_update().times(0).returning(|_, _, _| Ok(()));

        let service = build_service(dao);

        let update = RepaymentPhaseUpdate {
            fiscal_year: 2026,
            share_value: 15000,
            version: stored_version,
        };

        let result = service
            .update_repayment_phase(entity_id, &update, Authentication::Full)
            .await;

        match result {
            Err(ServiceError::Conflict(msg)) => {
                assert!(
                    msg.contains("Closed"),
                    "expected 'Closed' in message, got: {}",
                    msg
                );
            }
            other => panic!("expected Conflict, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_update_repayment_phase_fiscal_year_change_in_open_returns_conflict() {
        // D-04 / D-07: Open phase allows share_value but NOT fiscal_year.
        // Any change to fiscal_year is atomically rejected — even if the
        // request would also touch share_value, the entire mutation is
        // rejected (D-07 "atomare Ablehnung").
        let entity = phase_in_status(RepaymentPhaseStatus::Open);
        let entity_id = entity.id;
        let stored_version = entity.version;

        let mut dao = MockTestRepaymentPhaseDao::new();
        dao.expect_find_by_id()
            .returning(move |_, _| Ok(Some(entity.clone())));
        dao.expect_update().times(0).returning(|_, _, _| Ok(()));

        let service = build_service(dao);

        let update = RepaymentPhaseUpdate {
            fiscal_year: 2027, // different from stored (2026)
            share_value: 13000,
            version: stored_version,
        };

        let result = service
            .update_repayment_phase(entity_id, &update, Authentication::Full)
            .await;

        match result {
            Err(ServiceError::Conflict(msg)) => {
                assert!(
                    msg.contains("fiscal_year"),
                    "expected 'fiscal_year' in message, got: {}",
                    msg
                );
            }
            other => panic!("expected Conflict, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_update_repayment_phase_share_value_change_in_open_succeeds() {
        // D-04: Open phase allows share_value correction (PHAS-04 +
        // ROADMAP SC#5). audited_update! must be called → DAO.update
        // is invoked exactly once.
        let entity = phase_in_status(RepaymentPhaseStatus::Open);
        let entity_id = entity.id;
        let stored_fiscal_year = entity.fiscal_year;
        let stored_version = entity.version;
        let entity_for_find = entity.clone();

        let mut dao = MockTestRepaymentPhaseDao::new();
        dao.expect_find_by_id()
            .returning(move |_, _| Ok(Some(entity_for_find.clone())));
        dao.expect_update().times(1).returning(|_, _, _| Ok(()));

        let service = build_service(dao);

        let update = RepaymentPhaseUpdate {
            fiscal_year: stored_fiscal_year, // unchanged
            share_value: 13000,              // corrected
            version: stored_version,
        };

        let result = service
            .update_repayment_phase(entity_id, &update, Authentication::Full)
            .await
            .expect("update should succeed in Open when only share_value changes");

        assert_eq!(result.share_value, 13000);
        assert_eq!(result.fiscal_year, stored_fiscal_year);
        assert_eq!(result.status, RepaymentPhaseStatus::Open);
    }

    #[tokio::test]
    async fn test_update_repayment_phase_version_mismatch_returns_conflict() {
        // Optimistic locking: a stale version → Conflict("Version mismatch")
        // BEFORE any mutation.
        let entity = phase_in_status(RepaymentPhaseStatus::Preparation);
        let entity_id = entity.id;
        let stale_version = Uuid::new_v4();
        assert_ne!(stale_version, entity.version);

        let mut dao = MockTestRepaymentPhaseDao::new();
        dao.expect_find_by_id()
            .returning(move |_, _| Ok(Some(entity.clone())));
        dao.expect_update().times(0).returning(|_, _, _| Ok(()));

        let service = build_service(dao);

        let update = RepaymentPhaseUpdate {
            fiscal_year: 2026,
            share_value: 13000,
            version: stale_version,
        };

        let result = service
            .update_repayment_phase(entity_id, &update, Authentication::Full)
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

    // ---------- Lifecycle Guard tests (Test 9-11) ----------

    #[tokio::test]
    async fn test_open_repayment_phase_from_open_returns_conflict() {
        // D-05/D-06: doppel-open ist 409. Open ist auch eine
        // Concurrency-Defense für D-03 (open ohne version-Check würde
        // sonst beliebig oft idempotent wirken).
        let entity = phase_in_status(RepaymentPhaseStatus::Open);
        let entity_id = entity.id;

        let mut dao = MockTestRepaymentPhaseDao::new();
        dao.expect_find_by_id()
            .returning(move |_, _| Ok(Some(entity.clone())));
        dao.expect_update().times(0).returning(|_, _, _| Ok(()));

        let service = build_service(dao);

        let result = service
            .open_repayment_phase(entity_id, Authentication::Full)
            .await;

        match result {
            Err(ServiceError::Conflict(msg)) => {
                assert!(
                    msg.contains("Open"),
                    "expected 'Open' in message, got: {}",
                    msg
                );
            }
            other => panic!("expected Conflict, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_open_repayment_phase_from_closed_returns_conflict() {
        // D-06: Reverse-Transition Closed → Open ist verboten.
        let entity = phase_in_status(RepaymentPhaseStatus::Closed);
        let entity_id = entity.id;

        let mut dao = MockTestRepaymentPhaseDao::new();
        dao.expect_find_by_id()
            .returning(move |_, _| Ok(Some(entity.clone())));
        dao.expect_update().times(0).returning(|_, _, _| Ok(()));

        let service = build_service(dao);

        let result = service
            .open_repayment_phase(entity_id, Authentication::Full)
            .await;

        match result {
            Err(ServiceError::Conflict(_)) => {}
            other => panic!("expected Conflict, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_close_repayment_phase_from_preparation_returns_conflict() {
        // D-05: nur Open → Closed ist erlaubt.
        let entity = phase_in_status(RepaymentPhaseStatus::Preparation);
        let entity_id = entity.id;

        let mut dao = MockTestRepaymentPhaseDao::new();
        dao.expect_find_by_id()
            .returning(move |_, _| Ok(Some(entity.clone())));
        dao.expect_update().times(0).returning(|_, _, _| Ok(()));

        let service = build_service(dao);

        let result = service
            .close_repayment_phase(entity_id, Authentication::Full)
            .await;

        match result {
            Err(ServiceError::Conflict(_)) => {}
            other => panic!("expected Conflict, got {:?}", other),
        }
    }

    // ---------- Delete Guard tests (Test 12-13) ----------

    #[tokio::test]
    async fn test_delete_repayment_phase_in_open_returns_conflict() {
        // D-09: Soft-Delete nur in Preparation.
        let entity = phase_in_status(RepaymentPhaseStatus::Open);
        let entity_id = entity.id;

        let mut dao = MockTestRepaymentPhaseDao::new();
        dao.expect_find_by_id()
            .returning(move |_, _| Ok(Some(entity.clone())));
        dao.expect_update().times(0).returning(|_, _, _| Ok(()));

        let service = build_service(dao);

        let result = service
            .delete_repayment_phase(entity_id, Authentication::Full)
            .await;

        match result {
            Err(ServiceError::Conflict(msg)) => {
                assert!(
                    msg.contains("D-09"),
                    "expected 'D-09' in message, got: {}",
                    msg
                );
            }
            other => panic!("expected Conflict, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_delete_repayment_phase_in_preparation_succeeds() {
        // D-09 happy-path: Preparation → soft-delete via audited_delete!
        // which calls DAO.update with deleted=Some(_).
        let entity = phase_in_status(RepaymentPhaseStatus::Preparation);
        let entity_id = entity.id;
        let entity_for_find = entity.clone();

        let mut dao = MockTestRepaymentPhaseDao::new();
        dao.expect_find_by_id()
            .returning(move |_, _| Ok(Some(entity_for_find.clone())));
        // audited_delete! sets deleted=Some(_) and calls DAO.update once.
        dao.expect_update()
            .times(1)
            .withf(|entity: &RepaymentPhaseEntity, _process, _tx| entity.deleted.is_some())
            .returning(|_, _, _| Ok(()));

        let service = build_service(dao);

        let result = service
            .delete_repayment_phase(entity_id, Authentication::Full)
            .await;
        assert!(
            result.is_ok(),
            "delete_repayment_phase in Preparation should succeed, got {:?}",
            result
        );
    }
}
