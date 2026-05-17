---
phase: 06-teilnehmerlisten-export-f-r-generalversammlungen
verified: 2026-05-17T13:02:09Z
status: passed
score: 25/25 must-haves verified
overrides_applied: 0
re_verification:
  is_re_verification: false
---

# Phase 6: Teilnehmerlisten-Export — Verification Report

**Phase Goal:** Vorstand kann nach Schluss einer Generalversammlung die Teilnehmerliste in drei Formaten (PDF, CSV, XLSX) über REST und einen UI-Button im Vorstand-Detail-View einer geschlossenen GV exportieren. Datenbasis ist der unveränderliche Member-Universe-Snapshot aus Phase 1 plus der Anwesenheits-Stand aus Phase 3 — Phase 6 ist read-only.

**Verified:** 2026-05-17T13:02:09Z
**Status:** passed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths (ROADMAP Success Criteria + Plan must_haves)

Truths aus den fünf ROADMAP-Success-Criteria + den must_haves der vier Pläne. Code-Lokationen sind absolute Pfade.

| #   | Truth (Source)                                                                                                                                          | Status     | Evidence                                                                                                                                                                                                                  |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | SC-1: `GET /api/assembly/{aid}/attendance-export/{format}` mit `format ∈ {csv,pdf,xlsx}` und `?include=all\|present` liefert für Closed-GVs korrekte Binary-Bodies + Content-Disposition (D-01, D-14, D-15, D-16) | VERIFIED   | `genossi_rest/src/attendance_export.rs:107` registriert Pfad `/api/assembly/{assembly_id}/attendance-export/{format}`; Handler whitelist `csv\|pdf\|xlsx` (Z. 135-145); 3× E2E-Tests grün (pdf_magic_bytes, csv_BOM, xlsx_zip_magic). |
| 2   | SC-2a: Aufruf gegen Status `Vorbereitung` oder `Geöffnet` liefert 409 Conflict mit `assembly_not_closed` (D-11)                                          | VERIFIED   | `genossi_service_impl/src/attendance_export.rs:126-128` setzt `Conflict("assembly_not_closed")`; E2E-Tests `test_export_open_assembly_returns_409_conflict` + `test_export_preparation_assembly_returns_409_conflict` grün. |
| 3   | SC-2b: Helfer-Token-Aufruf liefert 403 Forbidden (D-13)                                                                                                | VERIFIED   | `genossi_service_impl/src/attendance_export.rs:117-122` ADMIN_PRIVILEGE-Check; `genossi_rest/src/attendance_export.rs:61` `map_export_error` mappt PermissionDenied→Forbidden(403); Unit-Tests `non_admin_returns_permission_denied` + `test_map_export_error_permission_denied_returns_forbidden` grün. E2E mit echtem Helper-Token bewusst out-of-scope (siehe 06-03-PLAN). |
| 4   | SC-3a: CSV-Output startet mit UTF-8-BOM `[0xEF, 0xBB, 0xBF]` und nutzt Semikolon-Delimiter (D-03)                                                       | VERIFIED   | `genossi_service_impl/src/attendance_export.rs:251` BOM; Z. 255 `.delimiter(b';')`; E2E `test_export_csv_closed_starts_with_utf8_bom_and_uses_semicolon` grün (prüft `&bytes[..3] == [0xEF,0xBB,0xBF]` + `;` in Header). |
| 5   | SC-3b: XLSX-Output ist gültiger ZIP-Container (PK\x03\x04-Magic) (D-01)                                                                                | VERIFIED   | E2E `test_export_xlsx_closed_returns_zip_magic_bytes` prüft `b"PK\x03\x04"`-Bytes-Präfix; Unit-Test `xlsx_starts_with_zip_magic` grün.                                                                                  |
| 6   | SC-3c: PDF-Output startet mit `%PDF-`-Magic + Kopfblock + 6-Spalten-Tabelle (D-04, D-08)                                                                | VERIFIED   | E2E `test_export_pdf_closed_returns_pdf_magic_bytes` grün; `templates/teilnehmerliste.typ:31-33` hat 6-Spalten-Header `Nr. Nachname Vorname Anrede Titel anwesend`; Kopfblock Z. 17-22 mit `X von Y anwesend`/`X anwesend`. |
| 7   | SC-4a: KEIN Audit-Hashchain-Eintrag (D-17, grep-gate `audited_*!` ist `== 0`)                                                                          | VERIFIED   | `grep -v '^[[:space:]]*//' genossi_service_impl/src/attendance_export.rs \| grep -cE 'audited_(create\|update\|delete)!'` liefert `0`. Unit-Test `no_audit_macros_used` grün.                                            |
| 8   | SC-4b: `tracing::info!` mit aid + format + include + user-context im Service-Layer (D-18)                                                              | VERIFIED   | `genossi_service_impl/src/attendance_export.rs:197-204` `tracing::info!` mit `target="attendance_export"`, `aid=%assembly_id, format=?format, include=?include, rows`. REST-Handler hat zusätzlich `#[instrument(skip(rest_state))]`. |
| 9   | SC-5: Im Frontend erscheint im Vorstand-Detail-View nur bei `assembly.status == Closed` ein vierter Tab `Export` mit Format-Auswahl + Include-Toggle + reaktivem Filename-Preview + Download-Button (D-19, D-20) | VERIFIED   | `genossi-frontend/src/page/assembly_details.rs:90-95` Tab-Push gated by `matches!(a.status, AssemblyStatusTO::Closed)`; ExportTab-Component inline Z. 332-511 (D-20 inline, kein File in `src/component/`); 3 Format-Cards + 2 Include-Radios + reaktive Filename-Preview. |
| 10  | D-02: `rust_xlsxwriter` als Workspace-Dependency                                                                                                       | VERIFIED   | `Cargo.toml:30` `rust_xlsxwriter = "0.82"`; `genossi_service_impl/Cargo.toml:39` `{ workspace = true }`.                                                                                                                 |
| 11  | D-05: Datenquelle ist `AttendanceDao::list_members_for_assembly` (Snapshot+attendance Reuse, keine neue DAO-Methode)                                   | VERIFIED   | `genossi_service_impl/src/attendance_export.rs:158-161` ruft `list_members_for_assembly(assembly_id, None, tx.clone())` ohne Modifikation auf.                                                                            |
| 12  | D-06: 6-Spalten-Whitelist im Output (member_id nicht im File)                                                                                          | VERIFIED   | CSV-Header (Z. 259-264) und XLSX-Header (Z. 309-314) liefern exakt 6 Spalten `Mitgliedsnummer, Nachname, Vorname, Anrede, Titel, anwesend`; `member_id` nur in interner DAO-Row, nicht in render_csv/render_xlsx/render_attendance_list-Output. |
| 13  | D-07: Sortierung `member_number ASC` (durch DAO-Query)                                                                                                 | VERIFIED   | DAO-Reuse impliziert die seit commit `ed754fc` etablierte member_number-ASC-Sortierung. Plan 02 nutzt DAO-Call ohne eigenen `.sort()`-Aufruf — Sortierung kommt direkt aus DAO.                                            |
| 14  | D-08: PDF-Kopfblock mit GV-Titel + Datum + Zähler "X von Y anwesend"                                                                                  | VERIFIED   | `templates/teilnehmerliste.typ:12-22` `#show: letter.with(title, date)` + bedingter Header `#meta.present von #meta.total anwesend` (Z. 19).                                                                              |
| 15  | D-09: include=all/present-Filter; include=Present filtert `is_present==true` (Default=All)                                                              | VERIFIED   | `genossi_service_impl/src/attendance_export.rs:167-169` `rows.retain(\|r\| r.is_present)`; `genossi_service/src/attendance_export.rs:46` `Default::default() == All`. E2E `test_export_include_present_filters_absent_members` grün. |
| 16  | D-10: PDF zeigt "X von Y" bei include=All; "X anwesend" bei include=Present                                                                            | VERIFIED   | `genossi_service_impl/src/attendance_export.rs:182-185` setzt `total = match include { All => Some(rows.len()), Present => None }`; Template-Branch Z. 18-22.                                                            |
| 17  | D-12: Post-Close-Anwesenheits-Korrekturen wirken sich auf nachfolgende Exporte aus                                                                     | VERIFIED   | E2E `test_export_reflects_post_close_attendance_edit_d12` zeigt Export-Anzahl-Differenz nach DELETE eines present-Members; OpenAPI-Doku in `genossi_rest/src/attendance_export.rs:99-103` dokumentiert das explizit.       |
| 18  | D-15: Filename-Schema `gv-{YYYY-MM-DD}-teilnehmer.{ext}`                                                                                              | VERIFIED   | `genossi_service_impl/src/attendance_export.rs:234` `format!("gv-{}-teilnehmer.{}", date_str, ext)`; E2E `test_export_filename_schema_matches_date` prüft alle 3 Extensions; Frontend `format_assembly_date_yyyy_mm_dd` + reaktive Preview Z. 491. |
| 19  | D-16: Content-Type pdf/csv/xlsx korrekt gemappt                                                                                                       | VERIFIED   | `genossi_service_impl/src/attendance_export.rs:209/214/226` setzt `text/csv; charset=utf-8`, `application/vnd.openxmlformats-officedocument.spreadsheetml.sheet`, `application/pdf`; E2E-Tests prüfen alle drei Header. |
| 20  | DI-Wiring: `RestStateImpl::new()` konstruiert `AttendanceExportServiceImpl` mit allen 6 Deps                                                            | VERIFIED   | `genossi_bin/src/lib.rs:205-223` Dependencies-Struct + Impl; Z. 453 `attendance_export_service`-Feld; Z. 696-... Konstruktion mit `pdf_generator.clone()` + `template_storage.base_path().to_path_buf()`; Z. 1284 `impl AttendanceExportRestState`. |
| 21  | Router-Mount unter `/api/assembly` mit OpenAPI-Nest + Trait-Bounds in create_app + start_server                                                          | VERIFIED   | `genossi_rest/src/lib.rs:4` `pub mod attendance_export`; Z. 271 OpenAPI-nest; Z. 438 + 751 Trait-Bound `+ attendance_export::AttendanceExportRestState`; Z. 628 Router-nest.                                              |
| 22  | i18n: 21 neue Keys in DE + EN (1 Tab-Label + 20 AttendanceExport*)                                                                                     | VERIFIED   | `grep -c "AttendanceExport" genossi-frontend/src/i18n/{mod,de,en}.rs` = 20 in jeder Datei; `AssemblyTabExport` = 1 (gesamt 21).                                                                                          |
| 23  | Frontend `api::export_attendance_url` baut korrekte URL und liefert Blob-URL                                                                            | VERIFIED   | `genossi-frontend/src/api.rs:1875-1922` baut `{backend}/api/assembly/{aid}/attendance-export/{format}?include={include}`; nutzt `Url::create_object_url_with_blob`; AppError-Status wird für 403/409-Map propagiert. |
| 24  | Default-Template `teilnehmerliste.typ` für Production via `provision_defaults`                                                                          | VERIFIED   | `genossi_service_impl/src/template_storage.rs:24-25` registriert `path: "teilnehmerliste.typ"` + `include_bytes!("../../templates/defaults/teilnehmerliste.typ")`. `templates/defaults/teilnehmerliste.typ` existiert. |
| 25  | Beide Browser-Verify-Fixes (form→div, ✓→ja/nein) sind im Code aktiv                                                                                    | VERIFIED   | (a) `genossi-frontend/src/page/assembly_details.rs:496-499` Button mit `r#type: "button"` + `onclick: on_submit` (kein `<form>`-Wrapper mehr — commit c6f41fd). (b) `templates/teilnehmerliste.typ:41` + `templates/defaults/teilnehmerliste.typ:41` enthalten `if r.is_present [ja] else [nein]` (kein `✓` — commit bb1be0b). |

