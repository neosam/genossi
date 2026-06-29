---
phase: 16-service-rest-teil-rueckgabe-auto-anlegen-phase
plan: 03
subsystem: api
tags: [rust, axum, sqlx, audit-log, repayment-phase, skip-pattern, mockall, tdd]

# Dependency graph
requires:
  - phase: 14-dao-domain-foundation
    provides: "RepaymentEntryDao::find_by_member_and_phase (trait + SQLite-Impl)"
  - phase: 07-repaymentphase-backend-foundation
    provides: "RepaymentPhaseService + open_repayment_phase auto-fill loop (v1.1)"
provides:
  - "Auto-Fill-Skip-Pattern: open_repayment_phase überspringt Members mit existierendem RepaymentEntry"
  - "Per-Member find_by_member_and_phase Existenz-Check VOR audited_create!"
  - "Mock-Erweiterung MockTestRepaymentEntryDao um find_by_member_and_phase"
  - "Unit-Test verifiziert dass create() nur für nicht-skip Members aufgerufen wird"
affects: [16-02-partial-repayment, 17-transfer-shares, 18-frontend-partial-repayment]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Skip-Pattern in Auto-Fill-Loop (Per-Member existence-check vor audited_create!)"
    - "Mockall .withf() für per-member-ID branching im selben Mock-Call"

key-files:
  created: []
  modified:
    - "genossi_service_impl/src/repayment_phase.rs"

key-decisions:
  - "Skip-Filter: ANY status (Open/Contacted/PaidOut) — nicht nur Open — auch PaidOut blockt zweiten Open-Entry (D-16-03)"
  - "Per-Member-Lookup statt Bulk-Prefetch (D-16-03; realistic <200 Members)"
  - "tx.clone() für Skip-Check (Tx-Sharing mit umgebendem audited_update!(Phase))"
  - "Existing DAO-Methode find_by_member_and_phase (Phase 14) wiederverwendet — keine neue DAO-Methode"

patterns-established:
  - "Auto-Fill-Skip-Pattern: pro Iteration find_by_member_and_phase, continue bei !is_empty()"

requirements-completed: [PART-04]

# Metrics
duration: 7min
completed: 2026-06-05
---

# Phase 16 Plan 03: Auto-Fill-Skip-Pattern für open_repayment_phase Summary

**find_by_member_and_phase-Skip-Check in open_repayment_phase Auto-Fill-Loop verhindert Duplikat-Entries wenn v1.2-partial_repayment vorher einen RepaymentEntry in derselben Phase erzeugt hat**

## Performance

- **Duration:** ~7 min
- **Started:** 2026-06-05T07:30:00Z (approx.)
- **Completed:** 2026-06-05T07:42:00Z (approx.)
- **Tasks:** 2
- **Files modified:** 1

## Accomplishments

- Skip-Pattern als erste Aktion im `for member in targets`-Loop in `open_repayment_phase` (genossi_service_impl/src/repayment_phase.rs)
- Per-Member `find_by_member_and_phase`-Aufruf; bei non-empty `continue;` — kein zweiter Entry für denselben Member in derselben Phase
- Inline-Kommentar referenziert D-16-03, PART-04, PITFALLS-Kat-1 (alle drei Anchor-IDs)
- MockTestRepaymentEntryDao um `find_by_member_and_phase` erweitert (Mockall-Override-Pflicht)
- 2 existing Auto-Fill-Tests (creates_entries_for_matching_members, atomic_on_dao_failure) mit `.expect_find_by_member_and_phase()` returning empty Arc nachgezogen
- Neuer Unit-Test `test_open_repayment_phase_skips_members_with_existing_entry` verifiziert per `.times(1)` auf `expect_create()`, dass nur der nicht-skip Member ein Entry bekommt
- 27 repayment_phase-Tests passen (vorher 26; +1 neu, 0 Regressionen)

## Task Commits

Each task was committed atomically (via jj, co-located mit git):

