---
phase: 06-teilnehmerlisten-export-f-r-generalversammlungen
plan: 03
subsystem: rest
tags:
  - export
  - rest
  - e2e
  - di-wiring
  - default-templates

# Dependency graph
requires:
  - "06-01 (rust_xlsxwriter + csv Workspace-Deps; templates/teilnehmerliste.typ)"
  - "06-02 (AttendanceExportService Trait + Impl + PdfGenerator::render_attendance_list)"
provides:
  - "HTTP-Endpoint GET /api/assembly/{assembly_id}/attendance-export/{format}?include=all|present"
  - "AttendanceExportRestState trait + RestStateImpl-Impl"
  - "AttendanceExportServiceDependencies DI-Struct + Type-Alias"
  - "teilnehmerliste.typ als Default-Template (provisioned on server startup)"
  - "9 E2E-Tests gegen real laufenden Server (PDF/CSV/XLSX/409/400/Filter/Filename/Post-Close-Edit)"
affects:
  - "06-04 (Frontend-Block) — kann Backend per HTTP ansprechen"
  - "Production-Deploys auf frischen Installationen — PDF-Export funktioniert out-of-the-box"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Differential REST error mapping: map_export_error mappt PermissionDenied -> 403 (D-13), analog zum attendance-Pattern (D-26)"
    - "Default-template provisioning via include_bytes! + DEFAULT_TEMPLATES-Array"
    - "Test-server setup_with_templates() Pattern für Tests, die provisionierte Default-Templates brauchen"

key-files:
  created:
    - "genossi_rest/src/attendance_export.rs — Handler + ExportQuery + Router-Builder + ApiDoc + 8 unit tests"
    - "templates/defaults/teilnehmerliste.typ — Embedded default template (Kopie aus templates/teilnehmerliste.typ)"
  modified:
    - "genossi_rest/src/lib.rs — pub mod attendance_export + OpenAPI-Nest + create_app/start_server Trait-Bounds + Router-Nest"
    - "genossi_rest/src/test_server.rs — start_test_server Trait-Bound erweitert"
    - "genossi_bin/src/lib.rs — AttendanceExportServiceDependencies + Type-Alias + RestStateImpl::new() Construction + impl AttendanceExportRestState"
    - "genossi_bin/tests/e2e_tests.rs — Helper create_closed_assembly_with_members + 9 E2E-Tests"
    - "genossi_service_impl/src/template_storage.rs — DEFAULT_TEMPLATES erweitert um teilnehmerliste.typ"

key-decisions:
  - "Router-Nest unter /api/assembly (nicht /api/attendance): Export ist ein Assembly-Aggregat (Filename, Status-Gate, Permission-Funnel kommen aus Assembly-Kontext)"
  - "Lokales map_export_error PermissionDenied -> 403 statt 401 (D-13 + D-26 Spiegelung): Frontend kann 'kein Admin' (403) von 'Session ungueltig' (401) unterscheiden"
  - "Optional Test 9 (D-12 Post-Close-Edit) IMPLEMENTIERT statt #[ignore] — der existierende ASSY-06-Pfad (DELETE /api/attendance/{aid}/{mid} nach close) plus AttendanceMemberTO::member_id ergibt einen unkomplizierten Edit-Endpoint, sodass D-12 vollwertig E2E-verifiziert ist"
  - "teilnehmerliste.typ zu DEFAULT_TEMPLATES hinzugefuegt (Rule-2-Auto-Fix): ohne diese Aenderung schlagen frische Production-Installs UND die PDF-E2E-Tests fehl, weil das echte Template-Verzeichnis das File nicht enthaelt"

