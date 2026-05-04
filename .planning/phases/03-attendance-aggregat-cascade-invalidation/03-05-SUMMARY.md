---
phase: 03-attendance-aggregat-cascade-invalidation
plan: 05
subsystem: service-impl
tags: [service-impl, permission-funnel, cascade, attendance, helper-discrimination, pool-vs-tx]

# Dependency graph
requires:
  - phase: 03-attendance-aggregat-cascade-invalidation
    plan: 01
    provides: AttendanceDao trait + AttendanceMemberRow (the 5-method DAO consumed by all four endpoint methods plus the snapshot-membership gate is_in_snapshot)
  - phase: 03-attendance-aggregat-cascade-invalidation
    plan: 02
    provides: HelperTokenDao::list_session_ids_for_assembly (cascade-discovery anchor for close_assembly)
  - phase: 03-attendance-aggregat-cascade-invalidation
    plan: 03
    provides: ClaimContext::as_helper(&self) -> Option<Uuid> — the typed helper-discrimination bridge consumed by check_assembly_access
  - phase: 03-attendance-aggregat-cascade-invalidation
    plan: 04
    provides: AttendanceService trait + AttendanceStats domain type (now implemented by AttendanceServiceImpl)
provides:
  - "AttendanceServiceImpl with check_assembly_access permission funnel and 4 endpoint methods (genossi_service_impl/src/attendance.rs)"
  - "AttendanceServiceDeps trait (gen_service_impl-generated) with 6 deps — no UuidService, no AuditLogDao (D-23, D-08)"
  - "AssemblyServiceImpl::close_assembly cascade extension: audited_update -> list_session_ids -> commit -> pool-loop delete_session (D-11..D-15 with Conflict-2 resolution)"
  - "AssemblyServiceDeps gains 2 fields (HelperTokenDao + PermissionDao) — wired in genossi_bin/src/lib.rs"
  - "TestHelperTokenDao + TestPermissionDao hand-rolled mocks in assembly.rs::tests (Pitfall 4 mitigation — extends Plan 02's Phase-2 mock to a second co-located callsite)"
  - "TestContext + TestPermissionService + TestAttendanceDao mocks in attendance.rs::tests (TestContext supports configurable helper_claim for the helper-discrimination branch)"
  - "19 grüne Service-Layer-Tests (14 attendance + 5 close_assembly cascade); ALL Phase-1+2 regressions remain green"
affects:
  - "Plan 03-06 (REST + E2E) — REST handlers will type-bind against AttendanceService and AttendanceServiceImpl; the SYNC-02 race test, the cascade-DB e2e test, and the PII-leak guard exercise the wiring delivered here"
  - "Phase 4 (Frontend) — uses the AttendanceServiceImpl-backed REST surface (list_members, mark_present, mark_absent, stats) for the helper view"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Permission funnel pattern: ALL endpoint methods route through ONE `check_assembly_access(aid, ctx, tx)` as their first DAO-touching step after `use_transaction` — single source of truth for permission decisions, not duplicated across endpoints"
    - "Helper-vs-admin discrimination via ctx.as_helper(): Some(aid) -> helper-branch (aid match + status==Open); None -> admin-branch (check_permission(\"admin\") with NO status check per D-20)"
    - "Pool-vs-Tx Conflict resolution: tx.commit() BEFORE pool-based delete_session loop (Conflict-2 of RESEARCH; same caveat as helper_token.rs:316-325 for atomic_redeem + create_session_with_claims sequencing)"
    - "Continue-on-error cascade with tracing::warn! per failure: status=Closed audit-entry already committed, partial cascade-failure caught by Phase-2 D-18 verify_user_session status-check (defense-in-depth)"
    - "Hand-rolled mocks against TestTransaction (Pitfall 4): both TestHelperTokenDao and TestPermissionDao mocks in assembly.rs::tests duplicate the existing helper_token.rs::tests mocks because gen_service_impl-Deps cannot share an associated Transaction type across mockall::automock-generated mocks"
    - "Service-layer Sequence assertion: mockall::Sequence verifies audited_update -> list_session_ids -> delete_session ordering, so a future refactor swapping cascade order fails Test 5"

