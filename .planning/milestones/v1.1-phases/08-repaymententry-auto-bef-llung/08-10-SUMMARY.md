---
phase: 08-repaymententry-auto-bef-llung
plan: 10
subsystem: testing
tags: [e2e, regression, optimistic-locking, http-404, http-409, repayment-entry, repayment-phase, audit-coverage]

requires:
  - phase: 08-repaymententry-auto-bef-llung
    provides: "Re-Read-Pattern in RepaymentEntryService (08-07), Re-Read-Pattern in RepaymentPhaseService (08-08), NotFound-Mapping in batch_toggle_status (08-09)"
provides:
  - "5 E2E-Regressionstests am Datei-Ende von genossi_bin/tests/e2e_tests.rs"
  - "End-to-end verifizierter Optimistic-Locking-Vertrag (Response-version ist für Folge-PUT verwendbar)"
  - "End-to-end verifizierter Aggregat-Konsistenz bei NotFound (batch_toggle liefert 404 wie get/update/delete)"
  - "Test-Coverage-Lücke IN-04 dauerhaft geschlossen"
affects: [09-payout-cascade, 10-mass-mail, 11-pdf-csv-export, 12-frontend, künftige RepaymentEntry/RepaymentPhase-Erweiterungen]

tech-stack:
  added: []
  patterns:
    - "Regression-Lock-In via E2E-Test: Bug-Fix wird mit explizit gestaltetem Test gegen Rückfall geschützt (extrahiert version aus 1. PUT-Response, nutzt sie im 2. PUT, asserted 200 statt 409)"
    - "Aggregat-Konsistenz-Test: Stale/Fake-ID in Batch-Endpoint produziert dieselbe HTTP-Semantik (404) wie Single-Resource-Endpoints im gleichen Aggregat"

key-files:
  created: []
  modified:
    - "genossi_bin/tests/e2e_tests.rs (+281 LOC: 5 neue E2E-Regression-Tests am Datei-Ende)"

key-decisions:
  - "5 Regression-Tests werden als ein einzelner `test`-Commit additiv hinzugefügt (NICHT TDD-RED→GREEN-Sequenz separat) — Fix war bereits in 08-07/08/09 deployed; die Tests sollen das gefixte Verhalten zementieren, nicht erst einen Fix erzwingen"
  - "CR-01-Regression-Assertion: explizite assert_ne!(version_after_put, create_version) verifiziert, dass der DAO tatsächlich eine neue UUID generiert hat (NICHT nur, dass die Service-Schicht die alte zurückliefert wäre)"
  - "Test 5 (CR-02) sendet `entry_ids=[real, fake]` statt nur fake — sicherstellt, dass auch im Loop-Body (nicht nur als First-Lookup-Fail) das NotFound-Mapping greift"
  - "Phase-Update-Bodies in Test 3+4 werden via `serde_json::json!` gebaut (konsistent mit Phase-7-Lifecycle-Test Z. 10645-10649), nicht via UpdateRepaymentPhaseRequest-Import"

patterns-established:
  - "E2E-Regression-Lock-In-Pattern: 1. Operation → version extrahieren → 2. Operation mit dieser version → asserted Success. Anwendbar auf jede Optimistic-Locking-Schicht."
  - "Mixed-Validity-Batch-Pattern: real+fake IDs im Array verifizieren, dass Aggregat-Konsistenz auch unter Tx-Atomarität (D-08 Rollback) erhalten bleibt"

requirements-completed: [ENTR-02, ENTR-06, PHAS-02, PHAS-03]

duration: 5min
completed: 2026-05-31
---

# Phase 8 Plan 10: E2E-Regressionstests für CR-01/CR-02 Summary

**5 E2E-Tests in genossi_bin/tests/e2e_tests.rs (281 LOC) zementieren die 08-07/08/09-Fixes gegen zukünftige Rückfälle und schließen IN-04 (Test-Coverage-Lücke fürs 2nd-PUT-mit-Response-version-Szenario)**

## Performance

- **Duration:** 5 min
- **Started:** 2026-05-31T06:53:31Z
- **Completed:** 2026-05-31T06:58:30Z
- **Tasks:** 1 (5 Sub-Tests am Datei-Ende)
- **Files modified:** 1

## Accomplishments

- 5 neue E2E-Tests am Datei-Ende von `genossi_bin/tests/e2e_tests.rs` (NACH `test_audit_chain_intact_after_phase_8_lifecycle`):
  - **Test 1** `test_update_entry_followup_put_uses_response_version_returns_200` — CR-01 RepaymentEntry: 1. PUT extrahiert version, 2. PUT mit dieser version → 200, asserted Status zurück auf Open
  - **Test 2** `test_batch_toggle_followup_put_uses_response_versions` — CR-01/WR-01 batch_toggle: extrahiert updated[0].version, Einzel-PUT mit dieser version → 200
  - **Test 3** `test_open_phase_response_version_usable_for_followup_update` — CR-01 RepaymentPhase::open + D-04: extrahiert opened.version, PUT update_phase mit share_value-Korrektur → 200
  - **Test 4** `test_update_phase_response_version_usable_for_followup_update` — CR-01 RepaymentPhase::update: 1. PUT extrahiert version, 2. PUT mit dieser version → 200
  - **Test 5** `test_batch_toggle_with_unknown_entry_id_returns_404` — CR-02: entry_ids=[real, fake] → 404 (NICHT 409)
