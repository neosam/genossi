---
phase: 11-export-pdf-csv
plan: 06
subsystem: e2e-tests
tags: [e2e-tests, regression, grep-gate, repayment-export, pdf]

# Dependency graph
requires:
  - phase: 11-export-pdf-csv
    provides: "Plan 11.03 RepaymentExportServiceImpl + Permission-Funnel + Grep-Gate-Test (`no_audit_macros_used`)"
  - phase: 11-export-pdf-csv
    provides: "Plan 11.04 REST-Handler `export_repayment` + Format-Whitelist + map_export_error"
  - phase: 11-export-pdf-csv
    provides: "Plan 11.05 DI-Wiring (RestStateImpl::repayment_export_service)"
  - phase: 08-repayment-entry
    provides: "create_member_with_exit_date, create_preparation_repayment_phase, create_open_repayment_phase E2E-Helper"
  - phase: 09-payout-cascade
    provides: "POST /api/repayment-entry/{id}/mark-paid-out, POST /api/repayment-entry/batch-status"
provides:
  - "8 E2E-Tests fuer RepaymentExport PDF gegen real-running Server"
  - "Helper `create_member_without_iban` (D-06-Edge-Case-Setup)"
  - "Phase 11 ist end-to-end verifiziert: EXPO-01/02/03/05 + D-05/D-06/D-10/D-11/D-12"
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Regression-Lock-In via additive `test`-Commit (Plan-08-10-Pattern): Tests werden hinzugefuegt fuer ein BEREITS implementiertes Feature; keine TDD-RED-Sequenz, weil Plan 11.03–11.05 das Feature bereits GREEN bereitgestellt haben. Alle 8 Tests grün on first run."
    - "Umlaut-Member-Setup-Pattern (REVISION-Fix W6): Inline POST `/api/members` mit `Müller` + nachgeschobener Austritt-Action — Vorlage fuer kuenftige E2E-Tests, die einen Member mit nicht-Default-Namen UND `exit_date` brauchen (sample_member() unterstuetzt Namen-Variation nicht, create_member_with_exit_date akzeptiert keinen Namen-Parameter)"
    - "Helper `create_member_without_iban` als 3-Schritt-Workflow (POST mit IBAN → PUT mit bank_account=None → GET zum Re-Read mit gebumpter Version) — Mirror zum Phase-7-Lifecycle-Pattern (post → mutate → re-read)"
    - "REVISION-Fix B2 Pattern: E2E-Test ENTFERNT, wenn mock_auth-Constraint die Mitigation nicht herstellen kann — stattdessen Service-Layer-Mock-Test in Vorgaenger-Plan (Plan 11.03). Anker fuer kuenftige Status-Leak-Tests unter mock_auth-Harnessen."

key-files:
  created: []
  modified:
    - "genossi_bin/tests/e2e_tests.rs (+554 Zeilen: 1 Helper + 8 E2E-Tests; -4 Zeilen formatting)"

