---
phase: 06-teilnehmerlisten-export-f-r-generalversammlungen
plan: 02
subsystem: service
tags:
  - export
  - service
  - permission-funnel
  - pdf
  - csv
  - xlsx

# Dependency graph
requires:
  - "06-01 (rust_xlsxwriter + csv Workspace-Deps; templates/teilnehmerliste.typ)"
provides:
  - "Trait `AttendanceExportService` mit Mock in `genossi_service::attendance_export`"
  - "AttendanceExportServiceImpl<Deps> + AttendanceExportServiceDeps in `genossi_service_impl::attendance_export`"
  - "PdfGenerator::render_attendance_list als oeffentliche Methode"
affects:
  - "06-03 (REST-Handler + DI-Wiring + E2E) — bindet die ServiceImpl-Struct in RestStateImpl::new()"
  - "06-04 (Frontend-Block) — Vertraegt sich mit der API, die Plan 03 freischaltet"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Manuelle Deps-Trait + Impl-Struct (statt gen_service_impl!) fuer Services mit Non-DAO/Service-Feldern"
    - "tempdir-Pattern fuer PDF-Render-Unit-Tests (kein Coupling an templates/-Dir)"
    - "Self-source grep gate fuer 'KEIN Audit' Invariante mit format!-konkatenierten needles (kein Selbst-Match)"

key-files:
  created:
    - "genossi_service/src/attendance_export.rs — Trait + Domain-Types + 5 Tests"
    - "genossi_service_impl/src/attendance_export.rs — Impl + Permission-Funnel + 3 Format-Writer + 13 Tests"
  modified:
    - "genossi_service/src/lib.rs — pub mod attendance_export"
    - "genossi_service_impl/src/lib.rs — pub mod attendance_export"
    - "genossi_service_impl/src/pdf_generation.rs — neue Methode render_attendance_list + build_inputs_attendance"

key-decisions:
  - "Trait::Transaction bound is `genossi_dao::Transaction` (NICHT `Clone + Debug + Send + Sync` wie in Plan-Action vorgeschlagen) — MockTransaction implementiert kein Debug; Pattern muss zu attendance.rs:49 spiegeln"
  - "render_attendance_list returnt `Result<Vec<u8>, ServiceError>` (NICHT TemplateError wie die Peer-Methoden) — der Service-Funnel kann dadurch `?`-propagieren; Konsistenz wurde fuer Boilerplate-Vermeidung geopfert"
  - "AttendanceExport bekommt manuelles Debug-Impl, das nur bytes_len druckt — sonst loggt jeder failed-Assert ein Hex-Dump der PDF/XLSX-Bytes"
  - "Audit-Gate-Test nutzt format!-Konkatenation der needle-Strings, damit das Test-File sich nicht selbst invalidiert; auskommentierte Macro-Namen + diese Erklaerung sind harmlos, weil sie ausserhalb von Code-Bloecken stehen"

patterns-established:
  - "Manueller Deps-Trait + Impl-Struct: Service mit nicht-DAO-Feldern (PdfGenerator, PathBuf) muss handschriftlich aufgebaut werden statt gen_service_impl!; Plan 03 kann diese Struktur 1:1 mit RestStateImpl wiren"
  - "render_*-Methode auf PdfGenerator pro Aggregat: Dritte render_*-Methode (nach `render` fuer Member und `render_application`), je mit eigener `build_inputs_*` Funktion. Bei Wachstum waere ein Generic-Trait die Nachfolge-Refaktor."

requirements-completed:
  - D-01
  - D-03
  - D-04
  - D-05
  - D-06
  - D-07
  - D-08
  - D-09
  - D-10
  - D-11
  - D-12
  - D-13
  - D-15
  - D-16
  - D-17
  - D-18

# Metrics
duration: 20min
completed: 2026-05-17
---

# Phase 6 Plan 02: Backend-Aggregat Summary

**AttendanceExportService Trait + Impl mit Admin+Closed-Funnel, drei Format-Writern (CSV BOM/Semikolon, XLSX rust_xlsxwriter, PDF via Typst-Template), 6-Spalten-DSGVO-Whitelist, kein Audit-Log (D-17), strukturiertes tracing::info! (D-18) — 16/20 Phase-6-Decisions in Code uebertragen.**

## Performance

- **Duration:** ca. 20 min
- **Started:** 2026-05-17T11:07:10Z
- **Completed:** 2026-05-17T11:26:40Z
- **Tasks:** 2 (Trait + Impl-mit-Tests)
- **Files modified:** 5 (2 neu + 3 erweitert)
- **Tests added:** 18 (5 in genossi_service + 13 in genossi_service_impl)
- **All workspace tests still green:** 39 / 221