**Score:** 25/25 truths verified

---

### Required Artifacts

| Artifact                                                        | Expected                                                                                | Status     | Details                                                              |
| --------------------------------------------------------------- | --------------------------------------------------------------------------------------- | ---------- | -------------------------------------------------------------------- |
| `Cargo.toml`                                                    | `rust_xlsxwriter` + `csv` in `[workspace.dependencies]`                                  | VERIFIED   | Z. 30 `rust_xlsxwriter = "0.82"`; csv ebenfalls workspace-deklariert. |
| `genossi_service_impl/Cargo.toml`                               | Konsumiert beide via `{ workspace = true }`                                              | VERIFIED   | Z. 39.                                                               |
| `templates/teilnehmerliste.typ`                                 | Typst-Template mit `_layout.typ`-Import + 6-Spalten-Tabelle + Kopf-Block                 | VERIFIED   | 43 Zeilen, alle Acceptance-Patterns vorhanden.                       |
| `templates/defaults/teilnehmerliste.typ`                        | Embedded-Default (1:1-Kopie für `provision_defaults`)                                    | VERIFIED   | Identisch mit `templates/teilnehmerliste.typ`.                       |
| `genossi_service/src/attendance_export.rs`                      | Trait `AttendanceExportService` + Enums `ExportFormat`/`ExportInclude` + `AttendanceExport` | VERIFIED   | 5 Unit-Tests grün; 154 Zeilen.                                       |
| `genossi_service_impl/src/attendance_export.rs`                 | Impl + Permission-Funnel + 3 Format-Writer                                               | VERIFIED   | 46 KB, 13 Unit-Tests grün. min_lines=200 weit überschritten.        |
| `genossi_service_impl/src/pdf_generation.rs`                    | Neue Methode `render_attendance_list`                                                    | VERIFIED   | Z. 279ff. (`build_inputs_attendance` Z. 264).                        |
| `genossi_rest/src/attendance_export.rs`                         | Handler + Router-Builder + ApiDoc + Trait                                                | VERIFIED   | 10.7 KB; 8 Unit-Tests grün.                                          |
| `genossi_rest/src/lib.rs`                                       | `pub mod` + Router-Nest + Trait-Bounds                                                   | VERIFIED   | Alle 5 Hooks (mod, OpenAPI-nest, 2× Trait-Bound, Router-nest).      |
| `genossi_rest/src/test_server.rs`                               | Trait-Bound um `AttendanceExportRestState` erweitert                                     | VERIFIED   | grep findet `AttendanceExportRestState`.                            |
| `genossi_bin/src/lib.rs`                                        | Deps-Struct + Type-Alias + Construction + Trait-Impl                                     | VERIFIED   | Z. 205-223 + 453 + 696 + 792 + 1284-1290.                            |
| `genossi_bin/tests/e2e_tests.rs`                                | 9 E2E-Tests + Helper                                                                     | VERIFIED   | 9 `test_export_*`-Funktionen, alle grün; `create_closed_assembly_with_members`-Helper. |
| `genossi_service_impl/src/template_storage.rs`                  | `teilnehmerliste.typ` in `DEFAULT_TEMPLATES` registriert                                 | VERIFIED   | Z. 24-25.                                                            |
| `genossi-frontend/src/api.rs`                                   | `pub async fn export_attendance_url`                                                     | VERIFIED   | Z. 1875-1922.                                                        |
| `genossi-frontend/src/page/assembly_details.rs`                 | `ExportTab` inline + status-gated tab + match-arm                                        | VERIFIED   | Z. 90-95 (Tab-Push), Z. 143 (match-arm), Z. 332-511 (Component).     |
| `genossi-frontend/src/i18n/{mod,de,en}.rs`                      | 21 neue Keys (1 Tab-Label + 20 AttendanceExport*)                                        | VERIFIED   | 20 + 1 = 21 in jeder der 3 Dateien.                                  |

