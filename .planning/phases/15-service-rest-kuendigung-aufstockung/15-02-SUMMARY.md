---
phase: 15-service-rest-kuendigung-aufstockung
plan: 02
subsystem: api
tags: [rust, service-impl, audit-macro, transaction, cancel-membership, admin-privilege]

requires:
  - phase: 15-service-rest-kuendigung-aufstockung
    plan: 01
    provides: "MembershipAdjustService trait, validate_willensbekundung_date pure function, recalc_dates free function"
  - phase: 14-dao-domain-foundation
    provides: "compute_effective_date pure function (H1/H2 Stichtag)"
provides:
  - "MembershipAdjustServiceImpl<Deps> via gen_service_impl! (6 Deps)"
  - "cancel_membership Service-Methode (CANC-01..05 + PERM-01 + AUDT-01)"
  - "Audit-Process-Konstanten CANCEL_PROCESS (member-adjust.cancel) + UPGRADE_PROCESS (member-adjust.upgrade, reserved)"
  - "Per-File mockall mock!-Pattern + TestTransaction + TestDeps fuer Service-Unit-Tests"
affects: [15-03, 15-04]

tech-stack:
  added: []
  patterns:
    - "Pattern: gen_service_impl! mit identischem Deps-Set wie MemberActionServiceImpl (MemberActionDao, MemberDao, AuditLogDao, PermissionService, UuidService, TransactionDao)"
    - "Pattern: Service-Method-Sequence — use_transaction -> current_user_id -> check_permission(ADMIN_PRIVILEGE) -> validate_date -> existence-check -> already-cancelled-guard -> audited_create! -> recalc_dates -> re-read -> commit"
    - "Pattern: Per-File mockall::mock! mit TestTransaction (Debug+Clone) statt globalem #[automock], weil genossi_dao::MockTransaction kein Debug-Derive hat (gen_service_impl! benoetigt Debug)"

key-files:
  created: []
  modified:
    - "genossi_service_impl/src/membership_adjust.rs"

key-decisions:
  - "PERM-01: ADMIN_PRIVILEGE statt MANAGE_MEMBERS_PRIVILEGE (D-15-01) — eigener v1.2-Permission-Funnel"
  - "AUDT-01: audited_create! mit CANCEL_PROCESS='member-adjust.cancel' (D-15-02) — separater Process-Namespace fuer v1.2"
  - "Already-Cancelled -> ServiceError::Conflict -> HTTP 409 (per ROADMAP-Success-Criteria; D-15-12 erwaehnt 403 ist Transkriptionsfehler — siehe Plan 02 must_haves)"
  - "CANC-04: exit_date AUSSCHLIESSLICH via crate::member_action::recalc_dates Free-Function — keine direkte Member.exit_date-Mutation im Service-Body"
  - "CANC-05: nur ActionType::Austritt erzeugt — kein Verkauf, kein RepaymentEntry (Phase 16 Territory)"
  - "Re-Read von Member nach recalc_dates fuer Response — Frontend bekommt updated exit_date ohne separaten GET"

patterns-established:
  - "audited_create!-Macro-Compliance fuer v1.2-Namespace: alle MemberAction-Writes via Macro, keine direkten DAO.create()-Calls (AUDT-01 Grep-Gate)"
  - "Year-Rollover-safe Test-Daten: H1/H2-Tests leiten willensbekundung relativ zu OffsetDateTime::now_utc().date() ab (replace_month/replace_day), damit Tests beim Year-Rollover nicht brechen"
  - "TestDeps-Pattern (verbatim aus member.rs:412-712 gespiegelt): MockTest{TxDao,MemberDao,MemberActionDao,AuditLogDao,PermissionService}-Mocks via mockall::mock! mit TestTransaction-Type"

requirements-completed: [CANC-01, CANC-03, CANC-04, CANC-05, PERM-01, AUDT-01]

metrics:
  duration: ~9min
  completed: 2026-06-04
  tasks: 2
  files_modified: 1
  tests_added: 4
  tests_passing: 16
---

# Phase 15 Plan 02: cancel_membership Implementation Summary

**MembershipAdjustServiceImpl::cancel_membership implementiert mit audited_create!-Macro-Compliance, ADMIN_PRIVILEGE-Permission-Funnel und 4 Service-Unit-Tests gegen Mock-DAOs — etabliert das v1.2-Audit-Pattern fuer Phase 15-17**

## Performance

- **Duration:** ~9 min
- **Started:** 2026-06-04T12:05:07Z
- **Completed:** 2026-06-04T12:14:19Z
- **Tasks:** 2
- **Files modified:** 1
- **Tests added:** 4 service tests + reuse 12 existing pure-function tests = 16 passing

## Accomplishments