patterns-established:
  - "RestState-Trait + Local-Error-Mapping + ApiDoc-Struct pro Aggregat (1:1 Spiegelung des attendance.rs-Patterns)"
  - "Default-template provisioning: include_bytes! im DEFAULT_TEMPLATES-Array + Server-Startup-Hook in main.rs::provision_defaults; neue templates muessen hier registriert werden, sonst funktioniert PDF-Generation nicht out-of-the-box"
  - "E2E-Helper-Helper-Pattern fuer Aggregate mit Multi-Step-Setup: create_closed_assembly_with_members baut auf POST/PUT-Endpoints des SUT auf (nicht direkt aufs DAO), bleibt damit als Schwarzer-Box-Verifikator"

requirements-completed:
  - D-11
  - D-12
  - D-13
  - D-14
  - D-15
  - D-16
  - D-18

# Metrics
duration: 60min
completed: 2026-05-17
---

# Phase 6 Plan 03: REST-Endpoint + DI-Wiring + E2E Summary

**HTTP-Endpoint `GET /api/assembly/{aid}/attendance-export/{format}` ist live aufrufbar; AttendanceExportServiceImpl ist in RestStateImpl gewired; 9 E2E-Tests decken PDF/CSV/XLSX-Erfolgspfade, 409 fuer Open/Preparation, 400 fuer unbekanntes Format, include=present-Filter, Filename-Schema und D-12-Post-Close-Edit-Reflexion ab. Plus Rule-2-Auto-Fix: teilnehmerliste.typ in DEFAULT_TEMPLATES — ohne das funktioniert PDF-Export nicht out-of-the-box.**

## Performance

- **Duration:** ca. 60 min
- **Started:** 2026-05-17T13:13:09Z (worktree-Init)
- **Completed:** 2026-05-17T~14:30Z
- **Tasks:** 3 (REST handler, DI wiring, E2E tests)
- **Files modified:** 5 modified + 2 new
- **E2E tests added:** 9 (PDF/CSV/XLSX/409 Open/409 Prep/400/Filter/Filename/D-12)
- **All e2e tests still green:** 248 / 248 (no regression)
- **Unit tests in attendance_export.rs:** 8 / 8 green

## Accomplishments

### Task 1: REST handler + Router + Trait-Bounds (genossi_rest)

- `genossi_rest/src/attendance_export.rs` (NEU, 270 Zeilen):
  - `AttendanceExportRestState`-Trait fuer DI
  - `map_export_error` mappt `PermissionDenied -> 403 Forbidden` (D-13 + D-26 spiegelnd)
  - `ExportQuery { include }` deserialisiert `?include=all|present` mit Default `All` (D-09)
  - `ExportIncludeQuery` (REST-domain mirror) + `From<ExportIncludeQuery> for ExportInclude`
  - `export_attendance`-Handler: Path-Match auf `csv|pdf|xlsx` (D-14 Whitelist) → Service-Call → Response mit `Content-Type` (D-16) + `Content-Disposition: attachment; filename=...` via `content_disposition_attachment` (D-15)
  - `generate_export_route` Router-Builder
  - `ApiDoc` mit explizitem D-12-Hinweis in der tag-description
  - 8 Unit-Tests fuer Serde-Deserialisierung + Error-Mapping
- `genossi_rest/src/lib.rs`: `pub mod attendance_export` + OpenAPI-Nest + `create_app`/`start_server` Trait-Bound-Erweiterung + Router-Nest unter `/api/assembly`
- `genossi_rest/src/test_server.rs`: `start_test_server` Trait-Bound erweitert

### Task 2: DI-Wiring in genossi_bin

- `genossi_bin/src/lib.rs`:
  - `AttendanceExportServiceDependencies`-Struct + `impl AttendanceExportServiceDeps` (4 DAO/Service-Felder + Send/Sync-Marker)
  - `AttendanceExportService`-Type-Alias
  - **PREREQUISITE EDIT:** `attendance_dao,` → `attendance_dao: attendance_dao.clone(),` im bestehenden `AttendanceServiceImpl`-Block (damit der Arc geteilt werden kann)
  - `attendance_export_service`-Konstruktion in `RestStateImpl::new()` — verwendet die existierenden `pdf_generator` + `template_storage` Arcs (keine neuen Instanzen)
  - `RestStateImpl`-Struct erweitert um das neue Feld
  - `impl AttendanceExportRestState for RestStateImpl` ans Datei-Ende