key-decisions:
  - "Regression-Lock-In statt TDD-RED→GREEN: Plan 11.03–11.05 haben den Endpoint bereits produktionsreif geliefert; ein synthetischer RED-Schritt (z.B. absichtliches Brechen einer Vorgaenger-Implementation, um den Test rot zu sehen) waere kontraproduktiv. Pattern-Anker aus Plan 08.10. Die `tdd=\"true\"`-Annotation im Plan-Frontmatter wird hier so interpretiert, dass die Tests den TDD-Cycle der GESAMTEN Phase abschliessen (Vorgaenger-Plans waren je TDD-RED→GREEN; Plan 11.06 ist die abschliessende E2E-Verifikation)."
  - "Umlaut-Member via inline POST (NICHT als Helper-Variante): `create_member_with_exit_date` akzeptiert nur `member_number`/`fiscal_year`/`share_count`; ein Refactor zu Namen-Parameter waere Rule-4-Architektur-Change. Inline-POST mit nachfolgender Austritt-Action ist 17 LOC und respektiert den `compute_dates`-Pfad (Member-Action-Single-Source-of-Truth) — Plan-08-06-Decision-Anker."
  - "REVISION-Fix B2 vollstaendig umgesetzt: KEIN E2E-Test `test_export_repayment_non_admin_on_preparation_returns_403_not_409`. Grep-Verifikation: `grep -c '...non_admin_on_preparation_returns_403_not_409' genossi_bin/tests/e2e_tests.rs == 0`. Pitfall #2 ist vollstaendig durch Plan 11.03 Service-Layer-Mock abgedeckt."
  - "REVISION-Fix W4 Filename-Schema-Assertion in JEDEM PDF-Success-Test (5 PDF-Tests asserten `auszahlung-{fy}-{include}.pdf`-Pattern; Format-String-Grep-Count: 5)."
  - "REVISION-Fix W6 Umlaut-Member im Happy-Path Test 1 (16 'Müller'-Treffer im File durch Setup-Code + Doc-Comments — Plan-Acceptance forderte >= 1)."
  - "REVISION-Fix W7 Audit-Chain-Test reduziert auf `valid: true` vor+nach Export. Compile-Time-Grep-Gate aus Plan 11.03 (`no_audit_macros_used`) garantiert, dass kein neuer Audit-Eintrag entstehen KANN. Ein zusaetzlicher Runtime-Count-Delta-Check waere redundant."

patterns-established:
  - "Phase-Final-E2E-Plan-Pattern: Wenn ein Endpoint in Waves entstanden ist (Service in Plan 03, REST in Plan 04, DI in Plan 05), liefert der finale E2E-Plan Regression-Lock-In gegen Production-Pfade. Tests sind grün on first run; trotzdem TDD-Annotation, weil sie den Cycle der GESAMTEN Phase abschliessen."
  - "Acceptance-Criteria-Grep-Pattern mit zwei Reading-Optionen: `grep -cE 'auszahlung-\\{\\}-[a-z]+\\.pdf' >= 3` ODER `grep -c 'auszahlung-2026' >= 3`. Format-String-Pattern fanger 5 Treffer (Plan-Acceptance erfuellt), substring-Pattern fanger 0 Treffer (weil `auszahlung-2026` ein laufzeit-formatierter String und kein Source-Literal ist). Lesart: das Format-Pattern matched die echte Quelle."

requirements-completed: [EXPO-01, EXPO-02, EXPO-03, EXPO-05]

# Metrics
duration: 7min
completed: 2026-06-01
---

# Phase 11 Plan 06: RepaymentExport E2E-Tests Summary

**8 neue E2E-Tests + 1 neuer Helper `create_member_without_iban` in `genossi_bin/tests/e2e_tests.rs` verifizieren die gesamte Phase 11 (EXPO-01/02/03/05) gegen einen real-running Server mit In-Memory-SQLite. Happy-Path inkl. Umlaut-Member `Hans Müller` (REVISION-Fix W6); Filename-Schema `auszahlung-{fy}-{include}.pdf` in jedem PDF-Erfolgsfall asserted (REVISION-Fix W4); Pitfall #2 (Status-Leak) bleibt durch Plan 11.03 Service-Layer-Mock abgedeckt — KEIN E2E-403-Test (REVISION-Fix B2). Plan-11.03-Grep-Gate (`no_audit_macros_used`) und Service-Layer-Pitfall-#2-Test bleiben gruen.**

## Performance

- **Duration:** ~7 min
- **Started:** 2026-06-01T05:58:22Z
- **Completed:** 2026-06-01T06:05:10Z
- **Tasks:** 2 (Task 1: 6 Tests + Helper, Task 2: 2 weitere Tests)
- **Files modified:** 1 (`genossi_bin/tests/e2e_tests.rs`, +554 Zeilen)
- **Tests added:** 8 (alle 8 gruen on first run)
- **Suite size:** 292 E2E-Tests pass im `cargo test --features mock_auth -p genossi_bin --test e2e_tests` (vorher 284, +8 neue, 0 Regression)

## Accomplishments

