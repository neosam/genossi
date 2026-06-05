//! v1.2 Mitgliedschaft-Anpassungen — Service-Trait (Foundation fuer Phase 15-17).
//!
//! Phase 15 definiert `cancel_membership` + `increase_shares`. Phase 16 ergaenzt
//! `partial_repayment`, Phase 17 ergaenzt `transfer_shares` (D-15-13 inkrementelles Wachsen).

use async_trait::async_trait;
use mockall::automock;
use std::fmt::Debug;
use uuid::Uuid;

use crate::member::Member;
use crate::member_action::MemberAction;
use crate::permission::Authentication;
use crate::repayment_entry::RepaymentEntry;
use crate::repayment_phase::RepaymentPhase;
use crate::ServiceError;

/// Service-Trait fuer v1.2-Mitgliedschaft-Anpassungen (PERM-01: admin-only).
///
/// Beide Phase-15-Methoden geben `(MemberAction, Member)` zurueck, damit das Frontend
/// nach dem Commit ohne zusaetzlichen GET-Round-Trip rendern kann (D-15-11).
#[automock(type Context=(); type Transaction = genossi_dao::MockTransaction;)]
#[async_trait]
pub trait MembershipAdjustService {
    type Context: Clone + Debug + PartialEq + Eq + Send + Sync + 'static;
    type Transaction: genossi_dao::Transaction;

    /// Kuendigt eine Mitgliedschaft via `MemberAction::Austritt` (CANC-01..05).
    async fn cancel_membership(
        &self,
        member_id: Uuid,
        willensbekundung_date: time::Date,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<(MemberAction, Member), ServiceError>;

    /// Stockt die Anteile eines aktiven Mitglieds atomar auf (UPGD-01..04).
    async fn increase_shares(
        &self,
        member_id: Uuid,
        shares: i32,
        willensbekundung_date: time::Date,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<(MemberAction, Member), ServiceError>;

    /// Erzeugt einen `RepaymentEntry` in der Ziel-Phase fuer eine Teil-Rueckgabe
    /// (PART-01, PART-03, PART-05, PART-06).
    ///
    /// - `shares` ist die Anzahl Anteile (`1 <= shares < member.current_shares`,
    ///   D-16-11/12). Type ist `i32` konsistent mit `MemberEntity.current_shares`
    ///   und `RepaymentEntryEntity.share_count_to_pay_out` (Research Pitfall 2).
    /// - `willensbekundung_date` muss im aktuellen oder naechsten Geschaeftsjahr
    ///   liegen (D-16-18, wiederverwendet `validate_willensbekundung_date`).
    /// - Wenn die Ziel-`RepaymentPhase` fuer das per `compute_effective_date`
    ///   ermittelte `fiscal_year` noch nicht existiert, wird sie automatisch in
    ///   Status `Open` angelegt (D-16-01 Variante B) und im Return als
    ///   `Some(phase)` geliefert. Andernfalls `None`.
    /// - Gekuendigte Mitglieder (`exit_date IS NOT NULL`) werden mit
    ///   `ServiceError::Conflict` geblockt (D-16-10 -> HTTP 409).
    /// - **PART-06 / D-16-19:** Diese Methode erzeugt KEINE `MemberAction` und
    ///   mutiert `Member.current_shares` NICHT. Die v1.1-PaidOut-Cascade
    ///   uebernimmt beides beim spaeteren Ausbezahlt-Toggle.
    ///   `recalc_dates`/`recalc_migrated` werden NICHT aufgerufen.
    async fn partial_repayment(
        &self,
        member_id: Uuid,
        shares: i32,
        willensbekundung_date: time::Date,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<(Member, RepaymentEntry, Option<RepaymentPhase>), ServiceError>;
}
