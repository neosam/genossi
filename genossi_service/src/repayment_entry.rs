//! Service-layer domain type and trait for the RepaymentEntry aggregate.
//!
//! Phase 8 Plan 03: `RepaymentEntryService` trait with CRUD methods
//! (`create`, `update`, `delete`, `get`, `list_by_phase`) plus the
//! `batch_toggle_status` endpoint for the all-or-nothing toggling of
//! `Open ↔ Contacted` (D-06, D-08).
//!
//! Pattern-Anker: `genossi_service/src/repayment_phase.rs` (1:1-Vorlage),
//! Domain-Substitutionen aus 08-CONTEXT.md (`member_id` + `phase_id` als
//! FK-UUIDs, `share_count_to_pay_out: i32`, `RepaymentEntryStatus { Open,
//! Contacted, PaidOut }`).
//!
//! Wichtige Unterschiede zur RepaymentPhase-Vorlage:
//! - **DTO `RepaymentEntryUpdate`** ist optional-field-based: Felder, die
//!   nicht im Body stehen, bleiben unverändert (D-12); `version` ist Pflicht.
//! - **`batch_toggle_status`** ist neu — ein dedizierter Endpoint für das
//!   atomare Toggle mehrerer Eintrags-Statuswechsel in einer Transaktion
//!   (D-08). PaidOut als Target ist verboten (D-07).
//! - **Kein Lifecycle-Endpoint** (kein open/close) — RepaymentEntries werden
//!   beim Phase-Open auto-erzeugt (PHAS-02, Plan 04) und manuell ergänzt.

use async_trait::async_trait;
use genossi_dao::repayment_entry::{RepaymentEntryEntity, RepaymentEntryStatus};
use mockall::automock;
use std::fmt::Debug;
use std::sync::Arc;
use uuid::Uuid;

use crate::permission::Authentication;
use crate::ServiceError;

/// Service-layer representation of a RepaymentEntry (Anteile-Rückzahlungs-Eintrag).
///
/// Mirrors `RepaymentEntryEntity` from `genossi_dao::repayment_entry` —
/// alle Felder sind Copy/Clone, die Konvertierung ist feldweise.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepaymentEntry {
    pub id: Uuid,
    pub member_id: Uuid,
    pub phase_id: Uuid,
    pub share_count_to_pay_out: i32,
    pub status: RepaymentEntryStatus,
    pub created: time::PrimitiveDateTime,
    pub deleted: Option<time::PrimitiveDateTime>,
    pub version: Uuid,
}

impl From<&RepaymentEntryEntity> for RepaymentEntry {
    fn from(e: &RepaymentEntryEntity) -> Self {
        Self {
            id: e.id,
            member_id: e.member_id,
            phase_id: e.phase_id,
            share_count_to_pay_out: e.share_count_to_pay_out,
            status: e.status.clone(),
            created: e.created,
            deleted: e.deleted,
            version: e.version,
        }
    }
}

impl From<&RepaymentEntry> for RepaymentEntryEntity {
    fn from(e: &RepaymentEntry) -> Self {
        Self {
            id: e.id,
            member_id: e.member_id,
            phase_id: e.phase_id,
            share_count_to_pay_out: e.share_count_to_pay_out,
            status: e.status.clone(),
            created: e.created,
            deleted: e.deleted,
            version: e.version,
        }
    }
}

/// Input for creating a new RepaymentEntry. Status startet immer in `Open`
/// (D-05), die Service-Schicht setzt id/version/created automatisch.
///
/// Validierung (D-11) erfolgt in `create_repayment_entry`:
/// - Phase muss existieren und Status `Open` haben (D-11.1)
/// - Member muss existieren (deleted IS NULL) (D-11.2)
/// - `share_count_to_pay_out` ∈ (0, `Member.current_shares`] (D-11.3)
#[derive(Clone, Debug)]
pub struct RepaymentEntrySubmission {
    pub phase_id: Uuid,
    pub member_id: Uuid,
    pub share_count_to_pay_out: i32,
}

