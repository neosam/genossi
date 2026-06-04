---
phase: 15-service-rest-kuendigung-aufstockung
plan: 03
subsystem: api
tags: [rust, service-impl, audit-macro, transaction, increase-shares, optimistic-locking, admin-privilege]

requires:
  - phase: 15-service-rest-kuendigung-aufstockung
    plan: 02
    provides: "MembershipAdjustServiceImpl<Deps> via gen_service_impl!, CANCEL_PROCESS + UPGRADE_PROCESS constants, increase_shares stub, TestDeps + MockTest* mocks, cancel_membership impl (4 tests green)"
  - phase: 14-dao-domain-foundation
    provides: "Member.current_shares pub field (i32), MemberEntity with current_shares + version + exit_date fields"
provides:
  - "MembershipAdjustServiceImpl::increase_shares — atomare Multi-Write-Operation (MemberAction::Aufstockung-Create + Member.current_shares-Update) in einer Transaction"
  - "audited_update!-Macro-Anwendung auf MemberDao fuer Member-Field-Mutation (D-15-03 — generischer MemberDao::update + Macro statt targeted DAO-method)"
  - "Optimistic-Locking-Pattern via uuid_service.new_v4()-Version-Bump VOR audited_update!"
  - "4 zusaetzliche Service-Unit-Tests (Happy-Path, Cancelled-Member-Block, Permission-Denied, invalid-shares=0)"
affects: [15-04]

tech-stack:
  added: []
  patterns:
    - "Pattern: Multi-Write-Atomare-Tx — audited_create! (MemberAction) + audited_update! (Member) in einer Transaction; beide Writes mit identischem Process-String UPGRADE_PROCESS fuer Audit-Linkbarkeit"
    - "Pattern: Optimistic-Locking-Bump — Member.version = uuid_service.new_v4().await VOR audited_update!; Macro liest alte Version via find_by_id und uebergibt new_entity an MemberDao::update"
    - "Pattern: Pre-Validation-Order — Permission-Check -> shares-Pre-Validation -> Datum-Validation -> DAO-Touches (jede Stufe bricht frueh ab, KEINE Mock-Erwartungen fuer spaetere Stufen in Block-Tests noetig)"
    - "Pattern: UPGD-04 Cancelled-Member-Block via ValidationError (HTTP 400) — NICHT Conflict (HTTP 409); Reservierung von Conflict ausschliesslich fuer race-conditional Konflikte wie already-cancelled-during-cancel"

key-files:
  created: []
  modified:
    - "genossi_service_impl/src/membership_adjust.rs"

key-decisions:
  - "UPGD-04: gekuendigte Mitglieder blocken via ValidationError (field=member_id, message contains 'cancelled') -> HTTP 400 — semantisch eine Eingabe-Validierung, nicht race-conditional Conflict"
  - "Pre-Validation-Reihenfolge: Permission -> shares>0 -> Datum -> DAO-Load -> exit_date-Check — Permission-Failure bleibt unter check_permission (PERM-01), nur danach faengt Service-Body Eingabe-Validation"
  - "D-15-03 strikt befolgt: Member.current_shares-Mutation via generischem MemberDao::update + audited_update! (KEIN targeted update_current_shares), so dass AUDT-01 Grep-Gate jetzt fuer BEIDE DAOs (member_action_dao + member_dao) gruen ist"
  - "Member.exit_date wird in increase_shares NICHT angefasst — Aufstockung beeinflusst kein Exit-Date (kein recalc_dates-Aufruf, anders als bei cancel_membership)"
  - "Optimistic-Locking via uuid_service.new_v4() (nicht direkt uuid::Uuid::new_v4()) — konsistent mit MemberActionServiceImpl::create-Pattern"

patterns-established:
  - "Audit-Multi-Write-in-einer-Tx: zwei audited_* Macro-Aufrufe mit identischem Process-String (UPGRADE_PROCESS) fuer atomare Multi-Entity-Mutation — Foundation fuer AUDT-02 (Phase 17 Transfer-Pair shared Process-String)"
  - "AUDT-01 Grep-Gate fuer BEIDE DAOs erfuellt: 0 direkte member_action_dao.create( oder member_dao.update( ausserhalb der audited_*! Macros"
  - "Test-Mock-Expectations fuer audited_update!: 2x find_by_id (1x Service-Body, 1x Macro-Body fuer old-entity-load) + 1x update mit withf-Process-Match"

requirements-completed: [UPGD-01, UPGD-02, UPGD-03, UPGD-04, AUDT-01]

metrics:
  duration: ~4min
  completed: 2026-06-04
  tasks: 1
  files_modified: 1
  tests_added: 4
  tests_passing: 20
---

