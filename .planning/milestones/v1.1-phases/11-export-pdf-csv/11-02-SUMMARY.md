---
phase: 11-export-pdf-csv
plan: 02
subsystem: api
tags: [service-trait, domain-types, mockall, async-trait, export, pdf]

# Dependency graph
requires:
  - phase: 11-export-pdf-csv
    provides: "Plan 11.01 PdfGenerator::render_repayment_list (für 11.03 Service-Impl konsumiert)"
  - phase: 06-attendance-export
    provides: "AttendanceExportService-Trait-Pattern (1:1-Vorlage)"
provides:
  - "pub trait RepaymentExportService mit #[automock] in genossi_service::repayment_export"
  - "ExportFormat-Enum (genau 1 Variante: Pdf, D-12)"
  - "ExportInclude-Enum (Open/All/Paid, Default=Open, D-03)"
  - "RepaymentExport-Bundle-Struct (bytes/content_type/filename) mit custom Debug-Impl (Pitfall #6)"
  - "MockRepaymentExportService via mockall-automock-Generation"
affects: [11-03-service-impl, 11-04-rest, 11-05-e2e, 11-06-frontend]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Trait-Definition in genossi_service, Impl in genossi_service_impl (Mirror Phase 6 attendance_export)"
    - "Custom Debug-Impl mit bytes_len statt Raw-Bytes (Pitfall #6 = PII/Megabyte-Spam-Guard für Test-Failure-Output)"
    - "ExportInclude::default() codifiziert Domain-Workflow-Default (Banking-Vorlage = Open)"
    - "ExportFormat als 1-Varianten-Enum mit exhaustivem Match-Test als Compile-Time-Guard gegen D-12-Regression"

key-files:
  created:
    - "genossi_service/src/repayment_export.rs (149 LOC: trait + 3 Enums + bundle + custom Debug + 6 unit tests)"
  modified:
    - "genossi_service/src/lib.rs (+1 Zeile: pub mod repayment_export, alphabetisch zwischen repayment_entry und repayment_phase)"

key-decisions:
  - "TDD-RED via absichtlich falsche Defaults (ExportInclude::default()=All, ExportFormat::Csv-Stub) statt todo!()-Stub — Tests scheitern semantisch korrekt auf Compile-Ebene (non-exhaustive match) und gibt sauberen RED→GREEN-Cycle ohne Runtime-Panic-Noise"
  - "Workspace-Level cargo test statt -p genossi_service: utoipa-Feature wird transitiv via genossi_rest aktiviert; -p genossi_service ohne --features utoipa schlägt am preexisting auth_types.rs::ToSchema-Derive fehl (unrelated zu Plan 11.02)"
  - "1:1-Mirror von attendance_export.rs ohne strukturelle Abweichungen — gleiche Trait-Signatur-Form (Authentication<Self::Context>, MockTransaction, ServiceError), gleiches Debug-Impl-Pattern, gleiche Test-Anordnung. Spart Plan 11.03/11.04 jede Form von Architektur-Bias."

patterns-established:
  - "Pattern 1: Test-as-PII-Guard — explizite Assertion `!dbg.contains(\"0xDE\")` UND `!dbg.contains(\"[222\")` schützt sowohl gegen Hex-Dump als auch gegen Vec-Debug-Format-Leak. Vorlage für alle künftigen Bundle-Structs mit Vec<u8>-Feldern."
  - "Pattern 2: Domain-Workflow-Default-Codification — `Default::default()` ist semantisch geladen (Banking-Vorlage), nicht arbiträr; ein Test sichert den Default als Domain-Decision."

requirements-completed: [EXPO-03]

# Metrics
duration: ~5min
completed: 2026-06-01
---

# Phase 11 Plan 02: RepaymentExportService-Trait + Domain-Types Summary

**Service-Layer-Interface `RepaymentExportService` mit Pdf-only ExportFormat (D-12), Open-default ExportInclude (D-03) und `RepaymentExport`-Bundle (bytes/content_type/filename) — 1:1-Mirror des Phase-6-AttendanceExportService-Patterns, vollständig automock-fähig.**

## Performance

- **Duration:** ~5 min
- **Started:** 2026-06-01T05:12:00Z (ca. nach Plan-11.01-Completion)
- **Completed:** 2026-06-01T05:18:00Z
- **Tasks:** 1 (mit TDD-RED→GREEN-Cycle)
- **Files modified:** 2 (1 created, 1 modified)

## Accomplishments

- `genossi_service::repayment_export`-Modul mit `RepaymentExportService`-Trait + `#[automock]`-Annotation (MockRepaymentExportService kompiliert und ist via `expect_export()` testbar)
- `ExportFormat`-Enum mit GENAU einer Variante `Pdf` — D-12 (CSV-Streichung) auf Type-Layer codifiziert; exhaustiver Match-Test wäre der Tripwire gegen versehentliche `Csv`/`Xlsx`-Erweiterung
- `ExportInclude::default()` liefert `Open` — D-03 (Banking-Vorlage-Default "noch nicht ausbezahlt") auf Type-Layer codifiziert
- `RepaymentExport`-Bundle mit manueller `Debug`-Impl die `bytes_len` statt der Raw-Bytes druckt — Pitfall #6 aus 11-RESEARCH.md mitigiert (kein Megabyte-Hex-Dump in Test-Failure-Output)
- 6 Unit-Tests verifizieren alle Domain-Invarianten (Default=Open, single-Pdf-Variant, 3-Includes-Set, Bundle-Construction, Debug-PII-Guard mit Hex+Decimal-Check, automock-Generation)