## Accomplishments

- **Trait + Domain-Types** (`genossi_service/src/attendance_export.rs`):
  - `pub trait AttendanceExportService` mit `#[automock(type Context=(); type Transaction = MockTransaction;)]` — Plan 03 hat einen testbaren Mock fuer REST-Tests
  - `ExportFormat { Csv, Pdf, Xlsx }`, `ExportInclude { All, Present }` mit `Default::default() == All` (D-09)
  - `AttendanceExport { bytes, content_type, filename }` mit manuellem `Debug`-Impl, das nur Bytes-Laenge druckt
- **Impl + Permission-Funnel** (`genossi_service_impl/src/attendance_export.rs`):
  - `AttendanceExportServiceImpl<Deps>` mit hand-geschriebenem `AttendanceExportServiceDeps`-Trait (4 Felder: 2 DAOs + PermissionService + TransactionDao) PLUS 2 Non-Trait-Felder (PdfGenerator + template_base)
  - `check_admin_and_closed`-Funnel: `assembly_dao.find_by_id` → optionales `permission_service.check_permission("admin", ...)` → Status-Gate (`assembly.status == Closed`)
  - `export(...)`-Methode: Funnel → DAO-Read (`list_members_for_assembly`, search=None) → in-Memory Filter (include=Present) → present/total-Counts → Format-Dispatch
  - `render_csv` mit UTF-8 BOM + Semikolon + 6 Spalten + ja/nein
  - `render_xlsx` mit rust_xlsxwriter Workbook + bold Header + ZIP-Bytes
  - PDF-Pfad via `PdfGenerator::render_attendance_list(template_path, template_base, &assembly, &rows, present, total)`
  - `tracing::info!(target: "attendance_export", aid, format, include, rows, "exporting attendance")` (D-18)
- **PDF-Generator-Erweiterung** (`genossi_service_impl/src/pdf_generation.rs`):
  - Neue Methode `render_attendance_list` analog zu `render_application`, returnt `Result<Vec<u8>, ServiceError>` fuer einfaches `?`-Propagieren
  - `build_inputs_attendance`-Helper (free function) baut die Typst-`sys.inputs`-Dict mit `meta` (JSON: title, date DD.MM.YYYY, present, total|null) und `rows` (JSON-Array mit 6 Spalten — `member_id` wird bewusst NICHT exportiert, DSGVO)
- **Test-Coverage**:
  - 5 Tests in genossi_service: ExportInclude default, 3-Varianten-ExportFormat, 2-Varianten-ExportInclude, AttendanceExport-Bundle, MockTrait-Builder
  - 13 Tests in genossi_service_impl: 4x Permission-Funnel (Full+Closed, Admin-Pass, PermissionDenied, EntityNotFound), 2x Conflict (Preparation, Open), Include-Filter, CSV-BOM+Semikolon+ja/nein+Zeilen-Count, XLSX-ZIP-Magic, Filename-Schema (CSV+XLSX), D-10-Invariante (5 Rows → 5 in Export), PDF-Magic via tempdir, D-17 No-Audit Grep-Gate

## Task Commits

1. **Task 1: Trait + Domain-Types** — `ba1054c` (feat)
2. **Task 2: Impl + Format-Writer + render_attendance_list** — `19c0931` (feat)

## Files Created/Modified

- `genossi_service/src/attendance_export.rs` (NEU, 153 Zeilen) — Trait + 3 Domain-Types + 5 Unit-Tests
- `genossi_service/src/lib.rs` (MOD, +1) — `pub mod attendance_export`
- `genossi_service_impl/src/attendance_export.rs` (NEU, 1187 Zeilen) — Impl + Permission-Funnel + 3 Format-Writer + 13 Unit-Tests
- `genossi_service_impl/src/lib.rs` (MOD, +1) — `pub mod attendance_export`
- `genossi_service_impl/src/pdf_generation.rs` (MOD, +123) — Imports + `render_attendance_list` + `build_inputs_attendance`

## Decisions Made