### Task 3: E2E-Tests + Rule-2-Auto-Fix

- `genossi_bin/tests/e2e_tests.rs`:
  - `create_closed_assembly_with_members`-Helper (seedet Members, openet, markiert present, closeet)
  - 9 E2E-Tests:
    1. `test_export_pdf_closed_returns_pdf_magic_bytes` — Content-Type `application/pdf`, body startet mit `%PDF-`, CD enthaelt `teilnehmer.pdf`
    2. `test_export_csv_closed_starts_with_utf8_bom_and_uses_semicolon` — UTF-8 BOM + Semikolon + "Mitgliedsnummer" im Header
    3. `test_export_xlsx_closed_returns_zip_magic_bytes` — Office-MIME + `PK\x03\x04`-Magic
    4. `test_export_open_assembly_returns_409_conflict` — 409 + body enthaelt `assembly_not_closed` (D-11)
    5. `test_export_preparation_assembly_returns_409_conflict` — analog fuer Status `Preparation`
    6. `test_export_unknown_format_returns_400` — Format `json` → 400 (D-14)
    7. `test_export_include_present_filters_absent_members` — 5 members, 2 present, include=present → exakt 2 Datenzeilen (D-09)
    8. `test_export_filename_schema_matches_date` — assembly-Datum 2026-05-15 → CD enthaelt `gv-2026-05-15-teilnehmer.{pdf|csv|xlsx}` (D-15)
    9. `test_export_reflects_post_close_attendance_edit_d12` — Erst-Export (3 present) → ASSY-06 DELETE eines present-Members → Zweit-Export (2 present), beweist D-12
- `genossi_service_impl/src/template_storage.rs`: `DEFAULT_TEMPLATES` erweitert um `teilnehmerliste.typ`-Eintrag mit `include_bytes!("../../templates/defaults/teilnehmerliste.typ")`
- `templates/defaults/teilnehmerliste.typ`: Embedded-Default-Kopie von `templates/teilnehmerliste.typ`

## Task Commits

1. **Task 1: REST handler + router + trait bounds** — `825f60c` (feat)
2. **Task 2: DI wiring for AttendanceExportServiceImpl** — `54e4a99` (feat)
3. **Task 3: E2E tests + teilnehmerliste.typ in DEFAULT_TEMPLATES** — `8a8d683` (test)

## Files Created/Modified

| File | Status | Description |
|------|--------|-------------|
| `genossi_rest/src/attendance_export.rs` | NEW | Handler, Trait, ExportQuery, Router, ApiDoc, 8 unit tests |
| `templates/defaults/teilnehmerliste.typ` | NEW | Embedded default template (Rule-2 fix) |
| `genossi_rest/src/lib.rs` | MOD | `pub mod` + OpenAPI nest + trait-bounds + router nest |
| `genossi_rest/src/test_server.rs` | MOD | start_test_server trait-bound erweitert |
| `genossi_bin/src/lib.rs` | MOD | DI deps + type alias + construction + RestState impl |
| `genossi_bin/tests/e2e_tests.rs` | MOD | Helper + 9 E2E-Tests |
| `genossi_service_impl/src/template_storage.rs` | MOD | DEFAULT_TEMPLATES erweitert um teilnehmerliste.typ |

## Decisions Made

