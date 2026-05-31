//! `RepaymentEntryServiceImpl` — Service-Layer für RepaymentEntry-CRUD +
//! Batch-Toggle (Phase 8 Plan 03).
//!
//! Edit-Matrix:
//! - **create**: nur wenn Phase.status == Open (D-11.1), Member existiert
//!   (D-11.2), share_count ∈ (0, Member.current_shares] (D-11.3, ENTR-02)
//! - **update share_count**: nur wenn entry.status ∈ {Open, Contacted} (ENTR-04)
//! - **update status**: Open ↔ Contacted bidirektional (D-06); PaidOut als
//!   target → 409 (D-05, "use Phase-9 mark_paid_out endpoint")
//! - **delete**: nur wenn entry.status != PaidOut (ENTR-05) — Pre-Check lädt
//!   Entity manuell vor `audited_delete!`, weil das Macro keinen Status-Guard hat
//! - **batch_toggle**: all-or-nothing in 1 Tx (D-08); PaidOut als target → 400
//!   (D-07); 409-Body ist strukturiertes JSON `{ failure_index, failure_id,
//!   failure_reason }` analog `CloseConflictResponse`-Pattern (Plan 04/05)
//!
//! Audit-Disziplin: ALLE Schreib-Operationen via `audited_create!` /
//! `audited_update!` / `audited_delete!`. KEIN direkter
//! `repayment_entry_dao.create/update`-Aufruf (Grep-Gate vor Merge, T-08-03-01).
//!
//! Imports-Konvention (Phase-7-Lektion, Checker-Review B-02): `ServiceError`
//! und `ValidationFailureItem` aus `genossi_service` importieren — NICHT aus
//! `genossi_dao` (dort nicht definiert).

use async_trait::async_trait;
use genossi_dao::audit_log::AuditLogDao;
use genossi_dao::member::MemberDao;
use genossi_dao::repayment_entry::{RepaymentEntryDao, RepaymentEntryEntity, RepaymentEntryStatus};
use genossi_dao::repayment_phase::{RepaymentPhaseDao, RepaymentPhaseStatus};
use genossi_dao::TransactionDao;
use genossi_service::permission::{Authentication, PermissionService};
use genossi_service::repayment_entry::{
    RepaymentEntry, RepaymentEntryBatchStatusInput, RepaymentEntryService,
    RepaymentEntrySubmission, RepaymentEntryUpdate,
};
use genossi_service::uuid_service::UuidService;
use genossi_service::{ServiceError, ValidationFailureItem};
use std::sync::Arc;
use uuid::Uuid;

use crate::gen_service_impl;

const REPAYMENT_ENTRY_PROCESS_CREATE: &str = "repayment-entry.create";
const REPAYMENT_ENTRY_PROCESS_UPDATE: &str = "repayment-entry.update";
const REPAYMENT_ENTRY_PROCESS_DELETE: &str = "repayment-entry.delete";
const REPAYMENT_ENTRY_PROCESS_BATCH_TOGGLE: &str = "repayment-entry.batch-toggle";
const ADMIN_PRIVILEGE: &str = "admin";

gen_service_impl! {
    struct RepaymentEntryServiceImpl: RepaymentEntryService = RepaymentEntryServiceDeps {
        RepaymentEntryDao: RepaymentEntryDao<Transaction = Self::Transaction> = repayment_entry_dao,
        RepaymentPhaseDao: RepaymentPhaseDao<Transaction = Self::Transaction> = repayment_phase_dao,
        MemberDao: MemberDao<Transaction = Self::Transaction> = member_dao,
        AuditLogDao: AuditLogDao<Transaction = Self::Transaction> = audit_log_dao,
        PermissionService: PermissionService<Context = Self::Context> = permission_service,
        UuidService: UuidService = uuid_service,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
    }
}

/// Range-Validator für `share_count_to_pay_out` (D-11.3, ENTR-02).
///
/// Wird sowohl in `create_repayment_entry` als auch in `update_repayment_entry`
/// (wenn `share_count_to_pay_out` editiert wird) verwendet.
///
/// Pattern: Inline-Validator analog Phase-7-`validate_phase_fields` (Plan 07-03
/// D-04: `validation.rs`-Refactor bewusst NICHT, weil dort anderer Concern).
fn validate_entry_create(
    share_count_to_pay_out: i32,
    member_current_shares: i32,
) -> Result<(), ServiceError> {
    let mut errors: Vec<ValidationFailureItem> = Vec::new();
    if share_count_to_pay_out <= 0 {
        errors.push(ValidationFailureItem {
            field: Arc::from("share_count_to_pay_out"),
            message: Arc::from(format!("must be > 0, got {}", share_count_to_pay_out)),
        });
    }
    if share_count_to_pay_out > member_current_shares {
        errors.push(ValidationFailureItem {
            field: Arc::from("share_count_to_pay_out"),
            message: Arc::from(format!(
                "must be <= member current_shares ({}), got {}",
                member_current_shares, share_count_to_pay_out
            )),
        });
    }
    if !errors.is_empty() {
        return Err(ServiceError::ValidationError(errors));
    }
    Ok(())
}

#[async_trait]
impl<Deps: RepaymentEntryServiceDeps> RepaymentEntryService for RepaymentEntryServiceImpl<Deps> {
    type Context = Deps::Context;
    type Transaction = Deps::Transaction;

    async fn create_repayment_entry(
        &self,
        submission: &RepaymentEntrySubmission,
        context: Authentication<Self::Context>,
    ) -> Result<RepaymentEntry, ServiceError> {
        let tx = self.transaction_dao.use_transaction(None).await?;

        let user_id = self
            .permission_service
            .current_user_id(context.clone())
            .await?
            .unwrap_or_else(|| "SYSTEM".to_string());
        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context)
            .await?;

        // D-11.1: Phase laden + Status-Check
        let phase = self
            .repayment_phase_dao
            .find_by_id(submission.phase_id, tx.clone())
            .await?
            .ok_or_else(|| {
                ServiceError::Conflict(Arc::from(format!(
                    "Phase {} not found",
                    submission.phase_id
                )))
            })?;
        if phase.status != RepaymentPhaseStatus::Open {
            return Err(ServiceError::Conflict(Arc::from(format!(
                "Phase status is '{}', expected 'Open' (D-11.1)",
                phase.status.as_str()
            ))));
        }

        // D-11.2: Member laden (find_by_id filtert deleted IS NULL per Default-Impl)
        let member = self
            .member_dao
            .find_by_id(submission.member_id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(submission.member_id))?;

        // D-11.3: Range-Check (>0 AND ≤ Member.current_shares)
        validate_entry_create(submission.share_count_to_pay_out, member.current_shares)?;