- **Task 1: Service-Impl-Scaffolding** — `MembershipAdjustServiceImpl<Deps>` via `gen_service_impl!`-Macro deklariert mit identischem 6-Deps-Set wie `MemberActionServiceImpl` (MemberActionDao, MemberDao, AuditLogDao, PermissionService, UuidService, TransactionDao). Process-Konstanten `CANCEL_PROCESS = "member-adjust.cancel"` und `UPGRADE_PROCESS = "member-adjust.upgrade"` (letztere `#[allow(dead_code)]` als Plan-03-Reservation) etabliert den v1.2-Audit-Namespace (D-15-02). Stub-Impl fuer `cancel_membership` + `increase_shares` (beide Plan-03-Stub mit `ServiceError::InternalError`) damit der Trait waehrend Wave 2 kompiliert.

- **Task 2: cancel_membership Body + 4 Service-Unit-Tests** — Volle Implementation der atomaren Single-Action-Sequence (`use_transaction` -> `current_user_id` -> `check_permission(ADMIN_PRIVILEGE)` -> `validate_willensbekundung_date` -> `find_by_id` member existence -> Already-Cancelled-Guard -> `compute_effective_date` -> Build `MemberAction::Austritt` mit `shares_change=0`, `transfer_member_id=None`, `effective_date=Some(stichtag)`, `date=willensbekundung` -> `audited_create!` mit `CANCEL_PROCESS` -> `crate::member_action::recalc_dates` Free-Function (CANC-04) -> Re-Read Member fuer Response -> `commit`). 4 Service-Unit-Tests gruen (Happy-Path-H1, Happy-Path-H2, Permission-Denied, Already-Cancelled) gegen Per-File-`mockall::mock!`-DAOs.

## Task Commits

Each task was committed atomically via `gsd-sdk query commit` (jj-aware):

1. **Task 1: gen_service_impl scaffolding + process constants + stub impl** — `bd5aa09` (feat)
2. **Task 2: cancel_membership body + audited_create! + 4 service unit tests** — `132081c` (feat)

## Files Modified

- `genossi_service_impl/src/membership_adjust.rs` (+437 lines total):
  - Imports: `async_trait`, `genossi_dao::audit_log::AuditLogDao`, `MemberDao`, `MemberActionDao`, `MemberActionEntity`, `ActionType`, `TransactionDao`, `Member`, `MemberAction`, `MembershipAdjustService`, `Authentication`, `PermissionService`, `ADMIN_PRIVILEGE`, `UuidService`, `ServiceError`, `Uuid`, `crate::gen_service_impl`
  - Konstanten: `CANCEL_PROCESS`, `UPGRADE_PROCESS` (D-15-02)
  - `gen_service_impl!`-Block fuer `MembershipAdjustServiceImpl: MembershipAdjustService = MembershipAdjustServiceDeps` mit 6 Deps
  - `impl<Deps: MembershipAdjustServiceDeps> MembershipAdjustService for MembershipAdjustServiceImpl<Deps>`-Block mit `cancel_membership` (vollstaendig) + `increase_shares` (Plan-03-Stub)
  - `#[cfg(test)] mod service_tests`-Modul mit `TestTransaction`, `TestDeps`, 5 Per-File-Mocks (TxDao, MemberDao, MemberActionDao, AuditLogDao, PermissionService) und 4 Tests

## Decisions Made

- **Decision: Mock find_by_id wird 3x aufgerufen, nicht 2x (Test-Daten-Korrektur)** — Initial im Plan stand `.times(2)` (Service-existence-Check + Re-Read), aber `recalc_dates` ruft intern selbst `find_by_id` auf (siehe `genossi_service_impl/src/member_action.rs:195`). Tests aktualisiert auf `.times(3)` mit dokumentiertem Kommentar. Verhalten-neutral, nur Test-Setup angepasst.
- **Decision: `StaticUuidService` muss `pub` sein** — Da `TestDeps::UuidService = StaticUuidService` in einer `pub struct TestDeps`-Trait-Impl als Associated-Type publik gemacht wird, verlangt der Compiler `pub` auf `StaticUuidService` (E0446). Spiegelt das nicht-publike Pattern aus `member.rs:639` nicht — dort ist `TestDeps` privat, hier publik (Plan-Vorlage hat `pub struct TestDeps`).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] StaticUuidService Visibility auf pub angehoben**
- **Found during:** Task 2 Test-Compile (`cargo test -p genossi_service_impl --lib membership_adjust::service_tests`)
- **Issue:** Compiler-Error E0446 — `StaticUuidService` ist private, aber wird als Associated-Type `type UuidService = StaticUuidService` in der publiken `impl MembershipAdjustServiceDeps for TestDeps`-Deklaration referenziert ("private type ... in public interface")
- **Fix:** `pub struct StaticUuidService;` statt `struct StaticUuidService;`
- **Files modified:** `genossi_service_impl/src/membership_adjust.rs`
- **Verification:** `cargo test -p genossi_service_impl --lib membership_adjust::service_tests` 4 tests pass
- **Committed in:** `132081c` (Task 2 commit)

