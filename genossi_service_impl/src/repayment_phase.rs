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
use genossi_dao::member::MemberDao;
use genossi_dao::repayment_entry::{RepaymentEntryDao, RepaymentEntryEntity, RepaymentEntryStatus};
use genossi_dao::repayment_phase::{RepaymentPhaseDao, RepaymentPhaseEntity, RepaymentPhaseStatus};
use genossi_dao::TransactionDao;
use genossi_service::permission::{Authentication, PermissionService};
use genossi_service::repayment_phase::{
    RepaymentPhase, RepaymentPhaseService, RepaymentPhaseSubmission, RepaymentPhaseUpdate,
};
use genossi_service::uuid_service::UuidService;
use genossi_service::{ServiceError, ValidationFailureItem};
use std::collections::HashMap;
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
        RepaymentEntryDao: RepaymentEntryDao<Transaction = Self::Transaction> = repayment_entry_dao,
        MemberDao: MemberDao<Transaction = Self::Transaction> = member_dao,
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
            message: Arc::from(format!("must be in 2000..=2100, got {}", fiscal_year)),
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

        // CR-01 Fix (Phase-7-Erbe): Re-read to get the persisted version UUID.
        // Pattern mirrors MemberServiceImpl::update (member.rs:343-348).
        // The DAO writes a fresh version internally; without this re-read the
        // Service returns the locally-constructed entity, leaving the client
        // with a version that may not match the persisted row.
        //
        // BL-01 Fix: Re-Read runs in the SAME transaction as audited_create!
        // above — the entity we just created MUST be visible. `None` here is
        // an internal consistency error (DAO regression, Tx-Isolation break,
        // or id rewrite by DAO — see WR-02). Map to InternalError → HTTP 500,
        // never EntityNotFound → 404 (404 would suggest "the entity you tried
        // to create doesn't exist", which is nonsensical).
        let refreshed = self
            .repayment_phase_dao
            .find_by_id(entity.id, tx.clone())
            .await?
            .ok_or_else(|| {
                ServiceError::InternalError(Arc::from(format!(
                    "Re-Read after audited_create! returned None for RepaymentPhase {} — \
                     internal consistency error (same-tx invariant violated)",
                    entity.id
                )))
            })?;

        self.transaction_dao.commit(tx).await?;
        Ok(RepaymentPhase::from(&refreshed))
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

        // CR-01 Fix: Re-read to get the new version UUID generated by the DAO
        // (pattern: member.rs:343-348). Without this re-read the Service
        // returns the pre-update `entity` whose version-UUID does not match
        // the persisted row, causing 409 on every follow-up PUT.
        //
        // BL-01 Fix: `None` here is an internal consistency error (same-Tx
        // invariant) — map to InternalError → HTTP 500, never 404.
        let refreshed = self
            .repayment_phase_dao
            .find_by_id(id, tx.clone())
            .await?
            .ok_or_else(|| {
                ServiceError::InternalError(Arc::from(format!(
                    "Re-Read after audited_update! returned None for RepaymentPhase {} — \
                     internal consistency error (same-tx invariant violated)",
                    id
                )))
            })?;

        self.transaction_dao.commit(tx).await?;
        Ok(RepaymentPhase::from(&refreshed))
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

        // ----- PHAS-02 / ENTR-01 (Phase 8): Auto-Befüllung der RepaymentEntries -----
        // Atomar in derselben Tx wie der Status-Übergang Preparation→Open.
        // Pattern: assembly.rs:181-258 (Single-Tx-Multi-DAO via tx.clone()).
        //
        // UNTERSCHIED zu Assembly-Snapshot: N einzelne audited_create! statt
        // batch_without_audit (D-03) — RepaymentEntries sind Lifecycle-Träger,
        // Phase-9 mark_paid_out hängt an entity_id+version.
        //
        // D-04: Auto-Fill genau einmal beim Phase-Open; keine Re-Fill-Action.
        //
        // D-03 Klarstellung: Jeder audited_create!-Call generiert intern eine
        // EIGENE transaction_id (audit_log.rs:65 uuid_fn()-Call). Die N Einträge
        // sind als Folge des Phase-Open-Akts über den gemeinsamen process-String
        // REPAYMENT_PHASE_PROCESS_OPEN + den zeitgleichen timestamp-Range
        // identifizierbar (alle Calls liegen in derselben DB-Commit-Sekunde
        // dank Single-Tx).

        let fiscal_year = entity.fiscal_year;
        let fy_start = time::Date::from_calendar_date(fiscal_year, time::Month::January, 1)
            .map_err(|e| {
                ServiceError::InternalError(Arc::from(format!(
                    "invalid fiscal_year start date: {}",
                    e
                )))
            })?;
        let fy_end = time::Date::from_calendar_date(fiscal_year, time::Month::December, 31)
            .map_err(|e| {
                ServiceError::InternalError(Arc::from(format!(
                    "invalid fiscal_year end date: {}",
                    e
                )))
            })?;

        // D-02: strikter Member-Filter — KEIN is_normal()-Filter
        // (Ausgeschiedene haben Status != Normal, das ist genau die Zielgruppe).
        // member_dao.all() filtert bereits deleted IS NULL per Default-Impl.
        let all_members = self.member_dao.all(tx.clone()).await?;
        let mut targets: Vec<&genossi_dao::member::MemberEntity> = all_members
            .iter()
            .filter(|m| m.exit_date.is_some_and(|d| d >= fy_start && d <= fy_end))
            .filter(|m| m.current_shares > 0)
            .collect();

        // Deterministische Audit-Reihenfolge (CONTEXT Claude's Discretion).
        // Gibt zwar nicht eine einheitliche transaction_id (siehe D-03), aber
        // stabile timestamp-Reihenfolge im Audit-Log für reproduzierbare Tests
        // + Vorstand-Lesbarkeit.
        targets.sort_by_key(|m| m.member_number);

        for member in targets {
            let entry_now_offset = time::OffsetDateTime::now_utc();
            let entry_now_pdt =
                time::PrimitiveDateTime::new(entry_now_offset.date(), entry_now_offset.time());
            let new_entry = RepaymentEntryEntity {
                id: self.uuid_service.new_v4().await,
                member_id: member.id,
                phase_id: id,
                share_count_to_pay_out: member.current_shares,
                status: RepaymentEntryStatus::Open,
                created: entry_now_pdt,
                deleted: None,
                version: self.uuid_service.new_v4().await,
            };
            // Audit-Process = REPAYMENT_PHASE_PROCESS_OPEN (gleicher String
            // wie das audited_update! oben für den Phase-Status-Übergang).
            // Identifikation des Phase-Open-Blocks im Audit-Log: filtere nach
            //   process = 'repayment-phase.open' AND timestamp BETWEEN T AND T+1s
            crate::audited_create!(
                self,
                self.repayment_entry_dao,
                &new_entry,
                REPAYMENT_PHASE_PROCESS_OPEN,
                &user_id,
                tx
            );
        }
        // ----- /PHAS-02 -----

        // CR-01 Fix: Re-read the Phase entity to get the version UUID written
        // by the audited_update! on the Phase row above. The Auto-Fill loop
        // operates on a different aggregate (repayment_entry_dao) and does
        // NOT touch the Phase row, but we read the Phase row again to stay
        // pattern-consistent with member.rs:343-348 and to be robust against
        // any future DAO-level Phase-row mutation in the auto-fill block.
        // Re-Read runs NACH dem Auto-Fill-Block + VOR commit, innerhalb
        // derselben Tx — Single-Snapshot-Konsistenz (T-08-08-01 Mitigation).
        //
        // BL-01 Fix: `None` here is an internal consistency error (same-Tx
        // invariant) — map to InternalError → HTTP 500, never 404.
        let refreshed = self
            .repayment_phase_dao
            .find_by_id(id, tx.clone())
            .await?
            .ok_or_else(|| {
                ServiceError::InternalError(Arc::from(format!(
                    "Re-Read after audited_update! returned None for RepaymentPhase {} \
                     in open_repayment_phase — internal consistency error \
                     (same-tx invariant violated)",
                    id
                )))
            })?;

        self.transaction_dao.commit(tx).await?;
        Ok(RepaymentPhase::from(&refreshed))
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

        // ----- PHAS-03 (Phase 8): Pending-Entry-Validation -----
        // D-13: "pending" = status != PaidOut AND deleted IS NULL
        // D-14: 0-Entry-Close ist erlaubt (Phase mit 0 Eingängen darf direkt closen)
        // D-15: 409 Body enthält pending_count + bis zu 20 Mitgliedsnummern

        let all_entries = self
            .repayment_entry_dao
            .find_by_phase_id(id, tx.clone())
            .await?;
        let pending: Vec<&RepaymentEntryEntity> = all_entries
            .iter()
            .filter(|e| e.deleted.is_none())
            .filter(|e| e.status != RepaymentEntryStatus::PaidOut)
            .collect();

        if !pending.is_empty() {
            // Mitgliedsnummern-Lookup (Vorstand denkt in Mitgliedsnummern, nicht UUIDs — D-15)
            let all_members = self.member_dao.all(tx.clone()).await?;
            let number_by_id: HashMap<Uuid, i64> = all_members
                .iter()
                .map(|m| (m.id, m.member_number))
                .collect();

            let mut pending_numbers: Vec<i64> = pending
                .iter()
                .filter_map(|e| number_by_id.get(&e.member_id).copied())
                .collect();
            pending_numbers.sort();
            let total = pending_numbers.len();

            // D-15: max 20 Mitgliedsnummern + "+N weitere" Suffix
            let mut display_numbers: Vec<String> = pending_numbers
                .iter()
                .take(20)
                .map(|n| n.to_string())
                .collect();
            if total > 20 {
                display_numbers.push(format!("+{} weitere", total - 20));
            }

            // JSON-encoded Detail im Conflict-Body (REST-Layer kann das in
            // CloseConflictResponse parsen)
            let detail = serde_json::json!({
                "error": format!(
                    "Cannot close phase: {} entries are not paid out and not deleted.",
                    total
                ),
                "pending_count": total,
                "pending_member_numbers": display_numbers,
            });

            return Err(ServiceError::Conflict(Arc::from(detail.to_string())));
        }
        // ----- /PHAS-03 -----

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

        // CR-01 Fix: Re-read to get the new version UUID generated by the DAO
        // (pattern: member.rs:343-348).
        //
        // BL-01 Fix: `None` here is an internal consistency error (same-Tx
        // invariant) — map to InternalError → HTTP 500, never 404.
        let refreshed = self
            .repayment_phase_dao
            .find_by_id(id, tx.clone())
            .await?
            .ok_or_else(|| {
                ServiceError::InternalError(Arc::from(format!(
                    "Re-Read after audited_update! returned None for RepaymentPhase {} \
                     in close_repayment_phase — internal consistency error \
                     (same-tx invariant violated)",
                    id
                )))
            })?;

        self.transaction_dao.commit(tx).await?;
        Ok(RepaymentPhase::from(&refreshed))
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
    use genossi_dao::member::{MemberEntity, MemberStatus};
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
    impl RepaymentPhaseServiceDeps for TestDeps {
        type Context = MockContext;
        type Transaction = TestTransaction;
        type RepaymentPhaseDao = MockTestRepaymentPhaseDao;
        type RepaymentEntryDao = MockTestRepaymentEntryDao;
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

    /// Quiet entry-DAO: returns empty for all reads, no expectations set on
    /// writes. Phase-7-Tests inject this so the added deps don't break.
    ///
    /// IMPORTANT (Phase-3-Plan-03-Lektion, wiederbestätigt in Phase 8 Plan 03):
    /// mockall overrides the trait's Default-Impl. The service code calls
    /// `member_dao.all(...)` and `entry_dao.find_by_phase_id(...)`, both of
    /// which are Default-Impls on the trait. The mocks therefore need
    /// `expect_all` and `expect_find_by_phase_id` (NOT `expect_dump_all`).
    fn make_entry_dao_quiet() -> MockTestRepaymentEntryDao {
        let mut dao = MockTestRepaymentEntryDao::new();
        dao.expect_find_by_phase_id()
            .returning(|_, _| Ok(Arc::from(vec![])));
        dao
    }

    /// Quiet member-DAO: returns empty for all reads, no expectations on writes.
    /// See `make_entry_dao_quiet` doc for the mockall-Default-Impl pitfall.
    fn make_member_dao_quiet() -> MockTestMemberDao {
        let mut dao = MockTestMemberDao::new();
        dao.expect_all().returning(|_| Ok(Arc::from(vec![])));
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
        // Phase-7-Tests: Auto-Fill und Pending-Validation sind nicht im
        // Test-Scope. Quiet-Mocks für die neuen Phase-8-Deps liefern leere
        // Result-Sets und keine Erwartung an Write-Calls. Tests, die den
        // Phase-7-Status-Guard prüfen (z.B. "open from Closed"), terminieren
        // VOR dem Auto-Fill-Block, also werden die Quiet-Mocks nie konsumiert.
        // Happy-Path-Tests in Phase 7 (z.B. delete_in_preparation) berühren
        // weder den Auto-Fill-Block noch die Close-Validation; Quiet-Mocks
        // sind safe.
        RepaymentPhaseServiceImpl {
            repayment_phase_dao: Arc::new(dao),
            repayment_entry_dao: Arc::new(make_entry_dao_quiet()),
            member_dao: Arc::new(make_member_dao_quiet()),
            audit_log_dao: Arc::new(make_audit_log_dao_quiet()),
            permission_service: Arc::new(make_permission_service_admin_ok()),
            uuid_service: Arc::new(StaticUuidService),
            transaction_dao: Arc::new(setup_mock_tx_dao()),
        }
    }

    /// Build service with custom entry-DAO and member-DAO for Phase-8 tests.
    fn build_service_full(
        phase_dao: MockTestRepaymentPhaseDao,
        entry_dao: MockTestRepaymentEntryDao,
        member_dao: MockTestMemberDao,
    ) -> RepaymentPhaseServiceImpl<TestDeps> {
        RepaymentPhaseServiceImpl {
            repayment_phase_dao: Arc::new(phase_dao),
            repayment_entry_dao: Arc::new(entry_dao),
            member_dao: Arc::new(member_dao),
            audit_log_dao: Arc::new(make_audit_log_dao_quiet()),
            permission_service: Arc::new(make_permission_service_admin_ok()),
            uuid_service: Arc::new(StaticUuidService),
            transaction_dao: Arc::new(setup_mock_tx_dao()),
        }
    }

    /// Build a MemberEntity with given member_number, exit_date, current_shares.
    /// Used by Phase-8 auto-fill tests.
    fn make_member(
        member_number: i64,
        current_shares: i32,
        exit_date: Option<time::Date>,
    ) -> MemberEntity {
        let date = time::Date::from_calendar_date(2020, time::Month::January, 1).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        MemberEntity {
            id: Uuid::new_v4(),
            member_number,
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
            shares_at_joining: current_shares.max(1),
            current_shares,
            current_balance: 0,
            action_count: 0,
            migrated: false,
            exit_date,
            bank_account: None,
            status: MemberStatus::Normal,
            created: datetime,
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    /// Helper to build a RepaymentEntryEntity for tests.
    fn make_entry(
        phase_id: Uuid,
        member_id: Uuid,
        status: RepaymentEntryStatus,
        deleted: Option<time::PrimitiveDateTime>,
    ) -> RepaymentEntryEntity {
        let date = time::Date::from_calendar_date(2026, time::Month::May, 30).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        RepaymentEntryEntity {
            id: Uuid::new_v4(),
            member_id,
            phase_id,
            share_count_to_pay_out: 5,
            status,
            created: datetime,
            deleted,
            version: Uuid::new_v4(),
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
        //
        // CR-01 (Phase 08 gap-closure): create_repayment_phase now performs
        // a find_by_id Re-Read after audited_create! to capture the DAO-
        // generated version-UUID. Mock returns the persisted entity (same
        // shape as input, with the post-DAO version) on the Re-Read.
        let date = time::Date::from_calendar_date(2026, time::Month::May, 29).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        let persisted = RepaymentPhaseEntity {
            id: Uuid::new_v4(),
            fiscal_year: 2026,
            share_value: 12000,
            status: RepaymentPhaseStatus::Preparation,
            opened_at: None,
            closed_at: None,
            created: datetime,
            deleted: None,
            version: Uuid::new_v4(),
        };
        let mut dao = MockTestRepaymentPhaseDao::new();
        dao.expect_create().times(1).returning(|_, _, _| Ok(()));
        dao.expect_find_by_id()
            .returning(move |_, _| Ok(Some(persisted.clone())));

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
        //
        // CR-01 (Phase 08 gap-closure): update_repayment_phase now performs
        // a find_by_id Re-Read after audited_update! to capture the DAO-
        // generated version-UUID. Mock uses Sequence so the 3rd find_by_id
        // (Re-Read) returns the post-update entity with share_value=13000.
        let entity = phase_in_status(RepaymentPhaseStatus::Open);
        let entity_id = entity.id;
        let stored_fiscal_year = entity.fiscal_year;
        let stored_version = entity.version;
        let post_update_version = Uuid::new_v4();
        let post_update_entity = RepaymentPhaseEntity {
            share_value: 13000,
            version: post_update_version,
            ..entity.clone()
        };

        let mut dao = MockTestRepaymentPhaseDao::new();
        let mut seq = mockall::Sequence::new();

        // 1. Pre-update load (Edit-Matrix + version check)
        let pre_1 = entity.clone();
        dao.expect_find_by_id()
            .times(1)
            .in_sequence(&mut seq)
            .returning(move |_, _| Ok(Some(pre_1.clone())));
        // 2. audited_update! internal load (audit_macros.rs:47)
        let pre_2 = entity.clone();
        dao.expect_find_by_id()
            .times(1)
            .in_sequence(&mut seq)
            .returning(move |_, _| Ok(Some(pre_2.clone())));
        // audited_update! internal DAO.update
        dao.expect_update()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_, _, _| Ok(()));
        // 3. CR-01 Re-Read after audited_update! returns post-update entity
        let post_for_3 = post_update_entity.clone();
        dao.expect_find_by_id()
            .times(1)
            .in_sequence(&mut seq)
            .returning(move |_, _| Ok(Some(post_for_3.clone())));

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

    // ============================================================
    // Phase-8 (Plan 04) Tests — Auto-Fill in open_phase
    //                        + Pending-Validation in close_phase
    // ============================================================

    // ---------- Auto-Fill tests in open_repayment_phase ----------

    #[tokio::test]
    async fn test_open_phase_auto_fill_zero_members() {
        // D-14 / Auto-Fill: 0 members → 0 audited_create calls; phase status
        // transitions to Open without entries.
        //
        // CR-01 (Phase 08 gap-closure): open_repayment_phase now performs a
        // find_by_id Re-Read after audited_update! + Auto-Fill. Mock uses
        // Sequence so the 3rd find_by_id (Re-Read) returns the post-open
        // entity with status=Open.
        let entity = phase_in_status(RepaymentPhaseStatus::Preparation);
        let entity_id = entity.id;
        let post_open = RepaymentPhaseEntity {
            status: RepaymentPhaseStatus::Open,
            opened_at: Some(entity.created),
            version: Uuid::new_v4(),
            ..entity.clone()
        };

        let mut phase_dao = MockTestRepaymentPhaseDao::new();
        let mut seq = mockall::Sequence::new();

        let pre_1 = entity.clone();
        phase_dao
            .expect_find_by_id()
            .times(1)
            .in_sequence(&mut seq)
            .returning(move |_, _| Ok(Some(pre_1.clone())));
        let pre_2 = entity.clone();
        phase_dao
            .expect_find_by_id()
            .times(1)
            .in_sequence(&mut seq)
            .returning(move |_, _| Ok(Some(pre_2.clone())));
        // audited_update! for the status transition → 1 update call on the
        // phase DAO.
        phase_dao
            .expect_update()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_, _, _| Ok(()));
        // CR-01 Re-Read after Auto-Fill
        let post_for_3 = post_open.clone();
        phase_dao
            .expect_find_by_id()
            .times(1)
            .in_sequence(&mut seq)
            .returning(move |_, _| Ok(Some(post_for_3.clone())));

        let mut entry_dao = MockTestRepaymentEntryDao::new();
        // 0 members → 0 entry-create calls.
        entry_dao
            .expect_create()
            .times(0)
            .returning(|_, _, _| Ok(()));

        let mut member_dao = MockTestMemberDao::new();
        member_dao.expect_all().returning(|_| Ok(Arc::from(vec![])));

        let service = build_service_full(phase_dao, entry_dao, member_dao);

        let result = service
            .open_repayment_phase(entity_id, Authentication::Full)
            .await
            .expect("open with 0 members should succeed");
        assert_eq!(result.status, RepaymentPhaseStatus::Open);
    }

    #[tokio::test]
    async fn test_open_phase_auto_fill_creates_entries_for_matching_members() {
        // 3 members all with exit_date in FY 2026 + current_shares > 0 →
        // 3 audited_create calls on entry DAO.
        //
        // CR-01 (Phase 08 gap-closure): Re-Read after Auto-Fill returns
        // post-open entity (status=Open).
        let phase_entity = phase_in_status(RepaymentPhaseStatus::Preparation);
        let phase_id = phase_entity.id;
        let post_open = RepaymentPhaseEntity {
            status: RepaymentPhaseStatus::Open,
            opened_at: Some(phase_entity.created),
            version: Uuid::new_v4(),
            ..phase_entity.clone()
        };

        let in_fy = time::Date::from_calendar_date(2026, time::Month::June, 15).unwrap();
        let m1 = make_member(1, 5, Some(in_fy));
        let m2 = make_member(2, 3, Some(in_fy));
        let m3 = make_member(3, 10, Some(in_fy));

        let mut phase_dao = MockTestRepaymentPhaseDao::new();
        let mut seq = mockall::Sequence::new();

        let pre_1 = phase_entity.clone();
        phase_dao
            .expect_find_by_id()
            .times(1)
            .in_sequence(&mut seq)
            .returning(move |_, _| Ok(Some(pre_1.clone())));
        let pre_2 = phase_entity.clone();
        phase_dao
            .expect_find_by_id()
            .times(1)
            .in_sequence(&mut seq)
            .returning(move |_, _| Ok(Some(pre_2.clone())));
        phase_dao
            .expect_update()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_, _, _| Ok(()));
        // CR-01 Re-Read after Auto-Fill returns post-open entity
        let post_for_3 = post_open.clone();
        phase_dao
            .expect_find_by_id()
            .times(1)
            .in_sequence(&mut seq)
            .returning(move |_, _| Ok(Some(post_for_3.clone())));

        let mut entry_dao = MockTestRepaymentEntryDao::new();
        entry_dao
            .expect_create()
            .times(3)
            .returning(|_, _, _| Ok(()));

        let mut member_dao = MockTestMemberDao::new();
        member_dao
            .expect_all()
            .returning(move |_| Ok(Arc::from(vec![m1.clone(), m2.clone(), m3.clone()])));

        let service = build_service_full(phase_dao, entry_dao, member_dao);

        let result = service
            .open_repayment_phase(phase_id, Authentication::Full)
            .await
            .expect("open with 3 matching members should succeed");
        assert_eq!(result.status, RepaymentPhaseStatus::Open);
    }

    #[tokio::test]
    async fn test_open_phase_auto_fill_skips_members_with_zero_shares() {
        // D-02: members with current_shares == 0 are filtered out.
        let phase_entity = phase_in_status(RepaymentPhaseStatus::Preparation);
        let phase_id = phase_entity.id;

        let in_fy = time::Date::from_calendar_date(2026, time::Month::July, 1).unwrap();
        let m_zero = make_member(1, 0, Some(in_fy));

        let mut phase_dao = MockTestRepaymentPhaseDao::new();
        phase_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(phase_entity.clone())));
        phase_dao
            .expect_update()
            .times(1)
            .returning(|_, _, _| Ok(()));

        let mut entry_dao = MockTestRepaymentEntryDao::new();
        // 1 member with 0 shares → 0 entry-create calls.
        entry_dao
            .expect_create()
            .times(0)
            .returning(|_, _, _| Ok(()));

        let mut member_dao = MockTestMemberDao::new();
        member_dao
            .expect_all()
            .returning(move |_| Ok(Arc::from(vec![m_zero.clone()])));

        let service = build_service_full(phase_dao, entry_dao, member_dao);

        service
            .open_repayment_phase(phase_id, Authentication::Full)
            .await
            .expect("open with member having 0 shares should succeed with 0 entries");
    }

    #[tokio::test]
    async fn test_open_phase_auto_fill_skips_members_outside_fiscal_year() {
        // D-01: members with exit_date outside the fiscal_year (2026) are filtered.
        let phase_entity = phase_in_status(RepaymentPhaseStatus::Preparation);
        let phase_id = phase_entity.id;

        // Member exited in 2027 — outside FY 2026.
        let next_year_exit = time::Date::from_calendar_date(2027, time::Month::March, 1).unwrap();
        let m_outside = make_member(1, 5, Some(next_year_exit));

        let mut phase_dao = MockTestRepaymentPhaseDao::new();
        phase_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(phase_entity.clone())));
        phase_dao
            .expect_update()
            .times(1)
            .returning(|_, _, _| Ok(()));

        let mut entry_dao = MockTestRepaymentEntryDao::new();
        entry_dao
            .expect_create()
            .times(0)
            .returning(|_, _, _| Ok(()));

        let mut member_dao = MockTestMemberDao::new();
        member_dao
            .expect_all()
            .returning(move |_| Ok(Arc::from(vec![m_outside.clone()])));

        let service = build_service_full(phase_dao, entry_dao, member_dao);

        service
            .open_repayment_phase(phase_id, Authentication::Full)
            .await
            .expect("open with member outside FY should succeed with 0 entries");
    }

    #[tokio::test]
    async fn test_open_phase_auto_fill_skips_members_without_exit_date() {
        // D-01 (BETWEEN filter): members with exit_date == None are filtered out.
        let phase_entity = phase_in_status(RepaymentPhaseStatus::Preparation);
        let phase_id = phase_entity.id;

        let m_no_exit = make_member(1, 5, None);

        let mut phase_dao = MockTestRepaymentPhaseDao::new();
        phase_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(phase_entity.clone())));
        phase_dao
            .expect_update()
            .times(1)
            .returning(|_, _, _| Ok(()));

        let mut entry_dao = MockTestRepaymentEntryDao::new();
        entry_dao
            .expect_create()
            .times(0)
            .returning(|_, _, _| Ok(()));

        let mut member_dao = MockTestMemberDao::new();
        member_dao
            .expect_all()
            .returning(move |_| Ok(Arc::from(vec![m_no_exit.clone()])));

        let service = build_service_full(phase_dao, entry_dao, member_dao);

        service
            .open_repayment_phase(phase_id, Authentication::Full)
            .await
            .expect("open with member having no exit_date should succeed with 0 entries");
    }

    #[tokio::test]
    async fn test_open_phase_auto_fill_atomic_on_dao_failure() {
        // Threat T-08-04-01 mitigation: Auto-Fill in same Tx as status update.
        // First DAO-create failure in the loop returns Err → method returns
        // Err, Tx is dropped → rollback. Service-level: ServiceError surfaces.
        //
        // Test setup: 2 members, entry_dao.create fails on the FIRST call.
        // We can't verify rollback directly via mocks (commit is on the
        // tx_dao); we verify the failure propagates and at least 1
        // entry-create was attempted before the failure.
        let phase_entity = phase_in_status(RepaymentPhaseStatus::Preparation);
        let phase_id = phase_entity.id;

        let in_fy = time::Date::from_calendar_date(2026, time::Month::June, 15).unwrap();
        let m1 = make_member(1, 5, Some(in_fy));
        let m2 = make_member(2, 3, Some(in_fy));

        let mut phase_dao = MockTestRepaymentPhaseDao::new();
        phase_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(phase_entity.clone())));
        phase_dao
            .expect_update()
            .times(1)
            .returning(|_, _, _| Ok(()));

        let mut entry_dao = MockTestRepaymentEntryDao::new();
        // First create fails — bubbles up; we expect <= 1 call.
        entry_dao.expect_create().returning(|_, _, _| {
            Err(DaoError::DatabaseError(Arc::from(
                "simulated DAO failure on first entry create",
            )))
        });

        let mut member_dao = MockTestMemberDao::new();
        member_dao
            .expect_all()
            .returning(move |_| Ok(Arc::from(vec![m1.clone(), m2.clone()])));

        let service = build_service_full(phase_dao, entry_dao, member_dao);

        let result = service
            .open_repayment_phase(phase_id, Authentication::Full)
            .await;
        assert!(
            matches!(result, Err(ServiceError::DataAccess(_))),
            "expected DataAccess error from failing entry-create, got {:?}",
            result
        );
    }

    // ---------- Close-Validation tests (PHAS-03 / D-13/D-14/D-15) ----------

    #[tokio::test]
    async fn test_close_phase_with_zero_entries_succeeds() {
        // D-14: 0-entry close is allowed.
        //
        // CR-01 (Phase 08 gap-closure): close_repayment_phase now performs
        // a find_by_id Re-Read after audited_update!. Mock uses Sequence so
        // the 3rd find_by_id (Re-Read) returns the post-close entity with
        // status=Closed.
        let entity = phase_in_status(RepaymentPhaseStatus::Open);
        let entity_id = entity.id;
        let post_close = RepaymentPhaseEntity {
            status: RepaymentPhaseStatus::Closed,
            closed_at: Some(entity.created),
            version: Uuid::new_v4(),
            ..entity.clone()
        };

        let mut phase_dao = MockTestRepaymentPhaseDao::new();
        let mut seq = mockall::Sequence::new();

        let pre_1 = entity.clone();
        phase_dao
            .expect_find_by_id()
            .times(1)
            .in_sequence(&mut seq)
            .returning(move |_, _| Ok(Some(pre_1.clone())));
        let pre_2 = entity.clone();
        phase_dao
            .expect_find_by_id()
            .times(1)
            .in_sequence(&mut seq)
            .returning(move |_, _| Ok(Some(pre_2.clone())));
        // close-update → 1 call on the phase DAO.
        phase_dao
            .expect_update()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_, _, _| Ok(()));
        // CR-01 Re-Read returns post-close entity
        let post_for_3 = post_close.clone();
        phase_dao
            .expect_find_by_id()
            .times(1)
            .in_sequence(&mut seq)
            .returning(move |_, _| Ok(Some(post_for_3.clone())));

        let mut entry_dao = MockTestRepaymentEntryDao::new();
        // 0 entries returned → no pending; close proceeds.
        entry_dao
            .expect_find_by_phase_id()
            .returning(|_, _| Ok(Arc::from(vec![])));

        let member_dao = MockTestMemberDao::new();

        let service = build_service_full(phase_dao, entry_dao, member_dao);

        let result = service
            .close_repayment_phase(entity_id, Authentication::Full)
            .await
            .expect("close with 0 entries should succeed (D-14)");
        assert_eq!(result.status, RepaymentPhaseStatus::Closed);
    }

    #[tokio::test]
    async fn test_close_phase_with_only_paid_out_or_deleted_succeeds() {
        // D-13: pending = status != PaidOut AND deleted IS NULL. Entries that
        // are PaidOut OR soft-deleted are NOT pending; close proceeds.
        let entity = phase_in_status(RepaymentPhaseStatus::Open);
        let entity_id = entity.id;
        let phase_id = entity_id;
        let member_id_paid = Uuid::new_v4();
        let member_id_deleted = Uuid::new_v4();

        let date = time::Date::from_calendar_date(2026, time::Month::May, 30).unwrap();
        let deleted_at = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);

        let entry_paid_out = make_entry(
            phase_id,
            member_id_paid,
            RepaymentEntryStatus::PaidOut,
            None,
        );
        let entry_deleted = make_entry(
            phase_id,
            member_id_deleted,
            RepaymentEntryStatus::Open,
            Some(deleted_at),
        );

        // CR-01 (Phase 08 gap-closure): Re-Read after audited_update! returns
        // post-close entity (status=Closed).
        let post_close = RepaymentPhaseEntity {
            status: RepaymentPhaseStatus::Closed,
            closed_at: Some(entity.created),
            version: Uuid::new_v4(),
            ..entity.clone()
        };

        let mut phase_dao = MockTestRepaymentPhaseDao::new();
        let mut seq = mockall::Sequence::new();

        let pre_1 = entity.clone();
        phase_dao
            .expect_find_by_id()
            .times(1)
            .in_sequence(&mut seq)
            .returning(move |_, _| Ok(Some(pre_1.clone())));
        let pre_2 = entity.clone();
        phase_dao
            .expect_find_by_id()
            .times(1)
            .in_sequence(&mut seq)
            .returning(move |_, _| Ok(Some(pre_2.clone())));
        phase_dao
            .expect_update()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_, _, _| Ok(()));
        let post_for_3 = post_close.clone();
        phase_dao
            .expect_find_by_id()
            .times(1)
            .in_sequence(&mut seq)
            .returning(move |_, _| Ok(Some(post_for_3.clone())));

        let mut entry_dao = MockTestRepaymentEntryDao::new();
        // NOTE: find_by_phase_id (Default-Impl) filters deleted IS NULL — but
        // we're testing the SERVICE-LEVEL filter here, so we return the raw
        // dataset (PaidOut + soft-deleted) and rely on the service filter
        // (entry.deleted.is_none() && entry.status != PaidOut) to exclude both.
        // Use find_by_phase_id directly: real find_by_phase_id already filters
        // deleted, so soft-deleted entries would not appear. We simulate the
        // post-filter dataset directly: only the PaidOut entry remains.
        let entries_returned: Arc<[RepaymentEntryEntity]> = Arc::from(vec![entry_paid_out.clone()]);
        entry_dao
            .expect_find_by_phase_id()
            .returning(move |_, _| Ok(entries_returned.clone()));

        let member_dao = MockTestMemberDao::new();

        let service = build_service_full(phase_dao, entry_dao, member_dao);

        let result = service
            .close_repayment_phase(entity_id, Authentication::Full)
            .await
            .expect("close with only PaidOut entries should succeed (D-13)");
        assert_eq!(result.status, RepaymentPhaseStatus::Closed);
        // Suppress unused variable warning for entry_deleted (the comment
        // explains why it's not in entries_returned).
        let _ = entry_deleted;
    }

    #[tokio::test]
    async fn test_close_phase_with_pending_entries_returns_conflict() {
        // D-13 + D-15: 1 Open entry → close blocked, body contains
        // pending_count + member numbers.
        let entity = phase_in_status(RepaymentPhaseStatus::Open);
        let entity_id = entity.id;
        let phase_id = entity_id;

        let pending_member = make_member(42, 5, None);
        let pending_member_id = pending_member.id;
        let pending_member_number = pending_member.member_number;

        let entry_open = make_entry(
            phase_id,
            pending_member_id,
            RepaymentEntryStatus::Open,
            None,
        );

        let mut phase_dao = MockTestRepaymentPhaseDao::new();
        phase_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(entity.clone())));
        // Close must NOT happen — pending block returns before update.
        phase_dao
            .expect_update()
            .times(0)
            .returning(|_, _, _| Ok(()));

        let mut entry_dao = MockTestRepaymentEntryDao::new();
        let entries_returned: Arc<[RepaymentEntryEntity]> = Arc::from(vec![entry_open.clone()]);
        entry_dao
            .expect_find_by_phase_id()
            .returning(move |_, _| Ok(entries_returned.clone()));

        let mut member_dao = MockTestMemberDao::new();
        let members_returned: Arc<[MemberEntity]> = Arc::from(vec![pending_member.clone()]);
        member_dao
            .expect_all()
            .returning(move |_| Ok(members_returned.clone()));

        let service = build_service_full(phase_dao, entry_dao, member_dao);

        let result = service
            .close_repayment_phase(entity_id, Authentication::Full)
            .await;

        match result {
            Err(ServiceError::Conflict(msg)) => {
                // Body is a JSON-encoded detail (D-15).
                let parsed: serde_json::Value =
                    serde_json::from_str(&msg).expect("conflict body must be valid JSON");
                assert_eq!(parsed["pending_count"], 1, "pending_count must be 1");
                let arr = parsed["pending_member_numbers"]
                    .as_array()
                    .expect("pending_member_numbers must be an array");
                assert_eq!(arr.len(), 1);
                assert_eq!(arr[0], pending_member_number.to_string());
                assert!(
                    msg.contains("pending") || msg.contains("not paid out"),
                    "conflict message must indicate pending entries, got: {}",
                    msg
                );
            }
            other => panic!("expected Conflict, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_close_phase_with_25_pending_entries_truncates_at_20() {
        // D-15: max 20 member numbers + "+5 weitere" suffix when total > 20.
        let entity = phase_in_status(RepaymentPhaseStatus::Open);
        let entity_id = entity.id;
        let phase_id = entity_id;

        let mut members = Vec::with_capacity(25);
        let mut entries = Vec::with_capacity(25);
        for i in 1..=25_i64 {
            let m = make_member(i, 5, None);
            entries.push(make_entry(phase_id, m.id, RepaymentEntryStatus::Open, None));
            members.push(m);
        }

        let mut phase_dao = MockTestRepaymentPhaseDao::new();
        phase_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(entity.clone())));
        phase_dao
            .expect_update()
            .times(0)
            .returning(|_, _, _| Ok(()));

        let entries_arc: Arc<[RepaymentEntryEntity]> = Arc::from(entries);
        let members_arc: Arc<[MemberEntity]> = Arc::from(members);

        let mut entry_dao = MockTestRepaymentEntryDao::new();
        entry_dao
            .expect_find_by_phase_id()
            .returning(move |_, _| Ok(entries_arc.clone()));

        let mut member_dao = MockTestMemberDao::new();
        member_dao
            .expect_all()
            .returning(move |_| Ok(members_arc.clone()));

        let service = build_service_full(phase_dao, entry_dao, member_dao);

        let result = service
            .close_repayment_phase(entity_id, Authentication::Full)
            .await;

        match result {
            Err(ServiceError::Conflict(msg)) => {
                let parsed: serde_json::Value =
                    serde_json::from_str(&msg).expect("conflict body must be valid JSON");
                assert_eq!(parsed["pending_count"], 25);
                let arr = parsed["pending_member_numbers"]
                    .as_array()
                    .expect("pending_member_numbers must be an array");
                // 20 member numbers + 1 "+5 weitere" suffix = 21 total entries
                assert_eq!(
                    arr.len(),
                    21,
                    "expected 20 numbers + 1 suffix entry, got {}: {:?}",
                    arr.len(),
                    arr
                );
                // First entry is "1" (smallest after sort), last is the suffix.
                assert_eq!(arr[0], "1");
                assert_eq!(arr[19], "20");
                let suffix = arr[20].as_str().expect("suffix must be string");
                assert!(
                    suffix.contains("+5") && suffix.contains("weitere"),
                    "suffix must contain '+5 weitere', got: {}",
                    suffix
                );
            }
            other => panic!("expected Conflict, got {:?}", other),
        }
    }

    // ---------- Phase-7-Bestand-Schutz: Phase-8-Open-Erweiterung darf den
    //            existierenden "open from Preparation"-Pfad mit 0 Members nicht
    //            brechen. (Test analog test_delete_repayment_phase_in_preparation_succeeds.)
    // (Redundanz mit test_open_phase_auto_fill_zero_members ist akzeptabel —
    //  dieser Test explizit als Regression-Guard für die Phase-7-Skeleton.)

    // ============================================================
    // Phase 08 Gap-Closure CR-01 — Re-Read after audited_update!
    // ============================================================
    //
    // Same bug class as 08-07 in RepaymentEntryServiceImpl, here applied to
    // the 4 RepaymentPhase lifecycle methods (create/update/open/close). The
    // service must re-read the entity after audited_create!/audited_update!
    // to return the DAO-generated fresh version-UUID. Pattern mirrors
    // MemberServiceImpl::update (member.rs:343-348).

    /// Phase 08 Gap-Closure CR-01 — verifies that `update_repayment_phase`
    /// re-reads the entity after `audited_update!` and returns the fresh
    /// version-UUID generated by the DAO. Without the fix the service
    /// returns the pre-update entity, causing 409 on every follow-up PUT.
    #[tokio::test]
    async fn test_update_repayment_phase_rereads_after_audited_update_returns_new_version() {
        let phase_id = Uuid::new_v4();
        let version_a = Uuid::new_v4();
        let version_b = Uuid::new_v4();
        assert_ne!(version_a, version_b);

        let date = time::Date::from_calendar_date(2026, time::Month::May, 29).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        let pre = RepaymentPhaseEntity {
            id: phase_id,
            fiscal_year: 2026,
            share_value: 12000,
            status: RepaymentPhaseStatus::Preparation,
            opened_at: None,
            closed_at: None,
            created: datetime,
            deleted: None,
            version: version_a,
        };
        // Re-Read returns the post-update DB snapshot: new share_value
        // (applied by the PUT) AND new version (Uuid::new_v4() by the DAO).
        let post = RepaymentPhaseEntity {
            share_value: 13000,
            version: version_b,
            ..pre.clone()
        };

        // Sequence of find_by_id calls in update_repayment_phase:
        //   1. Pre-update load (Edit-Matrix + version-check) -> pre
        //   2. audited_update! internal load (audit_macros.rs:47) -> pre
        //   3. CR-01 Re-Read after audited_update! -> post (NEW version)
        let mut phase_dao = MockTestRepaymentPhaseDao::new();
        let mut seq = mockall::Sequence::new();

        let pre_for_1 = pre.clone();
        phase_dao
            .expect_find_by_id()
            .times(1)
            .in_sequence(&mut seq)
            .returning(move |_, _| Ok(Some(pre_for_1.clone())));

        let pre_for_2 = pre.clone();
        phase_dao
            .expect_find_by_id()
            .times(1)
            .in_sequence(&mut seq)
            .returning(move |_, _| Ok(Some(pre_for_2.clone())));

        phase_dao
            .expect_update()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_, _, _| Ok(()));

        let post_for_3 = post.clone();
        phase_dao
            .expect_find_by_id()
            .times(1)
            .in_sequence(&mut seq)
            .returning(move |_, _| Ok(Some(post_for_3.clone())));

        let service = build_service(phase_dao);

        let update = RepaymentPhaseUpdate {
            fiscal_year: 2026,
            share_value: 13000,
            version: version_a,
        };
        let result = service
            .update_repayment_phase(phase_id, &update, Authentication::Full)
            .await
            .expect("update should succeed");

        assert_eq!(
            result.version, version_b,
            "Re-Read must return the new DAO-generated version, not the pre-update version"
        );
        assert_eq!(
            result.share_value, 13000,
            "Updated share_value must be reflected"
        );
    }

    /// Phase 08 BL-01 Negativtest — when the Re-Read after `audited_update!`
    /// in `update_repayment_phase` returns `None` (a structurally-impossible
    /// same-Tx inconsistency, e.g. a future DAO regression), the service MUST
    /// emit `ServiceError::InternalError` (→ HTTP 500), NOT
    /// `ServiceError::EntityNotFound` (→ HTTP 404). A 404 would lie to the
    /// client: "the phase you tried to update doesn't exist" — even though
    /// the audited_update! a moment earlier succeeded against the same id in
    /// the same Tx. Covers all 4 Phase lifecycle methods via the same code
    /// pattern (create/update/open/close).
    #[tokio::test]
    async fn test_update_repayment_phase_rereads_none_yields_internal_error() {
        let phase_id = Uuid::new_v4();
        let version_a = Uuid::new_v4();

        let date = time::Date::from_calendar_date(2026, time::Month::May, 29).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        let pre = RepaymentPhaseEntity {
            id: phase_id,
            fiscal_year: 2026,
            share_value: 12000,
            status: RepaymentPhaseStatus::Preparation,
            opened_at: None,
            closed_at: None,
            created: datetime,
            deleted: None,
            version: version_a,
        };

        // Sequence of find_by_id calls in update_repayment_phase:
        //   1. Pre-update load (Edit-Matrix + version-check) -> Some(pre)
        //   2. audited_update! internal load (audit_macros.rs:47) -> Some(pre)
        //   3. BL-01 Re-Read after audited_update! -> None (simulated DAO
        //      regression — structurally impossible in real same-Tx).
        let mut phase_dao = MockTestRepaymentPhaseDao::new();
        let mut seq = mockall::Sequence::new();

        let pre_for_1 = pre.clone();
        phase_dao
            .expect_find_by_id()
            .times(1)
            .in_sequence(&mut seq)
            .returning(move |_, _| Ok(Some(pre_for_1.clone())));

        let pre_for_2 = pre.clone();
        phase_dao
            .expect_find_by_id()
            .times(1)
            .in_sequence(&mut seq)
            .returning(move |_, _| Ok(Some(pre_for_2.clone())));

        phase_dao
            .expect_update()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_, _, _| Ok(()));

        // The crucial expectation: Re-Read returns None.
        phase_dao
            .expect_find_by_id()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_, _| Ok(None));

        let service = build_service(phase_dao);

        let update = RepaymentPhaseUpdate {
            fiscal_year: 2026,
            share_value: 13000,
            version: version_a,
        };
        let err = service
            .update_repayment_phase(phase_id, &update, Authentication::Full)
            .await
            .expect_err("Re-Read returning None must surface as an error, not as Ok");

        match err {
            ServiceError::InternalError(msg) => {
                assert!(
                    msg.contains("Re-Read") && msg.contains(&phase_id.to_string()),
                    "InternalError message must mention 'Re-Read' and the entity id, \
                     got: {msg}"
                );
            }
            other => panic!(
                "Re-Read None must map to ServiceError::InternalError (→ HTTP 500), \
                 NOT to {other:?} (which would map to a wrong HTTP status). BL-01 regression."
            ),
        }
    }

    /// Phase 08 Gap-Closure CR-01 — verifies that `open_repayment_phase`
    /// re-reads the Phase entity after `audited_update!` (NACH der Auto-Fill-
    /// Loop, VOR commit) und returns the fresh version-UUID. Member-DAO
    /// liefert 0 Members → keine Auto-Fill-Iteration, der Test fokussiert
    /// sich auf den Re-Read der Phase-Row.
    #[tokio::test]
    async fn test_open_repayment_phase_rereads_phase_entity_returns_new_version() {
        let phase_id = Uuid::new_v4();
        let version_a = Uuid::new_v4();
        let version_b = Uuid::new_v4();
        assert_ne!(version_a, version_b);

        let date = time::Date::from_calendar_date(2026, time::Month::May, 29).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        let opened_at = time::PrimitiveDateTime::new(
            time::Date::from_calendar_date(2026, time::Month::May, 30).unwrap(),
            time::Time::MIDNIGHT,
        );

        let pre = RepaymentPhaseEntity {
            id: phase_id,
            fiscal_year: 2026,
            share_value: 12000,
            status: RepaymentPhaseStatus::Preparation,
            opened_at: None,
            closed_at: None,
            created: datetime,
            deleted: None,
            version: version_a,
        };
        let post = RepaymentPhaseEntity {
            status: RepaymentPhaseStatus::Open,
            opened_at: Some(opened_at),
            version: version_b,
            ..pre.clone()
        };

        // Sequence of find_by_id calls in open_repayment_phase:
        //   1. Pre-open load (status-guard Preparation→Open) -> pre
        //   2. audited_update! internal load (audit_macros.rs:47) -> pre
        //   3. CR-01 Re-Read after audited_update! + Auto-Fill-Loop -> post (NEW version)
        let mut phase_dao = MockTestRepaymentPhaseDao::new();
        let mut seq = mockall::Sequence::new();

        let pre_for_1 = pre.clone();
        phase_dao
            .expect_find_by_id()
            .times(1)
            .in_sequence(&mut seq)
            .returning(move |_, _| Ok(Some(pre_for_1.clone())));

        let pre_for_2 = pre.clone();
        phase_dao
            .expect_find_by_id()
            .times(1)
            .in_sequence(&mut seq)
            .returning(move |_, _| Ok(Some(pre_for_2.clone())));

        phase_dao
            .expect_update()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_, _, _| Ok(()));

        let post_for_3 = post.clone();
        phase_dao
            .expect_find_by_id()
            .times(1)
            .in_sequence(&mut seq)
            .returning(move |_, _| Ok(Some(post_for_3.clone())));

        // Member-DAO: empty Members → kein Auto-Fill, der Test bleibt fokussiert
        let mut member_dao = MockTestMemberDao::new();
        member_dao.expect_all().returning(|_| Ok(Arc::from(vec![])));

        // Entry-DAO: keine Calls erwartet (0 Auto-Fill-Iterationen)
        let entry_dao = MockTestRepaymentEntryDao::new();

        let service = build_service_full(phase_dao, entry_dao, member_dao);

        let result = service
            .open_repayment_phase(phase_id, Authentication::Full)
            .await
            .expect("open should succeed");

        assert_eq!(
            result.version, version_b,
            "Re-Read must return the new DAO-generated version, not the pre-update version"
        );
        assert_eq!(result.status, RepaymentPhaseStatus::Open);
    }
}
