---
phase: 11-export-pdf-csv
plan: 03
subsystem: service-impl
tags: [service-impl, permission-funnel, read-only, audit-free, repayment-export, pdf]

# Dependency graph
requires:
  - phase: 11-export-pdf-csv
    provides: "Plan 11.01 PdfGenerator::render_repayment_list + RepaymentExportRow"
  - phase: 11-export-pdf-csv
    provides: "Plan 11.02 RepaymentExportService trait + ExportFormat + ExportInclude + RepaymentExport bundle"
  - phase: 06-attendance-export
    provides: "AttendanceExportServiceImpl-Pattern (1:1-Vorlage fuer Permission-Funnel + Mock-Tests)"
provides:
  - "RepaymentExportServiceImpl<Deps> in genossi_service_impl::repayment_export"
  - "RepaymentExportServiceDeps-Trait mit 5 Sub-Trait-Deps"
  - "check_admin_and_phase_status Permission-Funnel (D-10/D-11/Pitfall #2)"
  - "filter_and_enrich_rows Pure-Function (REVISION-Fix W1+W6: testbar ohne async/Mocks)"
  - "Server-generated filename `auszahlung-{fy}-{include}.pdf` (D-15)"
  - "tracing::info! Logging via EXPORT_TARGET=\"repayment_export\" (D-18-Pattern)"
  - "5 Service-Layer-Tests (Grep-Gate + 3 Pure-Function-Tests + 1 Pitfall-#2-Mock-Test)"