- **`type Transaction: genossi_dao::Transaction` statt `Clone + Debug + Send + Sync`:** `MockTransaction` (gewaehlt vom `automock`) implementiert kein `Debug`. Plan-Action schlug die strengere Bound vor; `attendance.rs` (Phase 3 Plan 04) hat dasselbe Pattern. Wenn ich der `attendance.rs`-Konvention folge, ist der Mock konstruierbar.
- **`render_attendance_list` returnt `ServiceError`, nicht `TemplateError`:** Die zwei Peer-Methoden (`render`, `render_application`) returnen `TemplateError`. Mein Plan-Action sagt aber `ServiceError`. Plan-Action gewinnt — der Service-Funnel kann `?`-propagieren ohne `.map_err(TemplateError -> ServiceError)`-Boilerplate. Bei einer kuenftigen Refaktor koennte man einen `From<TemplateError> for ServiceError` einfuehren und alle drei Methoden vereinheitlichen.
- **Manuelles `Debug` fuer `AttendanceExport`:** Standardmaessig wuerde `#[derive(Debug)]` einen 100KB-Hex-Dump in Test-Panic-Messages drucken. Manuelle Impl druckt `bytes_len` statt der Bytes — Tests bleiben lesbar.
- **`no_audit_macros_used` Test mit format!-Konkatenation:** Der Test inspiziert das eigene Source-File. Wenn er den literal-String `audited_create!` in seinem Body referenzieren wuerde, wuerde der grep-Gate sich selbst falsch invalidieren. `format!("{}!", "audited_create")` baut das needle zur Laufzeit; im Source-File steht nur `audited_create` (ohne `!`).
- **Tests in TestTransaction/TestContext-Pattern statt MockTransaction-Reuse:** Identisch zum `attendance.rs:243`-Pattern: hand-rolled Mocks gegen einen lokalen `TestTransaction` zu schreiben ist klarer als gegen den `automock`-gehärteten `MockTransaction`, dessen `Transaction`-Typ auf `MockTransaction` hartkodiert ist.

## Deviations from Plan

### Action-Block-Abweichungen (klein, dokumentiert oben)

**1. [Rule 3 — Blocker] `type Transaction: genossi_dao::Transaction` statt `Clone + Debug + Send + Sync + 'static`**

- **Found during:** Task 1 Test-Build
- **Issue:** Plan-Action schreibt `type Transaction: Clone + Debug + Send + Sync + 'static`, aber `MockTransaction` implementiert kein `Debug`. Compile schlaegt fehl: "the trait `Debug` is not implemented for `MockTransaction`".
- **Fix:** Bound auf `type Transaction: genossi_dao::Transaction;` reduziert — identisch zum `attendance.rs:49`-Pattern, das ohne Probleme mit `automock` arbeitet.
- **Files modified:** `genossi_service/src/attendance_export.rs`
- **Commit:** ba1054c

**2. [Rule 3 — Blocker] `AttendanceExport` Debug-Impl manuell hinzugefuegt**

- **Found during:** Task 2 Test-Compilation
- **Issue:** `assert!(res.is_ok(), "got {:?}", res)` braucht `Debug` fuer `AttendanceExport`; ich hatte das im Trait-File vergessen.
- **Fix:** Manuelles `impl Debug for AttendanceExport` mit `bytes_len` statt Bytes (sonst riesige Hex-Dumps).
- **Files modified:** `genossi_service/src/attendance_export.rs`
- **Commit:** 19c0931

**3. [Rule 1 — Bug] `no_audit_macros_used`-Test self-invalidating**

- **Found during:** Task 2 Test-Run
- **Issue:** Der Test-Body referenzierte den literal-String `"audited_create!"` in panik-Messages — der grep-Gate hat den eigenen panic-Text als "found" gemeldet und der Test schlug paradoxerweise gerade dann fehl, wenn alles korrekt war.
- **Fix:** needle-Strings via `format!("{}!", "audited_create")` zur Laufzeit zusammensetzen, sodass im statischen Source nur `audited_create` (ohne `!`) erscheint.
- **Files modified:** `genossi_service_impl/src/attendance_export.rs`
- **Commit:** 19c0931

### Out-of-Scope discoveries (gelogged, nicht gefixt)

- **`genossi_dao_impl_sqlite/src/timestamp.rs` unused import warning:** Pre-existing warning, nicht aus diesem Plan.
- **`genossi_mail/src/rest_templates.rs` unused imports + dead_code:** Pre-existing warnings, nicht aus diesem Plan.
- **`genossi_bin/src/lib.rs:779` unused import `Auditable`:** Pre-existing warning, nicht aus diesem Plan.

Diese Warnings haben sich nicht aus meinen Aenderungen ergeben — bestehen unveraendert vor und nach diesem Plan.

## Issues Encountered

- **`cargo build` ohne `SQLX_OFFLINE=true` schlaegt fehl** — wie in Plan 01 dokumentiert; `genossi_dao_impl_sqlite` braucht den `.sqlx/`-Cache oder eine DB. Ich habe alle Builds mit `SQLX_OFFLINE=true cargo ...` gefahren.
- **`cargo build -p genossi_service` ohne `--features utoipa` schlaegt fehl** — `auth_types.rs` benutzt `#[derive(utoipa::ToSchema)]` ohne `cfg(feature = "utoipa")`. Das ist pre-existing in der Codebase (Phase 1+2). Ich habe alle `genossi_service`-Tests mit `--features utoipa` gefahren.
- **Pre-existing staged changes:** Beim Start waren `06-UI-SPEC.md` (deleted) und `research/SUMMARY.md` (added) gestaged sowie ein neu-untracked `.gitkeep`. Sie sind nicht aus diesem Plan; ich habe `git reset HEAD .` ausgefuehrt, um nur meine Plan-2-Aenderungen zu staged zu haben.