- **8 E2E-Tests** verifizieren alle 4 Requirements (EXPO-01/02/03/05) und alle relevanten User-Decisions (D-05/D-06/D-10/D-11/D-12) gegen den real-running Server:
  1. `test_export_repayment_pdf_open_happy_path` — Open-Phase + 2 Members (1 davon `Hans Müller` mit Umlaut), 200, `application/pdf`, `%PDF-` Magic, Filename `auszahlung-2026-open.pdf` (EXPO-01/02/03 + D-05 + REVISION-Fix W4/W6)
  2. `test_export_repayment_pdf_closed_phase_returns_200` — Geschlossene Phase weiterhin exportierbar inkl. Filename `auszahlung-2026-all.pdf` (EXPO-01 + D-10 + REVISION-Fix W4)
  3. `test_export_repayment_unknown_format_returns_400` — 4 Negative-Formate (`csv`, `xlsx`, `json`, `html`) — alle 400 (D-12 + Pitfall #3)
  4. `test_export_repayment_preparation_phase_returns_409` — Preparation-Phase → 409 mit Body-Substring `phase_not_exportable` (D-10)
  5. `test_export_repayment_unknown_phase_id_returns_404` — Random UUID → 404
  6. `test_export_repayment_does_not_break_audit_chain` — `/api/audit/verify` bleibt `valid: true` vor+nach Export inkl. Filename-Assertion (EXPO-05 + D-11 + REVISION-Fix W4/W7)
  7. `test_export_repayment_include_filter_smoke_all_three_variants` — Setup mit 2 Open + 1 Contacted + 1 PaidOut; jede Variante (`open`/`all`/`paid`) liefert 200, PDF-Magic, korrektes Filename (EXPO-03 + REVISION-Fix W1/W4)
  8. `test_export_repayment_empty_iban_renders_empty_column` — Member ohne IBAN rendert ohne Crash; Filename-Schema asserted (D-06 + REVISION-Fix W4)
- **Helper `create_member_without_iban`** (3-Schritt-Workflow): POST Member mit Sample-IBAN → PUT mit `bank_account: None` → GET zum Re-Read mit gebumpter Version. Doc-Comment erklaert D-06-Workflow und Member-Service-Konvention (`current_shares = shares_at_joining` beim Create).
- **REVISION-Fix B2 vollstaendig umgesetzt:** KEIN E2E-403-Test (das Test-Harness laeuft `#![cfg(feature = "mock_auth")]` und injiziert IMMER admin; ein 403-Test waere ein No-op oder rot). Pitfall #2 ist vollstaendig durch Plan 11.03 Service-Layer-Mock-Test (`test_non_admin_on_preparation_returns_permission_denied_not_conflict`) abgedeckt.
- **Plan-11.03-Grep-Gate weiter gruen** (`cargo test -p genossi_service_impl --lib no_audit_macros_used`): `0` Treffer fuer `audited_(create|update|delete)!` im `repayment_export.rs`-Code-Pfad — der Export-Service ist read-only und schreibt keine Audit-Eintraege. Compile-time abgesichert.
- **Filename-Schema-Assertion in 5 Tests** (Plan-Acceptance: >= 3 Treffer): jeder erfolgreiche PDF-Test asserted explizit `auszahlung-{fy}-{include}.pdf`, sodass eine versehentliche Renamings (z.B. zu `repayment-export-...`) deterministisch von der Test-Suite gefangen wird.

## Task Commits

Each task was committed atomically:

1. **Task 1: 6 Kern-E2E-Tests + Helper `create_member_without_iban`** — `67f6957` (test)
   - 6 Tests laut Plan-Schritte 2–7 (Happy-Path mit Umlaut, Closed-Phase, Format-Whitelist, Status-409, 404, Audit-Chain) + Helper-Funktion.
   - Build clean, alle 6 Tests gruen on first run.

2. **Task 2: Include-Filter-Smoke + Empty-IBAN** — `33f7e16` (test)
   - 2 weitere Tests laut Plan-Schritte 1–2 von Task 2.
   - Include-Filter-Test triggert batch-status (→ Contacted) + mark-paid-out (→ PaidOut), dann pro Variante ein Export-Call mit Filename-Assertion.
   - Empty-IBAN-Test nutzt den Helper aus Task 1.
   - Build clean, beide Tests gruen on first run.

**Plan metadata commit:** folgt nach diesem SUMMARY (docs-Commit mit STATE.md + ROADMAP.md + REQUIREMENTS.md-Updates).

## Files Created/Modified

- `genossi_bin/tests/e2e_tests.rs` (MODIFIED, +554 Zeilen / -4 Zeilen formatting): 1 Helper (`create_member_without_iban`, ~50 LOC) + 8 E2E-Tests (~500 LOC zusammen).

## Decisions Made

- **Regression-Lock-In statt synthetischer TDD-RED-Schritt:** Plan-Frontmatter hat `tdd="true"`, aber Plan 11.03–11.05 haben das Feature bereits GREEN bereitgestellt. Ein synthetischer RED-Schritt (z.B. absichtliches Brechen der Vorgaenger-Implementation) waere kontraproduktiv. Pattern aus Plan 08.10 uebernommen — die TDD-Annotation gilt fuer den TDD-Cycle der GESAMTEN Phase 11, dessen RED- und GREEN-Schritte in den Plans 11.01–11.05 verteilt sind (z.B. Plan 11.04 RED `ee64d88` + GREEN `cf9ce76`).
- **Umlaut-Member via inline POST + Austritt-Action**, NICHT als neue Helper-Variante. `create_member_with_exit_date` akzeptiert nur `member_number/fiscal_year/share_count`; ein Refactor mit Namen-Parameter waere Rule-4-Architektur-Change (out-of-scope). Stattdessen wird `sample_member()` mit modifiziertem `first_name="Hans"`/`last_name="Müller"` lokal im Test kombiniert und der `Austritt`-Action-Workflow nachgezogen (`exit_date`-Single-Source-of-Truth bleibt `MemberAction::Austritt` per Phase-8-D-04). 17 LOC zusaetzliches Setup im Happy-Path-Test.
- **REVISION-Fix B2 vollstaendig umgesetzt:** Plan-Acceptance-Criterion `grep -c "test_export_repayment_non_admin_on_preparation_returns_403_not_409" genossi_bin/tests/e2e_tests.rs == 0` ist erfuellt — der Test wurde nie hinzugefuegt, weil mock_auth-Middleware (`genossi_rest/src/auth_middleware.rs::mock_auth`) immer einen admin-MockContext injiziert (`#![cfg(feature = "mock_auth")]` ist Pflicht fuer alle E2E-Tests). Verifikation der Pitfall-#2-Mitigation lebt vollstaendig im Service-Layer (Plan 11.03 Mock-Test).
- **Audit-Chain-Test reduziert auf `valid: true` (REVISION-Fix W7):** Compile-Time-Grep-Gate aus Plan 11.03 (`no_audit_macros_used`) garantiert, dass kein neuer Audit-Eintrag entstehen kann — ein zusaetzlicher Runtime-Count-Delta-Check wuerde nur compile-time-bereits-abgedeckte Garantien doppeln. Statt aufwendiger `GET /api/audit/{entity_type}/{id}`-Counts und `assert_eq!(pre_count, post_count)` reicht die Endpoint-`valid: true`-Verifikation vor + nach dem Export-Call.
- **Filename-Schema-Assertion in JEDEM PDF-Erfolgs-Test (REVISION-Fix W4):** 5 PDF-Success-Tests asserten explizit das Filename-Schema (Happy-Path, Closed-Phase, Audit-Chain, alle 3 Include-Smoke-Varianten, Empty-IBAN). Ein versehentliches Refactoring der Filename-Konvention (z.B. zu `repayment-payout-...` oder Aenderung der Reihenfolge der Slugs) wird deterministisch gefangen.
- **rustfmt-Drift in 2 Stellen automatisch korrigiert:** Plan-Snippets enthielten `.get(server.url(&format!(\n  "/api/...{}/export/pdf",\n  phase.id\n)))`-Multiline-Strukturen, die nach rustfmt-Edition-2021-Regeln zu `.get(server.url(&format!("/api/...{}/export/pdf", phase.id)))` zusammengeklappt werden. `rustfmt` via Nix-Store (`/nix/store/b5snbh757b2ryz02xalqz0sqg1gqsjk7-rustfmt-preview-1.93.0-x86_64-unknown-linux-gnu/bin/rustfmt`) angewendet — Memory-Lektion "Nix-Toolchain nicht sofort aufgeben" befolgt.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 — Blocking] rustfmt-Drift in Plan-Snippets**