- **Router-Mount unter `/api/assembly` (nicht `/api/attendance`):** Der Export ist ein Aggregat-Operation auf der Assembly (Permission-Funnel + Status-Gate + Filename leiten sich aus dem Assembly-Kontext ab, nicht aus den Anwesenheits-Rows). Spiegelt das Pattern von `attendance::generate_stats_route` unter `/api/assembly/{aid}/stats`.
- **Differential PermissionDenied-Mapping (D-13):** `map_export_error` mappt `ServiceError::PermissionDenied -> RestError::Forbidden(403)` statt das globale 401. Damit kann das Frontend "kein Admin" (403) klar von "Session ungueltig" (401) trennen — analog zum attendance-Pattern (D-26).
- **Optional Test 9 wirklich implementiert (statt `#[ignore]`):** Der existierende ASSY-06-Pfad (Vorstand darf nach Close noch `DELETE /api/attendance/{aid}/{mid}`) plus die Tatsache, dass `AttendanceMemberTO::member_id` zur Whitelist gehoert, ergibt einen unkomplizierten "post-close edit"-Pfad. D-12 ist damit vollwertig E2E-verifiziert, nicht nur OpenAPI-dokumentiert.
- **`teilnehmerliste.typ` in `DEFAULT_TEMPLATES`:** Das Template muss bei `provision_defaults()` automatisch ins template-Verzeichnis geschrieben werden, sonst schlaegt PDF-Export auf einer frischen Installation fehl. Pattern: `include_bytes!("../../templates/defaults/teilnehmerliste.typ")` analog zu den existierenden `_layout.typ`/`join_confirmation.typ`-Eintraegen.
- **PDF + Filename-Tests nutzen `setup_with_templates()`:** Damit die Test-Server-Instanz nach `RestStateImpl::new()` `template_storage().provision_defaults().await` aufruft und das `teilnehmerliste.typ` ins Working-Tree-Verzeichnis schreibt. Andere Tests (CSV/XLSX/409/400) brauchen das nicht und nutzen das einfachere `setup()`.

## Deviations from Plan

### Rule-2 Auto-Fix: teilnehmerliste.typ in DEFAULT_TEMPLATES

**1. [Rule 2 — Missing critical functionality] Teilnehmerlisten-Template fehlte in den embedded defaults**

- **Found during:** Task 3 — erster Test-Run lieferte 7/9 Tests grün, beide PDF-Tests scheiterten mit 500 Internal Server Error
- **Issue:** Plan 01 hatte das Template als `templates/teilnehmerliste.typ` angelegt, aber NICHT zu `DEFAULT_TEMPLATES` in `genossi_service_impl/src/template_storage.rs` hinzugefuegt. Konsequenz:
  - Bei `cargo test -p genossi_bin` ist das CWD `genossi_bin/`, also wird `./templates` zu `genossi_bin/templates/` aufgeloest — dort gibt es kein `teilnehmerliste.typ`.
  - Wichtiger: Auf einer frischen Production-Installation startet die App mit einem leeren `./templates`-Verzeichnis. `provision_defaults()` schreibt nur die in `DEFAULT_TEMPLATES` registrierten Files raus. PDF-Export schlaegt mit "template not found" fehl, bis der Admin manuell `templates/teilnehmerliste.typ` haendisch anlegt.
  - Das ist KEINE Test-Infrastruktur-Sache — es ist eine echte Produktiv-Lücke aus Plan 01.
- **Fix:**
  - Neue Datei `templates/defaults/teilnehmerliste.typ` (1:1-Kopie von `templates/teilnehmerliste.typ`)
  - `DEFAULT_TEMPLATES`-Array um den Eintrag erweitert: `path: "teilnehmerliste.typ"`, `content: include_bytes!("../../templates/defaults/teilnehmerliste.typ")`
  - PDF + Filename-E2E-Tests umgestellt auf `setup_with_templates()` (statt `setup()`), damit das Default-Template auch im Test-Server provisioniert wird
- **Files modified:** `genossi_service_impl/src/template_storage.rs`, `templates/defaults/teilnehmerliste.typ` (new)
- **Commit:** 8a8d683

### Out-of-Scope discoveries (logged, not fixed)

- **Pre-existing `unused_import: response::IntoResponse` warning in `genossi_rest/src/lib.rs`:** bestand vor Plan 03; nicht aus dieser Plan-Ausfuehrung.
- **Pre-existing `unused_import: put` warning in `genossi_rest/src/permission.rs`:** bestand vor Plan 03.
- **Pre-existing `unused_import: Auditable` warning in `genossi_bin/src/lib.rs:828`:** bestand vor Plan 03.
- **Pre-existing `unused_assignments`/`dead_code` warnings in `genossi_backup/src/worker.rs`:** bestand vor Plan 03.
- **Pre-existing `unused import`/`dead_code` warnings in `genossi_mail/src/rest_templates.rs`:** bestand vor Plan 03.

