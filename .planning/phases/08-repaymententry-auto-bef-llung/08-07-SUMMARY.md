---
phase: 08-repaymententry-auto-bef-llung
plan: 07
subsystem: api
tags: [rust, axum, sqlx, optimistic-locking, audit, bug-fix, gap-closure, regression-test]

# Dependency graph
requires:
  - phase: 08-repaymententry-auto-bef-llung
    provides: "RepaymentEntryServiceImpl + audited_update! macro (Plans 08-01..03)"
  - phase: 08-repaymententry-auto-bef-llung
    provides: "MemberServiceImpl re-read pattern at member.rs:343-348 (canonical template)"
provides:
  - "Re-Read after audited_update! in update_repayment_entry: clients receive the DAO-generated post-update version-UUID instead of the pre-update stale one"
  - "Re-Read after audited_update! per iteration in batch_toggle_status: returned Vec carries fresh per-entry version-UUIDs"
  - "Two CR-01 regression tests dokumentieren das Re-Read-Verhalten dauerhaft gegen Refactoring"
  - "Strikt-sequenzielle Mock-Verdrahtung (mockall::Sequence) als Pattern für künftige Tests mit pre-update + audit-load + Re-Read"
affects: [08-08-repayment-phase-re-read, 08-09-rest-404-409, 09-payout-cascade, 11-export, future-PUT-flows]

# Tech tracking
tech-stack:
  added: []
  patterns: ["Re-Read-after-audited_update! (canonical pattern from MemberServiceImpl)", "mockall::Sequence für strikt-sequenzielle Mock-Erwartungen im Triple-find_by_id-Pfad"]

key-files:
  modified:
    - genossi_service_impl/src/repayment_entry.rs

key-decisions:
  - "Re-Read pattern 1:1 wörtlich aus MemberServiceImpl::update (member.rs:343-348) übernommen — keine Variation"
  - "Re-Read läuft in derselben Transaction wie das audited_update! (tx.clone()), damit der Snapshot single-snapshot-konsistent ist (T-08-07-01 Mitigation)"
  - "Bei EntityNotFound vom Re-Read wird ServiceError::EntityNotFound zurückgegeben (nicht conflict_body), weil die Entry gerade in derselben Tx geupdated wurde — soft-delete in derselben Tx ist mit single-writer per Service-Methode ausgeschlossen, also ist None hier ein interner Konsistenzfehler, kein User-Race"
  - "Drei bestehende erfolgreiche Update-Tests (test_update_entry_status_open_to_contacted_succeeds, ..._contacted_to_open_succeeds, test_batch_toggle_success) wurden auf mockall::Sequence umgestellt — find_by_id liefert pre-update entity für die ersten 2 Calls (pre-load + audit-macro-load), post-update entity (neuer Status + neue Version) für den 3. Re-Read-Call"
  - "Plan-Acceptance-Criterion 'grep -c Ok(RepaymentEntry::from(&entity)) == 0' war zu strikt formuliert — gilt nur für Update/Batch-Pfade, nicht für legitime create_repayment_entry und get_repayment_entry. Effektive Compliance verifiziert via grep auf updated.push(...&entity) (== 0) und updated.push(...&refreshed) (>= 1)"

patterns-established:
  - "Re-Read-after-audited_update!: jede Service-Methode die nach audited_update! eine Entity an den Client returnt MUSS sie via find_by_id mit tx.clone() erneut lesen, sonst stale version-UUID"
  - "mockall::Sequence für audit-Pfad-Tests: pre-update find_by_id, audited_update!-internal find_by_id, DAO.update, post-update find_by_id — 4 Mock-Erwartungen in fester Reihenfolge pro Update-Operation"
  - "CR-01-Marker-Konvention: alle Re-Read-Fix-Stellen tragen den exakten String 'CR-01 Fix' im Doc-Comment für grep-basierte Verification"

requirements-completed: [ENTR-02, ENTR-06]

# Metrics
duration: 9min
completed: 2026-05-31
---

# Phase 08 Plan 07: RepaymentEntry CR-01 Re-Read Fix Summary

**Re-Read nach audited_update! in update_repayment_entry und batch_toggle_status — Clients erhalten jetzt die frische DAO-generierte version-UUID statt der stale pre-update Version, sodass realistische Edit-Flows keine 409-Endlosschleife mehr produzieren**