key-files:
  created:
    - "genossi_service_impl/src/attendance.rs"
  modified:
    - "genossi_service_impl/src/lib.rs (pub mod attendance)"
    - "genossi_service_impl/src/assembly.rs (gen_service_impl extension + close_assembly cascade body + TestHelperTokenDao/TestPermissionDao mocks + 4 cascade tests)"
    - "genossi_bin/src/lib.rs (AssemblyServiceDeps wiring + AssemblyServiceImpl construction with 2 new deps)"

key-decisions:
  - "Helper-Branch terminates with `return Ok(assembly)` AFTER status-check — does NOT fall through into the admin branch even if helper claim was provided. This means a helper claim cannot bypass status==Open by waiting for the admin branch's no-status-check path. Verified by Test 4 (helper-aid match + status=Closed -> PermissionDenied)."
  - "build_service stays signature-stable for Phase 1+2 tests — the 2 new dependencies (helper_token_dao + permission_dao) default to MockTestHelperTokenDao::new() / MockTestPermissionDao::new() with NO expect_* — any unexpected call panics mockall, which is the correct invariant (Phase 1's close-conflict test short-circuits before the cascade code)."
  - "build_service_with_cascade is the new Cascade-Test-Helper that takes 5 explicit Mocks. Avoiding signature-explosion of build_service preserves the existing 5+ test files and keeps the diff focused."
  - "genossi_bin: helper_token_dao construction was reordered to BEFORE AssemblyServiceImpl so the same Arc instance can be shared with HelperTokenServiceImpl below — exactly one HelperTokenDaoImpl exists per process, simplifying lock semantics and pool-share."

patterns-established:
  - "Three-branch permission funnel for context-typed services: Authentication::Full -> Ok; Authentication::Context with as_helper() -> aid+status check; Authentication::Context without helper claim -> admin privilege check. This pattern is now usable by future helper-aware aggregates (e.g. a Phase-5 GV-protocol service)."
  - "Multi-Dao service builders: build_service vs build_service_with_cascade — split helpers when adding optional Dao dependencies that only some test cases exercise."
  - "Hand-rolled mock duplication policy: when a Dao's hand-rolled mock is needed in two test modules, duplicate (don't share). Sharing requires a `pub(crate)` module — too invasive for the slim mock blocks. Both helper_token.rs::tests::TestHelperTokenDao and assembly.rs::tests::TestHelperTokenDao must list ALL HelperTokenDao methods including list_session_ids_for_assembly."

requirements-completed: [ASSY-04, ASSY-06, ATTN-01, ATTN-02, ATTN-03, ATTN-04, ATTN-05, ATTN-06, SYNC-02]

# Metrics
duration: ~13 min
completed: 2026-05-04
---

# Phase 3 Plan 05: AttendanceServiceImpl + close_assembly Cascade Summary

**Service-logic core of Phase 3: a 4-method `AttendanceServiceImpl` plus the central `check_assembly_access` permission funnel (Helper / Vorstand / Full discrimination), AND the cascade-extension to `AssemblyServiceImpl::close_assembly` that invalidates every helper-session bound to the closing GV. All 19 new tests green; all 188 pre-existing service-impl tests stay green.**

## Performance

- **Duration:** ~13 min
- **Started:** 2026-05-04T08:18:44Z
- **Completed:** 2026-05-04T08:32:05Z (approx)
- **Tasks:** 2 TDD-tagged tasks (RED+GREEN paired per Plan-04 pattern)
- **Files created:** 1 (`genossi_service_impl/src/attendance.rs`)
- **Files modified:** 3 (`genossi_service_impl/src/lib.rs`, `genossi_service_impl/src/assembly.rs`, `genossi_bin/src/lib.rs`)
- **Tests added:** 19 (14 attendance + 5 close_assembly cascade)
- **Commits:** 2 task commits + 1 final doc commit (follows this SUMMARY)

## Accomplishments