- **275/275 E2E-Tests grün** (270 baseline = 255 Phase-7 + 15 Phase-8 Plan-06 + 5 neu)
- IN-04 Test-Coverage-Lücke geschlossen — zukünftige Re-Read-Pattern- oder NotFound-Mapping-Regressionen werden sofort gefangen
- Optimistic-Locking-Vertrag und Aggregat-Konsistenz sind jetzt end-to-end positiv abgesichert (vorher nur indirekt via Service-Unit-Tests)

## Task Commits

Atomar als 1 Commit (Regression-Lock-In, kein neuer Production-Code):

1. **Task 1: 5 E2E-Regressionstests** — `0262b63` (test)

**Plan metadata commit (final):** wird nach SUMMARY/STATE/ROADMAP/REQUIREMENTS-Updates erstellt.

## Files Created/Modified

- `genossi_bin/tests/e2e_tests.rs` (MOD +281 LOC) — 5 neue E2E-Tests am Datei-Ende mit Inline-Doc-Kommentaren, die CR-01/CR-02-Referenz und Pre-Fix-Verhalten erklären

## Decisions Made

- **Kein TDD-RED-Schritt:** Die Fixes (08-07/08/09) sind bereits im `phase-08-gap-closure`-Branch committed. Die 5 Tests sind als Regression-Lock-In gedacht, NICHT als Trigger für einen neuen Fix. Ein RED-Schritt würde künstlich den Branch in einen instabilen Zustand bringen, ohne neuen Erkenntnisgewinn. Stattdessen verifizieren die Tests von Anfang an positiv das gefixte Verhalten und sind in der Commit-Nachricht klar als Regression-Tests gelabelt.
- **`serde_json::json!` statt UpdateRepaymentPhaseRequest-Import in Test 3+4:** Konsistent mit dem Phase-7-Lifecycle-Test (e2e_tests.rs Z. 10645-10649), der bereits diese Form benutzt. Vermeidet zusätzlichen Import nur für 2 Tests.
- **Mixed-Validity-Batch in Test 5 (real+fake) statt nur fake:** Verifiziert, dass die NotFound-Erkennung im Loop-Body greift (nicht nur als initialer Validation-Pass). Robuster gegen künftige Refactorings, die den Validation-Pfad ändern.
- **Test-Nummerierung:** Tests sind durchnummeriert in der Reihenfolge (Update Entry → Batch Toggle Entry → Open Phase → Update Phase → Batch 404), passend zur 4× CR-01 + 1× CR-02 Verteilung aus dem Plan.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- **`cargo test`-Filter-Einschränkung:** `cargo test` akzeptiert nur einen `[TESTNAME]`-Filter pro Aufruf, nicht mehrere. Workaround: 5 einzelne `cargo test`-Aufrufe für den Smoke-Test der neuen Test-Namen (alle 5 zeigten `274 filtered out` + `1 passed`, was die Gesamtsumme von 275 Tests bestätigt). Anschließend ein finaler Full-Run mit `cargo test --test e2e_tests --features mock_auth` zur Regression-Verifikation (275/275 grün).

## Acceptance-Criteria Verification

| Criterion | Result |
|-----------|--------|
| `grep -c "test_update_entry_followup_put_uses_response_version_returns_200"` == 1 | ✓ 1 |
| `grep -c "test_batch_toggle_followup_put_uses_response_versions"` == 1 | ✓ 1 |
| `grep -c "test_open_phase_response_version_usable_for_followup_update"` == 1 | ✓ 1 |
| `grep -c "test_update_phase_response_version_usable_for_followup_update"` == 1 | ✓ 1 |
| `grep -c "test_batch_toggle_with_unknown_entry_id_returns_404"` == 1 | ✓ 1 |
| `grep -c "StatusCode::NOT_FOUND"` >= 2 | ✓ 22 (gesamtes File) |
| `grep -nE "version from .*response must succeed.*CR-01 regression" \| wc -l` >= 3 | ✓ 3 |
| `cargo build --tests -p genossi_bin --features mock_auth` exit 0 | ✓ Finished in 50.09s |
| `cargo test --test e2e_tests --features mock_auth` exit 0, ≥ 275 grün, 0 failed | ✓ 275 passed; 0 failed |
| 5 neue Tests einzeln grün (Smoke-Check per name-filter) | ✓ Alle 5 einzeln verifiziert |

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

**Phase 8 Gap-Closure ist vollständig abgeschlossen.** Alle 3 BLOCKER-Bugs aus dem Code-Review (CR-01 RepaymentEntry, CR-01 RepaymentPhase, CR-02 batch_toggle 404 vs 409) sind:

1. Im Production-Code gefixt (08-07, 08-08, 08-09)
2. Mit Unit-Tests im Service-Layer dauerhaft abgesichert (08-07: 2 neue Tests, 08-08: 2 neue Tests, 08-09: 1 neuer Test)
3. Mit E2E-Tests im REST-Layer dauerhaft abgesichert (08-10: 5 neue Tests)
4. In OpenAPI dokumentiert (08-09: 404-Response in batch_toggle_status)

Nächster Schritt: `/gsd-verify-phase 08` zur Final-Verifikation mit allen 4 Gap-Closures (08-07..08-10), oder direkt Phase 9 (PayoutCascade) starten.

## Self-Check: PASSED

- File `genossi_bin/tests/e2e_tests.rs` exists with 5 new tests at end (line 11668+)
- Commit `0262b63` exists in git log (verified via `git rev-parse --short HEAD`)
- All 5 test names verified via grep (count == 1 each)
- `cargo test` shows 275 passed, 0 failed

---
*Phase: 08-repaymententry-auto-bef-llung*
*Completed: 2026-05-31*