/// Input für PUT /api/repayment-entry/{id}. Optional-Field-Pattern:
/// Felder, die nicht im Body stehen, bleiben unverändert (D-12).
///
/// Edit-Matrix (D-05, D-06, ENTR-04, durchgesetzt im Service-Impl):
/// - `entity.status == PaidOut` → Update verboten (D-05, final)
/// - `update.status == Some(PaidOut)` → 409 ("use Phase-9 mark_paid_out", D-05/D-07)
/// - `share_count_to_pay_out`-Edit nur wenn `entity.status ∈ {Open, Contacted}` (ENTR-04)
/// - Status-Toggle bidirektional `Open ↔ Contacted` (D-06)
/// - `version` ist Pflicht (optimistic locking)
#[derive(Clone, Debug)]
pub struct RepaymentEntryUpdate {
    pub share_count_to_pay_out: Option<i32>,
    pub status: Option<RepaymentEntryStatus>,
    pub version: Uuid,
}

/// Input für POST /api/repayment-entry/batch-status. All-or-nothing
/// (D-08): erster Fehler → komplette Tx rollt zurück, 409 mit strukturiertem
/// JSON-Body `{ failure_index, failure_id, failure_reason }`.
///
/// `target_status` darf nur `Open` oder `Contacted` sein; `PaidOut` als
/// Target → 400 ValidationError (D-07).
#[derive(Clone, Debug)]
pub struct RepaymentEntryBatchStatusInput {
    pub entry_ids: Arc<[Uuid]>,
    pub target_status: RepaymentEntryStatus,
}

#[automock(type Context = (); type Transaction = genossi_dao::MockTransaction;)]
#[async_trait]
pub trait RepaymentEntryService {
    type Context: Clone + Debug + PartialEq + Eq + Send + Sync + 'static;
    type Transaction: genossi_dao::Transaction;

    /// Create a new RepaymentEntry in status `Open` (D-05).
    /// Validation: D-11.1 (phase Open), D-11.2 (member exists), D-11.3 (range).
    /// Audit-process `repayment-entry.create`. Requires `admin`.
    async fn create_repayment_entry(
        &self,
        submission: &RepaymentEntrySubmission,
        context: Authentication<Self::Context>,
    ) -> Result<RepaymentEntry, ServiceError>;

    /// Update share_count_to_pay_out and/or status of an existing entry.
    /// Edit-Matrix (D-05/D-06/ENTR-04). PaidOut als Target → 409.
    /// Optimistic locking via version. Audit-process `repayment-entry.update`.
    async fn update_repayment_entry(
        &self,
        id: Uuid,
        update: &RepaymentEntryUpdate,
        context: Authentication<Self::Context>,
    ) -> Result<RepaymentEntry, ServiceError>;

    /// Soft-delete a RepaymentEntry (ENTR-05).
    /// Guard: `entity.status != PaidOut`, sonst 409.
    /// Audit-process `repayment-entry.delete`.
    async fn delete_repayment_entry(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
    ) -> Result<(), ServiceError>;

    /// Get a single RepaymentEntry by id. Requires `admin`.
    async fn get_repayment_entry(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
    ) -> Result<RepaymentEntry, ServiceError>;

    /// List all active RepaymentEntries for a phase (D-10: phase_id filter
    /// only; weitere Filter sind Frontend-Concern). Requires `admin`.
    async fn list_repayment_entries_by_phase(
        &self,
        phase_id: Uuid,
        context: Authentication<Self::Context>,
    ) -> Result<Arc<[RepaymentEntry]>, ServiceError>;