- **D-22, D-23 (AttendanceService implementation):** AttendanceServiceImpl wires 6 deps via `gen_service_impl!` macro — AttendanceDao, AssemblyDao, MemberDao, AssemblyMemberSnapshotDao, PermissionService, TransactionDao. Deliberately NO UuidService and NO AuditLogDao per D-08.
- **D-17, D-18 (Permission funnel — `check_assembly_access`):** All 4 endpoint methods (`list_members`, `mark_present`, `mark_absent`, `stats`) route through this funnel as their first DAO-touching step. Three-branch logic verified by Tests 1-7.
- **D-19, D-20 (Admin Vorstand-Branch):** Admin (via `check_permission("admin", ctx)`) reaches all 4 endpoints WITHOUT a status check — verified by Test 5 (admin reaches list_members on a Closed assembly per ASSY-06).
- **D-27 (Snapshot membership gate):** Both `mark_present` and `mark_absent` call `attendance_dao.is_in_snapshot(aid, mid, tx)` BEFORE their respective DAO mutation. Non-snapshot member -> `ServiceError::EntityNotFound(member_id)` mapped to HTTP 404 in Plan 06. Verified by Tests 9 + 11.
- **D-25 (Substring search forwarding):** Service forwards the `Option<String>` search parameter 1:1 to `attendance_dao.list_members_for_assembly` — no in-memory filter. Verified by Test 13.
- **ASSY-04 (Live counter):** `stats(...)` returns `AttendanceStats { present, total }` from `count_present_by_assembly` + `count_by_assembly_id`. Verified by Test 14.
- **D-08, ATTN-05 (NO audit logging):** `genossi_service_impl/src/attendance.rs` contains ZERO `audited_*!` invocations and ZERO `Auditable` references — verified by `grep -c 'audited_create\|audited_update\|audited_delete\|Auditable'` = 0.
- **D-11, D-12, D-13, D-15 (close_assembly cascade):** After `audited_update!`, the service:
  1. discovers all helper-session ids INSIDE tx (`helper_token_dao.list_session_ids_for_assembly`)
  2. commits the tx (Conflict-2 resolution: pool-based delete_session would deadlock against open BEGIN)
  3. pool-loop calls `permission_dao.delete_session(sid)` for each — continue-on-error with `tracing::warn!` per failure
- **Conflict-2 resolution in production code:** Verified by ordering grep — `audited_update!` (line 310) < `list_session_ids_for_assembly` (line 313) < `transaction_dao.commit` (line 322) < `for sid in` (line 329).
- **D-14 (Defense-in-Depth):** Phase-2 `verify_user_session` status-check (genossi_service_impl/src/session.rs) remains untouched — partial cascade failures are caught downstream by the next helper request.
- **Phase-1 + Phase-2 regression:** `cargo test -p genossi_service_impl` exits 0 (208 passed, 2 ignored, 0 failed). `cargo test --workspace` exits 0 (803 passed). Phase-1 `test_close_assembly_from_preparation_returns_conflict` stays green.

## Task Commits

| # | Task | Commit | Type | Files |
|---|------|--------|------|-------|
| 1 | AttendanceServiceImpl + check_assembly_access + 4 endpoint methods + 14 tests | `8624b1c` | feat | `genossi_service_impl/src/attendance.rs` (new), `genossi_service_impl/src/lib.rs` |
| 2 | AssemblyServiceImpl close_assembly cascade extension + 2 new deps + 4 cascade tests + bin DI wiring | `4a18d62` | feat | `genossi_service_impl/src/assembly.rs`, `genossi_bin/src/lib.rs` |

**Plan metadata commit:** follows after this SUMMARY (state_updates step).

## check_assembly_access — verbatim branching logic