- **Found during:** Pre-Commit rustfmt-Check beider Tasks
- **Issue:** Plan-Text-Snippets verwenden bewusst gut-lesbare Multiline-format!-Strukturen (`format!("/api/...{}/export/pdf", phase.id)` ueber 3 Zeilen). rustfmt-Edition-2021 klappt das zu Single-Line zusammen, wenn das Resultat unter Zeilenlimit bleibt.
- **Fix:** `rustfmt --edition 2021 genossi_bin/tests/e2e_tests.rs` nach jedem Task; recheck mit `--check` ist gruen (Exit 0).
- **Files modified:** `genossi_bin/tests/e2e_tests.rs` (rein kosmetisch, kein Semantik-Change)
- **Verification:** Beide Task-Commits sind nach rustfmt-Anwendung im File-State; `rustfmt --check` Exit 0.
- **Committed in:** `67f6957` (Task 1) und `33f7e16` (Task 2)

---

**Total deviations:** 1 (Rule 3 — formatting only, kein Semantik-Change)
**Impact on plan:** Keine Scope-Aenderung. Alle 4 Requirements (EXPO-01/02/03/05) erfuellt; alle 8 Tests gruen; Plan-11.03-Grep-Gate + Pitfall-#2-Mock-Test bleiben gruen; 292 E2E-Tests insgesamt pass (vorher 284, +8 neue, 0 Regression).