---

### Key Link Verification

| From                                                | To                                                                       | Via                                                                  | Status   | Details                                                                  |
| --------------------------------------------------- | ------------------------------------------------------------------------ | -------------------------------------------------------------------- | -------- | ------------------------------------------------------------------------ |
| `templates/teilnehmerliste.typ`                     | `templates/_layout.typ`                                                  | `#import "_layout.typ": letter`                                      | WIRED    | Z. 7.                                                                    |
| `attendance_export.rs::export()`                    | `AttendanceDao::list_members_for_assembly`                               | DAO-Call mit `search=None`                                           | WIRED    | Z. 158-161.                                                              |
| `attendance_export.rs::export()`                    | `PdfGenerator::render_attendance_list`                                   | `self.pdf_generator.render_attendance_list("teilnehmerliste.typ",...)` | WIRED    | Z. 218-227.                                                              |
| `attendance_export.rs::export()`                    | `templates/teilnehmerliste.typ`                                          | String-Pfad-Argument                                                 | WIRED    | Z. 219 (`"teilnehmerliste.typ"`).                                        |
| `genossi_rest::attendance_export::export_attendance` | `AttendanceExportService::export`                                        | `rest_state.attendance_export_service().export(...)`                 | WIRED    | Z. 148-152.                                                              |
| `genossi_bin/src/lib.rs::RestStateImpl::new`        | `AttendanceExportServiceImpl`                                            | `Arc::new(AttendanceExportServiceImpl { ... })`                      | WIRED    | Z. 696-... Konstruktion mit allen 6 Feldern (4 DAO/Service + pdf_generator + template_base). |
| `genossi_rest/src/lib.rs`                           | `/api/assembly/{assembly_id}/attendance-export/{format}`                  | `.nest("/api/assembly", attendance_export::generate_export_route::<RestState>())` | WIRED    | Z. 627-628.                                                              |
| `assembly_details.rs::ExportTab`                    | `api::export_attendance_url`                                              | `spawn(async move { ... api::export_attendance_url(&cfg, ...) })`    | WIRED    | Z. 364.                                                                  |
| `assembly_details.rs`                               | `GET /api/assembly/{aid}/attendance-export/{format}`                      | fetch + blob + revoke (anchor.click in `on_submit`)                  | WIRED    | `api.rs:1885` + `assembly_details.rs:366-380`.                           |
| Tab-Visibility                                       | `assembly.status == AssemblyStatusTO::Closed`                            | `if matches!(a.status, AssemblyStatusTO::Closed) { tab_defs.push(...) }` | WIRED    | Z. 90-95.                                                                |