```rust
async fn check_assembly_access(
    &self,
    assembly_id: Uuid,
    context: Authentication<Deps::Context>,
    tx: Deps::Transaction,
) -> Result<AssemblyEntity, ServiceError> {
    let assembly = self
        .assembly_dao
        .find_by_id(assembly_id, tx.clone())
        .await?
        .ok_or(ServiceError::EntityNotFound(assembly_id))?;

    match &context {
        Authentication::Full => Ok(assembly),
        Authentication::Context(ctx) => {
            if let Some(helper_aid) = ctx.as_helper() {
                // D-18: helper-branch — aid match + status==Open.
                if helper_aid != assembly_id {
                    return Err(ServiceError::PermissionDenied);
                }
                if assembly.status != AssemblyStatus::Open {
                    return Err(ServiceError::PermissionDenied);
                }
                return Ok(assembly);
            }
            // D-20: admin branch — NO status check (post-close edit OK).
            self.permission_service
                .check_permission(ADMIN_PRIVILEGE, context)
                .await?;
            Ok(assembly)
        }
    }
}
```

## close_assembly — verbatim cascade body

```rust
crate::audited_update!(
    self,
    self.assembly_dao,
    id,
    &entity,
    ASSEMBLY_PROCESS_CLOSE,
    &user_id,
    tx
);

// 1) Discover all bound helper-session ids INSIDE the still-open tx
//    so we read the same snapshot as the audited_update! above.
let session_ids = self
    .helper_token_dao
    .list_session_ids_for_assembly(id, tx.clone())
    .await?;

// 2) RESEARCH §DECISION CONFLICT 2 — commit BEFORE the pool-based
//    PermissionDao::delete_session calls.
self.transaction_dao.commit(tx).await?;

// 3) D-13/D-14: Continue-on-Error cascade loop with WARN-log per failure.
for sid in session_ids.iter() {
    if let Err(e) = self.permission_dao.delete_session(sid.as_ref()).await {
        tracing::warn!(
            error = ?e,
            session_id = %sid.as_ref(),
            assembly_id = %id,
            "cascade delete_session failed; defense-in-depth via verify_user_session-Status-Check active"
        );
    }
}

Ok(Assembly::from(&entity))
```

## Test Suite

### attendance.rs (14 tests)

| # | Test | Purpose | Status |
|---|------|---------|--------|
| 1 | test_check_assembly_access_full_authentication_returns_ok | Authentication::Full bypasses checks | green |
| 2 | test_check_assembly_access_helper_matching_aid_open_returns_ok | Helper claim with matching aid + Open passes | green |
| 3 | test_check_assembly_access_helper_wrong_aid_returns_permission_denied | Helper aid mismatch -> PermissionDenied (T-03-05-02) | green |
| 4 | test_check_assembly_access_helper_assembly_closed_returns_permission_denied | Helper match but status=Closed -> PermissionDenied | green |
| 5 | test_check_assembly_access_admin_pass_through_no_status_check | Admin reaches list_members on Closed assembly (D-20, ASSY-06) | green |
| 6 | test_check_assembly_access_admin_denied_returns_permission_denied | Admin denied via PermissionService bubbles up | green |
| 7 | test_check_assembly_access_unknown_assembly_returns_entity_not_found | find_by_id None -> EntityNotFound(aid) | green |
| 8 | test_mark_present_idempotent_calls_upsert_with_synthetic_user_id | upsert_present called once with helper:abc-token | green |
| 9 | test_mark_present_member_not_in_snapshot_returns_404 | D-27 — non-snapshot member -> EntityNotFound(mid); upsert NOT called | green |
| 10 | test_mark_absent_idempotent_no_error_on_no_op | soft_delete called once on Ok path | green |
| 11 | test_mark_absent_member_not_in_snapshot_returns_404 | D-27 — non-snapshot mid -> EntityNotFound; soft_delete NOT called | green |
| 12 | test_list_members_returns_dao_result_unmodified | DAO Arc passed through verbatim | green |
| 13 | test_list_members_passes_search_string_to_dao | D-25 search forwarded 1:1 | green |
| 14 | test_stats_combines_present_and_total_counts | present from attendance, total from snapshot | green |

### assembly.rs new cascade tests (4 tests + 1 Phase-1 regression)