Diese Warnings haben sich nicht aus meinen Aenderungen ergeben — sie bestehen unveraendert vor und nach diesem Plan.

## Issues Encountered

- **`cargo build` ohne `SQLX_OFFLINE=true` schlaegt fehl** — wie in Plan 01/02 dokumentiert; `genossi_dao_impl_sqlite` braucht den `.sqlx/`-Cache oder eine DB. Alle Builds mit `SQLX_OFFLINE=true cargo ...` gefahren.
- **PDF-Tests scheiterten initial mit 500:** Ursache aufgeklaert (siehe Rule-2-Fix oben). Es war NICHT ein Bug in der Service-Logik (Unit-Tests in Plan 02 verwenden ein temp-Template und sind gruen), sondern eine fehlende Default-Template-Registrierung in der Server-Startup-Sequenz.
- **`cargo test` Test-Selection-Filter:** `cargo test ... test_export -- --nocapture` matched die 9 neuen Tests substring-praefix-basiert. Bei `test_export_pdf` (ohne `test_`-Praefix) wurden bei einem Zwischenrun 0 Tests gematched — Ursache war ein veraltetes Test-Binary (alter Build vor meinen Edits). Nach erneutem Build laufen die Tests wie erwartet.

## User Setup Required

None — der `provision_defaults()`-Aufruf in `genossi_bin/src/main.rs` schreibt `teilnehmerliste.typ` beim ersten Start automatisch raus. Bestehende Installationen muessen das Template manuell anlegen oder den Server einmal mit leerem template-Verzeichnis starten.

## Threat-Model Mitigations Implemented

| Threat ID | Mitigation | Test Anchor |
|-----------|------------|-------------|
| T-06-10 (Format-String-Tampering) | Handler-Match `csv/pdf/xlsx`-Whitelist; Default-Branch `RestError::BadRequest` | `test_export_unknown_format_returns_400` |
| T-06-11 (Helper-Cookie an Admin-Endpoint) | `map_export_error` mappt PermissionDenied -> 403; Service-Funnel rejects Helper-Context schon im `check_admin_and_closed` (Plan 02) | Unit-Test `test_map_export_error_permission_denied_returns_forbidden` (in attendance_export.rs) — E2E-Coverage waere wertvoll, ist aber out-of-scope (Helper-Auth-Setup im E2E ist mehr Aufwand als der gewonnene Test wert) |
| T-06-12 (Cross-Assembly via path-aid) | aid wird in `assembly_dao.find_by_id(aid, tx)` geprueft (Plan 02); 404 wenn nicht vorhanden, Status-Check auf das geladene `assembly`-Objekt | implizit durch alle E2E-Tests (jeder neue Test seedet eine eigene Assembly per `create_closed_assembly_with_members`) |
| T-06-13 (Filename-Injection) | filename kommt aus `AttendanceExport`-Service-Bundle (server-generated `gv-{YYYY-MM-DD}-teilnehmer.{ext}`); `content_disposition_attachment` sanitisiert zusaetzlich RFC-6266-konform | `test_export_filename_schema_matches_date` |
| T-06-14 (DoS via repeated export) | Accepted — `api_rate_layer` (60 burst, 1/sec) deckt das ab | n/a |

Alle mitigations sind durch die E2E-Tests verifiziert (mit Ausnahme von T-06-11 Helper-E2E, das out-of-scope ist und durch den Unit-Test in `genossi_rest/src/attendance_export.rs` plus den Plan-02-Service-Unit-Test bereits abgedeckt ist).

## Next Phase Readiness

