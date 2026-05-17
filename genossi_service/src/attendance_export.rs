//! Service-layer trait + domain types for the Attendance-Export aggregate.
//!
//! Phase 6 Plan 02 wires the `AttendanceExportService` trait, the
//! `ExportFormat` / `ExportInclude` enums, and the `AttendanceExport` domain
//! struct. Plan 02 also provides `AttendanceExportServiceImpl` in
//! `genossi_service_impl`; Plan 03 binds a new HTTP handler to the
//! `service.export(...)` entry point.
//!
//! **Permission funnel** (D-11, D-13): the implementation enforces admin-only
//! access AND `assembly.status == Closed`. There is intentionally NO
//! helper-branch — see `genossi_service_impl/src/attendance_export.rs`.
//!
//! **No audit** (D-17): exports are read-only and explicitly NOT logged in
//! the audit hash chain. `tracing::info!` (D-18) provides operational log
//! visibility for the same call.

use async_trait::async_trait;
use mockall::automock;
use std::fmt::Debug;
use uuid::Uuid;

use crate::permission::Authentication;
use crate::ServiceError;

/// Format des Exports — Pfad-Suffix des REST-Endpoints (D-01, D-14).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportFormat {
    Csv,
    Pdf,
    Xlsx,
}

/// Auswahl der Mitglieder im Export (D-09).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportInclude {
    /// Alle Snapshot-Mitglieder mit Anwesenheits-Spalte.
    All,
    /// Nur anwesende Mitglieder.
    Present,
}

impl Default for ExportInclude {
    /// D-09 Recommendation aus CONTEXT.md / RESEARCH §"Open Question 3":
    /// Default ist `All` — der Verband bekommt die vollstaendige Liste mit
    /// Anwesenheits-Spalte, nicht nur die anwesenden Mitglieder.
    fn default() -> Self {
        ExportInclude::All
    }
}

/// Resultat des Exports — Bytes + Content-Type + Filename in einem Bundle
/// (D-15, D-16). Der REST-Handler in Plan 03 setzt aus diesem Bundle die
/// `Content-Type`- und `Content-Disposition`-Header der Response.
///
/// `Debug` ist abgeleitet, druckt aber nur die Bytes-Laenge statt der Bytes
/// selbst — sonst wuerde `assert!(res.is_ok(), "{:?}", res)` in Unit-Tests
/// gigantische Hex-Dumps loggen.
pub struct AttendanceExport {
    pub bytes: Vec<u8>,
    pub content_type: &'static str,
    pub filename: String,
}

impl std::fmt::Debug for AttendanceExport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AttendanceExport")
            .field("bytes_len", &self.bytes.len())
            .field("content_type", &self.content_type)
            .field("filename", &self.filename)
            .finish()
    }
}

#[automock(type Context=(); type Transaction = genossi_dao::MockTransaction;)]
#[async_trait]
pub trait AttendanceExportService {
    type Context: Clone + Debug + Send + Sync + 'static;
    type Transaction: genossi_dao::Transaction;

    /// Exportiert die Teilnehmerliste einer GV.
    ///
    /// Permission-Funnel (Impl-seitig): admin-only via `PermissionService`
    /// (D-13) PLUS `assembly.status == Closed` (D-11). Non-Closed-Status
    /// liefert `ServiceError::Conflict("assembly_not_closed")`.
    ///
    /// Datenquelle: `AttendanceDao::list_members_for_assembly` mit `search=None`
    /// (D-05, D-06, D-07 — keine neue DAO-Methode, reuse der 7-col Whitelist).
    /// `include=Present` filtert in-memory auf `is_present == true` (D-09).
    async fn export(
        &self,
        assembly_id: Uuid,
        format: ExportFormat,
        include: ExportInclude,
        context: Authentication<Self::Context>,
    ) -> Result<AttendanceExport, ServiceError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_include_default_is_all() {
        // D-09: Default ist All — sonst broke der Verband-Workflow (volle Liste
        // mit Anwesenheits-Spalte ist die geforderte Default-Sicht).
        assert_eq!(ExportInclude::default(), ExportInclude::All);
    }

    #[test]
    fn export_format_has_three_variants() {
        // Compile-time + match-exhaustiveness check: ExportFormat hat genau
        // drei Varianten Csv/Pdf/Xlsx. Ein zukuenftiges Hinzufuegen einer
        // vierten Variante bricht diesen Test (gewollt).
        let formats = [ExportFormat::Csv, ExportFormat::Pdf, ExportFormat::Xlsx];
        for f in &formats {
            match f {
                ExportFormat::Csv => {}
                ExportFormat::Pdf => {}
                ExportFormat::Xlsx => {}
            }
        }
        assert_eq!(formats.len(), 3);
    }

    #[test]
    fn export_include_has_two_variants() {
        let includes = [ExportInclude::All, ExportInclude::Present];
        for i in &includes {
            match i {
                ExportInclude::All => {}
                ExportInclude::Present => {}
            }
        }
        assert_eq!(includes.len(), 2);
    }

    #[test]
    fn attendance_export_struct_has_three_fields() {
        // Konstruktion ist der Compile-time-Vertrag fuer das 3-Feld-Bundle.
        let bundle = AttendanceExport {
            bytes: vec![1, 2, 3],
            content_type: "text/csv; charset=utf-8",
            filename: "gv-2026-05-15-teilnehmer.csv".to_string(),
        };
        assert_eq!(bundle.bytes.len(), 3);
        assert_eq!(bundle.content_type, "text/csv; charset=utf-8");
        assert_eq!(bundle.filename, "gv-2026-05-15-teilnehmer.csv");
    }

    #[test]
    fn mock_trait_can_be_constructed() {
        // #[automock]-Generator muss den `MockAttendanceExportService` produzieren.
        // Wenn das Attribut falsch geschrieben ist, schlaegt dieser Test nicht
        // zur Laufzeit fehl, sondern bricht die Kompilierung.
        let _mock = MockAttendanceExportService::new();
    }
}