## Task Commits

Each task was committed atomically:

1. **Task 1 RED: Failing Tests + Stub mit falschen Defaults** — `08d88ef` (test)
2. **Task 1 GREEN: Remove Csv-Variante + Flip Default zu Open** — `af56848` (feat)

**REFACTOR phase:** Skipped — Code ist bereits sauber 1:1 mit attendance_export-Pattern, keine offensichtlichen Cleanups.

**Plan metadata:** (folgender docs-Commit nach diesem SUMMARY)

## Files Created/Modified

- `genossi_service/src/repayment_export.rs` (NEW, 149 LOC) — Service-Trait `RepaymentExportService`, Domain-Types `ExportFormat`/`ExportInclude`, Bundle-Struct `RepaymentExport` + custom Debug, 6 Unit-Tests
- `genossi_service/src/lib.rs` (+1 Zeile) — Modul-Deklaration `pub mod repayment_export;` zwischen `repayment_entry` und `repayment_phase` (alphabetisch)

## Decisions Made

- **TDD-RED-Strategie für reine Type-Definitionen:** Statt `todo!()`-Stub (Plan 11.01 Pattern für Funktionen mit Body) wurden für Plan 11.02 die Type-Defaults absichtlich verfälscht (`ExportInclude::default()=All`, `ExportFormat::Csv` als Stub-Variante). Resultat: Compile-Failure auf `test_export_format_only_has_pdf_variant` (non-exhaustive match) = semantisch korrekter RED-Trigger ohne Runtime-Panic-Noise. Pattern-Anker für künftige Trait/Enum-Module ohne Funktions-Bodies.
- **cargo test Workspace-Level statt -p genossi_service:** `cargo test -p genossi_service --lib` schlägt mit `utoipa::ToSchema unresolved` in `auth_types.rs` fehl, weil das `utoipa`-Feature nicht aktiviert ist (preexisting Cargo.toml: `utoipa = { workspace = true, optional = true }`). Workspace-Level-Test aktiviert das Feature transitiv via `genossi_rest`-dep-graph. Plan-Verify-Snippet `cargo test -p genossi_service repayment_export` ist also nur unter aktivem Feature funktional; Workspace-Test ist der korrekte Verify-Pfad.
- **1:1-Mirror von attendance_export.rs:** Strukturelle Abweichungen wurden NICHT eingeführt — gleiche Trait-Signatur, gleiches Debug-Impl-Pattern, gleiche Test-Anordnung. Plan 11.03 (Service-Impl), Plan 11.04 (REST) und Plan 11.05 (E2E) können dieselbe Architektur-Erwartung aus dem Phase-6-Pattern übernehmen.

## Deviations from Plan

None - plan executed exactly as written. Alle Acceptance Criteria automatisch verifiziert (grep + cargo build + cargo test grün).

## Issues Encountered

- **utoipa-Feature im genossi_service-Crate:** `cargo test -p genossi_service repayment_export` (wie im Plan-Verify-Snippet) schlägt mit `utoipa::ToSchema unresolved` fehl. Root-Cause: preexisting `auth_types.rs`-File hat `utoipa::ToSchema`-Derive ohne `#[cfg(feature = "utoipa")]`-Gate. Resolution: Workspace-Level-Test (`cargo test --workspace --lib repayment_export`) aktiviert das Feature transitiv. Out-of-Scope für Plan 11.02 (preexisting structural issue im Crate, nicht durch diesen Plan eingeführt). Logged für Future-Cleanup: optional `#[cfg(feature = "utoipa")]`-Gating der Derives in `genossi_service/src/auth_types.rs` würde `-p`-Tests ohne Feature-Flag ermöglichen.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- **Plan 11.03 (Service-Impl) ist unblocked:** Trait + Types existieren; `impl RepaymentExportService for RepaymentExportServiceImpl` kann direkt geschrieben werden.
- **Plan 11.04 (REST) ist unblocked:** Bundle-Struct `RepaymentExport { bytes, content_type, filename }` ist die fertige Service→REST-Schnittstelle; REST-Handler reicht das Bundle 1:1 als HTTP-Response durch.
- **Plan 11.05 (E2E) erbt automock-Coverage:** Mock-Tests können via `MockRepaymentExportService::new().expect_export()` ohne echte DAO-Setup beliebige Service-Antworten injizieren.
- **D-12 Compile-Time-Guard etabliert:** Versuche, später `ExportFormat::Csv` zu reaktivieren, brechen `test_export_format_only_has_pdf_variant` durch non-exhaustive match — keine Doku-Disziplin-Abhängigkeit, sondern Compiler-Enforcement.

## Self-Check: PASSED

- `genossi_service/src/repayment_export.rs`: FOUND
- `genossi_service/src/lib.rs` mit `pub mod repayment_export`: FOUND (Zeile 16)
- Commit `08d88ef` (test-RED): FOUND in git log
- Commit `af56848` (feat-GREEN): FOUND in git log
- 6 Unit-Tests grün auf Workspace-Level: VERIFIED (output above)
- Acceptance Criteria (grep counts): VERIFIED (Csv == 0, Pdf == 2, default() == 1, Open == 3, pub mod == 1, trait == 1)

---
*Phase: 11-export-pdf-csv*
*Completed: 2026-06-01*