        let now = time::OffsetDateTime::now_utc();
        let created = time::PrimitiveDateTime::new(now.date(), now.time());
        let entity = RepaymentEntryEntity {
            id: self.uuid_service.new_v4().await,
            member_id: submission.member_id,
            phase_id: submission.phase_id,
            share_count_to_pay_out: submission.share_count_to_pay_out,
            status: RepaymentEntryStatus::Open, // D-05: default
            created,
            deleted: None,
            version: self.uuid_service.new_v4().await,
        };

        // audited via macro (Audit-Disziplin T-08-03-01)
        crate::audited_create!(
            self,
            self.repayment_entry_dao,
            &entity,
            REPAYMENT_ENTRY_PROCESS_CREATE,
            &user_id,
            tx
        );

        self.transaction_dao.commit(tx).await?;
        Ok(RepaymentEntry::from(&entity))
    }

    async fn update_repayment_entry(
        &self,
        id: Uuid,
        update: &RepaymentEntryUpdate,
        context: Authentication<Self::Context>,
    ) -> Result<RepaymentEntry, ServiceError> {
        let tx = self.transaction_dao.use_transaction(None).await?;

        let user_id = self
            .permission_service
            .current_user_id(context.clone())
            .await?
            .unwrap_or_else(|| "SYSTEM".to_string());
        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context)
            .await?;

        // WR-04: duplicate find_by_id intentional — needed for Edit-Matrix &
        // version-check BEFORE mutation. audited_update! lädt nochmal intern.
        let mut entity = self
            .repayment_entry_dao
            .find_by_id(id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(id))?;

        // D-05: PaidOut ist final
        if entity.status == RepaymentEntryStatus::PaidOut {
            return Err(ServiceError::Conflict(Arc::from(
                "Cannot update: entry is PaidOut; final per PAYO-04 (Phase 9)",
            )));
        }

        // D-05 / D-07: target_status=PaidOut via PUT → 409
        if let Some(ref target) = update.status {
            if *target == RepaymentEntryStatus::PaidOut {
                return Err(ServiceError::Conflict(Arc::from(
                    "PaidOut transition must use Phase-9 mark_paid_out endpoint (D-05)",
                )));
            }
        }

        // Optimistic locking — version must match persisted snapshot
        if entity.version != update.version {
            return Err(ServiceError::Conflict(Arc::from("Version mismatch")));
        }

        // ENTR-04: share_count-Edit nur wenn status ∈ {Open, Contacted}
        if let Some(new_count) = update.share_count_to_pay_out {
            if !matches!(
                entity.status,
                RepaymentEntryStatus::Open | RepaymentEntryStatus::Contacted
            ) {
                return Err(ServiceError::Conflict(Arc::from(format!(
                    "Cannot edit share_count: entry status is '{}' (ENTR-04)",
                    entity.status.as_str()
                ))));
            }

            // Range-Check gegen aktuelle Member.current_shares
            let member = self
                .member_dao
                .find_by_id(entity.member_id, tx.clone())
                .await?
                .ok_or(ServiceError::EntityNotFound(entity.member_id))?;
            validate_entry_create(new_count, member.current_shares)?;

            entity.share_count_to_pay_out = new_count;
        }

        // D-06: Status-Toggle Open ↔ Contacted bidirektional
        if let Some(target) = update.status.clone() {
            // PaidOut wurde oben bereits abgelehnt; hier dürfen nur
            // Open/Contacted ankommen. Verteidigung in Depth:
            if !matches!(
                target,
                RepaymentEntryStatus::Open | RepaymentEntryStatus::Contacted
            ) {
                return Err(ServiceError::Conflict(Arc::from(format!(
                    "Invalid status target '{}'; only Open and Contacted allowed via PUT (D-06)",
                    target.as_str()
                ))));
            }
            entity.status = target;
        }

        // audited via macro (Audit-Disziplin T-08-03-01)
        crate::audited_update!(
            self,
            self.repayment_entry_dao,
            id,
            &entity,
            REPAYMENT_ENTRY_PROCESS_UPDATE,
            &user_id,
            tx
        );

        self.transaction_dao.commit(tx).await?;
        Ok(RepaymentEntry::from(&entity))
    }

    async fn delete_repayment_entry(
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

        // Pre-Check (ENTR-05): Entity manuell laden für Status-Guard BEVOR
        // audited_delete! sie nochmal lädt. Das audited_delete!-Macro
        // (audit_macros.rs:86) hat 6 Argumente und lädt Entity intern, hat
        // aber keinen Status-Guard — der muss hier vor dem Macro laufen.
        let entity = self
            .repayment_entry_dao
            .find_by_id(id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(id))?;
        if entity.status == RepaymentEntryStatus::PaidOut {
            return Err(ServiceError::Conflict(Arc::from(
                "Cannot delete: entry is PaidOut (ENTR-05)",
            )));
        }

        // audited via macro (Audit-Disziplin T-08-03-01); 6-Arg-Signatur
        crate::audited_delete!(
            self,
            self.repayment_entry_dao,
            id,
            REPAYMENT_ENTRY_PROCESS_DELETE,
            &user_id,
            tx
        );

        self.transaction_dao.commit(tx).await?;
        Ok(())
    }

    async fn get_repayment_entry(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
    ) -> Result<RepaymentEntry, ServiceError> {
        let tx = self.transaction_dao.use_transaction(None).await?;
        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context)
            .await?;

        let entity = self
            .repayment_entry_dao
            .find_by_id(id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(id))?;

        self.transaction_dao.commit(tx).await?;
        Ok(RepaymentEntry::from(&entity))
    }

    async fn list_repayment_entries_by_phase(
        &self,
        phase_id: Uuid,
        context: Authentication<Self::Context>,
    ) -> Result<Arc<[RepaymentEntry]>, ServiceError> {
        let tx = self.transaction_dao.use_transaction(None).await?;
        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context)
            .await?;

        // DAO-Default-Impl filtert phase_id + deleted IS NULL
        let entities = self
            .repayment_entry_dao
            .find_by_phase_id(phase_id, tx.clone())
            .await?;

        self.transaction_dao.commit(tx).await?;
        let domain: Vec<RepaymentEntry> = entities.iter().map(RepaymentEntry::from).collect();
        Ok(domain.into())
    }

    async fn batch_toggle_status(
        &self,
        input: &RepaymentEntryBatchStatusInput,
        context: Authentication<Self::Context>,
    ) -> Result<Arc<[RepaymentEntry]>, ServiceError> {
        let tx = self.transaction_dao.use_transaction(None).await?;

        let user_id = self
            .permission_service
            .current_user_id(context.clone())
            .await?
            .unwrap_or_else(|| "SYSTEM".to_string());
        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context)
            .await?;

        // Helper: strukturierter 409-Body (D-08; analog CloseConflictResponse-Pattern)
        let conflict_body = |idx: usize, entry_id: Uuid, reason: &str| -> ServiceError {
            let detail = serde_json::json!({
                "failure_index": idx,
                "failure_id": entry_id.to_string(),
                "failure_reason": reason,
            });
            ServiceError::Conflict(Arc::from(detail.to_string()))
        };

        // D-07: PaidOut als target → 400 ValidationError
        if input.target_status == RepaymentEntryStatus::PaidOut {
            return Err(ServiceError::ValidationError(vec![ValidationFailureItem {
                field: Arc::from("target_status"),
                message: Arc::from(
                    "PaidOut not allowed via batch-status; use Phase-9 mark_paid_out (D-07)",
                ),
            }]));
        }

        // D-06: nur Open/Contacted als Target
        if !matches!(
            input.target_status,
            RepaymentEntryStatus::Open | RepaymentEntryStatus::Contacted
        ) {
            return Err(ServiceError::ValidationError(vec![ValidationFailureItem {
                field: Arc::from("target_status"),
                message: Arc::from(format!(
                    "Only Open and Contacted allowed as batch target_status, got '{}'",
                    input.target_status.as_str()
                )),
            }]));
        }

        let mut updated: Vec<RepaymentEntry> = Vec::with_capacity(input.entry_ids.len());

        // D-08: All-or-nothing in EINER Tx; erster Fehler → strukturierter
        // JSON-Conflict + Tx-Drop = Rollback (audited_update wurde für
        // vorherige Entries committet, aber Tx wurde nicht committed →
        // SQLite verwirft alles bei Drop).
        for (idx, entry_id) in input.entry_ids.iter().enumerate() {
            let mut entity = self
                .repayment_entry_dao
                .find_by_id(*entry_id, tx.clone())
                .await?
                .ok_or_else(|| conflict_body(idx, *entry_id, "entry not found"))?;

            // Source-Status muss Open oder Contacted sein
            if !matches!(
                entity.status,
                RepaymentEntryStatus::Open | RepaymentEntryStatus::Contacted
            ) {
                return Err(conflict_body(
                    idx,
                    *entry_id,
                    &format!(
                        "source status is '{}', expected Open or Contacted",
                        entity.status.as_str()
                    ),
                ));
            }

            entity.status = input.target_status.clone();
            // audited via macro (Audit-Disziplin T-08-03-01)
            crate::audited_update!(
                self,
                self.repayment_entry_dao,
                *entry_id,
                &entity,
                REPAYMENT_ENTRY_PROCESS_BATCH_TOGGLE,
                &user_id,
                tx
            );
            updated.push(RepaymentEntry::from(&entity));
        }

        self.transaction_dao.commit(tx).await?;
        Ok(updated.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use genossi_dao::audit_log::{AuditLogEntry, AuditQueryFilter};
    use genossi_dao::member::{MemberEntity, MemberStatus};
    use genossi_dao::repayment_phase::RepaymentPhaseEntity;
    use genossi_dao::{DaoError, Transaction};
    use genossi_service::permission::MockContext;
    use mockall::mock;

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
        pub TestRepaymentEntryDao {}
        #[async_trait]
        impl RepaymentEntryDao for TestRepaymentEntryDao {
            type Transaction = TestTransaction;
            async fn dump_all(
                &self,
                tx: TestTransaction,
            ) -> Result<Arc<[RepaymentEntryEntity]>, DaoError>;
            async fn create(
                &self,
                entity: &RepaymentEntryEntity,
                process: &str,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn update(
                &self,
                entity: &RepaymentEntryEntity,
                process: &str,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn all(
                &self,
                tx: TestTransaction,
            ) -> Result<Arc<[RepaymentEntryEntity]>, DaoError>;
            async fn find_by_id(
                &self,
                id: Uuid,
                tx: TestTransaction,
            ) -> Result<Option<RepaymentEntryEntity>, DaoError>;
            async fn find_by_phase_id(
                &self,
                phase_id: Uuid,
                tx: TestTransaction,
            ) -> Result<Arc<[RepaymentEntryEntity]>, DaoError>;
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
        pub TestMemberDao {}
        #[async_trait]
        impl MemberDao for TestMemberDao {
            type Transaction = TestTransaction;
            async fn dump_all(
                &self,
                tx: TestTransaction,
            ) -> Result<Arc<[MemberEntity]>, DaoError>;
            async fn create(
                &self,
                entity: &MemberEntity,
                process: &str,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn update(
                &self,
                entity: &MemberEntity,
                process: &str,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn all(
                &self,
                tx: TestTransaction,
            ) -> Result<Arc<[MemberEntity]>, DaoError>;
            async fn find_by_id(
                &self,
                id: Uuid,
                tx: TestTransaction,
            ) -> Result<Option<MemberEntity>, DaoError>;
            async fn update_migrated(
                &self,
                id: Uuid,
                migrated: bool,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
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
            async fn count_active(
                &self,
                today: time::Date,
                tx: TestTransaction,
            ) -> Result<u64, DaoError>;
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

    struct TestDeps;
    impl RepaymentEntryServiceDeps for TestDeps {
        type Context = MockContext;
        type Transaction = TestTransaction;
        type RepaymentEntryDao = MockTestRepaymentEntryDao;
        type RepaymentPhaseDao = MockTestRepaymentPhaseDao;
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

    fn make_permission_service_admin_denied() -> MockTestPermissionService {
        let mut p = MockTestPermissionService::new();
        p.expect_current_user_id()
            .returning(|_| Ok(Some("user".to_string())));
        p.expect_check_permission()
            .returning(|_, _| Err(ServiceError::PermissionDenied));
        p
    }

    fn make_audit_log_dao_quiet() -> MockTestAuditLogDao {
        let mut dao = MockTestAuditLogDao::new();
        dao.expect_get_latest_hash().returning(|_| Ok(None));
        dao.expect_create_entries().returning(|_, _| Ok(()));
        dao
    }

    fn make_test_datetime() -> time::PrimitiveDateTime {
        let date = time::Date::from_calendar_date(2026, time::Month::May, 30).unwrap();
        time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT)
    }

    fn entry_in_status(
        member_id: Uuid,
        phase_id: Uuid,
        status: RepaymentEntryStatus,
        share_count_to_pay_out: i32,
    ) -> RepaymentEntryEntity {
        RepaymentEntryEntity {
            id: Uuid::new_v4(),
            member_id,
            phase_id,
            share_count_to_pay_out,
            status,
            created: make_test_datetime(),
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    fn phase_in_status(status: RepaymentPhaseStatus) -> RepaymentPhaseEntity {
        RepaymentPhaseEntity {
            id: Uuid::new_v4(),
            fiscal_year: 2026,
            share_value: 12000,
            status,
            opened_at: None,
            closed_at: None,
            created: make_test_datetime(),
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    fn member_with_current_shares(current_shares: i32) -> MemberEntity {
        let date = time::Date::from_calendar_date(2020, time::Month::January, 1).unwrap();
        MemberEntity {
            id: Uuid::new_v4(),
            member_number: 1,
            first_name: Arc::from("Test"),
            last_name: Arc::from("Member"),
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
            current_shares,
            current_balance: 0,
            action_count: 0,
            migrated: false,
            exit_date: None,
            bank_account: None,
            status: MemberStatus::Normal,
            created: make_test_datetime(),
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    /// Build a complete service with full deps.
    fn build_service(
        entry_dao: MockTestRepaymentEntryDao,
        phase_dao: MockTestRepaymentPhaseDao,
        member_dao: MockTestMemberDao,
        perm_service: MockTestPermissionService,
    ) -> RepaymentEntryServiceImpl<TestDeps> {
        RepaymentEntryServiceImpl {
            repayment_entry_dao: Arc::new(entry_dao),
            repayment_phase_dao: Arc::new(phase_dao),
            member_dao: Arc::new(member_dao),
            audit_log_dao: Arc::new(make_audit_log_dao_quiet()),
            permission_service: Arc::new(perm_service),
            uuid_service: Arc::new(StaticUuidService),
            transaction_dao: Arc::new(setup_mock_tx_dao()),
        }
    }

    /// Build a service with admin permission ok.
    fn build_service_admin(
        entry_dao: MockTestRepaymentEntryDao,
        phase_dao: MockTestRepaymentPhaseDao,
        member_dao: MockTestMemberDao,
    ) -> RepaymentEntryServiceImpl<TestDeps> {
        build_service(
            entry_dao,
            phase_dao,
            member_dao,
            make_permission_service_admin_ok(),
        )
    }

    // ---------- Create Validation Tests (D-11) ----------

    #[tokio::test]
    async fn test_create_entry_rejects_when_phase_not_open() {
        // D-11.1: Phase im Status Preparation → 409 Conflict ("Phase status").
        let phase = phase_in_status(RepaymentPhaseStatus::Preparation);
        let phase_id = phase.id;
        let member_id = Uuid::new_v4();

        let mut entry_dao = MockTestRepaymentEntryDao::new();
        entry_dao
            .expect_create()
            .times(0)
            .returning(|_, _, _| Ok(()));

        let mut phase_dao = MockTestRepaymentPhaseDao::new();
        phase_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(phase.clone())));

        let member_dao = MockTestMemberDao::new();

        let service = build_service_admin(entry_dao, phase_dao, member_dao);

        let submission = RepaymentEntrySubmission {
            phase_id,
            member_id,
            share_count_to_pay_out: 2,
        };
        let result = service
            .create_repayment_entry(&submission, Authentication::Full)
            .await;

        match result {
            Err(ServiceError::Conflict(msg)) => {
                assert!(
                    msg.contains("Phase status"),
                    "expected 'Phase status' in message, got: {}",
                    msg
                );
            }
            other => panic!("expected Conflict, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_create_entry_rejects_when_member_not_found() {
        // D-11.2: Phase Open, aber Member existiert nicht → EntityNotFound.
        let phase = phase_in_status(RepaymentPhaseStatus::Open);
        let phase_id = phase.id;
        let member_id = Uuid::new_v4();

        let mut entry_dao = MockTestRepaymentEntryDao::new();
        entry_dao
            .expect_create()
            .times(0)
            .returning(|_, _, _| Ok(()));

        let mut phase_dao = MockTestRepaymentPhaseDao::new();
        phase_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(phase.clone())));

        let mut member_dao = MockTestMemberDao::new();
        // mockall überschreibt die DAO-Default-Impl von find_by_id — wir
        // müssen find_by_id direkt mocken (nicht über dump_all).
        member_dao.expect_find_by_id().returning(|_, _| Ok(None));

        let service = build_service_admin(entry_dao, phase_dao, member_dao);

        let submission = RepaymentEntrySubmission {
            phase_id,
            member_id,
            share_count_to_pay_out: 2,
        };
        let result = service
            .create_repayment_entry(&submission, Authentication::Full)
            .await;

        match result {
            Err(ServiceError::EntityNotFound(id)) => {
                assert_eq!(id, member_id);
            }
            other => panic!("expected EntityNotFound, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_create_entry_validation_rejects_share_count_zero_or_negative() {
        // D-11.3: share_count <= 0 → ValidationError.
        let phase = phase_in_status(RepaymentPhaseStatus::Open);
        let phase_id = phase.id;
        let member = member_with_current_shares(10);
        let member_id = member.id;

        let mut entry_dao = MockTestRepaymentEntryDao::new();
        entry_dao
            .expect_create()
            .times(0)
            .returning(|_, _, _| Ok(()));

        let mut phase_dao = MockTestRepaymentPhaseDao::new();
        phase_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(phase.clone())));

        let mut member_dao = MockTestMemberDao::new();
        let member_for_find = member.clone();
        member_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(member_for_find.clone())));

        let service = build_service_admin(entry_dao, phase_dao, member_dao);

        // share_count = 0
        let submission = RepaymentEntrySubmission {
            phase_id,
            member_id,
            share_count_to_pay_out: 0,
        };
        let result = service
            .create_repayment_entry(&submission, Authentication::Full)
            .await;

        match result {
            Err(ServiceError::ValidationError(items)) => {
                assert!(items
                    .iter()
                    .any(|i| i.field.as_ref() == "share_count_to_pay_out"));
            }
            other => panic!("expected ValidationError, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_create_entry_validation_rejects_share_count_exceeds_member_current_shares() {
        // D-11.3: share_count > member.current_shares → ValidationError.
        let phase = phase_in_status(RepaymentPhaseStatus::Open);
        let phase_id = phase.id;
        let member = member_with_current_shares(5);
        let member_id = member.id;

        let mut entry_dao = MockTestRepaymentEntryDao::new();
        entry_dao
            .expect_create()
            .times(0)
            .returning(|_, _, _| Ok(()));

        let mut phase_dao = MockTestRepaymentPhaseDao::new();
        phase_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(phase.clone())));

        let mut member_dao = MockTestMemberDao::new();
        let member_for_find = member.clone();
        member_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(member_for_find.clone())));

        let service = build_service_admin(entry_dao, phase_dao, member_dao);

        // share_count = 6 > current_shares = 5
        let submission = RepaymentEntrySubmission {
            phase_id,
            member_id,
            share_count_to_pay_out: 6,
        };
        let result = service
            .create_repayment_entry(&submission, Authentication::Full)
            .await;

        match result {
            Err(ServiceError::ValidationError(items)) => {
                assert!(items.iter().any(|i| {
                    i.field.as_ref() == "share_count_to_pay_out"
                        && i.message.as_ref().contains("current_shares")
                }));
            }
            other => panic!("expected ValidationError, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_create_entry_success() {
        // Happy-Path: Phase Open + Member existiert + share_count in range
        // → audited_create wird genau 1x aufgerufen, Status = Open.
        let phase = phase_in_status(RepaymentPhaseStatus::Open);
        let phase_id = phase.id;
        let member = member_with_current_shares(10);
        let member_id = member.id;

        let mut entry_dao = MockTestRepaymentEntryDao::new();
        entry_dao
            .expect_create()
            .times(1)
            .returning(|_, _, _| Ok(()));

        let mut phase_dao = MockTestRepaymentPhaseDao::new();
        phase_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(phase.clone())));

        let mut member_dao = MockTestMemberDao::new();
        let member_for_find = member.clone();
        member_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(member_for_find.clone())));

        let service = build_service_admin(entry_dao, phase_dao, member_dao);

        let submission = RepaymentEntrySubmission {
            phase_id,
            member_id,
            share_count_to_pay_out: 3,
        };
        let result = service
            .create_repayment_entry(&submission, Authentication::Full)
            .await
            .expect("create should succeed");

        assert_eq!(result.member_id, member_id);
        assert_eq!(result.phase_id, phase_id);
        assert_eq!(result.share_count_to_pay_out, 3);
        assert_eq!(result.status, RepaymentEntryStatus::Open);
        assert!(result.deleted.is_none());
    }

    // ---------- Update Edit-Matrix Tests (D-05/D-06/ENTR-04) ----------

    #[tokio::test]
    async fn test_update_entry_paid_out_returns_conflict() {
        // D-05: PaidOut ist final → Update verboten ("Cannot update").
        let entity = entry_in_status(
            Uuid::new_v4(),
            Uuid::new_v4(),
            RepaymentEntryStatus::PaidOut,
            5,
        );
        let entity_id = entity.id;
        let stored_version = entity.version;

        let mut entry_dao = MockTestRepaymentEntryDao::new();
        entry_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(entity.clone())));
        entry_dao
            .expect_update()
            .times(0)
            .returning(|_, _, _| Ok(()));

        let phase_dao = MockTestRepaymentPhaseDao::new();
        let member_dao = MockTestMemberDao::new();

        let service = build_service_admin(entry_dao, phase_dao, member_dao);

        let update = RepaymentEntryUpdate {
            share_count_to_pay_out: Some(3),
            status: None,
            version: stored_version,
        };
        let result = service
            .update_repayment_entry(entity_id, &update, Authentication::Full)
            .await;

        match result {
            Err(ServiceError::Conflict(msg)) => {
                assert!(
                    msg.contains("PaidOut"),
                    "expected 'PaidOut' in message, got: {}",
                    msg
                );
            }
            other => panic!("expected Conflict, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_update_entry_status_to_paid_out_via_put_returns_conflict() {
        // D-05/D-07: target_status = PaidOut via PUT → 409 (Phase-9-Endpoint Hinweis).
        let entity = entry_in_status(
            Uuid::new_v4(),
            Uuid::new_v4(),
            RepaymentEntryStatus::Open,
            5,
        );
        let entity_id = entity.id;
        let stored_version = entity.version;

        let mut entry_dao = MockTestRepaymentEntryDao::new();
        entry_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(entity.clone())));
        entry_dao
            .expect_update()
            .times(0)
            .returning(|_, _, _| Ok(()));

        let phase_dao = MockTestRepaymentPhaseDao::new();
        let member_dao = MockTestMemberDao::new();

        let service = build_service_admin(entry_dao, phase_dao, member_dao);

        let update = RepaymentEntryUpdate {
            share_count_to_pay_out: None,
            status: Some(RepaymentEntryStatus::PaidOut),
            version: stored_version,
        };
        let result = service
            .update_repayment_entry(entity_id, &update, Authentication::Full)
            .await;

        match result {
            Err(ServiceError::Conflict(msg)) => {
                assert!(
                    msg.contains("Phase-9") || msg.contains("mark_paid_out"),
                    "expected Phase-9/mark_paid_out hint, got: {}",
                    msg
                );
            }
            other => panic!("expected Conflict, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_update_entry_status_open_to_contacted_succeeds() {
        // D-06: Open → Contacted erlaubt; audited_update wird 1x aufgerufen.
        let entity = entry_in_status(
            Uuid::new_v4(),
            Uuid::new_v4(),
            RepaymentEntryStatus::Open,
            5,
        );
        let entity_id = entity.id;
        let stored_version = entity.version;
        let entity_for_find = entity.clone();

        let mut entry_dao = MockTestRepaymentEntryDao::new();
        entry_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(entity_for_find.clone())));
        entry_dao
            .expect_update()
            .times(1)
            .withf(|e: &RepaymentEntryEntity, _process, _tx| {
                e.status == RepaymentEntryStatus::Contacted
            })
            .returning(|_, _, _| Ok(()));

        let phase_dao = MockTestRepaymentPhaseDao::new();
        let member_dao = MockTestMemberDao::new();

        let service = build_service_admin(entry_dao, phase_dao, member_dao);

        let update = RepaymentEntryUpdate {
            share_count_to_pay_out: None,
            status: Some(RepaymentEntryStatus::Contacted),
            version: stored_version,
        };
        let result = service
            .update_repayment_entry(entity_id, &update, Authentication::Full)
            .await
            .expect("update Open→Contacted should succeed");

        assert_eq!(result.status, RepaymentEntryStatus::Contacted);
    }

    #[tokio::test]
    async fn test_update_entry_status_contacted_to_open_succeeds() {
        // D-06 bidirektional: Contacted → Open auch erlaubt (z.B. Mail-Korrektur).
        let entity = entry_in_status(
            Uuid::new_v4(),
            Uuid::new_v4(),
            RepaymentEntryStatus::Contacted,
            5,
        );
        let entity_id = entity.id;
        let stored_version = entity.version;
        let entity_for_find = entity.clone();

        let mut entry_dao = MockTestRepaymentEntryDao::new();
        entry_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(entity_for_find.clone())));
        entry_dao
            .expect_update()
            .times(1)
            .withf(|e: &RepaymentEntryEntity, _process, _tx| e.status == RepaymentEntryStatus::Open)
            .returning(|_, _, _| Ok(()));

        let phase_dao = MockTestRepaymentPhaseDao::new();
        let member_dao = MockTestMemberDao::new();

        let service = build_service_admin(entry_dao, phase_dao, member_dao);

        let update = RepaymentEntryUpdate {
            share_count_to_pay_out: None,
            status: Some(RepaymentEntryStatus::Open),
            version: stored_version,
        };
        let result = service
            .update_repayment_entry(entity_id, &update, Authentication::Full)
            .await
            .expect("update Contacted→Open should succeed (D-06 bidirektional)");

        assert_eq!(result.status, RepaymentEntryStatus::Open);
    }

    #[tokio::test]
    async fn test_update_entry_version_mismatch_returns_conflict() {
        // Optimistic locking: stale version → "Version mismatch".
        let entity = entry_in_status(
            Uuid::new_v4(),
            Uuid::new_v4(),
            RepaymentEntryStatus::Open,
            5,
        );
        let entity_id = entity.id;
        let stale_version = Uuid::new_v4();
        assert_ne!(stale_version, entity.version);

        let mut entry_dao = MockTestRepaymentEntryDao::new();
        entry_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(entity.clone())));
        entry_dao
            .expect_update()
            .times(0)
            .returning(|_, _, _| Ok(()));

        let phase_dao = MockTestRepaymentPhaseDao::new();
        let member_dao = MockTestMemberDao::new();

        let service = build_service_admin(entry_dao, phase_dao, member_dao);

        let update = RepaymentEntryUpdate {
            share_count_to_pay_out: Some(3),
            status: None,
            version: stale_version,
        };
        let result = service
            .update_repayment_entry(entity_id, &update, Authentication::Full)
            .await;

        match result {
            Err(ServiceError::Conflict(msg)) => {
                assert!(
                    msg.contains("Version mismatch"),
                    "expected 'Version mismatch', got: {}",
                    msg
                );
            }
            other => panic!("expected Conflict, got {:?}", other),
        }
    }

    // ---------- Delete Guard Tests (ENTR-05) ----------

    #[tokio::test]
    async fn test_delete_entry_in_paid_out_returns_conflict() {
        // ENTR-05: PaidOut → delete verboten ("Cannot delete").
        let entity = entry_in_status(
            Uuid::new_v4(),
            Uuid::new_v4(),
            RepaymentEntryStatus::PaidOut,
            5,
        );
        let entity_id = entity.id;

        let mut entry_dao = MockTestRepaymentEntryDao::new();
        entry_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(entity.clone())));
        entry_dao
            .expect_update()
            .times(0)
            .returning(|_, _, _| Ok(()));

        let phase_dao = MockTestRepaymentPhaseDao::new();
        let member_dao = MockTestMemberDao::new();

        let service = build_service_admin(entry_dao, phase_dao, member_dao);

        let result = service
            .delete_repayment_entry(entity_id, Authentication::Full)
            .await;

        match result {
            Err(ServiceError::Conflict(msg)) => {
                assert!(
                    msg.contains("PaidOut") || msg.contains("ENTR-05"),
                    "expected PaidOut/ENTR-05 in message, got: {}",
                    msg
                );
            }
            other => panic!("expected Conflict, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_delete_entry_in_open_succeeds() {
        // ENTR-05: Open → soft-delete via audited_delete!; DAO.update wird
        // 1x aufgerufen mit deleted=Some(_). audited_delete! lädt Entity
        // intern nochmal nach Pre-Check, daher find_by_id-times = 2.
        let entity = entry_in_status(
            Uuid::new_v4(),
            Uuid::new_v4(),
            RepaymentEntryStatus::Open,
            5,
        );
        let entity_id = entity.id;
        let entity_for_find = entity.clone();

        let mut entry_dao = MockTestRepaymentEntryDao::new();
        entry_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(entity_for_find.clone())));
        entry_dao
            .expect_update()
            .times(1)
            .withf(|e: &RepaymentEntryEntity, _process, _tx| e.deleted.is_some())
            .returning(|_, _, _| Ok(()));

        let phase_dao = MockTestRepaymentPhaseDao::new();
        let member_dao = MockTestMemberDao::new();

        let service = build_service_admin(entry_dao, phase_dao, member_dao);

        let result = service
            .delete_repayment_entry(entity_id, Authentication::Full)
            .await;

        assert!(
            result.is_ok(),
            "delete Open should succeed, got {:?}",
            result
        );
    }

    // ---------- Batch Toggle Tests (D-07/D-08) ----------

    #[tokio::test]
    async fn test_batch_toggle_paid_out_target_returns_validation_error() {
        // D-07: PaidOut als target → ValidationError 400.
        let entry_dao = MockTestRepaymentEntryDao::new();
        let phase_dao = MockTestRepaymentPhaseDao::new();
        let member_dao = MockTestMemberDao::new();
        let service = build_service_admin(entry_dao, phase_dao, member_dao);

        let input = RepaymentEntryBatchStatusInput {
            entry_ids: vec![Uuid::new_v4()].into(),
            target_status: RepaymentEntryStatus::PaidOut,
        };
        let result = service
            .batch_toggle_status(&input, Authentication::Full)
            .await;

        match result {
            Err(ServiceError::ValidationError(items)) => {
                assert!(
                    items.iter().any(|i| i.field.as_ref() == "target_status"),
                    "expected target_status validation failure, got {:?}",
                    items
                );
            }
            other => panic!("expected ValidationError, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_batch_toggle_all_or_nothing_on_failure() {
        // D-08: 3 IDs, 2. ist PaidOut → 409 mit strukturiertem JSON-Body
        // { failure_index: 1, failure_id: <uuid>, failure_reason: "..." }.
        // audited_update wird höchstens 1x für den ersten Eintrag aufgerufen.
        let phase_id = Uuid::new_v4();
        let entry1 = entry_in_status(Uuid::new_v4(), phase_id, RepaymentEntryStatus::Open, 1);
        let entry2 = entry_in_status(Uuid::new_v4(), phase_id, RepaymentEntryStatus::PaidOut, 2);
        let entry3 = entry_in_status(Uuid::new_v4(), phase_id, RepaymentEntryStatus::Open, 3);

        let id1 = entry1.id;
        let id2 = entry2.id;
        let id3 = entry3.id;

        let map: std::collections::HashMap<Uuid, RepaymentEntryEntity> = [
            (id1, entry1.clone()),
            (id2, entry2.clone()),
            (id3, entry3.clone()),
        ]
        .into_iter()
        .collect();

        let mut entry_dao = MockTestRepaymentEntryDao::new();
        entry_dao
            .expect_find_by_id()
            .returning(move |id, _| Ok(map.get(&id).cloned()));
        // audited_update wird maximal 1x für entry1 aufgerufen, bevor entry2 fehlt.
        entry_dao
            .expect_update()
            .times(0..=1)
            .returning(|_, _, _| Ok(()));

        let phase_dao = MockTestRepaymentPhaseDao::new();
        let member_dao = MockTestMemberDao::new();

        let service = build_service_admin(entry_dao, phase_dao, member_dao);

        let input = RepaymentEntryBatchStatusInput {
            entry_ids: vec![id1, id2, id3].into(),
            target_status: RepaymentEntryStatus::Contacted,
        };
        let result = service
            .batch_toggle_status(&input, Authentication::Full)
            .await;

        match result {
            Err(ServiceError::Conflict(msg)) => {
                // Body ist strukturiertes JSON
                let parsed: serde_json::Value =
                    serde_json::from_str(msg.as_ref()).expect("conflict body must be valid JSON");
                assert_eq!(parsed["failure_index"], serde_json::json!(1));
                assert_eq!(parsed["failure_id"], serde_json::json!(id2.to_string()));
                let reason = parsed["failure_reason"]
                    .as_str()
                    .expect("failure_reason must be string");
                assert!(
                    reason.contains("source status"),
                    "expected 'source status' in failure_reason, got: {}",
                    reason
                );
            }
            other => panic!("expected Conflict with JSON body, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_batch_toggle_success() {
        // D-08 happy: 3 IDs alle Open, target=Contacted → 3 audited_update! in 1 Tx.
        let phase_id = Uuid::new_v4();
        let entry1 = entry_in_status(Uuid::new_v4(), phase_id, RepaymentEntryStatus::Open, 1);
        let entry2 = entry_in_status(Uuid::new_v4(), phase_id, RepaymentEntryStatus::Open, 2);
        let entry3 = entry_in_status(Uuid::new_v4(), phase_id, RepaymentEntryStatus::Open, 3);

        let id1 = entry1.id;
        let id2 = entry2.id;
        let id3 = entry3.id;

        let map: std::collections::HashMap<Uuid, RepaymentEntryEntity> = [
            (id1, entry1.clone()),
            (id2, entry2.clone()),
            (id3, entry3.clone()),
        ]
        .into_iter()
        .collect();

        let mut entry_dao = MockTestRepaymentEntryDao::new();
        entry_dao
            .expect_find_by_id()
            .returning(move |id, _| Ok(map.get(&id).cloned()));
        // 3x audited_update! → DAO.update genau 3x mit target=Contacted
        entry_dao
            .expect_update()
            .times(3)
            .withf(|e: &RepaymentEntryEntity, _process, _tx| {
                e.status == RepaymentEntryStatus::Contacted
            })
            .returning(|_, _, _| Ok(()));

        let phase_dao = MockTestRepaymentPhaseDao::new();
        let member_dao = MockTestMemberDao::new();

        let service = build_service_admin(entry_dao, phase_dao, member_dao);

        let input = RepaymentEntryBatchStatusInput {
            entry_ids: vec![id1, id2, id3].into(),
            target_status: RepaymentEntryStatus::Contacted,
        };
        let result = service
            .batch_toggle_status(&input, Authentication::Full)
            .await
            .expect("batch toggle should succeed");

        assert_eq!(result.len(), 3);
        for r in result.iter() {
            assert_eq!(r.status, RepaymentEntryStatus::Contacted);
        }
    }

    // ---------- Permission Tests (T-08-03-02) ----------

    #[tokio::test]
    async fn test_create_entry_requires_admin_privilege() {
        // T-08-03-02: ohne admin → PermissionDenied.
        let entry_dao = MockTestRepaymentEntryDao::new();
        let phase_dao = MockTestRepaymentPhaseDao::new();
        let member_dao = MockTestMemberDao::new();

        let service = build_service(
            entry_dao,
            phase_dao,
            member_dao,
            make_permission_service_admin_denied(),
        );

        let submission = RepaymentEntrySubmission {
            phase_id: Uuid::new_v4(),
            member_id: Uuid::new_v4(),
            share_count_to_pay_out: 1,
        };
        let result = service
            .create_repayment_entry(&submission, Authentication::Full)
            .await;

        match result {
            Err(ServiceError::PermissionDenied) => {}
            other => panic!("expected PermissionDenied, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_update_entry_requires_admin_privilege() {
        let entry_dao = MockTestRepaymentEntryDao::new();
        let phase_dao = MockTestRepaymentPhaseDao::new();
        let member_dao = MockTestMemberDao::new();

        let service = build_service(
            entry_dao,
            phase_dao,
            member_dao,
            make_permission_service_admin_denied(),
        );

        let update = RepaymentEntryUpdate {
            share_count_to_pay_out: Some(1),
            status: None,
            version: Uuid::new_v4(),
        };
        let result = service
            .update_repayment_entry(Uuid::new_v4(), &update, Authentication::Full)
            .await;

        match result {
            Err(ServiceError::PermissionDenied) => {}
            other => panic!("expected PermissionDenied, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_delete_entry_requires_admin_privilege() {
        let entry_dao = MockTestRepaymentEntryDao::new();
        let phase_dao = MockTestRepaymentPhaseDao::new();
        let member_dao = MockTestMemberDao::new();

        let service = build_service(
            entry_dao,
            phase_dao,
            member_dao,
            make_permission_service_admin_denied(),
        );

        let result = service
            .delete_repayment_entry(Uuid::new_v4(), Authentication::Full)
            .await;

        match result {
            Err(ServiceError::PermissionDenied) => {}
            other => panic!("expected PermissionDenied, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_batch_toggle_requires_admin_privilege() {
        let entry_dao = MockTestRepaymentEntryDao::new();
        let phase_dao = MockTestRepaymentPhaseDao::new();
        let member_dao = MockTestMemberDao::new();

        let service = build_service(
            entry_dao,
            phase_dao,
            member_dao,
            make_permission_service_admin_denied(),
        );

        let input = RepaymentEntryBatchStatusInput {
            entry_ids: vec![Uuid::new_v4()].into(),
            target_status: RepaymentEntryStatus::Contacted,
        };
        let result = service
            .batch_toggle_status(&input, Authentication::Full)
            .await;

        match result {
            Err(ServiceError::PermissionDenied) => {}
            other => panic!("expected PermissionDenied, got {:?}", other),
        }
    }

    // ---------- Phase 08 Gap-Closure CR-01: Re-Read after audited_update! ----------

    /// Phase 08 Gap-Closure CR-01 — verifies that `update_repayment_entry`
    /// re-reads the entity after `audited_update!` and returns the fresh
    /// version-UUID generated by the DAO (mirrors `MemberServiceImpl` pattern
    /// at `member.rs:343-348`). Without the fix the service returns the
    /// pre-update entity, causing 409 on every follow-up PUT.
    #[tokio::test]
    async fn test_update_repayment_entry_rereads_after_audited_update_returns_new_version() {
        let entry_id = Uuid::new_v4();
        let member_id = Uuid::new_v4();
        let phase_id = Uuid::new_v4();
        let version_a = Uuid::new_v4();
        let version_b = Uuid::new_v4();
        assert_ne!(version_a, version_b);

        let pre_entity = RepaymentEntryEntity {
            id: entry_id,
            member_id,
            phase_id,
            share_count_to_pay_out: 2,
            status: RepaymentEntryStatus::Open,
            created: make_test_datetime(),
            deleted: None,
            version: version_a,
        };
        let post_entity = RepaymentEntryEntity {
            version: version_b,
            ..pre_entity.clone()
        };

        // Sequence of find_by_id calls in update_repayment_entry:
        //   1. Pre-update load (Edit-Matrix + version-check) -> pre_entity
        //   2. audited_update! internal load (audit_macros.rs:47) -> pre_entity
        //   3. CR-01 Re-Read after audited_update! -> post_entity (NEW version)
        let mut entry_dao = MockTestRepaymentEntryDao::new();
        let mut seq = mockall::Sequence::new();

        let pre_for_call_1 = pre_entity.clone();
        entry_dao
            .expect_find_by_id()
            .times(1)
            .in_sequence(&mut seq)
            .returning(move |_, _| Ok(Some(pre_for_call_1.clone())));

        let pre_for_call_2 = pre_entity.clone();
        entry_dao
            .expect_find_by_id()
            .times(1)
            .in_sequence(&mut seq)
            .returning(move |_, _| Ok(Some(pre_for_call_2.clone())));

        entry_dao
            .expect_update()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_, _, _| Ok(()));

        let post_for_call_3 = post_entity.clone();
        entry_dao
            .expect_find_by_id()
            .times(1)
            .in_sequence(&mut seq)
            .returning(move |_, _| Ok(Some(post_for_call_3.clone())));

        let phase_dao = MockTestRepaymentPhaseDao::new();
        let member_dao = MockTestMemberDao::new();

        let service = build_service_admin(entry_dao, phase_dao, member_dao);

        let update = RepaymentEntryUpdate {
            share_count_to_pay_out: None,
            status: Some(RepaymentEntryStatus::Contacted),
            version: version_a,
        };
        let result = service
            .update_repayment_entry(entry_id, &update, Authentication::Full)
            .await
            .expect("update should succeed");

        assert_eq!(
            result.version, version_b,
            "Re-Read must return the new DAO-generated version, not the pre-update version"
        );
        assert_eq!(result.status, RepaymentEntryStatus::Contacted);
    }

    /// Phase 08 Gap-Closure CR-01/WR-01 — verifies that `batch_toggle_status`
    /// re-reads each entry after `audited_update!` so the returned Vec carries
    /// the DAO-generated new versions per entry (not stale pre-update ones).
    #[tokio::test]
    async fn test_batch_toggle_status_rereads_each_entry_returns_new_versions() {
        let phase_id = Uuid::new_v4();
        let member_id_1 = Uuid::new_v4();
        let member_id_2 = Uuid::new_v4();

        let entry_id_1 = Uuid::new_v4();
        let entry_id_2 = Uuid::new_v4();

        let v1_old = Uuid::new_v4();
        let v1_new = Uuid::new_v4();
        let v2_old = Uuid::new_v4();
        let v2_new = Uuid::new_v4();
        assert_ne!(v1_old, v1_new);
        assert_ne!(v2_old, v2_new);

        let make_entry =
            |id: Uuid, member_id: Uuid, version: Uuid, status: RepaymentEntryStatus| {
                RepaymentEntryEntity {
                    id,
                    member_id,
                    phase_id,
                    share_count_to_pay_out: 1,
                    status,
                    created: make_test_datetime(),
                    deleted: None,
                    version,
                }
            };

        let pre_1 = make_entry(entry_id_1, member_id_1, v1_old, RepaymentEntryStatus::Open);
        let post_1 = make_entry(
            entry_id_1,
            member_id_1,
            v1_new,
            RepaymentEntryStatus::Contacted,
        );
        let pre_2 = make_entry(entry_id_2, member_id_2, v2_old, RepaymentEntryStatus::Open);
        let post_2 = make_entry(
            entry_id_2,
            member_id_2,
            v2_new,
            RepaymentEntryStatus::Contacted,
        );

        // Sequence per iteration in batch_toggle_status loop body:
        //   1. Pre-update load (status-guard) -> pre
        //   2. audited_update! internal load (audit_macros.rs:47) -> pre
        //   3. audited_update! internal DAO.update -> Ok
        //   4. CR-01/WR-01 Re-Read after audited_update! -> post (NEW version)
        //
        // For 2 entries: 8 mock interactions total in strict sequence.
        let mut entry_dao = MockTestRepaymentEntryDao::new();
        let mut seq = mockall::Sequence::new();

        // ----- Iteration 1 (entry_id_1) -----
        let pre_1_call_1 = pre_1.clone();
        entry_dao
            .expect_find_by_id()
            .times(1)
            .in_sequence(&mut seq)
            .returning(move |_, _| Ok(Some(pre_1_call_1.clone())));

        let pre_1_call_2 = pre_1.clone();
        entry_dao
            .expect_find_by_id()
            .times(1)
            .in_sequence(&mut seq)
            .returning(move |_, _| Ok(Some(pre_1_call_2.clone())));

        entry_dao
            .expect_update()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_, _, _| Ok(()));

        let post_1_call = post_1.clone();
        entry_dao
            .expect_find_by_id()
            .times(1)
            .in_sequence(&mut seq)
            .returning(move |_, _| Ok(Some(post_1_call.clone())));

        // ----- Iteration 2 (entry_id_2) -----
        let pre_2_call_1 = pre_2.clone();
        entry_dao
            .expect_find_by_id()
            .times(1)
            .in_sequence(&mut seq)
            .returning(move |_, _| Ok(Some(pre_2_call_1.clone())));

        let pre_2_call_2 = pre_2.clone();
        entry_dao
            .expect_find_by_id()
            .times(1)
            .in_sequence(&mut seq)
            .returning(move |_, _| Ok(Some(pre_2_call_2.clone())));

        entry_dao
            .expect_update()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_, _, _| Ok(()));

        let post_2_call = post_2.clone();
        entry_dao
            .expect_find_by_id()
            .times(1)
            .in_sequence(&mut seq)
            .returning(move |_, _| Ok(Some(post_2_call.clone())));

        let phase_dao = MockTestRepaymentPhaseDao::new();
        let member_dao = MockTestMemberDao::new();

        let service = build_service_admin(entry_dao, phase_dao, member_dao);

        let input = RepaymentEntryBatchStatusInput {
            entry_ids: vec![entry_id_1, entry_id_2].into(),
            target_status: RepaymentEntryStatus::Contacted,
        };
        let result = service
            .batch_toggle_status(&input, Authentication::Full)
            .await
            .expect("batch should succeed");

        assert_eq!(result.len(), 2);
        assert_eq!(
            result[0].version, v1_new,
            "Entry 1 must return the new DAO-generated version after Re-Read"
        );
        assert_eq!(
            result[1].version, v2_new,
            "Entry 2 must return the new DAO-generated version after Re-Read"
        );
        assert_eq!(result[0].status, RepaymentEntryStatus::Contacted);
        assert_eq!(result[1].status, RepaymentEntryStatus::Contacted);
    }
}