# Phase 15 Plan 03: increase_shares Implementation Summary

**MembershipAdjustServiceImpl::increase_shares implementiert als atomare Multi-Write-Operation mit audited_create!(MemberAction::Aufstockung) + audited_update!(Member.current_shares) in derselben Transaction, Optimistic-Locking via Version-Bump und 4 Service-Unit-Tests — etabliert das v1.2-Multi-Write-Audit-Pattern als Foundation fuer Phase 17 Transfer-Pair**

## Performance

- **Duration:** ~4 min
- **Started:** 2026-06-04T12:19:07Z
- **Completed:** 2026-06-04T12:23:00Z
- **Tasks:** 1
- **Files modified:** 1
- **Tests added:** 4 new service tests; 20 total membership_adjust tests passing (12 Pure-Function + 4 cancel_membership + 4 increase_shares)

## Accomplishments

- **Task 1: increase_shares-Body + 4 Service-Unit-Tests** — Vollstaendige Implementation der atomaren Multi-Write-Sequence (`use_transaction` -> `current_user_id` -> `check_permission(ADMIN_PRIVILEGE)` -> `shares > 0` Pre-Validation -> `validate_willensbekundung_date` -> `find_by_id` member existence -> UPGD-04 cancelled-member-block via ValidationError -> Build `MemberAction::Aufstockung` mit `shares_change=+shares`, `transfer_member_id=None`, `effective_date=None` (UPGD-02 sofort wirksam), `date=willensbekundung` -> `audited_create!` mit UPGRADE_PROCESS -> Member.current_shares += shares + Version-Bump via uuid_service -> `audited_update!` mit UPGRADE_PROCESS -> `commit`). 4 Service-Unit-Tests gruen (Happy-Path mit konkret asserter `member.current_shares == 5`, Cancelled-Member-Block via ValidationError mit message='cancelled', Permission-Denied propagation, invalid-shares=0 mit field='shares').

- **AUDT-01 Grep-Gate jetzt fuer BEIDE DAOs gruen** — Vor Plan 03 war nur `member_action_dao.create(` ausserhalb der Macros = 0. Plan 03 etabliert die zweite Haelfte: `member_dao.update(` ausserhalb der Macros = 0. Das ist die D-15-03-Compliance-Foundation fuer alle nachfolgenden v1.2-Member-Field-Mutationen.

- **UPGRADE_PROCESS aktiviert** — `#[allow(dead_code)]` von der Konstante entfernt; jetzt von zwei `audited_*!`-Macro-Calls innerhalb derselben Transaction konsumiert.

## Task Commits

Each task was committed atomically via `gsd-sdk query commit` (jj-aware):

1. **Task 1: increase_shares body + 4 service unit tests** — `faea68f` (feat)

## Files Modified

- `genossi_service_impl/src/membership_adjust.rs` (+~250 lines):
  - Konstanten: `#[allow(dead_code)]` von `UPGRADE_PROCESS` entfernt — Konstante jetzt aktiv genutzt
  - `MembershipAdjustService for MembershipAdjustServiceImpl::increase_shares` — Stub-Body durch volle Implementation ersetzt (~95 Zeilen)
  - `service_tests`-Modul: 4 neue `#[tokio::test]`-Funktionen ergaenzt (`test_increase_shares_happy_path`, `test_increase_shares_cancelled_member_blocked`, `test_increase_shares_permission_denied`, `test_increase_shares_invalid_shares_zero`)

## Decisions Made

- **UPGD-04 Mapping: ValidationError (HTTP 400) statt Conflict (HTTP 409)** — Der Plan-Header sagt explizit "HTTP 400 (validation error, NOT 409 — that's reserved for state conflicts like already-cancelled-in-cancel-flow)". Semantisch ist `cannot upgrade cancelled member` eine Eingabe-Validierung ("dieses Member ist nicht ein gueltiger Aufstockung-Empfaenger"), waehrend `member already cancelled` (cancel_membership Plan 02) ein race-conditional State-Conflict ist ("du versuchst zu kuendigen, was bereits gekuendigt ist").

- **Mock-Times-Pattern fuer audited_update!: 2x find_by_id (1x Service-Body, 1x Macro), 1x update** — Anders als bei `cancel_membership` (wo `recalc_dates` einen zusaetzlichen find_by_id-Call macht = 3x), ist `increase_shares` ohne `recalc_dates`-Hook. Das audited_update!-Macro selbst macht intern 1x `find_by_id` (fuer old-entity-load zum Diff). Plus 1x im Service-Body fuer existence-check + clone-Vorlage = 2x total.