## User Setup Required

None — keine externen Konfigurationen noetig.

## Threat-Model Mitigations Implemented

| Threat ID | Mitigation | Test Anchor |
|-----------|------------|-------------|
| T-06-04 (Helper exfiltriert Liste) | `check_admin_and_closed` ruft `permission_service.check_permission("admin", ...)`. Helper-Context hat keine admin-Privilege → PermissionDenied → 403 in Plan 03 | `non_admin_returns_permission_denied` |
| T-06-05 (Export auf nicht-Closed GV) | Status-Gate nach Permission-Check: `assembly.status != Closed → Conflict("assembly_not_closed")` | `non_closed_returns_conflict_open` + `non_closed_returns_conflict_preparation` |
| T-06-06 (PII-Leak ueber neue Spalten) | Service ruft `list_members_for_assembly` ohne Modifikation auf → exakt die DAO-7-col Whitelist; `member_id` wird im PDF-Builder zusaetzlich gedroppt (6 sichtbare Spalten) | Doc-Comment im `build_inputs_attendance`-Helper + 6-Spalten-Header in CSV+XLSX |
| T-06-09 (Format-String-Injection via assembly.name) | `assembly.name` geht via `serde_json::to_string` an Typst → JSON-decoded als opake String → kein Template-Compile-Pfad | `pdf_export_returns_pdf_magic` (Smoke-Test) |
| T-06-07 (DoS via >10k Mitglieder) | Accepted per A2 (<500 aktive Mitglieder) — keine Mitigation | n/a |
| T-06-08 (Vorstand exfiltriert unbemerkt) | Accepted per D-17 — kein Audit, dafuer `tracing::info!` als Soft-Visibility | `tracing::info!` line im `export(...)` body |

Beide kritischen Mitigations (Permission-Funnel + Status-Gate) sind unit-getestet.

## Next Phase Readiness

- **Plan 03 (REST + DI + E2E) ist freigeschaltet:**
  - HTTP-Handler kann `RestStateImpl::attendance_export_service().export(aid, format, include, auth)` aufrufen
  - `AttendanceExportServiceDeps`-Trait mit 4 DAO/Service-Feldern + 2 Non-Trait-Feldern (PdfGenerator + template_base) ist die Vertrag-Shape fuer `RestStateImpl::new()`
  - Filename + Content-Type kommen direkt aus dem `AttendanceExport`-Bundle — Handler muss nur die `Content-Disposition: attachment; filename="..."`-Header setzen
  - Body-Type ist `application/octet-stream`-kompatibel (`Vec<u8>`)
- **MockAttendanceExportService** ist via `#[automock]` verfuegbar fuer REST-Unit-Tests
- **Keine Blocker.**

## Self-Check: PASSED

Files verifiziert:
- FOUND: `genossi_service/src/attendance_export.rs`
- FOUND: `genossi_service/src/lib.rs` (mit `pub mod attendance_export`)
- FOUND: `genossi_service_impl/src/attendance_export.rs`
- FOUND: `genossi_service_impl/src/lib.rs` (mit `pub mod attendance_export`)
- FOUND: `genossi_service_impl/src/pdf_generation.rs` (mit `fn render_attendance_list`)
- FOUND: `.planning/phases/06-teilnehmerlisten-export-f-r-generalversammlungen/06-02-SUMMARY.md`

Commits verifiziert:
- FOUND: `ba1054c` (Task 1: Trait + Domain-Types)
- FOUND: `19c0931` (Task 2: Impl + Format-Writer + render_attendance_list)

Build verifiziert:
- `SQLX_OFFLINE=true cargo build --workspace` → Exit 0 (nur pre-existing warnings)
- `SQLX_OFFLINE=true cargo test -p genossi_service --features utoipa --lib` → 39/39 grün
- `SQLX_OFFLINE=true cargo test -p genossi_service_impl --lib` → 221/221 grün (13 neue + 208 alte)

D-17 NO-AUDIT-Gate verifiziert:
- `grep -v '^[[:space:]]*//' genossi_service_impl/src/attendance_export.rs | grep -cE 'audited_(create|update|delete)!'` → 0

---
*Phase: 06-teilnehmerlisten-export-f-r-generalversammlungen*
*Completed: 2026-05-17*
