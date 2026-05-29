//! Service-layer domain types and trait for the RepaymentPhase aggregate.
//!
//! Phase 7 Plan 03: full `RepaymentPhaseService` trait with lifecycle
//! methods (`create`, `update`, `open`, `close`, `delete`, `get`,
//! `get_all`) plus the input DTOs (`RepaymentPhaseSubmission`,
//! `RepaymentPhaseUpdate`). Pattern-Anker: `genossi_service/src/assembly.rs`
//! (1:1-Vorlage), Domain-Substitutionen aus 07-CONTEXT.md (`fiscal_year: i32`,
//! `share_value: i64` in Cent, kein `name`/`date`/`location`).
//!
//! Unterschiede zur Assembly-Vorlage:
//! - **Kein** `RepaymentPhaseDetail`-Wrapper-Typ — Phase 7 hat keinen
//!   Snapshot-Counter, `get_repayment_phase` liefert direkt `RepaymentPhase`
//!   (CONTEXT.md `<canonical_refs>`, PATTERNS §4).
//! - **Plus** `delete_repayment_phase` — Assembly hat keinen DELETE,
//!   aber D-09 macht Soft-Delete (nur in `Preparation`) zur Pflicht-Operation.

use async_trait::async_trait;
use genossi_dao::repayment_phase::{RepaymentPhaseEntity, RepaymentPhaseStatus};
use mockall::automock;
use std::fmt::Debug;
use std::sync::Arc;
use uuid::Uuid;

use crate::permission::Authentication;
use crate::ServiceError;

/// Service-layer representation of a RepaymentPhase (Anteile-Rückzahlungsphase).
///
/// Mirrors `RepaymentPhaseEntity` from `genossi_dao::repayment_phase` —
/// since both `fiscal_year: i32` and `share_value: i64` are Copy-types,
/// the conversion is straight feldweise.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepaymentPhase {
    pub id: Uuid,
    pub fiscal_year: i32,
    pub share_value: i64,
    pub status: RepaymentPhaseStatus,
    pub opened_at: Option<time::PrimitiveDateTime>,
    pub closed_at: Option<time::PrimitiveDateTime>,
    pub created: time::PrimitiveDateTime,
    pub deleted: Option<time::PrimitiveDateTime>,
    pub version: Uuid,
}

impl From<&RepaymentPhaseEntity> for RepaymentPhase {
    fn from(entity: &RepaymentPhaseEntity) -> Self {
        Self {
            id: entity.id,
            fiscal_year: entity.fiscal_year,
            share_value: entity.share_value,
            status: entity.status.clone(),
            opened_at: entity.opened_at,
            closed_at: entity.closed_at,
            created: entity.created,
            deleted: entity.deleted,
            version: entity.version,
        }
    }
}

impl From<&RepaymentPhase> for RepaymentPhaseEntity {
    fn from(p: &RepaymentPhase) -> Self {
        Self {
            id: p.id,
            fiscal_year: p.fiscal_year,
            share_value: p.share_value,
            status: p.status.clone(),
            opened_at: p.opened_at,
            closed_at: p.closed_at,
            created: p.created,
            deleted: p.deleted,
            version: p.version,
        }
    }
}

/// Input for creating a new RepaymentPhase. The service sets status,
/// opened_at, closed_at, version, created automatically (PHAS-01).
///
/// Field-level validation (D-11: `fiscal_year` in 2000..=2100, D-12:
/// `share_value > 0`) happens in `RepaymentPhaseServiceImpl::create_*`.
#[derive(Clone, Debug)]
pub struct RepaymentPhaseSubmission {
    pub fiscal_year: i32,
    pub share_value: i64,
}

/// Input for updating an existing RepaymentPhase. The Edit-Matrix (D-04)
/// is enforced in the service impl:
/// - `Preparation`: alle Felder editierbar
/// - `Open`: nur `share_value` editierbar; jede Mutation an `fiscal_year`
///   wird mit `ServiceError::Conflict` atomar abgelehnt (D-07).
/// - `Closed`: kein Update möglich (PHAS-04).
///
/// `version` ist Pflicht (optimistic locking).
#[derive(Clone, Debug)]
pub struct RepaymentPhaseUpdate {
    pub fiscal_year: i32,
    pub share_value: i64,
    pub version: Uuid,
}

#[automock(type Context = (); type Transaction = genossi_dao::MockTransaction;)]
#[async_trait]
pub trait RepaymentPhaseService {
    type Context: Clone + Debug + PartialEq + Eq + Send + Sync + 'static;
    type Transaction: genossi_dao::Transaction;

    /// Create a new RepaymentPhase in status `Preparation` (PHAS-01).
    /// Audit-process `repayment-phase.create`. Requires `admin` privilege.
    async fn create_repayment_phase(
        &self,
        submission: &RepaymentPhaseSubmission,
        context: Authentication<Self::Context>,
    ) -> Result<RepaymentPhase, ServiceError>;

    /// Update the editable fields of a RepaymentPhase according to the
    /// Edit-Matrix (D-04). In `Open` only `share_value` may change; any
    /// attempt to mutate `fiscal_year` is atomically rejected (D-07).
    /// `Closed` is final — every update returns Conflict.
    /// Requires matching version (optimistic locking).
    /// Audit-process `repayment-phase.update`.
    async fn update_repayment_phase(
        &self,
        id: Uuid,
        update: &RepaymentPhaseUpdate,
        context: Authentication<Self::Context>,
    ) -> Result<RepaymentPhase, ServiceError>;