## Issues Encountered

None — beide Tasks haben Build-clean und Test-gruen on first run. rustfmt-Drift war kosmetisch und im selben Task-Commit korrigiert.

## User Setup Required

None — keine externen Services, keine ENV-Variablen, keine Dashboard-Konfiguration. Tests laufen vollstaendig gegen In-Memory-SQLite + DEFAULT_TEMPLATES via `setup_with_templates()`.

## Next Phase Readiness

**Bereit fuer Phase 12** (Frontend `RepaymentEntryList` + Phase-Lifecycle-Page + Eintrag-Bearbeiten-Page):

- REST-Endpoint `GET /api/repayment-phase/{phase_id}/export/{format}` ist produktiv und durch 8 E2E-Tests abgesichert.
- Filename-Schema `auszahlung-{fy}-{include}.pdf` ist deterministisch — Frontend kann den Download-Filename per URL-Inspektion (oder Content-Disposition-Header-Parse) konsumieren ohne Server-Konfiguration zu duplizieren.
- D-06 (leere IBAN) ist E2E-verifiziert — Frontend kann den Download-Button auch fuer Phases mit unvollstaendiger Bank-Datenlage anbieten, ohne Crash-Risiko.
- D-10 (Status-Gate) ist E2E-verifiziert — Frontend muss `status != Preparation` als Voraussetzung fuer den Download-Button enforcen (oder das 409 vom Server als UX-Fehler anzeigen).
- Phase 11 ist damit vollstaendig abgeschlossen.

## Self-Check: PASSED

Verifications run after writing SUMMARY:

- [x] `genossi_bin/tests/e2e_tests.rs` modified (+554 Zeilen, 1 Helper + 8 E2E-Tests)
- [x] Commit `67f6957` (Task 1 test): FOUND in git log
- [x] Commit `33f7e16` (Task 2 test): FOUND in git log
- [x] `grep -c "fn create_member_without_iban" genossi_bin/tests/e2e_tests.rs` == 1
- [x] `grep -c "test_export_repayment_pdf_open_happy_path" genossi_bin/tests/e2e_tests.rs` == 1
- [x] `grep -c "test_export_repayment_pdf_closed_phase_returns_200" genossi_bin/tests/e2e_tests.rs` == 1
- [x] `grep -c "test_export_repayment_unknown_format_returns_400" genossi_bin/tests/e2e_tests.rs` == 1
- [x] `grep -c "test_export_repayment_preparation_phase_returns_409" genossi_bin/tests/e2e_tests.rs` == 1
- [x] `grep -c "test_export_repayment_unknown_phase_id_returns_404" genossi_bin/tests/e2e_tests.rs` == 1
- [x] `grep -c "test_export_repayment_does_not_break_audit_chain" genossi_bin/tests/e2e_tests.rs` == 1
- [x] `grep -c "test_export_repayment_include_filter_smoke_all_three_variants" genossi_bin/tests/e2e_tests.rs` == 1
- [x] `grep -c "test_export_repayment_empty_iban_renders_empty_column" genossi_bin/tests/e2e_tests.rs` == 1
- [x] `grep -c "test_export_repayment_non_admin_on_preparation_returns_403_not_409" genossi_bin/tests/e2e_tests.rs` == 0 (REVISION-Fix B2 erfuellt)
- [x] `grep -c "Müller" genossi_bin/tests/e2e_tests.rs` == 16 (REVISION-Fix W6: Plan-Acceptance >= 1)
- [x] `grep -cE "auszahlung-\{\}-[a-z]+\.pdf" genossi_bin/tests/e2e_tests.rs` == 5 (REVISION-Fix W4: Plan-Acceptance >= 3)
- [x] `grep -E "audited_(create|update|delete)!" genossi_service_impl/src/repayment_export.rs | grep -v "^[[:space:]]*//" | grep -cE "audited_create!|audited_update!|audited_delete!"` == 0 (Plan 11.03 Grep-Gate clean)
- [x] `cargo test --features mock_auth -p genossi_bin --test e2e_tests -- test_export_repayment 2>&1 | grep "test result: ok"`: 8 passed
- [x] `cargo test --features mock_auth -p genossi_service_impl --lib -- no_audit_macros_used test_non_admin_on_preparation_returns_permission_denied_not_conflict 2>&1 | grep "test result: ok"`: 3 passed (2 Grep-Gates + 1 Pitfall-#2-Mock)
- [x] `cargo test --features mock_auth -p genossi_bin --test e2e_tests 2>&1 | grep "test result: ok"`: 292 passed (vorher 284, +8 neue, 0 Regression)

## TDD Gate Compliance

Plan 11.06 ist `tdd="true"` deklariert. Wie in den Decisions oben dokumentiert, gilt die TDD-Annotation hier fuer den TDD-Cycle der GESAMTEN Phase 11:

- **RED gate (`test(...)` commits):** Plan 11.04 `ee64d88` (REST-Handler RED), Plan 11.03 `1ddd1b9` (Service-Layer-Mock-RED).
- **GREEN gate (`feat(...)` commits) nach RED:** Plan 11.04 `cf9ce76` (REST-Handler GREEN), Plan 11.03 GREEN-Commits (Service-Layer-Impl).
- **REFACTOR gate:** in Plan 11.04 explizit als skipped dokumentiert (1:1-Mirror zu Phase-6-AttendanceExport-Pattern).
- **E2E gate (Plan 11.06):** 8 `test(11-06)`-Commits zementieren das Verhalten end-to-end. KEINE neuen RED-Schritte hier, weil das Feature bereits produktionsreif ist; das ist konsistent mit dem Plan-08-10-Regression-Lock-In-Pattern (siehe Decisions oben).

Plan-11.06-Commits sind beide `test(...)`-Typen, was den TDD-Annotations-Spirit respektiert: kein neuer Production-Code, ausschliesslich Test-Hinzufuegungen. Gate-Sequence verified via `git log --oneline -3`: `67f6957` (test Task 1) → `33f7e16` (test Task 2) → (folgender docs-Commit fuer Plan-Metadata).

---
*Phase: 11-export-pdf-csv*
*Completed: 2026-06-01*