Alle 10 Key-Links sind WIRED.

---

### Data-Flow Trace (Level 4)

Für jeden render-bezogenen Artifact: Werden echte Daten gerendert (nicht hardcoded leer/null)?

| Artifact                          | Data Variable          | Source                                                                | Produces Real Data | Status     |
| --------------------------------- | ---------------------- | --------------------------------------------------------------------- | ------------------ | ---------- |
| `export()` Service-Method         | `rows`                 | `attendance_dao.list_members_for_assembly(aid, None, tx)`              | Yes (DB-Query)     | FLOWING    |
| `render_csv(&rows)`               | Rows-Iteration         | Service-Parameter `rows` (DB-Query-Result)                            | Yes                | FLOWING    |
| `render_xlsx(&rows)`              | Rows-Iteration         | Service-Parameter `rows`                                              | Yes                | FLOWING    |
| `render_attendance_list`          | `assembly`, `rows`, `present`, `total` | Service-Parameter; JSON-serialisiert in `sys.inputs`           | Yes                | FLOWING    |
| Filename `gv-{date}-teilnehmer.{ext}` | `assembly.date`       | `find_by_id`-Query → `assembly.date.date().format(...)`                | Yes                | FLOWING    |
| `ExportTab` Filename-Preview      | `filename`             | `selected_format.read()` + `format_assembly_date_yyyy_mm_dd(&assembly.date)` | Yes (Props+Signal) | FLOWING    |
| `ExportTab` Submit-Klick          | Blob-Response          | `api::export_attendance_url` → `Url::create_object_url_with_blob`     | Yes                | FLOWING    |