    /// Transition `Preparation` → `Open` (PHAS-02 skeleton).
    /// Status-Guard `!= Preparation` → 409 Conflict (D-05/D-06: no
    /// reverse-transition, no re-opening).
    /// Audit-process `repayment-phase.open`. Requires `admin`.
    ///
    /// **Phase 8 wird hier die Auto-Befüllung der `RepaymentEntries`
    /// ergänzen (PHAS-02 vollständig).**
    async fn open_repayment_phase(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
    ) -> Result<RepaymentPhase, ServiceError>;

    /// Transition `Open` → `Closed` (PHAS-03 skeleton).
    /// Status-Guard `!= Open` → 409 Conflict.
    /// Audit-process `repayment-phase.close`. Requires `admin`.
    ///
    /// **Phase 8 wird hier die Pending-Entry-Validation ergänzen
    /// ("alle `RepaymentEntries` paid_out oder soft-deleted") — PHAS-03
    /// vollständig.**
    async fn close_repayment_phase(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
    ) -> Result<RepaymentPhase, ServiceError>;

    /// Soft-delete a RepaymentPhase (D-09: only in `Preparation`).
    /// Any other status returns Conflict.
    /// Audit-process `repayment-phase.delete`. Requires `admin`.
    async fn delete_repayment_phase(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
    ) -> Result<(), ServiceError>;

    /// Get a single RepaymentPhase by id. Requires `admin`.
    async fn get_repayment_phase(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
    ) -> Result<RepaymentPhase, ServiceError>;

    /// List all non-deleted RepaymentPhases (D-10: `deleted IS NULL`
    /// filter is applied at DAO-trait level). Requires `admin`.
    async fn get_all_repayment_phases(
        &self,
        context: Authentication<Self::Context>,
    ) -> Result<Arc<[RepaymentPhase]>, ServiceError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entity() -> RepaymentPhaseEntity {
        let date = time::Date::from_calendar_date(2026, time::Month::May, 29).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        RepaymentPhaseEntity {
            id: Uuid::new_v4(),
            fiscal_year: 2026,
            share_value: 12000, // 120,00 EUR in Cent
            status: RepaymentPhaseStatus::Preparation,
            opened_at: None,
            closed_at: None,
            created: datetime,
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    #[test]
    fn entity_to_repayment_phase_roundtrip() {
        // All 9 fields must survive the Entity → Domain → Entity roundtrip
        // without loss. Since both fiscal_year (i32) and share_value (i64)
        // are Copy-types and status is Clone, the conversion is feldweise.
        let entity = make_entity();
        let domain: RepaymentPhase = (&entity).into();
        let back: RepaymentPhaseEntity = (&domain).into();
        assert_eq!(back.id, entity.id);
        assert_eq!(back.fiscal_year, entity.fiscal_year);
        assert_eq!(back.share_value, entity.share_value);
        assert_eq!(back.status, entity.status);
        assert_eq!(back.opened_at, entity.opened_at);
        assert_eq!(back.closed_at, entity.closed_at);
        assert_eq!(back.created, entity.created);
        assert_eq!(back.deleted, entity.deleted);
        assert_eq!(back.version, entity.version);
        assert_eq!(back, entity);
    }

    #[test]
    fn test_repayment_phase_submission_constructible() {
        // RepaymentPhaseSubmission has exactly 2 fields (no status, no
        // opened_at, no version) — service sets the rest.
        let submission = RepaymentPhaseSubmission {
            fiscal_year: 2026,
            share_value: 12000,
        };
        assert_eq!(submission.fiscal_year, 2026);
        assert_eq!(submission.share_value, 12000);
    }

    #[test]
    fn test_repayment_phase_update_requires_version() {
        // RepaymentPhaseUpdate has exactly 3 fields: fiscal_year,
        // share_value, version (no status — Lifecycle goes via /open
        // /close per D-02). version is mandatory (optimistic locking).
        let version = Uuid::new_v4();
        let update = RepaymentPhaseUpdate {
            fiscal_year: 2027,
            share_value: 13000,
            version,
        };
        assert_eq!(update.version, version);
        assert_eq!(update.fiscal_year, 2027);
        assert_eq!(update.share_value, 13000);
    }

    #[test]
    fn test_mock_repayment_phase_service_compiles() {
        // Compile-only test: ensure #[automock] generated
        // MockRepaymentPhaseService and that all 7 trait methods are
        // mockable via .expect_<method_name>(). The .new() call
        // verifies that the mock struct itself compiles.
        let mut mock = MockRepaymentPhaseService::new();
        // Verify each of the 7 expect_* builders compiles — if any
        // trait method were missing, this test would fail to compile.
        let _ = mock.expect_create_repayment_phase();
        let _ = mock.expect_update_repayment_phase();
        let _ = mock.expect_open_repayment_phase();
        let _ = mock.expect_close_repayment_phase();
        let _ = mock.expect_delete_repayment_phase();
        let _ = mock.expect_get_repayment_phase();
        let _ = mock.expect_get_all_repayment_phases();
    }
}