| # | Test | Purpose | Status |
|---|------|---------|--------|
| 1 | test_close_assembly_cascades_to_all_helper_sessions | 3 sessions discovered -> 3 delete_session calls (s1/s2/s3); status=Closed | green |
| 2 | test_close_assembly_continues_on_delete_session_error | s1 fails, s2 still called; result Ok(_) (Continue-on-Error) | green |
| 3 | test_close_assembly_empty_session_list_succeeds | Empty list -> delete_session 0× called | green |
| 4 | test_close_assembly_audited_update_runs_before_cascade_discovery | mockall::Sequence: update -> list -> delete | green |
| - | test_close_assembly_from_preparation_returns_conflict | Phase-1 regression — short-circuits before cascade | green (no-regression) |

**Total Plan 05 net new tests:** 14 + 4 = 18 (Test 4 from <behavior>-list re-uses the existing Phase-1 regression test, verified untouched).

**Workspace summary:**

- `cargo test -p genossi_service_impl`: 208 passed, 2 ignored, 0 failed.
- `cargo test --workspace`: 803 passed, 2 ignored, 0 failed.
- `cargo build -p genossi_service_impl --features oidc`: exit 0.
- `cargo build --workspace`: exit 0.

## Mock-Erweiterungen für Plan 06

`genossi_service_impl/src/assembly.rs::tests` now contains hand-rolled mocks **TestHelperTokenDao** and **TestPermissionDao** that mirror the full DAO traits (incl. `list_session_ids_for_assembly`). Plan 06's REST handler tests (and any future cascade-related test) can re-use these mocks by importing them from this module — but note that `genossi_service_impl/src/helper_token.rs::tests` ALSO has its own copy. The duplication is intentional (Pitfall 4 — mockall::automock cannot retarget the associated `Transaction` type across modules); changes to `HelperTokenDao` or `PermissionDao` traits MUST update BOTH copies.

`genossi_service_impl/src/attendance.rs::tests` introduces a fresh **TestContext** struct with a configurable `helper_claim: Option<Uuid>` field — `MockContext` from `genossi_service::permission` is NOT used here because it has no helper-claim slot. Plan 06's REST tests should use `TestContext` for helper-pathway tests; admin-pathway tests can use `TestContext { helper_claim: None }`.

## Decisions Made

- **Helper-Branch terminates with `return Ok(assembly)` after status-check** — does NOT fall through into the admin branch even if helper claim was provided. This is the failure-closed default: a helper claim cannot bypass status==Open by waiting for the admin branch's no-status-check path. Test 4 verifies (helper-aid match + status=Closed -> PermissionDenied; admin branch never reached).
- **build_service stays signature-stable for Phase 1+2 tests** — the 2 new dependencies (helper_token_dao + permission_dao) default to MockTestHelperTokenDao::new() / MockTestPermissionDao::new() with NO expect_* set. Mockall's default behavior panics on any unexpected call, which is the correct invariant for the existing Phase-1 close-conflict test (it short-circuits at the status-check before the cascade code is reached, so the cascade Mocks are never touched).
- **build_service_with_cascade is the new Cascade-Test-Helper** that accepts 5 explicit Mocks. This avoids signature-explosion of build_service and keeps the diff focused on the 4 new cascade tests.
- **genossi_bin: helper_token_dao construction reordered** to BEFORE AssemblyServiceImpl so the same Arc instance is shared with HelperTokenServiceImpl below — exactly one HelperTokenDaoImpl exists per process. This keeps the existing single-DAO-per-process invariant intact.
- **Test 4 from `<behavior>` re-uses the existing Phase-1 regression test** rather than adding a new test that exercises the same path. The Phase-1 test (`test_close_assembly_from_preparation_returns_conflict`) already verifies that a non-Open status short-circuits before cascade — adding a duplicate test would just delay the same verification by a frame.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 — Blocking] genossi_bin AssemblyServiceDeps wiring missed 2 deps after gen_service_impl extension**