Keine HOLLOW oder DISCONNECTED Artifacts.

---

### Behavioral Spot-Checks

Automated checks gegen den real laufenden Test-Server.

| Behavior                                       | Command                                                                                       | Result                                  | Status |
| ---------------------------------------------- | --------------------------------------------------------------------------------------------- | --------------------------------------- | ------ |
| All Phase-6 E2E tests pass                     | `SQLX_OFFLINE=true cargo test -p genossi_bin --test e2e_tests test_export`                    | `9 passed; 0 failed; 0 ignored`         | PASS   |
| Service Unit-Tests (Service)                   | `cargo test -p genossi_service --features utoipa --lib attendance_export`                     | `5 passed; 0 failed`                    | PASS   |
| Service Unit-Tests (ServiceImpl)               | `cargo test -p genossi_service_impl --lib attendance_export`                                  | `13 passed; 0 failed`                   | PASS   |
| REST Unit-Tests                                | `cargo test -p genossi_rest --lib attendance_export`                                          | `8 passed; 0 failed`                    | PASS   |
| Frontend Pure-Logic-Tests                      | `cargo test --bin genossi-frontend export_tab_tests` (in `genossi-frontend/`)                  | `3 passed; 0 failed`                    | PASS   |
| D-17 No-Audit Grep-Gate                        | `grep -v '^[[:space:]]*//' genossi_service_impl/src/attendance_export.rs \| grep -cE 'audited_(create\|update\|delete)!'` | `0`                                     | PASS   |
| Workspace builds clean                         | `SQLX_OFFLINE=true cargo build --workspace --tests`                                           | Exit 0 (nur pre-existing Warnings)      | PASS   |
| Phase-6-Tests gegen real running server (PDF magic) | E2E `test_export_pdf_closed_returns_pdf_magic_bytes`                                          | OK + `%PDF-` Magic-Bytes                | PASS   |
| Phase-6-Tests gegen real running server (CSV BOM) | E2E `test_export_csv_closed_starts_with_utf8_bom_and_uses_semicolon`                          | OK + `0xEF,0xBB,0xBF` + `;`-Delim       | PASS   |
| Phase-6-Tests gegen real running server (XLSX zip) | E2E `test_export_xlsx_closed_returns_zip_magic_bytes`                                         | OK + `PK\x03\x04`-Magic                 | PASS   |