affects: [11-04-rest, 11-05-e2e, 11-06-frontend]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Permission-Funnel `load -> permission -> status` (Pitfall #2 Status-Leak-Prevention)"
    - "Pure-Function-Extraction `filter_and_enrich_rows` als pub(crate)-Helper fuer testbare Filter/Sort/Enrichment-Logik ohne Mocks"
    - "Pre-Computed-Service-Pattern (amount_str + purpose vom Service uebergeben — Renderer macht keine Lokalisierung)"
    - "Self-Reference-Grep-Gate via `include_str!` + `format!()`-Konstruktion (Audit-Macros-Negative-Assertion ohne Source-Self-Invalidation)"
    - "Defense-in-Depth-Negative-Assertion via Laufzeit-`format!()`-Konstruktion (ASCII-Variant des Verwendungszwecks erscheint NICHT als Literal im Source)"
    - "Mock-basierte Permission-Funnel-Order-Verifikation (B2/Pitfall #2 — non-admin auf Preparation -> PermissionDenied, NICHT Conflict)"

key-files:
  created:
    - "genossi_service_impl/src/repayment_export.rs (891 LOC: Deps-Trait + ServiceImpl + Permission-Funnel + filter_and_enrich_rows + export-Impl + 5 Tests)"
  modified:
    - "genossi_service_impl/src/lib.rs (+1 Zeile: pub mod repayment_export, alphabetisch zwischen repayment_entry und repayment_phase)"

key-decisions:
  - "Pure-Function `filter_and_enrich_rows` als `pub(crate)`-Helper extrahiert (REVISION-Fix W1/W6) — direkt testbar ohne `mock!`-Setup; eliminiert Async-Boilerplate in 3 von 5 Tests"
  - "TDD-RED-Stub via `todo!()` in zwei Bodies (`filter_and_enrich_rows` UND `export()` nach Permission-Funnel-Path) — 3 von 5 Tests scheitern korrekt mit `not yet implemented`, 2 Tests (Grep-Gate + B2-Funnel-Order) laufen schon im RED-Stand grün, weil sie Code-Pfade greifen, die NICHT in `todo!()` münden. Saubere, semantisch konsistente RED-Verifikation."
  - "`.abs()`-Count Acceptance-Criterion strikt = 0 wird via Comment-Filter erfüllt (`grep -c '.abs()' raw` == 4 in Doku/Assertion-Strings; Code-Pfad-Count == 0). Auf-Buchstaben-Lesart wäre nur ein Reformulierung der Plan-Kommentare nötig, semantisch identisch. Plan-Intention (kein `.abs()` in Euro-Format-Berechnung) ist erfüllt — siehe Deviation 1."
  - "MemberEntity-Felder explizit in test_member-Helper gesetzt (MemberEntity hat KEIN Default-Impl) — Plan-Text hatte `member_defaults_filler` als `todo!()`-Reminder; Auto-Fix ersetzt durch explizite Vollbelegung aller 26 Pflichtfelder (Pattern aus genossi_dao::member::tests::make_entity)."

patterns-established:
  - "REVISION-Fix-Pattern-Stack: B1 (Source-Literal-Free Negative-Assertion via Laufzeit-`format!()`) + B2 (Mock-Test mit konkretem Phase-DAO + Permission-Service-Setup für Funnel-Order-Verifikation) + B3 (Phase-10-D-04-Pattern OHNE `.abs()`) + W1 (Include-Filter-Counts direkt asserten) + W6 (D-04 Verwendungszweck wortwörtlich mit ORIGINAL-Umlaut)"
  - "Pitfall #8 (Tx.commit VOR sync Render-Call) konsequent eingehalten — Pattern für alle künftigen Service-Methoden, die nach Tx-Read eine sync-Rendering-Stufe haben"
  - "Self-Reference-Grep-Gate-Pattern aus attendance_export.rs:1167-1198 1:1 übernommen für EXPO-05/D-11 Audit-Macro-Verbot"

requirements-completed: [EXPO-01, EXPO-02, EXPO-03, EXPO-05]

# Metrics
duration: 8min
completed: 2026-06-01
---

# Phase 11 Plan 03: RepaymentExportServiceImpl Summary

**`RepaymentExportServiceImpl<Deps>` mit Permission-Funnel `load -> admin -> status` (D-10/D-11/Pitfall #2), N+1-DAO-Read-Pipeline in einer Tx, In-Memory-Include-Filter (D-01/D-02), stabile Sortierung (D-09), Verwendungszweck-Pre-Computing mit ORIGINAL-Umlaut `Anteilsrückzahlung` (D-04/D-05), Euro-Format-Pre-Computing OHNE `.abs()` (REVISION-Fix B3), `tx.commit()` VOR PdfGenerator-Render (Pitfall #8), und 5 Service-Layer-Tests (Grep-Gate + B1/W6 + W1 + B3 + B2/Pitfall #2 Mock).**

## Performance

- **Duration:** 8 min
- **Started:** 2026-06-01T05:23:42Z
- **Completed:** 2026-06-01T05:32:11Z
- **Tasks:** 1 (mit TDD-RED → GREEN-Cycle)
- **Files modified:** 2 (1 created, 1 modified)
- **Tests added:** 5 (`no_audit_macros_used`, `test_purpose_string_preserves_umlaut_per_d04`, `test_include_filter_row_counts`, `test_amount_str_uses_phase_10_d04_pattern_without_abs`, `test_non_admin_on_preparation_returns_permission_denied_not_conflict`)
- **Suite size:** 283 tests pass im `genossi_service_impl --lib` (vorher 278, +5 neue, 0 Regression)

## Accomplishments

- **`RepaymentExportServiceImpl<Deps>`** mit `RepaymentExportServiceDeps`-Trait (5 Sub-Deps: `RepaymentPhaseDao`, `RepaymentEntryDao`, `MemberDao`, `PermissionService`, `TransactionDao`) und 7 Arc-wrapped Feldern (`transaction_dao`, `permission_service`, drei DAOs, `pdf_generator`, `template_base`). 1:1-Pattern-Mirror von `AttendanceExportServiceImpl` aus Phase 6.
- **`check_admin_and_phase_status`** Permission-Funnel implementiert die D-10/D-11-Reihenfolge `load_by_id (404) → check_permission("admin") (403) → match status (409)` mit `Authentication::Full`-Short-Circuit. D-10: `Open | Closed` akzeptiert; `Preparation` → `ServiceError::Conflict("phase_not_exportable")` → REST 409.
- **`filter_and_enrich_rows`** (`pub(crate)` Pure-Function) implementiert In-Memory-Filter (D-01/D-02 mit `retain`), stabile Sortierung (`member_number ASC, created ASC` per `then_with`), und Pre-Computing der 6 RepaymentExportRow-Felder (`member_number`, `name`, `iban` mit `unwrap_or_default()` für D-06/D-07, `share_count`, `amount_str` Phase-10-D-04-Format OHNE `.abs()`, `purpose` mit ORIGINAL-Umlaut `ü` für D-04/D-05).
- **`export()`** öffnet Tx, ruft Permission-Funnel, liest Entries via `find_by_phase_id`, liest Member pro Entry via `find_by_id` (N+1 — RESEARCH-Q5-Discretion), committed Tx VOR `pdf_generator.render_repayment_list` (Pitfall #8), filtert/sortiert/anreichert via `filter_and_enrich_rows`, emittiert `tracing::info!(target="repayment_export", ...)` (D-18), und liefert `RepaymentExport` mit Server-generiertem Filename `auszahlung-{fiscal_year}-{include}.pdf`.
- **5 Service-Layer-Tests** verankern alle Plan-Verifikationen: (1) `no_audit_macros_used` Grep-Gate via `include_str!` für EXPO-05; (2) `test_purpose_string_preserves_umlaut_per_d04` für D-04/D-05/W6/B1 (positive Umlaut-Assertion + Laufzeit-konstruierte Negative-Assertion ohne Source-Literal-Match); (3) `test_include_filter_row_counts` für D-01/D-02/W1 (Open=3/All=4/Paid=1 Counts plus Sort-Verifikation); (4) `test_amount_str_uses_phase_10_d04_pattern_without_abs` für REVISION-Fix B3 (`"120,00"` für 12000-Cent share_value × 1 Anteil); (5) `test_non_admin_on_preparation_returns_permission_denied_not_conflict` für B2/Pitfall #2 (konkretes Mock-Setup: Phase-DAO liefert Preparation, PermissionService liefert PermissionDenied → erwartet `PermissionDenied`, NICHT `Conflict`).

## Task Commits

1. **Task 1 RED:** `27cc1bc` (test)
   - Neue Datei `repayment_export.rs` mit voller Test-Infrastruktur (mock!-Blocks für 5 DAOs/Services, TestTransaction, TestContext, TestDeps, build_service, tx_dao_no_commit, test_member/test_entry/test_phase Helper).
   - 5 Tests definiert; `filter_and_enrich_rows` und `export()` (nach Permission-Funnel) als `todo!()`-Stubs.
   - lib.rs ergänzt um `pub mod repayment_export;`.
   - Verifizierter RED-State: 3 Tests panic mit `not yet implemented: filter_and_enrich_rows - GREEN`; 2 Tests (Grep-Gate + B2-Funnel-Order) laufen schon grün, weil sie Code-Pfade greifen, die NICHT in `todo!()` münden — semantisch korrekter RED ohne Runtime-Panic-Noise auf Tests, die unabhängig vom Stub-Body sind.

2. **Task 1 GREEN:** `13f8424` (feat)
   - `filter_and_enrich_rows` voll implementiert (D-01/D-02/D-04/D-05/D-06/D-07/D-09 + REVISION-Fix B3).
   - `export()` voll implementiert (Tx-Lifecycle, N+1-Read, Pitfall #8 commit-vor-Render, tracing, Filename-Schema).
   - Alle 5 Tests grün; `cargo test -p genossi_service_impl --lib`: 283/283 OK (vorher 278, +5 neue, 0 Regression).

**REFACTOR phase:** Skipped — Implementation spiegelt `attendance_export.rs`-Pattern 1:1, kein offensichtlicher Cleanup nötig.

**Plan metadata commit:** folgt nach diesem SUMMARY (docs-Commit mit `.planning/STATE.md` + `.planning/ROADMAP.md`-Updates).

## Files Created/Modified

- `genossi_service_impl/src/repayment_export.rs` (CREATED, 891 LOC) — Deps-Trait + ServiceImpl-Struct + Permission-Funnel + `filter_and_enrich_rows` Pure-Function + Trait-Impl + 5 Tests (1 sync Grep-Gate + 3 sync Pure-Function + 1 `#[tokio::test]` Mock-Test) + `mock!`-Blocks für `TestTxDao`/`TestPhaseDao`/`TestEntryDao`/`TestMemberDao`/`TestPermissionService` (alle Trait-Methoden vollständig gelistet, weil `mockall::mock!` Vollständigkeit verlangt).
- `genossi_service_impl/src/lib.rs` (MODIFIED, +1 Zeile) — `pub mod repayment_export;` alphabetisch zwischen `pub mod repayment_entry;` und `pub mod repayment_phase;`.

## Decisions Made

- **Pure-Function-Extraktion `filter_and_enrich_rows`** (REVISION-Fix W1/W6): 3 von 5 Tests (Filter-Counts, Umlaut-Erhalt, Euro-Format) lassen sich ohne `mock!`-Setup direkt gegen die Pure-Function asserten. Spart pro Test ~30 LOC `mock!`-Boilerplate und macht das Filter/Sort/Enrichment-Verhalten einzeln verifizierbar.
- **TDD-RED-Form via 2 `todo!()`-Stubs** (Plan 11.01-Pattern für Funktions-Bodies wiederverwendet): `filter_and_enrich_rows` und `export()` (nach `check_admin_and_phase_status`) sind im RED-Stand `todo!()`. Der B2-Funnel-Order-Test (Pitfall #2) trifft Permission-Denied BEVOR `todo!()` und kann daher schon im RED grün laufen — das ist semantisch korrekt: Der Test verifiziert die Funnel-Order, NICHT die GREEN-Logik dahinter.
- **`.abs()`-Acceptance-Criterion** strikte Lesart (`grep -c == 0`) **wäre** durch Kommentar-Umformulierung erreichbar, semantisch ist aber das Code-Pfad-Verhalten (kein `.abs()` in der Euro-Format-Berechnung) das Ziel. Code-Pfad-`.abs()`-Count = 0 verifiziert via `grep '\.abs()' | grep -v '//' | grep -v '"' | wc -l`. Plan-Intention (Phase-10-D-04-Konsistenz, PATTERNS.md S9) ist erfüllt. Siehe Deviation 1.
- **`test_member`-Helper explizit voll belegt** (26 Pflichtfelder): MemberEntity hat KEIN Default-Impl; Plan-Text hatte einen `member_defaults_filler`-`todo!()`-Reminder. Auto-Fix nutzt die Vollbelegung-Pattern aus `genossi_dao::member::tests::make_entity` (`shares_at_joining=1`, `current_shares=5`, `bank_account=Some("DE89..."`, `status=Normal`, `migrated=false`, `join_date=2026-01-01`, etc.). Kein `todo!()` mehr in Test-Helpers.
- **`mock!`-Block-Vollständigkeit**: Alle 9 MemberDao-Methoden, 6 RepaymentEntryDao-Methoden, 5 RepaymentPhaseDao-Methoden, 21 PermissionService-Methoden, 3 TransactionDao-Methoden explizit gelistet. `mockall::mock!` verlangt vollständige Trait-Impl-Liste; Auslassen führt zu E0046 Compile-Error.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 — Acceptance-Criterion-Lesart] `.abs()`-Count im Source-File ist 4 (in Kommentaren/Assertion-Strings), nicht 0**

- **Found during:** Acceptance-Criteria-Grep-Verifikation nach GREEN-Commit
- **Issue:** Acceptance-Criterion sagt `grep -c '\.abs()' genossi_service_impl/src/repayment_export.rs == 0`. Buchstabengetreue Lesart wäre verletzt: 4 Treffer in (a) `//!`-Modul-Doc-Kommentar, (b) `//`-Code-Kommentar in `filter_and_enrich_rows`, (c) `//`-Test-Kommentar, (d) Assertion-String `"no `.abs()`, no leading zeros"`.
- **Semantik-Check:** Plan-Verification-Section sagt "REVISION-Fix B3: Euro-Format OHNE `.abs()` (Phase-10-D-04-Konsistenz)". Code-Pfad: `let amount_str = format!("{},{:02}", amount_cents / 100, amount_cents % 100);` — KEIN `.abs()`-Call. Verifiziert via `grep '\.abs()' | grep -v '//' | grep -v '"' | wc -l` == 0.
- **Disposition:** Plan-Intention erfüllt; Buchstabengetreue Lesart durch das Self-Reference-Phänomen (gleich wie beim Audit-Macros-Grep-Gate) unrealistisch. Kommentare/Assertion-Strings, die das anti-Pattern erwähnen, dürfen das Acceptance-Criterion nicht invalidieren.
- **Files modified:** None — keine Code-Änderung nötig.
- **Verification:** Code-Pfad-Grep == 0 (oben).
- **Decision:** Plan-Acceptance-Criterion-Lesart wird in zukünftigen Plan-Erstellungen mit Self-Reference-Filter-Klausel formuliert (Pattern aus Audit-Macros-Grep-Gate): `grep '\.abs()' file | grep -v '//' | grep -v '"' | wc -l == 0`. Dieser Plan wurde nicht aktualisiert, weil eine Plan-Modifikation post-execution den Audit-Trail trübt.

---

**Total deviations:** 1 (Rule 2 — Acceptance-Criterion-Lesart, keine Code-Änderung, Plan-Intention erfüllt)
**Impact on plan:** Keine Scope-Änderung. Alle 4 Requirements (EXPO-01..03+05) erfüllt; alle 5 Tests grün; Plan-Intention (Phase-10-D-04-Konsistenz ohne `.abs()` im Code-Pfad) verifiziert.

## Issues Encountered

None — RED- und GREEN-Build kompilieren beide ohne Errors; alle 5 Tests laufen wie erwartet (RED: 3 fail / 2 pass; GREEN: 5 pass); volle Crate-Test-Suite 283/283 OK ohne Regression in den 278 bestehenden Tests; alle Acceptance-Criteria-Greps verifiziert (außer Deviation 1 oben).

## User Setup Required

None — keine externen Services, keine ENV-Variablen, keine Dashboard-Konfiguration.

## Next Phase Readiness

**Bereit für Plan 11.04** (REST-Endpoint `/api/repayment-phase/{id}/export`):

- `RepaymentExportServiceImpl<Deps>` ist `pub` und über `genossi_service_impl::repayment_export::{RepaymentExportServiceImpl, RepaymentExportServiceDeps}` importierbar.
- Permission-Funnel garantiert, dass REST-Handler nur den Service-Aufruf machen muss (kein zweiter Admin-Check auf REST-Ebene nötig — analog `attendance_export`-Pattern).
- Service-Bundle `RepaymentExport { bytes, content_type, filename }` ist die fertige Service→REST-Schnittstelle; REST-Handler kann es 1:1 als HTTP-Response durchreichen.
- `ServiceError`-Mapping (PermissionDenied → 401, Conflict → 409, EntityNotFound → 404, ValidationError → 400) ist über das globale `From<ServiceError>`-Mapping in `genossi_rest/src/lib.rs:97-113` abgedeckt — Plan 11.04 braucht ggf. einen lokalen `map_export_error` für 403-Permission (analog Phase-3-Plan-06-Pattern), wenn das Anti-Status-Leak-Verhalten 401→403 mapped werden soll.

**Bereit für Plan 11.06** (Frontend): `mock_auth`-E2E-Tests können `RepaymentExportServiceImpl` ohne externe Dependencies aufrufen (in-memory SQLite + mock admin context).

Keine Blocker.

## Self-Check: PASSED

Verifications run after writing SUMMARY:

- [x] `genossi_service_impl/src/repayment_export.rs` exists (891 LOC, 1 file)
- [x] `genossi_service_impl/src/lib.rs` modified (new `pub mod repayment_export;` entry)
- [x] Commit `27cc1bc` exists (Task 1 RED: test — failing tests + `todo!()` stubs)
- [x] Commit `13f8424` exists (Task 1 GREEN: feat — full impl, 5 tests pass)
- [x] `cargo build -p genossi_service_impl 2>&1 | grep -c "^error"` == 0
- [x] `cargo test -p genossi_service_impl --lib repayment_export 2>&1 | grep "test result: ok"` == 1 (5 passed)
- [x] `cargo test -p genossi_service_impl --lib`: 283/283 OK, 0 failures
- [x] `grep -cE "pub struct RepaymentExportServiceImpl"` == 1
- [x] `grep -cE "async fn check_admin_and_phase_status"` == 1
- [x] `grep -cE "fn filter_and_enrich_rows"` == 1
- [x] `grep -cE "phase_not_exportable"` >= 1 (== 4: 1 in Conflict-Body, 3 in Test/Assertion-Strings)
- [x] `grep -c "Anteilsrückzahlung"` >= 2 (== 2: format!-Call im filter_and_enrich_rows + Test-Assertion)
- [x] `grep -c "Anteilsrueckzahlung"` == 0 (B1: ASCII-Variant nur zur Laufzeit via `format!("Anteilsr{}ckzahlung", "ue")` konstruiert)
- [x] `grep -cE "auszahlung-\\{\\}-\\{\\}\\.pdf"` >= 1 (== 1)
- [x] `grep -cE "self\\.transaction_dao\\.commit"` >= 1 (== 1, Pitfall #8)
- [x] `grep -cE "render_repayment_list"` >= 1 (== 1)
- [x] `grep '\.abs()' | grep -v '//' | grep -v '"' | wc -l` == 0 (Code-Pfad-`.abs()`-Count; Plan-Lesart `grep -c '\.abs()' == 0` siehe Deviation 1)
- [x] Grep-Gate: `grep -E "audited_(create|update|delete)!" | grep -v '^[[:space:]]*//' | grep -v 'format!' | wc -l` == 0
- [x] `grep -cE "^pub mod repayment_export"` in lib.rs == 1
- [x] `grep -c "matches!(result, Err(ServiceError::PermissionDenied))"` >= 1 (== 1, B2-Assertion)
- [x] `grep -c "Funnel-Order is broken"` >= 1 (== 1, B2-Assertion-Message)
- [x] `grep -c "Status-Leak via 409 detected"` >= 1 (== 1, B2-Assertion-Message)
- [x] `cargo test test_non_admin_on_preparation_returns_permission_denied_not_conflict | grep "test result: ok. 1 passed"` >= 1 — strikter Check erfüllt (genau 1 Test passed, nicht 0)

## TDD Gate Compliance

- **RED gate (`test(...)` commit):** `27cc1bc` — fügt failing tests + zwei `todo!()`-Stubs (`filter_and_enrich_rows`, `export()` nach Permission-Funnel). Tests scheitern mit `not yet implemented`.
- **GREEN gate (`feat(...)` commit) nach RED:** `13f8424` — voll implementiert. Alle 5 Tests grün; volle Suite 283/283.
- **REFACTOR gate:** skipped — Implementation spiegelt `attendance_export.rs`-Pattern 1:1; kein Cleanup nötig.

Gate-Sequence verified via `git log --oneline -5`.

---
*Phase: 11-export-pdf-csv*
*Completed: 2026-06-01*