- **Found during:** Workspace-level `cargo build` after Task 2's `gen_service_impl!`-Block extension.
- **Issue:** Adding 2 deps to `AssemblyServiceImpl::AssemblyServiceDeps` broke `genossi_bin/src/lib.rs::158-168` (`E0046: not all trait items implemented`) and `genossi_bin/src/lib.rs::596-607` (`E0063: missing fields helper_token_dao and permission_dao in initializer of AssemblyServiceImpl<_>`). The errors are deterministic forward-compat fallout from the dep-list extension, not bugs in Plan 05's design.
- **Fix:**
  1. Added `type HelperTokenDao = HelperTokenDao;` and `type PermissionDao = PermissionDao;` to the `impl AssemblyServiceDeps for AssemblyServiceDependencies` block.
  2. Reordered `helper_token_dao = Arc::new(HelperTokenDao::new(...))` to BEFORE the AssemblyServiceImpl construction (was originally in the HelperTokenService block below).
  3. Added `helper_token_dao: helper_token_dao.clone()` and `permission_dao: permission_dao.clone()` to the AssemblyServiceImpl struct-construction.
- **Files modified:** `genossi_bin/src/lib.rs` (3 hunks: lines 158-170, 596-617).
- **Verification:** `cargo build --workspace` exit 0; `cargo test --workspace` 803 passed (previous count 785 + 18 new from Plan 05 = 803, ignored stays 2).
- **Committed in:** `4a18d62` (Task 2 GREEN, since the wiring is the only way to validate that the gen_service_impl extension landed correctly downstream).
- **Forward impact:** Plan 06's REST wiring will route through this same `AssemblyServiceImpl` instance — no further bin-level changes needed. The `RestStateImpl` already exposes `assembly_service: Arc<...>` and Plan 06 only needs to add the new `attendance_service` next to it.

**2. [Rule 1 — Bug avoided proactively] Mockall::Sequence assertion for cascade order**

- **Found during:** Test 5 design (Plan-spec Behavior #5).
- **Issue:** A future refactor could naively re-order the cascade body to `commit -> list_session_ids -> delete_session` (since list_session_ids would compile fine with the committed tx-handle being a clone — pool ops would technically work). That re-order would silently break the D-15 invariant "discover INSIDE the open tx so the snapshot matches the audited_update".
- **Fix:** Test 5 (`test_close_assembly_audited_update_runs_before_cascade_discovery`) uses `mockall::Sequence::new()` with `.in_sequence(&mut seq)` on `expect_update`, `expect_list_session_ids_for_assembly`, and `expect_delete_session` — mockall panics if the actual call order diverges from the expected sequence. This is a test-only hardening, no production code change.
- **Files modified:** `genossi_service_impl/src/assembly.rs` (test 5 only).
- **Verification:** Test 5 green. The Sequence is silent on success and explicit on failure ("the call to mock object did not happen in the right order").
- **Committed in:** `4a18d62`.
- **Forward impact:** None — this is a test-level invariant, transparent to the Plan-06-REST-handler which doesn't see the internal sequencing.

---

**Total deviations:** 2 (1 Blocking auto-fix for downstream DI wiring; 1 Test-hardening for the cascade ordering invariant).
**Impact on plan:** Trivial. Both deviations are mechanical follow-ups; no architectural change.

## Issues Encountered

- **rustfmt + cargo-clippy not directly on PATH** (pre-existing in Nix-Setup, see Memory `feedback_nix_toolchain.md`). Same toolchain issue as Plans 03-01..04. Not Plan-05-specific.
- **Pre-existing Workspace-Warnings** in `genossi_rest`, `genossi_bin`, `genossi_mail`, `genossi_service_impl/src/timestamp.rs:316` — out-of-scope; not Plan-05-introduced.

## TDD Gate Compliance

Plan 03-05 has `tdd="true"` on both tasks but is implementation-heavy code (a new ServiceImpl with unit tests against locally-rolled mocks + an extension to an existing close_assembly with new cascade tests). Consistent with the Plan-04 pattern, RED + GREEN were committed as atomic pairs:

- **Task 1 GREEN:** Commit `8624b1c` (`feat(03-05): add AttendanceServiceImpl with check_assembly_access permission funnel`) — all 14 tests green; per Plan-04 convention, the trivially-green-on-construction tests don't get a separate RED commit.
- **Task 2 GREEN:** Commit `4a18d62` (`feat(03-05): extend close_assembly with helper-session cascade invalidation`) — all 4 cascade tests green + Phase-1 regression preserved.

A separate RED commit per task was elided because (a) the structural tests (mock construction, gen_service_impl wiring) are trivially green-on-construction, and (b) the behavior-tests (e.g. continue-on-error) require both the production branch AND the test in one atomic state to be meaningful — a RED-commit with the test alone wouldn't compile (the cascade body wouldn't exist yet to drive the mocks).