1. **Task 1: Insert skip-pattern into open_repayment_phase auto-fill loop** — `0022fec` (feat)
2. **Task 2: Add unit test verifying the skip-pattern** — `cee23b8` (test)

## Files Created/Modified

- `genossi_service_impl/src/repayment_phase.rs` — Skip-Pattern (Z. 369-396), Mock-Erweiterung (Z. 768-775), 2 angepasste Tests, 1 neuer Test (`test_open_repayment_phase_skips_members_with_existing_entry`)

## Decisions Made

Alle Entscheidungen waren bereits im CONTEXT als Locked Decisions vorgegeben (D-16-03, PART-04, PITFALLS-Kat-1). Plan-Execution folgte den vorgegebenen Pfaden:

- Skip-Filter: **ANY status** (Open/Contacted/PaidOut) — wie im Plan spezifiziert.
- Per-Member-Lookup statt Bulk-Prefetch — wie D-16-03 vorgibt.
- `tx.clone()` für Tx-Sharing mit dem umgebenden `audited_update!(Phase)`-Block — konsistent mit existing `audited_create!`-Pattern im selben Loop.
- Keine neue DAO-Methode — die existierende `find_by_member_and_phase` (Phase 14) wird verwendet.

## Deviations from Plan

Plan executed exactly as written. Eine kleine Anpassung des Kommentartextes:

### Auto-fixed Issues

**1. [Rule 1 - Acceptance-Criterion-Fix] Kommentar-Referenz auf `find_by_member_and_phase` umformuliert**
- **Found during:** Task 1 (post-edit Acceptance-Check)
- **Issue:** Plan-Acceptance-Criterion verlangt `grep -c "find_by_member_and_phase"` im `for member in targets`-Block soll genau `1` ergeben (1 Skip-Call). Der ursprüngliche Inline-Kommentar referenzierte den Methodennamen ein zweites Mal in der Foundation-Begründung, was die Grep-Count auf 2 trieb.
- **Fix:** Comment-Text zu "die Lookup-Methode existiert seit Phase 14..." umgeschrieben — semantisch identisch, methodename nur einmal (im echten Call).
- **Files modified:** genossi_service_impl/src/repayment_phase.rs (Inline-Comment vor Skip-Call)
- **Verification:** `awk '/for member in targets/,/^        \}/' ... | grep -c "find_by_member_and_phase"` → 1 (war: 2)
- **Committed in:** 0022fec (part of Task 1 commit)

---

**Total deviations:** 1 auto-fixed (Cosmetic Comment-Adjustment zur Acceptance-Compliance)
**Impact on plan:** Keine semantische Änderung — Skip-Pattern, Behavior, Tests alle identisch zum Plan-Wortlaut. Nur Comment-Wording.

## Issues Encountered

- **Mockall + Default-Impl-Override:** Wie in `genossi_dao/src/repayment_entry.rs:159-161` notiert, überschreibt `#[automock]` Default-Impls. Die existing Auto-Fill-Tests (creates_entries_for_matching_members, atomic_on_dao_failure) liefen nach dem Einfügen des Skip-Checks ins Panic `MockTestRepaymentEntryDao::find_by_member_and_phase: No matching expectation found`. Lösung: `find_by_member_and_phase` zum `mock!`-Block hinzugefügt und in beiden Tests `.expect_find_by_member_and_phase().returning(|_, _, _| Ok(Arc::from(vec![])))` ergänzt (genau die im Plan-action vorgeschlagene Variante a). Alle 27 Tests grün.
- **Worktree-Setup-Verständnis:** Der Worktree ist als jj-workspace co-located mit git aufgesetzt (kein klassischer git-worktree). `git status` aus dem Worktree-CWD zeigt unrelated Plan-16-01-Modifikationen aus dem main-tree (von einem parallelen Agent), aber `jj status` zeigt korrekt nur meine `repayment_phase.rs`-Modifikation. Commits via `jj commit` (statt `git commit --no-verify`); die jj-Commits sind co-located in git sichtbar (Hash `0022fec`, `cee23b8`).

## TDD Gate Compliance