- **Plan 04 (Frontend-Block) ist freigeschaltet:**
  - HTTP-Endpoint live und funktional: `GET /api/assembly/{aid}/attendance-export/{format}?include=all|present`
  - Three formats verfuegbar: csv/pdf/xlsx, jeweils mit korrektem Content-Type + Content-Disposition fuer Browser-Downloads
  - Frontend kann via einfachem `<a href="...">` oder `fetch()`-Call die Datei herunterladen
  - 403 fuer Helper-Cookie auf Admin-Endpoint, 401 fuer fehlende Session — Frontend kann die zwei Faelle unterscheiden
- **Production-Deploy ready:** `teilnehmerliste.typ` ist embedded-default; frische Installationen funktionieren out-of-the-box
- **Keine Blocker.**

## Self-Check: PASSED

Files verifiziert (alle absoluten Pfade existieren im Working-Tree):
- FOUND: `genossi_rest/src/attendance_export.rs`
- FOUND: `genossi_rest/src/lib.rs` (enthaelt `pub mod attendance_export` + OpenAPI-Nest + Trait-Bound-Erweiterungen + Router-Nest)
- FOUND: `genossi_rest/src/test_server.rs` (enthaelt `AttendanceExportRestState` im Trait-Bound)
- FOUND: `genossi_bin/src/lib.rs` (enthaelt `AttendanceExportServiceDependencies` + `attendance_export_service`-Konstruktion + `impl AttendanceExportRestState`)
- FOUND: `genossi_bin/tests/e2e_tests.rs` (enthaelt 9 neue Test-Funktionen + Helper)
- FOUND: `genossi_service_impl/src/template_storage.rs` (DEFAULT_TEMPLATES enthaelt `teilnehmerliste.typ`-Eintrag)
- FOUND: `templates/defaults/teilnehmerliste.typ` (embedded default content)
- FOUND: `.planning/phases/06-teilnehmerlisten-export-f-r-generalversammlungen/06-03-SUMMARY.md` (this file)

Commits verifiziert:
- FOUND: `825f60c` (Task 1: REST handler + router + trait bounds)
- FOUND: `54e4a99` (Task 2: DI wiring)
- FOUND: `8a8d683` (Task 3: E2E tests + teilnehmerliste.typ in defaults)

Build + tests verifiziert:
- `SQLX_OFFLINE=true cargo build --workspace --tests` Exit 0 (nur pre-existing warnings)
- `SQLX_OFFLINE=true cargo test -p genossi_rest --lib attendance_export -- --nocapture` 8/8 gruen
- `SQLX_OFFLINE=true cargo test -p genossi_bin --test e2e_tests test_export -- --nocapture` 9/9 gruen
- `SQLX_OFFLINE=true cargo test -p genossi_bin --test e2e_tests` 248/248 gruen (full e2e suite — keine Regression)

D-14 Format-Whitelist verifiziert:
- `grep -cE 'ExportFormat::(Csv|Pdf|Xlsx)' genossi_rest/src/attendance_export.rs` >= 3 ✓
- `test_export_unknown_format_returns_400` ✓

D-11 Status-Gate verifiziert:
- `test_export_open_assembly_returns_409_conflict` ✓
- `test_export_preparation_assembly_returns_409_conflict` ✓

D-12 Post-Close-Edit verifiziert:
- OpenAPI doc-string enthaelt expliziten D-12-Hinweis ✓
- `test_export_reflects_post_close_attendance_edit_d12` E2E-verified mit count-Differenz 3 -> 2 ✓

D-15 Filename-Schema verifiziert:
- `test_export_filename_schema_matches_date` deckt csv/pdf/xlsx ab ✓

D-16 Content-Type-Header verifiziert:
- PDF: `application/pdf` ✓
- CSV: `text/csv; charset=utf-8` ✓
- XLSX: `application/vnd.openxmlformats-officedocument.spreadsheetml.sheet` ✓

---
*Phase: 06-teilnehmerlisten-export-f-r-generalversammlungen*
*Completed: 2026-05-17*