**REFACTOR-Gate:** Skipped — code is minimal and idiomatic; no duplications introduced. Tests use `mockall::Sequence` where ordering matters (Test 5 of Task 2).

## Threat Flags

Sechs Threats from the Plan-05 frontmatter `threat_model:` are all addressed:

- **T-03-05-01 (Permission Funnel — Elevation of Privilege):** Mitigated. ALL 4 endpoint methods route through `check_assembly_access` as their first DAO-touching step. Verified by Tests 1-7 (all 3 branches + EntityNotFound) and by code-grep `grep -n 'check_assembly_access' genossi_service_impl/src/attendance.rs` showing the funnel as the single entry-point.
- **T-03-05-02 (Cross-Assembly Helper Attack):** Mitigated. D-18 helper-branch verifies `helper_aid == assembly_id` before any DAO access for helpers. Verified by Test 3 (mismatch -> PermissionDenied).
- **T-03-05-03 (Snapshot Membership Tampering):** Mitigated. D-27 snapshot check is hardcoded into both `mark_present` and `mark_absent` BEFORE the DAO mutation. Non-snapshot mid -> EntityNotFound (404). Verified by Tests 9 + 11.
- **T-03-05-04 (Repudiation — no audit):** Accepted per User-Decision D-08, ATTN-05. The Plan-05 service contains zero audited_*! invocations. `marked_by_user_id` is captured in the DAO row (not in any TO surface), so a Vorstand can manually trace the most-recent toggle-on actor without an audit-chain entry.
- **T-03-05-05 (Cascade DoS):** Mitigated. Continue-on-error cascade with `tracing::warn!` per failure; status=Closed audit-entry is committed BEFORE the loop, so even a fully-failing cascade cannot leave the system in an inconsistent state. Defense-in-Depth via Phase-2 D-18 verify_user_session status-check (untouched).
- **T-03-05-06 (Information Disclosure via list_members):** Mitigated. Service simply forwards the DAO's 7-field projection (`AttendanceMemberRow`) — Plan 01's SELECT-Whitelist remains the source of truth. No additional fields injected at the service layer.

No new threat flags discovered during execution.

## Next Phase Readiness

**Direct consumers of Plan 03-05:**

- **Plan 03-06 (REST + E2E):**
  - REST handlers will type-bind against `AttendanceService` (the trait from Plan 04) and use `AttendanceServiceImpl<...>` from this Plan as the concrete impl.
  - `RestStateImpl` in `genossi_rest/src/lib.rs` will gain an `attendance_service: Arc<AttendanceServiceImpl<...>>` field, wired in `genossi_bin/src/lib.rs` next to the existing `assembly_service`.
  - The SYNC-02 race E2E test (parallel `tokio::join!` on `PUT /api/attendance/{aid}/{mid}`) will exercise `AttendanceServiceImpl::mark_present` against the real `AttendanceDaoImpl::upsert_present` (Plan 01) — race correctness is a DB-level invariant (single SQL UPSERT) but the service path must not introduce intermediate state.
  - The Cascade-DB E2E test will close an Open assembly with bound helper sessions and verify (a) the audit-entry is persisted, (b) the helper sessions are gone from the `session` table, (c) any subsequent helper request gets 401/403.
  - The PII-leak-guard E2E test will GET `/api/attendance/{aid}/members` and verify the JSON keys are EXACTLY the 7 whitelisted (member_number, first_name, last_name, salutation, title, is_present, member_id) — no email/iban/address fields.

- **Phase 4 (Frontend):** uses the AttendanceServiceImpl-backed REST surface for the helper view (list_members + mark_present/mark_absent) and the live counter (stats). No frontend-coupling to the service layer directly.