Insgesamt **35 Test-Cases** (5 Service-Unit + 13 ServiceImpl-Unit + 8 REST-Unit + 3 Frontend-Unit + 9 E2E) decken die Phase ab — alle grün.

---

### Requirements Coverage (D-01..D-20)

| Requirement | Source Plan(s)    | Description                                                                                | Status      | Evidence                                                                                              |
| ----------- | ----------------- | ------------------------------------------------------------------------------------------ | ----------- | ----------------------------------------------------------------------------------------------------- |
| D-01        | 02, 03, 04        | Drei Formate (PDF/CSV/XLSX) parallel                                                       | SATISFIED   | `ExportFormat { Csv, Pdf, Xlsx }`; alle drei Pfade in Service + REST + E2E-Tests.                     |
| D-02        | 01                | `rust_xlsxwriter` als Workspace-Dep                                                        | SATISFIED   | `Cargo.toml:30` + `genossi_service_impl/Cargo.toml:39`.                                              |
| D-03        | 01, 02            | CSV Semikolon-getrennt + UTF-8 mit BOM                                                     | SATISFIED   | BOM Z. 251, `.delimiter(b';')` Z. 255 in `attendance_export.rs`.                                     |
| D-04        | 01, 02            | PDF via bestehender Typst-Toolchain                                                        | SATISFIED   | `render_attendance_list` ruft `TemplateWorld` + `typst::compile` + `typst_pdf::pdf`; analog `render_application`. |
| D-05        | 02                | Datenquelle Snapshot + attendance (historisch stabil)                                      | SATISFIED   | DAO-Call `list_members_for_assembly(_, None, _)` — Snapshot+attendance JOIN.                        |
| D-06        | 02                | Spalten-Whitelist (member_id NICHT im Export-File)                                         | SATISFIED   | CSV/XLSX/PDF haben exakt 6 Daten-Spalten (Mitgliedsnummer..anwesend); member_id wird nur DAO-intern verwendet. |
| D-07        | 02                | Sortierung `member_number ASC`                                                              | SATISFIED   | DAO-Reuse (Sortierung kommt aus DAO-Query); kein eigenes `.sort()` im Export-Service.                |
| D-08        | 01, 02            | PDF-Kopfblock mit Titel + Datum + Zähler                                                   | SATISFIED   | `templates/teilnehmerliste.typ:12-22`.                                                               |
| D-09        | 02, 04            | Query-Parameter `?include=all\|present`; Default=All                                       | SATISFIED   | `ExportInclude::default() == All`; `rows.retain(\|r\| r.is_present)` bei Present.                    |
| D-10        | 01, 02            | "X von Y" bei include=All; "X anwesend" bei include=Present                                 | SATISFIED   | Service `total = match include { All => Some(rows.len()), Present => None }`; Template-Branch.       |
| D-11        | 02, 03, 04        | Export NUR für `Closed`-GVs; sonst 409 Conflict                                            | SATISFIED   | `assembly.status != AssemblyStatus::Closed → Conflict("assembly_not_closed")`; 2 E2E-Tests grün.    |
| D-12        | 02, 03            | Post-Close-Korrekturen wirken sich auf nachfolgende Exporte aus                            | SATISFIED   | E2E `test_export_reflects_post_close_attendance_edit_d12` + OpenAPI-Doku in REST-Handler.            |
| D-13        | 02, 03            | Zugriff nur für Vorstand (Admin); Helfer haben KEINEN Zugriff                              | SATISFIED   | `check_admin_and_closed` ruft `check_permission("admin")`; REST mappt PermissionDenied→403; Service- + REST-Unit-Test. |
| D-14        | 03                | Endpoint `/api/assembly/{aid}/attendance-export/{format}`                                  | SATISFIED   | Router-Mount `/api/assembly` + Route `/{assembly_id}/attendance-export/{format}`; Format-Whitelist im Handler. |
| D-15        | 02, 03, 04        | Filename `gv-{YYYY-MM-DD}-teilnehmer.{ext}`                                                | SATISFIED   | Service Z. 234; E2E `test_export_filename_schema_matches_date` deckt alle 3 Extensions; Frontend reaktive Preview. |
| D-16        | 02, 03            | MIME-Types pdf/csv/xlsx                                                                    | SATISFIED   | `application/pdf`, `text/csv; charset=utf-8`, `application/vnd.openxmlformats-officedocument.spreadsheetml.sheet`. |
| D-17        | 02                | KEIN Audit-Hashchain-Eintrag                                                                | SATISFIED   | Grep-Gate `audited_*!` ist `0`; Unit-Test `no_audit_macros_used` grün.                              |
| D-18        | 02, 03            | `tracing::info!` mit GV-ID, Format, Include, User-ID                                       | SATISFIED   | Service Z. 197-204 mit allen Feldern; REST-Handler `#[instrument(skip(rest_state))]`.               |
| D-19        | 04                | Download-Block sichtbar/aktivierbar NUR bei `assembly.status == Closed`                    | SATISFIED   | Tab-Push gated by `matches!(a.status, AssemblyStatusTO::Closed)`.                                    |
| D-20        | 04                | Component-First gilt; im Phase-6-Scope ist nur EINE Page betroffen → inline                | SATISFIED   | `ExportTab` ist inline in `assembly_details.rs`; kein `src/component/export*`-File existiert.        |