**2. [Rule 1 - Bug] Test-Mock `expect_find_by_id().times(2)` auf `.times(3)` korrigiert**
- **Found during:** Task 2 Test-Run (Mockall-Panic: "called 3 times which is more than the expected 2")
- **Issue:** Plan-Vorlage zaehlte 2 `find_by_id`-Aufrufe (existence-check + re-read), aber `recalc_dates` ruft intern auch `find_by_id` auf (siehe `genossi_service_impl/src/member_action.rs:195` in der Free-Function aus Plan 01). Tatsaechliche Anzahl ist 3.
- **Fix:** Beide Happy-Path-Tests (H1 + H2) auf `.times(3)` mit Erklaerungs-Kommentar
- **Files modified:** `genossi_service_impl/src/membership_adjust.rs` (Test-Module nur)
- **Verification:** Alle 4 Service-Tests gruen
- **Committed in:** `132081c` (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (1 blocking visibility-bump, 1 bug-fix test-expectation count)
**Impact on plan:** Beide Auto-Fixes verhaltens-neutral (Service-Body unveraendert; nur Test-Setup-Korrektur). Plan-Acceptance-Criteria voll erfuellt.

## Issues Encountered

Keine sonstigen — Build geht workspace-weit clean durch (`cargo build --workspace` exit 0), alle 24 bestehenden `member_action`-Tests bleiben gruen, alle 16 `membership_adjust`-Tests (12 Pure-Function aus Plan 01 + 4 Service aus Plan 02) gruen.

## Verification Gates (alle erfuellt)

- `cargo build -p genossi_service_impl` exit 0 (clean, keine dead_code-Warnings mehr fuer `compute_effective_date`/`validate_willensbekundung_date` — beide jetzt im Service-Body aufgerufen)
- `cargo test -p genossi_service_impl --lib membership_adjust` — 16 tests pass (12 from Plan 01 + 4 new service_tests)
- `cargo build --workspace` exit 0 (keine Regression in REST-Layer oder Bin-Crate)
- **AUDT-01 Grep-Gate:** `grep -E 'self\.member_action_dao\.create\(' membership_adjust.rs | grep -v '^//'` = 0 ✓
- **AUDT-01 Grep-Gate:** `grep -E 'self\.member_dao\.update\(' membership_adjust.rs | grep -v '^//'` = 0 ✓
- **CANC-05 Grep-Gate:** `grep -c 'ActionType::Verkauf\|RepaymentEntry' membership_adjust.rs` = 0 ✓
- **CANC-04 Grep-Gate:** `grep -cE 'member_dao\.update_dates|member_dao\.update_migrated' membership_adjust.rs` = 0 (Service-Body) ✓
- **recalc_dates wired:** `grep -c 'crate::member_action::recalc_dates' membership_adjust.rs` = 1 ✓
- **audited_create! present:** `grep -c 'audited_create!' membership_adjust.rs` = 3 (1 macro call + 2 doc comments) ✓

## User Setup Required

None — keine externen Services oder Konfigurationen erforderlich. Pure Service-Layer-Implementation.

## Next Phase Readiness

- **Plan 03 (increase_shares)** kann sofort starten:
  - `MembershipAdjustServiceImpl<Deps>` und `MembershipAdjustServiceDeps` sind via `gen_service_impl!` deklariert
  - `UPGRADE_PROCESS = "member-adjust.upgrade"` ist als Konstante vorhanden (aktuell `#[allow(dead_code)]` — wird in Plan 03 verwendet, dann Marker entfernen)
  - Plan-03 muss nur den `increase_shares`-Stub-Body durch die volle Implementation ersetzen + 4 weitere Tests ergaenzen
  - `audited_update!`-Macro fuer `Member.current_shares`-Mutation (D-15-03) ist im selben Crate verfuegbar
- **Plan 04 (REST + E2E)** wartet auf Plan 03 abgeschlossen — Patterns sind etabliert (Process-Strings, ADMIN_PRIVILEGE-Funnel, audited_create!/audited_update!-Macro-Compliance)
- Keine bekannten Blocker

## Self-Check: PASSED

Verified:
- `genossi_service_impl/src/membership_adjust.rs` (modified, contains `MembershipAdjustServiceImpl`, `cancel_membership` impl, 4 service_tests) — FOUND
- Commits: `bd5aa09` (Task 1) und `132081c` (Task 2) — beide via `gsd-sdk query commit` JSON-Return bestaetigt
- All 16 membership_adjust tests pass (`cargo test -p genossi_service_impl --lib membership_adjust`)
- All 24 existing member_action tests still pass (no regression from recalc_dates free-function refactor in Plan 01)
- Workspace build clean (`cargo build --workspace`)

---
*Phase: 15-service-rest-kuendigung-aufstockung*
*Completed: 2026-06-04*