## Performance

- **Duration:** 9 min
- **Started:** 2026-05-31T06:19:35Z
- **Completed:** 2026-05-31T06:28:06Z
- **Tasks:** 1 (TDD: RED + GREEN + 3 Test-Anpassungen)
- **Files modified:** 1 (genossi_service_impl/src/repayment_entry.rs)

## Accomplishments
- CR-01 Bug behoben: update_repayment_entry und batch_toggle_status returnen jetzt die aktuell persistierte version-UUID, nicht die stale pre-update Version
- Re-Read-Pattern 1:1 wörtlich aus MemberServiceImpl::update (member.rs:343-348) übernommen
- 2 neue Regression-Tests verifizieren das Re-Read-Verhalten dauerhaft (test_update_repayment_entry_rereads_after_audited_update_returns_new_version + test_batch_toggle_status_rereads_each_entry_returns_new_versions)
- 3 bestehende Update-Tests auf mockall::Sequence umgestellt, damit das Mock pro Iteration sauber zwischen pre-update und post-update Entity unterscheidet
- Audit-Disziplin bleibt intakt: 0 direkte DAO-update-Calls außerhalb der audited_*!-Macros (Grep-Gate verifiziert)

## Task Commits

Each task was committed atomically (TDD-Zyklus):

1. **RED — Failing tests for CR-01 stale-version bug** — `ee44b26` (test)
2. **GREEN — Re-read after audited_update! + adapt 3 existing tests** — `2c0f503` (fix)

**Plan metadata:** to-be-committed (docs: complete plan)

## Files Created/Modified
- `genossi_service_impl/src/repayment_entry.rs` (MOD — +131 LOC -24 LOC):
  - update_repayment_entry: Re-Read-Block (Z. 265-278) nach audited_update! eingefügt, returnt RepaymentEntry::from(&refreshed)
  - batch_toggle_status: Re-Read-Block (Z. 457-468) pro Iteration nach audited_update! eingefügt, push(RepaymentEntry::from(&refreshed))
  - 2 neue Tests am Datei-Ende des mod tests-Blocks (test_update_repayment_entry_rereads_..., test_batch_toggle_status_rereads_...)
  - 3 bestehende Tests mit mockall::Sequence umgestellt (test_update_entry_status_open_to_contacted_succeeds, ..._contacted_to_open_succeeds, test_batch_toggle_success)

## Decisions Made

- Re-Read-Pattern bewusst 1:1 aus MemberServiceImpl::update übernommen — keine Optimization, keine Variation. Konsistenz mit dem etablierten Service-Pattern hat Vorrang über Mikro-Performance-Erwägungen (ein zusätzliches find_by_id pro Update ist gegenüber dem DB-Roundtrip vernachlässigbar).
- ServiceError::EntityNotFound (statt conflict_body-JSON) im Re-Read-Pfad: Wenn der Re-Read None liefert, ist das ein interner Konsistenzfehler in derselben Transaction (T-08-07-02 Mitigation), nicht ein User-Race — daher 404-ähnliche Semantik statt strukturiertem 409.
- 3 bestehende Tests auf mockall::Sequence umgestellt statt globaler Mock-Times-Erhöhung: Sequence dokumentiert die exakte Call-Reihenfolge (pre-update load → audit-macro load → update → Re-Read), was bei künftigen Refactorings (z.B. Eliminierung des audit-macro-internen find_by_id) sofort auffliegt.

## Deviations from Plan

### Plan-Inkonsistenz aufgedeckt (kein Code-Bug)

**1. [Rule 1 - Plan-Korrektur] Acceptance-Criterion `grep Ok(RepaymentEntry::from(&entity)) == 0` zu strikt**
- **Found during:** Schritt "Acceptance Criteria Verify"
- **Issue:** Das Plan-Criterion verlangt 0 Vorkommen von `Ok(RepaymentEntry::from(&entity))` im File — aber `create_repayment_entry` (Z. 166) und `get_repayment_entry` (Z. 343) verwenden dieses Pattern legitim und sind nicht von CR-01 betroffen (create generiert die Version selbst via uuid_service; get ist Read-only).
- **Fix:** Effektive Compliance des Plan-Intents verifiziert via präziseren Greps:
  - `updated.push(RepaymentEntry::from(&entity))` == 0 (batch_toggle pusht jetzt &refreshed)
  - `updated.push(RepaymentEntry::from(&refreshed))` >= 1 (batch_toggle korrekt umgestellt)
  - `find_by_id(id, tx.clone())` >= 2 (update_repayment_entry hat Pre-Load + Re-Read)
  - `find_by_id(*entry_id, tx.clone())` == 2 (batch hat Pre-Load + Re-Read pro Iteration)
