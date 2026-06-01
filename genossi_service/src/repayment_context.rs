//! Phase 13 D-13-04 / D-13-10: Shared aggregation resolver fuer RepaymentEntry-Kontext.
//!
//! Extrahiert die heute in `genossi_mail/src/worker.rs:332-360` inline lebende
//! Logik (Filter Open+Contacted, SUM share_count, deutsche Euro-Formatierung) in
//! eine testbare, mockable Komponente.
//!
//! Zwei Eingaenge:
//! - `resolve(phase_id, member_id, tx)` — laedt Phase + Entries selbst (Worker-Use-Case).
//! - `aggregate(phase, entries, member_id)` — pure-fn-Wrapper, kein DB-Round-Trip
//!   (Letter-Service Plan 04 laedt phase+entries EINMAL und ruft aggregate N-mal).
//!
//! Erster Caller von `aggregate`: `RepaymentLetterServiceImpl` (Plan 04).
//! Folge-Caller von `resolve`: Mail-Worker-Refactor (out-of-scope hier, siehe
//! Todo `.planning/todos/pending/phase-10-worker-refactor-resolver.md`).

use async_trait::async_trait;
use mockall::automock;
use uuid::Uuid;

use genossi_dao::repayment_entry::RepaymentEntryEntity;
use genossi_dao::repayment_phase::RepaymentPhaseEntity;

use crate::ServiceError;

/// Aggregierter Auszahlungs-Kontext fuer EIN Mitglied in EINER Phase.
/// Felder-Reihenfolge ist frozen — Aenderungen brechen Template-Renderer (Plan 03).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepaymentContext {
    /// Summe aller `share_count_to_pay_out` ueber relevante Entries (Open + Contacted).
    pub share_count: i32,
    /// Deutsche Euro-Formatierung "X,YZ" (KEIN Tausenderpunkt, KEIN Euro-Symbol).
    /// Konvention aus Phase 10 D-04.
    pub payout_amount: String,
    /// Geschaeftsjahr der Phase.
    pub fiscal_year: i32,
}

/// Trait fuer die Aggregation des Repayment-Kontexts pro (Phase, Member).
///
/// `resolve` laedt Phase + Entries in der Transaktion (fuer Worker-Use-Case).
/// `aggregate` ist ein pure-fn-Wrapper — Caller laedt phase+entries EINMAL und
/// ruft aggregate N-mal (Plan 04 Letter-Service vermeidet so 1+N DB-Reads im Loop).
///
/// Fehler-Konventionen:
/// - `resolve`: fehlende Phase → `ServiceError::EntityNotFound(phase_id)`;
///   keine relevanten Entries → `ServiceError::EntityNotFound(member_id)`.
/// - `aggregate`: keine relevanten Entries → `ServiceError::EntityNotFound(member_id)`.
#[automock(type Transaction = genossi_dao::MockTransaction;)]
#[async_trait]
pub trait RepaymentContextResolver: Send + Sync + 'static {
    type Transaction: genossi_dao::Transaction;

    /// Async-Variante: laedt Phase + Entries selbst (Worker-Use-Case).
    async fn resolve(
        &self,
        phase_id: Uuid,
        member_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<RepaymentContext, ServiceError>;

    /// Pure-fn-Wrapper: Caller liefert Phase + Entries vorab.
    /// Vermeidet 1+N DB-Reads im Letter-Service-Loop (Plan 04).
    /// Keine async-Semantik noetig (kein DB-Round-Trip).
    fn aggregate(
        &self,
        phase: &RepaymentPhaseEntity,
        entries: &[RepaymentEntryEntity],
        member_id: Uuid,
    ) -> Result<RepaymentContext, ServiceError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repayment_context_equality() {
        let a = RepaymentContext {
            share_count: 3,
            payout_amount: "12,00".to_string(),
            fiscal_year: 2025,
        };
        let b = RepaymentContext {
            share_count: 3,
            payout_amount: "12,00".to_string(),
            fiscal_year: 2025,
        };
        assert_eq!(a, b);
    }

    #[test]
    fn test_repayment_context_clone() {
        let a = RepaymentContext {
            share_count: 5,
            payout_amount: "60,00".to_string(),
            fiscal_year: 2025,
        };
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn test_mock_repayment_context_resolver_compiles() {
        let _mock = MockRepaymentContextResolver::new();
        // Verifies automock generated MockRepaymentContextResolver successfully.
    }

    #[test]
    fn test_mock_repayment_context_resolver_has_aggregate_expect() {
        // Verifies automock generated expect_aggregate() (sync-Methode auf dem Trait).
        let mut mock = MockRepaymentContextResolver::new();
        mock.expect_aggregate().returning(|_phase, _entries, _mid| {
            Ok(RepaymentContext {
                share_count: 1,
                payout_amount: "1,00".to_string(),
                fiscal_year: 2025,
            })
        });
        // No call — only verifies that the expect_*-Setter exists at compile-time.
    }
}