- **Member.exit_date strikt unangetastet in increase_shares** — Roadmap-Grep-Gate `grep -cE 'member_dao.update_dates|member_dao.update_migrated' membership_adjust.rs` muss 0 ergeben. Erfuellt. UPGD selbst aendert keine Member-Status-Daten ausserhalb von current_shares + version.

## Deviations from Plan

None - plan executed exactly as written.

Plan-Vorlage war vollstaendig copy-paste-executable. Alle Edge-Cases (Mock-Times-Counts, Year-Rollover-safe Test-Daten, withf-Process-Match-Predicates, UPGD-04-Block-vor-DAO-Touch) waren im Plan korrekt vorgegeben und sind verbatim umgesetzt.

**Total deviations:** 0
**Impact on plan:** None.

## Issues Encountered

Keine — Build geht workspace-weit clean durch (`cargo build --workspace` exit 0), alle 24 bestehenden `member_action`-Tests bleiben gruen (keine Regression durch Plan 01s recalc_dates-Free-Function-Refactor), alle 20 `membership_adjust`-Tests (12 Pure-Function + 4 cancel_membership + 4 increase_shares) gruen.

## Verification Gates (alle erfuellt)

- `cargo build -p genossi_service_impl` exit 0
- `cargo test -p genossi_service_impl --lib membership_adjust` — 20 tests pass (12 Plan 01 + 4 Plan 02 + 4 Plan 03)
- `cargo test -p genossi_service_impl --lib member_action` — 24 tests pass (keine Regression)
- `cargo build --workspace` exit 0
- **Plan-03 Acceptance-Criteria (alle gruen):**
  - `grep -c 'increase_shares — Plan 03' membership_adjust.rs` = 0 (Stub-Marker entfernt)
  - `grep -c 'ActionType::Aufstockung' membership_adjust.rs` = 2 (1 Service-Body + 1 Test-Assertion)
  - `grep -c 'shares_change: shares' membership_adjust.rs` = 1
  - `grep -c 'effective_date: None' membership_adjust.rs` = 1 (UPGD-02)
  - `grep -c 'audited_update!' membership_adjust.rs` = 5 (1 Macro-Call + 4 textuelle Erwaehnungen in Kommentaren/Docs)
  - `grep -c 'audited_create!' membership_adjust.rs` = 7 (1 Macro-Call cancel + 1 Macro-Call upgrade + 5 textuelle Erwaehnungen)
  - **AUDT-01 BEIDE-DAOs-Grep-Gate:** `grep -v '^//' membership_adjust.rs | grep -v '^ \*' | grep -cE 'self\.member_action_dao\.create\(|self\.member_dao\.update\('` = 0
  - **Member.exit_date-NICHT-getoucht:** `grep -cE 'member_dao\.update_dates|member_dao\.update_migrated' membership_adjust.rs` = 0
  - **Test-5 konkrete Assertion (BLOCKER 4 fix):** `grep -c 'member.current_shares, 5' membership_adjust.rs` = 1

## User Setup Required

None — keine externen Services oder Konfigurationen erforderlich. Pure Service-Layer-Implementation.

## Next Phase Readiness

- **Plan 04 (REST + E2E)** kann sofort starten:
  - `MembershipAdjustServiceImpl::cancel_membership` (Plan 02) und `MembershipAdjustServiceImpl::increase_shares` (Plan 03) sind beide voll implementiert und gegen Mock-DAOs verifiziert
  - 8 Service-Tests insgesamt im `service_tests`-Modul (4 cancel + 4 upgrade) — Wave 4 muss nur noch REST-Layer + E2E-Tests bauen
  - Process-Strings `member-adjust.cancel` und `member-adjust.upgrade` sind etabliert — REST-Layer-Audit-Endpoints koennen via `process LIKE 'member-adjust.%'`-Filter darauf zugreifen
  - DI-Wiring in `genossi_bin/src/lib.rs::RestStateImpl` ist noch ausstehend (Plan 04 Scope)
- Keine bekannten Blocker

## Self-Check: PASSED

Verified:
- `genossi_service_impl/src/membership_adjust.rs` (modified, contains `increase_shares` full impl + 4 new tests in `service_tests`) — FOUND
- Commit `faea68f` (Task 1) via `gsd-sdk query commit` JSON-Return bestaetigt — FOUND
- All 20 membership_adjust tests pass (`cargo test -p genossi_service_impl --lib membership_adjust`)
- All 24 existing member_action tests still pass (no regression)
- Workspace build clean (`cargo build --workspace`)
- AUDT-01 Grep-Gate fuer BEIDE DAOs gruen (0 direct bypass calls)
- Roadmap exit_date-Touch Grep-Gate gruen (0 direct exit_date mutations in increase_shares)

---
*Phase: 15-service-rest-kuendigung-aufstockung*
*Completed: 2026-06-04*