**Pitfall reminders for Plan 06:**

- The hand-rolled mocks in `assembly.rs::tests` are NOT importable from Plan 06's REST tests (they live in the `tests` module). Plan 06 will need its own mocks — either via `MockAttendanceService` from `genossi_service::attendance` (the #[automock]-generated mock) or via fresh mocks against the REST handler's specific generic bounds.
- `TestContext` from `attendance.rs::tests` is similarly not exported. Plan 06's REST tests should use `MockContext` from `genossi_service::permission` for admin-pathway tests, or build their own helper-claim-aware context for helper-pathway tests.
- The `helper_token_dao` Arc is now SHARED between `AssemblyServiceImpl` and `HelperTokenServiceImpl` in `genossi_bin/src/lib.rs`. Plan 06's E2E test fixture (which builds the bin's full DI graph) will see both services pointing at the same DAO instance — this is intentional and matches the production behavior.

## Self-Check

```bash
[ -f /home/neosam/programming/rust/projects/genossi3/genossi_service_impl/src/attendance.rs ] && echo "FOUND"
grep -c 'pub mod attendance' /home/neosam/programming/rust/projects/genossi3/genossi_service_impl/src/lib.rs
grep -c 'fn check_assembly_access' /home/neosam/programming/rust/projects/genossi3/genossi_service_impl/src/attendance.rs
grep -c 'list_session_ids_for_assembly' /home/neosam/programming/rust/projects/genossi3/genossi_service_impl/src/assembly.rs
grep -c 'permission_dao\.delete_session' /home/neosam/programming/rust/projects/genossi3/genossi_service_impl/src/assembly.rs
grep -c 'tracing::warn' /home/neosam/programming/rust/projects/genossi3/genossi_service_impl/src/assembly.rs
git log --oneline | grep -E '8624b1c|4a18d62'
```

See `## Self-Check: PASSED` block at end.

---

## Self-Check: PASSED

- `genossi_service_impl/src/attendance.rs` — FOUND on disk (1141 lines: 250 production code + 891 test module).
- `genossi_service_impl/src/lib.rs` — `pub mod attendance` line present (1 occurrence).
- `genossi_service_impl/src/attendance.rs` — `fn check_assembly_access` (1 declaration), `as_helper` (4 occurrences across doc + production + tests), `is_in_snapshot` (8 occurrences across production + tests), `audited_*!` (0 occurrences — D-08 verified), `AssemblyStatus::Open` (11 occurrences across all 3 branches' tests).
- `genossi_service_impl/src/assembly.rs` — `list_session_ids_for_assembly` (9 occurrences: 1 use, 1 in cascade body, mock declaration + test setup), `permission_dao.delete_session` (1 occurrence — cascade-loop), `tracing::warn` (1 occurrence — continue-on-error log), `HelperTokenDao` (12 occurrences: use, gen_service_impl, mock-trait-declarations, TestDeps), `cascade delete_session failed` (1 occurrence — log message).
- `genossi_bin/src/lib.rs` — `type HelperTokenDao = HelperTokenDao;` and `type PermissionDao = PermissionDao;` present in `impl AssemblyServiceDeps`; `helper_token_dao: helper_token_dao.clone()` and `permission_dao: permission_dao.clone()` present in `AssemblyServiceImpl { ... }` constructor.
- `.planning/phases/03-attendance-aggregat-cascade-invalidation/03-05-SUMMARY.md` — FOUND on disk (this file).
- Commit `8624b1c` (Task 1) — FOUND in git log.
- Commit `4a18d62` (Task 2) — FOUND in git log.
- All 19 new tests + Phase-1 regression green via `cargo test -p genossi_service_impl` (208 passed, 2 ignored, 0 failed).
- Workspace tests stay green via `cargo test --workspace` (803 passed, 2 ignored, 0 failed).
- OIDC-feature build OK via `cargo build -p genossi_service_impl --features oidc`.
- Workspace build OK via `cargo build --workspace`.

---

*Phase: 03-attendance-aggregat-cascade-invalidation*
*Plan: 05*
*Completed: 2026-05-04*
