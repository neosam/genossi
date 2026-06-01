//! Phase 13 D-13-01..11: RepaymentLetterService — Bulk-PDF-Anschreiben fuer
//! Mitglieder einer RepaymentPhase. Hybrid-Bundle-Strategie:
//! N persistierte MemberDocuments + 1 transientes Bundle-PDF im Response.

use async_trait::async_trait;
use mockall::automock;
use std::fmt::Debug;
use std::sync::Arc;
use uuid::Uuid;

use crate::permission::Authentication;
use crate::ServiceError;

/// Output des Letter-Service: transientes Bundle-PDF + persistierte MemberDocument-IDs.
/// Caller (REST-Layer) gibt `bundle_bytes` als application/pdf Response zurueck.
/// `document_ids.len()` ist die Anzahl der eindeutigen Members nach Aggregation —
/// dient REST-Layer als X-Document-Count Header (Plan 05) und Frontend-Toast.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepaymentLetterBundle {
    /// Transientes gebuendeltes Druck-PDF — NICHT persistiert (D-13-01).
    pub bundle_bytes: Vec<u8>,
    /// Filename fuer Content-Disposition: e.g. "auszahlungs_anschreiben_GJ_2025.pdf".
    pub filename: String,
    /// IDs aller persistierten MemberDocuments (1 pro unique member nach Aggregation).
    /// document_ids.len() == Anzahl der Briefe (NICHT entry_ids.len() — D-13-04 Aggregation).
    pub document_ids: Vec<Uuid>,
}

/// Bulk-Brief-Service. Generiert pro Member EIN auditiertes MemberDocument-PDF
/// und ein transientes Bundle-PDF fuer den Druck-Workflow.
///
/// D-13-04: Mehrere entry_ids des gleichen Members werden zu EINEM Brief
/// mit aggregiertem share_count + payout_amount kombiniert.
///
/// D-13-09: Backend toucht RepaymentEntry NIE — weder DAO::update noch
/// audited_update! noch repayment_entry_service. Auto-Toggle muss vom
/// Vorstand via existierendem Phase-8-Batch-Endpoint getriggert werden.
#[automock(type Context = (); type Transaction = genossi_dao::MockTransaction;)]
#[async_trait]
pub trait RepaymentLetterService: Send + Sync + 'static {
    type Context: Clone + Debug + PartialEq + Eq + Send + Sync + 'static;
    type Transaction: genossi_dao::Transaction;

    /// Generiert Briefe fuer die selektierten entry_ids einer Phase.
    ///
    /// Returns: Bundle-PDF-Bytes + Filename + Liste der persistierten MemberDocument-IDs.
    async fn generate(
        &self,
        phase_id: Uuid,
        entry_ids: Arc<[Uuid]>,
        context: Authentication<Self::Context>,
    ) -> Result<RepaymentLetterBundle, ServiceError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bundle_struct_fields() {
        let b = RepaymentLetterBundle {
            bundle_bytes: vec![1, 2, 3],
            filename: "x.pdf".to_string(),
            document_ids: vec![Uuid::new_v4(), Uuid::new_v4()],
        };
        assert_eq!(b.bundle_bytes.len(), 3);
        assert_eq!(b.filename, "x.pdf");
        assert_eq!(b.document_ids.len(), 2);
    }

    #[test]
    fn test_mock_letter_service_compiles() {
        let _m = MockRepaymentLetterService::new();
    }
}