**Coverage:** 20/20 D-Decisions SATISFIED. Keine ORPHANED requirements: jede D-ID ist mindestens einem Plan zugeordnet.

---

### Anti-Patterns Found

| File                                            | Line     | Pattern             | Severity | Impact                                                                          |
| ----------------------------------------------- | -------- | ------------------- | -------- | ------------------------------------------------------------------------------- |
| `genossi_bin/src/lib.rs`                        | 828      | Unused import `Auditable` | INFO     | Pre-existing warning, nicht aus Phase 6 (siehe 06-02-SUMMARY).                |
| `genossi_rest/src/lib.rs`                       | 30       | Unused import `IntoResponse` | INFO     | Pre-existing warning, nicht aus Phase 6.                                       |
| `genossi_rest/src/permission.rs`                | 780      | Unused import `put` | INFO     | Pre-existing warning, nicht aus Phase 6.                                       |

Keine Phase-6-spezifischen Anti-Patterns. Keine BLOCKER, keine WARNINGS.

Geprüft (alles negativ — also kein Fund):
- `grep -nE "TODO\|FIXME\|XXX\|HACK\|PLACEHOLDER\|placeholder\|coming soon\|not yet implemented"` auf allen Phase-6-Dateien → keine Befunde.
- `grep -nE "return null\|return \\\[\\\]\|return \\\{\\\}\|=> \\\{\\\}"` als Stub-Indikatoren → keine im Export-Pfad.
- Hardcoded leere Props an ExportTab-Aufrufsite (`<ExportTab data={[]}>`) → nicht vorhanden (Props sind `assembly: AssemblyTO` + `on_error: EventHandler<String>` mit echten Werten).