- **Files modified:** None (Plan-Doc-Korrektur, keine Code-Änderung nötig)
- **Verification:** alle anderen 8/9 Acceptance-Criteria-Greps grün; einziger Miss ist die zu-strikte Plan-Formulierung
- **Committed in:** N/A (Plan-Korrektur dokumentiert hier in SUMMARY)

**2. [Rule 1 - Marker-Konvention] Initial CR-01-Marker-Kommentar matchte Plan-Grep nicht**
- **Found during:** Acceptance-Criteria-Grep "CR-01 Fix" count
- **Issue:** Erster batch_toggle-Marker lautete "CR-01 / WR-01 Fix" mit Schrägstrich-Spacing, was den Plan-Grep `CR-01 Fix` (literal) nicht matchte.
- **Fix:** Marker auf "CR-01 Fix (WR-01 same root cause)" umformuliert, sodass exakt-Match-Grep beide Stellen erkennt
- **Files modified:** genossi_service_impl/src/repayment_entry.rs (1 Zeile)
- **Verification:** `grep -nE "Re-read.*new version UUID|CR-01 Fix" ... | wc -l` == 2
- **Committed in:** Bestandteil von `2c0f503`

---

**Total deviations:** 2 (beide Plan-Konsistenz-Klarstellungen, keine Code-Korrekturen)
**Impact on plan:** Beide Deviations sind Plan-Wording-Klarstellungen, kein Scope-Creep, keine ungeplante Funktionalität. Code-Fix ist exakt das im Plan beschriebene Re-Read-Pattern.

## Issues Encountered

- **Initial test_batch_toggle_success post_entity definition vergaß den Status zu ändern**: Die ersten 3 bestehenden Update-Tests schlugen nach dem GREEN-Fix fehl, weil die Mock-Setups das `entity_for_find.clone()` für ALLE find_by_id-Calls verwendeten — der Re-Read returnte also wieder das pre-update Entity (Status Open) statt des post-update Entitys (Contacted). Behoben durch Umstellung auf mockall::Sequence mit expliziter post_entity-Definition pro Test.

- **Initial neuer test_update_repayment_entry_..._returns_new_version post_entity vergaß Status-Update**: Der `..pre_entity.clone()`-Spread erbte den Open-Status, aber die Assertion erwartete Contacted (weil das Service entity.status = Contacted vor audit setzt). Behoben durch explizites `status: RepaymentEntryStatus::Contacted` im post_entity-Constructor.

## User Setup Required

None — keine externen Services oder Konfigurationsänderungen.

## Next Phase Readiness

- Plan 08-08 (parallele Wave-1-Task) kann denselben Re-Read-Pattern auf RepaymentPhaseServiceImpl anwenden (selber Bug-Typ, selber Pattern).
- Phase-9-PayoutCascade kann sich auf konsistente version-UUIDs in RepaymentEntry-Returns verlassen.
- Frontend-Edit-Flows funktionieren ab jetzt ohne 409-Endlosschleife.

## Self-Check: PASSED

- File modified verified: `genossi_service_impl/src/repayment_entry.rs` exists
- Commit `ee44b26` (RED) exists in git log
- Commit `2c0f503` (GREEN) exists in git log
- `cargo build -p genossi_service_impl` exit 0
- `cargo test -p genossi_service_impl --lib repayment_entry` exit 0 (21/21 grün)
- `cargo test -p genossi_service_impl --lib` exit 0 (265/265 grün)
- Audit-Disziplin grep gate: 0 direkte DAO-update-Calls außerhalb audited_*!-Macros

---
*Phase: 08-repaymententry-auto-bef-llung*
*Completed: 2026-05-31*