Plan ist `type: execute`, einzelne Tasks haben `tdd="true"`:
- Task 1 (`tdd="true"`): Die Skip-Pattern-Implementierung **plus** Mock-Anpassungen wurden in einem feat-Commit `0022fec` zusammengeführt. Der "RED-Beweis" lief implizit beim ersten Test-Run nach dem Skip-Pattern-Insert (2 existing Tests schlugen mit "No matching expectation" fehl); nach Mock-Adjustment grün.
- Task 2 (`tdd="true"`): Der neue Test `test_open_repayment_phase_skips_members_with_existing_entry` wurde im test-Commit `cee23b8` hinzugefügt. Da das Skip-Pattern bereits implementiert war, war der Test sofort grün (kein expliziter RED-Schritt). Dies ist akzeptabel weil Task 2 explizit als "verifying the skip-pattern" (existing implementation) ausgewiesen ist — der Test ist ein **Verification-Test**, kein Driver-Test.

Beide commit-Typen sind im git-Log sichtbar:
- `0022fec feat(16-03): add auto-fill skip-pattern for partial_repayment duplicates`
- `cee23b8 test(16-03): add unit test verifying auto-fill skip-pattern`

## Self-Check: PASSED

- File `genossi_service_impl/src/repayment_phase.rs` enthält Skip-Pattern mit allen 3 Anchor-IDs (D-16-03, PART-04, PITFALLS-Kat-1): VERIFIED (grep)
- Skip-Check ist VOR `audited_create!(...REPAYMENT_PHASE_PROCESS_OPEN...)`: VERIFIED (awk-range f=24, c=48, f<c)
- `cargo build -p genossi_service_impl --lib`: EXIT 0
- `cargo test -p genossi_service_impl --lib repayment_phase`: 27 tests passed (0 failed, 0 regressions vs. baseline 26)
- AUDT-01 Grep-Gate: `grep "self\.repayment_entry_dao\.create\s*("` returns 0 matches in repayment_phase.rs — no direct DAO create outside macros
- Commits exist: `jj log` shows `0022fec0`, `cee23b8c`; both visible via `git rev-parse`

## Per-Plan Output Specifics

Plan §<output> verlangte:
- **Skip-Pattern-Insertion-Line:** Loop-Start bei Z. 368 (unchanged from research); Skip-Check als erste Action des Loop-Bodies von Z. 369-396 (find_by_member_and_phase-Call bei Z. 391).
- **Existing tests requiring `.expect_find_by_member_and_phase()`:** 2 angepasst:
  - `test_open_phase_auto_fill_creates_entries_for_matching_members` (Z. ~1737 jetzt)
  - `test_open_phase_auto_fill_atomic_on_dao_failure` (Z. ~1900 jetzt)
- **New test count:** +1 (`test_open_repayment_phase_skips_members_with_existing_entry`)
- **AUDT-01 grep-gate:** 0 new direct DAO `.create()` calls außerhalb von audited_create! Macros.

## Next Plan/Phase Readiness

- Plan 16-03 ist semantisch komplementär zu Plan 16-02 (`partial_repayment` impl im `MembershipAdjustService`); beide laufen in parallelem Wave 2/3. Plan 02 erzeugt v1.2-Entries via `audited_create!(repayment_entry_dao,...,PARTIAL_REPAYMENT_PROCESS,...)`. Plan 03 stellt sicher, dass v1.1's `open_repayment_phase`-Auto-Fill diese v1.2-Entries nicht dupliziert.
- Die E2E-Test `test_partial_repayment_auto_fill_skip_after_v12` (Plan 04) kann das Zusammenspiel beider Pfade durch echte HTTP-Calls verifizieren.

## Threat Flags

Keine — Plan modifiziert nur die Auto-Fill-Logik in `open_repayment_phase` (existing admin-only Operation). Keine neuen Endpoints, keine neuen Daten-Flows, keine neuen Trust-Boundaries.

---
*Phase: 16-service-rest-teil-rueckgabe-auto-anlegen-phase*
*Completed: 2026-06-05*
