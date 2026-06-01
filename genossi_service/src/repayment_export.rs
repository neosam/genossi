//! Phase 11 (EXPO-01..03, EXPO-05): RepaymentExportService trait + domain types.
//!
//! Read-only export of Auszahlungsliste (PDF) for RepaymentPhase.
//! Vorbild: genossi_service/src/attendance_export.rs (Phase 6).
//! Anpassungen:
//!   - D-12: NUR Pdf (kein Csv/Xlsx)
//!   - D-03: Default-Include = Open (Banking-Vorlage-Use-Case)
//!   - D-10: Export erlaubt fuer Open ODER Closed (Impl in Plan 11.03)
//!
use async_trait::async_trait;
use mockall::automock;
use std::fmt::Debug;
use uuid::Uuid;

use crate::permission::Authentication;
use crate::ServiceError;

/// Output formats. D-12: NUR Pdf in Phase 11. Re-Add von Csv ist additiv
/// (neue Variante hier + Match-Arm im Service-Impl + Format-Whitelist im REST).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportFormat {
    Pdf,
}

/// Include filter for the export.
/// D-01: Open  = RepaymentEntryStatus in {Open, Contacted}
/// D-02: All   = Open + Contacted + PaidOut
/// D-02: Paid  = nur PaidOut
/// Soft-deleted entries/members werden in JEDEM Filter ausgeschlossen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportInclude {
    Open,
    All,
    Paid,
}

impl Default for ExportInclude {
    /// D-03: Default ist Open fuer Banking-Workflow ("noch nicht ausbezahlt").
    fn default() -> Self {
        ExportInclude::Open
    }
}

/// Export bundle returned by `RepaymentExportService::export`.
/// Manuelles Debug-Impl: druckt `bytes_len` statt der Bytes, sonst Megabyte-Spam
/// bei Test-Failures (Pitfall #6 aus 11-RESEARCH.md).
pub struct RepaymentExport {
    pub bytes: Vec<u8>,
    pub content_type: &'static str,
    pub filename: String,
}

impl std::fmt::Debug for RepaymentExport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RepaymentExport")
            .field("bytes_len", &self.bytes.len())
            .field("content_type", &self.content_type)
            .field("filename", &self.filename)
            .finish()
    }
}

/// Phase 11 Read-only Export-Service fuer Auszahlungslisten.
///
/// - Vorstand-only via PermissionService::check_permission("admin", ...) — D-11
/// - Null `audited_*!`-Calls im Impl (Plan 11.03 Grep-Gate-Test) — EXPO-05
/// - Permission-Funnel-Order: load_by_id (404) -> admin-check (403) -> status-check (409) — D-10
#[automock(type Context=(); type Transaction = genossi_dao::MockTransaction;)]
#[async_trait]
pub trait RepaymentExportService {
    type Context: Clone + Debug + Send + Sync + 'static;
    type Transaction: genossi_dao::Transaction;

    async fn export(
        &self,
        phase_id: Uuid,
        format: ExportFormat,
        include: ExportInclude,
        context: Authentication<Self::Context>,
    ) -> Result<RepaymentExport, ServiceError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_include_default_is_open() {
        // D-03: Default ist Open fuer Banking-Workflow.
        assert_eq!(ExportInclude::default(), ExportInclude::Open);
    }

    #[test]
    fn test_export_format_only_has_pdf_variant() {
        // D-12: CSV gestrichen, NUR Pdf in Phase 11.
        // Compile-time-Guarantee: dieses Match-Statement bricht wenn neue Variante hinzukommt.
        let f = ExportFormat::Pdf;
        match f {
            ExportFormat::Pdf => {}
        }
    }

    #[test]
    fn test_export_include_has_three_variants() {
        // D-01/D-02: Open, All, Paid.
        let variants = [
            ExportInclude::Open,
            ExportInclude::All,
            ExportInclude::Paid,
        ];
        assert_eq!(variants.len(), 3);
    }

    #[test]
    fn test_repayment_export_bundle_construction() {
        let e = RepaymentExport {
            bytes: vec![0xDE, 0xAD, 0xBE, 0xEF],
            content_type: "application/pdf",
            filename: "auszahlung-2026-open.pdf".to_string(),
        };
        assert_eq!(e.bytes.len(), 4);
        assert_eq!(e.content_type, "application/pdf");
        assert!(e.filename.ends_with(".pdf"));
    }

    #[test]
    fn test_repayment_export_debug_hides_bytes() {
        // Pitfall #6: Debug-Impl darf KEINE Bytes-Hex-Dumps drucken.
        let e = RepaymentExport {
            bytes: vec![0xDE, 0xAD],
            content_type: "application/pdf",
            filename: "x.pdf".to_string(),
        };
        let dbg = format!("{:?}", e);
        assert!(dbg.contains("bytes_len"), "Debug should show bytes_len");
        assert!(!dbg.contains("0xDE"), "Debug must not leak raw bytes");
        assert!(
            !dbg.contains("[222"),
            "Debug must not leak Vec-format bytes (decimal)"
        );
    }

    #[test]
    fn test_mock_repayment_export_service_compiles() {
        // mockall #[automock] muss MockRepaymentExportService generieren.
        // Wenn dieser Test compiliert, ist die automock-Annotation korrekt.
        let _mock = MockRepaymentExportService::new();
    }
}
