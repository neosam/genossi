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

/// Quick 260602-sgp: Format-Wahl fuer Bulk-Download. ZIP packt Einzel-PDFs;
/// PDF mergt sie zu einer Datei.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepaymentLetterDownloadFormat {
    Zip,
    Pdf,
}

/// Quick 260602-sgp: Output des Bulk-Download-Service.
///
/// `bytes` ist application/zip ODER application/pdf je nach format.
/// `document_count` zaehlt erfolgreich eingepackte Letters,
/// `skipped_count` zaehlt MemberDocuments deren Files im Storage fehlten.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepaymentLetterDownload {
    /// Output bytes — entweder ZIP-Archiv oder gemerged Bundle-PDF.
    pub bytes: Vec<u8>,
    /// MIME type — "application/zip" oder "application/pdf".
    pub content_type: &'static str,
    /// Filename fuer Content-Disposition.
    pub filename: String,
    /// Anzahl erfolgreich zusammengefasster Letters.
    pub document_count: usize,
    /// Anzahl MemberDocuments deren Files im Storage fehlten (skipped).
    pub skipped_count: usize,
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

    /// Quick 260602-sgp: Bulk-Download aller bereits persistierten RepaymentLetter-PDFs einer Phase.
    ///
    /// NICHT-Neu-Render: liest ausschliesslich MemberDocuments mit
    /// `DocumentType::RepaymentLetter`, deren description "Anschreiben Auszahlung GJ {fy}"
    /// zur Phase passt. Bei `RepaymentLetterDownloadFormat::Zip` werden Einzel-PDFs
    /// in einem ZIP-Archiv geliefert; bei `Pdf` werden sie via `lopdf` zu einer
    /// Bundle-PDF zusammengefuegt.
    ///
    /// Returns:
    /// - 0 persistierte Letters -> `ServiceError::EntityNotFound(phase_id)` (REST -> 404)
    /// - Files im Storage teilweise fehlend -> erfolgreiche werden gepackt,
    ///   `skipped_count` zaehlt fehlende (REST liefert sie als Header)
    ///
    /// Reiner Lese-Endpoint — KEIN audited_*! Macro, KEIN MemberDocument-Mutation.
    async fn download_bundle(
        &self,
        phase_id: Uuid,
        format: RepaymentLetterDownloadFormat,
        context: Authentication<Self::Context>,
    ) -> Result<RepaymentLetterDownload, ServiceError>;
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

    // ─── Quick 260602-sgp ───────────────────────────────────────────

    #[test]
    fn test_download_format_enum_variants() {
        assert_ne!(
            RepaymentLetterDownloadFormat::Zip,
            RepaymentLetterDownloadFormat::Pdf
        );
    }

    #[test]
    fn test_download_struct_fields() {
        let d = RepaymentLetterDownload {
            bytes: vec![1, 2, 3],
            content_type: "application/zip",
            filename: "x.zip".to_string(),
            document_count: 2,
            skipped_count: 1,
        };
        assert_eq!(d.bytes.len(), 3);
        assert_eq!(d.content_type, "application/zip");
        assert_eq!(d.document_count, 2);
        assert_eq!(d.skipped_count, 1);
    }

    #[test]
    fn test_mock_download_bundle_compiles() {
        // Smoke-Test fuer automock-Generation auf die neue Methode.
        let _m = MockRepaymentLetterService::new();
    }
}