---

### Human Verification

Browser-Verifikation wurde bereits durch User-Approval beim 06-04-Task-3-Checkpoint abgeschlossen ("Jetzt ist es gut." nach den zwei inline Hotfixes c6f41fd + bb1be0b). Die zwei Fixes sind in der Codebase verifiziert. Keine zusätzliche Human-Verification erforderlich.

---

### Gaps Summary

Keine BLOCKER gefunden. Keine WARNINGS.

Anmerkungen (nicht-Gap):
- **D-13 Helper-403 ist NICHT E2E-getestet**, sondern nur via Service-Unit-Test (`non_admin_returns_permission_denied`) + REST-Unit-Test (`test_map_export_error_permission_denied_returns_forbidden`). Plan 03 hat diesen E2E-Test explizit out-of-scope deklariert (Helper-Auth-Setup zu komplex relativ zum Erkenntnisgewinn). Das ist eine bewusste Plan-Entscheidung, kein Gap.
- **Template-Glyph wurde von `✓` auf `[ja]/[nein]`** geändert (Hotfix bb1be0b) wegen fehlendem Symbol-Font im Liberation-Sans-Bundle. Die ursprüngliche Plan-01-Acceptance-Klausel `grep -c '✓' templates/teilnehmerliste.typ == 1` ist damit nicht mehr erfüllt — aber die übergeordnete D-08-Requirement "Anwesenheits-Markierung im PDF" ist semantisch besser umgesetzt (Liberation-Sans rendert "ja"/"nein" sauber, statt einen `.notdef`-Rechteck-Glyph zu zeigen). Plus: CSV/XLSX nutzten schon vorher "ja"/"nein", jetzt sind alle drei Formate konsistent. Die Plan-01-grep-Klausel ist überholt, aber das User-Goal ist erfüllt.

---

## Final Status

**status: passed**

- 25/25 must-haves verified
- 20/20 D-Decisions (D-01..D-20) satisfied
- 9/9 E2E-Tests grün
- 5+13+8+3 = 29 Unit-Tests grün
- 5/5 ROADMAP Success Criteria erfüllt
- DI + Router + Frontend-Wiring vollständig
- D-17 No-Audit-Invariante via Grep-Gate verifiziert
- Beide User-Verify-Hotfixes (form→div, ✓→ja/nein) im Code aktiv
- Keine BLOCKER, keine WARNINGS, keine ORPHANED requirements

Phase-Goal achieved.

---

_Verified: 2026-05-17T13:02:09Z_
_Verifier: Claude (gsd-verifier)_