    /// Atomically toggle status of multiple entries (D-08). PaidOut als
    /// target → 400. Source-Status muss `Open` oder `Contacted` sein.
    /// Erster Fehler → Tx-Rollback durch Drop + 409 mit strukturiertem
    /// JSON-Body. Audit-process `repayment-entry.batch-toggle`.
    async fn batch_toggle_status(
        &self,
        input: &RepaymentEntryBatchStatusInput,
        context: Authentication<Self::Context>,
    ) -> Result<Arc<[RepaymentEntry]>, ServiceError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entity() -> RepaymentEntryEntity {
        let date = time::Date::from_calendar_date(2026, time::Month::May, 30).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        RepaymentEntryEntity {
            id: Uuid::new_v4(),
            member_id: Uuid::new_v4(),
            phase_id: Uuid::new_v4(),
            share_count_to_pay_out: 7,
            status: RepaymentEntryStatus::Open,
            created: datetime,
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    #[test]
    fn entity_to_repayment_entry_roundtrip() {
        // Alle 8 Felder müssen den Entity → Domain → Entity Roundtrip
        // verlustfrei überstehen (alle Copy/Clone, Konvertierung feldweise).
        let entity = make_entity();
        let domain: RepaymentEntry = (&entity).into();
        let back: RepaymentEntryEntity = (&domain).into();
        assert_eq!(back.id, entity.id);
        assert_eq!(back.member_id, entity.member_id);
        assert_eq!(back.phase_id, entity.phase_id);
        assert_eq!(back.share_count_to_pay_out, entity.share_count_to_pay_out);
        assert_eq!(back.status, entity.status);
        assert_eq!(back.created, entity.created);
        assert_eq!(back.deleted, entity.deleted);
        assert_eq!(back.version, entity.version);
        assert_eq!(back, entity);
    }

    #[test]
    fn test_repayment_entry_submission_constructible() {
        // 3 Felder: phase_id, member_id, share_count_to_pay_out.
        // Status/id/created/version setzt der Service.
        let phase_id = Uuid::new_v4();
        let member_id = Uuid::new_v4();
        let s = RepaymentEntrySubmission {
            phase_id,
            member_id,
            share_count_to_pay_out: 5,
        };
        assert_eq!(s.phase_id, phase_id);
        assert_eq!(s.member_id, member_id);
        assert_eq!(s.share_count_to_pay_out, 5);
    }

    #[test]
    fn test_repayment_entry_update_requires_version() {
        // share_count_to_pay_out + status sind Optional; version ist Pflicht.
        // Beispiel: nur Status togglen (share_count_to_pay_out = None).
        let version = Uuid::new_v4();
        let update = RepaymentEntryUpdate {
            share_count_to_pay_out: None,
            status: Some(RepaymentEntryStatus::Contacted),
            version,
        };
        assert_eq!(update.version, version);
        assert!(update.share_count_to_pay_out.is_none());
        assert_eq!(update.status, Some(RepaymentEntryStatus::Contacted));

        // Beispiel: nur share_count_to_pay_out ändern.
        let update_count_only = RepaymentEntryUpdate {
            share_count_to_pay_out: Some(3),
            status: None,
            version,
        };
        assert_eq!(update_count_only.share_count_to_pay_out, Some(3));
        assert!(update_count_only.status.is_none());
    }

    #[test]
    fn test_batch_status_input_constructible() {
        // entry_ids als Arc<[Uuid]>, target_status als RepaymentEntryStatus.
        let ids: Arc<[Uuid]> = vec![Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()].into();
        let ids_for_assert = ids.clone();
        let input = RepaymentEntryBatchStatusInput {
            entry_ids: ids,
            target_status: RepaymentEntryStatus::Contacted,
        };
        assert_eq!(input.entry_ids.len(), 3);
        assert_eq!(input.target_status, RepaymentEntryStatus::Contacted);
        assert_eq!(input.entry_ids[0], ids_for_assert[0]);
    }

    #[test]
    fn test_mock_repayment_entry_service_compiles() {
        // Compile-only Test: #[automock] muss MockRepaymentEntryService
        // generieren mit expect_* für jede der 6 Trait-Methoden. Fehlt eine,
        // schlägt der Compile fehl.
        let mut mock = MockRepaymentEntryService::new();
        let _ = mock.expect_create_repayment_entry();
        let _ = mock.expect_update_repayment_entry();
        let _ = mock.expect_delete_repayment_entry();
        let _ = mock.expect_get_repayment_entry();
        let _ = mock.expect_list_repayment_entries_by_phase();
        let _ = mock.expect_batch_toggle_status();
    }
}
