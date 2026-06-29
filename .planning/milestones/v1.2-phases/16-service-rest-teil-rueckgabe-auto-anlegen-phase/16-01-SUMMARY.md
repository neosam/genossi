---
phase: 16-service-rest-teil-rueckgabe-auto-anlegen-phase
plan: 01
subsystem: api
tags: [rust, axum, serde, utoipa, mockall, async-trait, sqlx, tdd]

# Dependency graph
requires:
  - phase: 15-service-rest-kuendigung-aufstockung
    provides: "MembershipAdjustService trait (cancel_membership + increase_shares), validate_willensbekundung_date pure helper, audit-process-string dot-hierarchy convention"
  - phase: 14-dao-domain-foundation
    provides: "compute_effective_date pure function, ADMIN_PRIVILEGE constant, ISO8601 date serde, RepaymentEntryDao::find_by_member_and_phase query"
provides:
  - "MembershipAdjustService::partial_repayment trait method with shares: i32 and Result<(Member, RepaymentEntry, Option<RepaymentPhase>), ServiceError> return type"
  - "PartialRepaymentRequestTO + PartialRepaymentResponseTO with ToSchema (OpenAPI/Swagger-UI)"
  - "PARTIAL_REPAYMENT_PROCESS audit-string constant ('member-adjust.partial-repayment')"
  - "DEFAULT_SHARE_VALUE_CENT constant (10000 i64, 100 EUR per share fallback)"
  - "validate_partial_repayment_shares pure helper with 7 unit tests covering D-16-11 + D-16-12 edge cases"
  - "Plan-16-02 stub impl in MembershipAdjustServiceImpl so the workspace compiles during Wave-1"
affects: [phase-16-02-service-impl, phase-16-03-autofill-skip, phase-16-04-rest-endpoint, phase-17-transfer-shares, phase-18-frontend-partial-repayment]

# Tech tracking
tech-stack:
  added: []  # no new dependencies — pure feature addition within existing workspace
  patterns:
    - "Trait grows incrementally (D-15-13 / D-16-17): new methods append, existing methods untouched"
    - "DAO-free pure-function range validators colocated with the service impl (validate_partial_repayment_shares mirrors validate_willensbekundung_date)"
    - "Wave-1 stub impls with #[allow(dead_code)] keep the workspace green until Wave-2 wires them"
    - "Response DTOs use skip_serializing_if = Option::is_none for optional auto-create signals (PartialRepaymentResponseTO.phase)"

key-files:
  created: []
  modified:
    - "genossi_service/src/membership_adjust.rs (trait extended)"
    - "genossi_service_impl/src/membership_adjust.rs (constants + helper + Plan-02 stub impl)"
    - "genossi_rest_types/src/lib.rs (request/response TOs)"

key-decisions:
  - "shares: i32 chosen (NOT i64) for consistency with MemberEntity.current_shares and RepaymentEntryEntity.share_count_to_pay_out (research Pitfall 2 / 4)"
  - "Return tuple ordering (Member, RepaymentEntry, Option<RepaymentPhase>) — freshly-loaded Member first so Wave-2 returns the entity it already loaded for the exit-date check without a re-read"
  - "Plan-02 stub returns ServiceError::InternalError so accidental calls fail loudly rather than silently corrupting data"
  - "Constants gated with #[allow(dead_code)] until Plan 16-02 wires them — keeps clippy clean without weakening the contract"
  - "Test file location: extended the existing mod tests block in genossi_service_impl/src/membership_adjust.rs instead of creating a new file (mirrors validate_willensbekundung_date_* tests)"

patterns-established:
  - "TDD RED/GREEN per task: failing tests committed first (df1defe), then implementation that turns them green (a0e4c39)"
  - "Wave-1 contracts compile-clean via stub impls + dead_code allowances; Wave-2 replaces stubs without touching the trait"
  - "PartialRepaymentResponseTO uses Option<RepaymentPhaseTO> with skip_serializing_if so the JSON payload only carries the auto-create signal when relevant"

requirements-completed:
  - PART-01
  - PART-06

# Metrics
duration: ~15 min
completed: 2026-06-05
---

# Phase 16 Plan 01: Service+REST Contracts for Teil-Rückgabe Summary

**MembershipAdjustService trait grew to a third method `partial_repayment` (i32 shares, tuple return with Option<RepaymentPhase>), the request/response TOs landed in genossi_rest_types with ToSchema, and the pure-function range validator + audit/default constants now back Wave-2 with 7 passing unit tests.**

## Performance

- **Duration:** ~15 min (interactive)
- **Started:** 2026-06-05T05:21:00Z
- **Completed:** 2026-06-05T05:36:28Z
- **Tasks:** 3 (1 trait extension, 1 DTO pair, 1 TDD constants+helper)
- **Files modified:** 3

## Accomplishments

- Trait `MembershipAdjustService` now declares `async fn partial_repayment(member_id, shares: i32, willensbekundung_date, context, tx) -> Result<(Member, RepaymentEntry, Option<RepaymentPhase>), ServiceError>` with full PART-06 doc-comment contract (no MemberAction, no current_shares mutation, no recalc_dates/recalc_migrated).
- `PartialRepaymentRequestTO` and `PartialRepaymentResponseTO` registered with `ToSchema` and `iso8601_date_required` serde; `phase` is omitted from JSON when None.
- Pure-function `validate_partial_repayment_shares(shares: i32, current_shares: i32)` covers the strict `1 <= shares < current_shares` range (D-16-11 cancel_membership hint + D-16-12 exceeds-message) with 7 passing tests.
- Constants `PARTIAL_REPAYMENT_PROCESS` and `DEFAULT_SHARE_VALUE_CENT` ready for Plan 16-02 wiring.
- Workspace builds clean (`cargo build --workspace`) and lints clean (`cargo clippy --workspace --all-targets`).

## Task Commits

1. **Task 1: Extend MembershipAdjustService trait with partial_repayment** — `a4080a8` (feat)
2. **Task 2: Add PartialRepaymentRequestTO + PartialRepaymentResponseTO** — `79d1e83` (feat)
3. **Task 3a: Failing tests for validate_partial_repayment_shares (RED)** — `df1defe` (test)
4. **Task 3b: Constants + validate_partial_repayment_shares impl (GREEN)** — `a0e4c39` (feat)

## Final Trait Signature

```rust
async fn partial_repayment(
    &self,
    member_id: Uuid,
    shares: i32,                              // NOT i64
    willensbekundung_date: time::Date,
    context: Authentication<Self::Context>,
    tx: Option<Self::Transaction>,
) -> Result<(Member, RepaymentEntry, Option<RepaymentPhase>), ServiceError>;
```

## Final Constant Values

```rust
const PARTIAL_REPAYMENT_PROCESS: &str = "member-adjust.partial-repayment";
pub(crate) const DEFAULT_SHARE_VALUE_CENT: i64 = 10000;  // 100 EUR/Anteil
```

## Final Pure-Function Signature

```rust
pub(crate) fn validate_partial_repayment_shares(
    shares: i32,
    current_shares: i32,
) -> Result<(), Vec<ValidationFailureItem>>
```

7 unit tests, all passing:

| # | Test | Input | Expected |
|---|------|-------|----------|
| 1 | zero_rejected | shares=0, current=5 | Err, "at least 1" |
| 2 | negative_rejected | shares=-5, current=5 | Err, "at least 1" |
| 3 | equal_to_current_rejected_with_cancel_hint | shares=10, current=10 | Err, contains "cancel_membership" |
| 4 | above_current_rejected | shares=11, current=10 | Err, "exceeds current_shares" |
| 5 | full_one_member_rejected | shares=1, current=1 | Err, contains "cancel_membership" |
| 6 | happy_path_minimum | shares=1, current=2 | Ok |
| 7 | happy_path_middle | shares=5, current=10 | Ok |

## Files Created/Modified

- `genossi_service/src/membership_adjust.rs` — Added trait method `partial_repayment` plus `use crate::repayment_entry::RepaymentEntry` and `use crate::repayment_phase::RepaymentPhase` imports. Doc-comment references PART-01, PART-03, PART-05, PART-06, D-16-10, D-16-18, D-16-19.
- `genossi_service_impl/src/membership_adjust.rs` — Added `PARTIAL_REPAYMENT_PROCESS` + `DEFAULT_SHARE_VALUE_CENT` constants near existing `CANCEL_PROCESS`/`UPGRADE_PROCESS`; added `validate_partial_repayment_shares` pure helper after `validate_willensbekundung_date`; added 7 unit tests at the end of the existing `mod tests` block; added Plan-16-02 stub impl of `partial_repayment` returning `ServiceError::InternalError`.
- `genossi_rest_types/src/lib.rs` — Added `PartialRepaymentRequestTO` and `PartialRepaymentResponseTO` immediately after `MembershipAdjustResponseTO`. Reused existing `iso8601_date_required` serde, `RepaymentEntryTO`, `RepaymentPhaseTO`, `MemberTO` (all in scope at module-level).

## Decisions Made

- **Type for shares: i32** (NOT i64). CONTEXT.md uses i64 in narrative prose but D-16-15 / D-16-12 and the RESEARCH document's Pitfalls 2 + 4 lock it to i32 for consistency with the rest of the codebase. The TO and the trait both use i32; the `DEFAULT_SHARE_VALUE_CENT` constant remains i64 because it mirrors `RepaymentPhase.share_value` (currency-cents).
- **Return tuple order: (Member, RepaymentEntry, Option<RepaymentPhase>).** The plan text shows both orderings; I followed the action block's explicit code sample. Member first lets Plan 16-02 return the entity it already loaded (for the gekündigt-Check) without a re-read.
- **Doc-comment phrasing.** Mirrored Phase 15's style: bullet list of business rules, PART-06 contract called out as a separate bold block, explicit "KEINE MemberAction" / "mutiert current_shares NICHT" / "recalc_* NICHT" statements.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Stub impl required to keep workspace green**
- **Found during:** Task 1 (trait extension)
- **Issue:** Plan 16-01 only changes the trait, but `MembershipAdjustServiceImpl` in `genossi_service_impl/src/membership_adjust.rs` `impl`s the trait. Adding a method without implementing it triggers `E0046: not all trait items implemented`, breaking `cargo build --workspace` and blocking downstream tasks.
- **Fix:** Added a stub `partial_repayment` impl returning `Err(ServiceError::InternalError("not yet implemented (Plan 16-02)"))`. Documented in doc-comment that Plan 16-02 replaces it. Also added matching `use ... ::{RepaymentEntry, RepaymentPhase}` imports.
- **Files modified:** `genossi_service_impl/src/membership_adjust.rs`
- **Verification:** `cargo build --workspace` finished clean; `cargo clippy --workspace --all-targets` finished clean.
- **Committed in:** `a4080a8` (Task 1 commit)

**2. [Rule 3 - Blocking] #[allow(dead_code)] on Wave-1 constants/helper**
- **Found during:** Task 3 (GREEN step)
- **Issue:** Constants `PARTIAL_REPAYMENT_PROCESS`/`DEFAULT_SHARE_VALUE_CENT` and the helper `validate_partial_repayment_shares` are intentionally unused in Plan 16-01 (they get wired in Plan 16-02). Clippy would flag them as dead code and the verification step `cargo clippy --workspace --all-targets` would fail.
- **Fix:** Annotated all three with `#[allow(dead_code)]` plus inline comments pointing to Plan 16-02 (`wired into ... in Plan 16-02`).
- **Files modified:** `genossi_service_impl/src/membership_adjust.rs`
- **Verification:** Clippy is clean. The allow can be removed in Plan 16-02 when the wiring happens.
- **Committed in:** `a0e4c39` (Task 3 GREEN commit)

---

**Total deviations:** 2 auto-fixed (both Rule 3 — blocking compile/lint issues caused by the Wave-1 / Wave-2 split)
**Impact on plan:** Both fixes are mechanical scaffolding needed for the contracts-only nature of Plan 16-01. No scope creep, no architectural changes, no behavioral semantics added beyond what the plan locks down.

## Issues Encountered

- `cargo build -p genossi_service` standalone fails with pre-existing E0433 against the `utoipa` crate (auth_types.rs references it but the package manifest doesn't declare it directly). This is a pre-existing condition on the base commit, verified by stashing and re-running. Worked around by running `cargo build --workspace` instead, which is the verification step the plan actually mandates.

## TDD Gate Compliance

| Gate | Commit | Status |
|------|--------|--------|
| RED (Task 3) | `df1defe` (`test(16-01): ...`) | ✓ tests fail to compile (E0425) before impl |
| GREEN (Task 3) | `a0e4c39` (`feat(16-01): ...`) | ✓ all 7 tests pass after impl |
| REFACTOR | — | not needed; code is already minimal and tested |

Tasks 1 and 2 are structural contract additions (trait method declaration and DTO structs). Their "RED" is the workspace-compile failure that the planned acceptance criteria already capture; both tasks were committed in single feat commits per plan instruction.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

Wave-2 (Plan 16-02 service impl) can:
- Replace the `partial_repayment` stub with the real implementation (Permission-Funnel, Range-Validation via `validate_partial_repayment_shares`, `compute_effective_date` reuse, `ensure_repayment_phase` inlining per Resolved Question #1 Variante b, Sum-Check via `find_by_member_and_phase`, `audited_create!(RepaymentEntry, PARTIAL_REPAYMENT_PROCESS, ...)`).
- Remove the three `#[allow(dead_code)]` annotations once the constants/helper are wired.
- Reuse `PartialRepaymentRequestTO`/`PartialRepaymentResponseTO` directly in Plan 16-04 REST handler without additional schema work.

No blockers. Workspace is clean (build + clippy).

## Self-Check: PASSED

- File `genossi_service/src/membership_adjust.rs` modified — FOUND
- File `genossi_service_impl/src/membership_adjust.rs` modified — FOUND
- File `genossi_rest_types/src/lib.rs` modified — FOUND
- Commit `a4080a8` — FOUND in `git log`
- Commit `79d1e83` — FOUND in `git log`
- Commit `df1defe` — FOUND in `git log`
- Commit `a0e4c39` — FOUND in `git log`
- All 7 validate_partial_repayment_shares tests pass (`cargo test -p genossi_service_impl --lib validate_partial_repayment_shares` → 7 passed, 0 failed)
- `cargo build --workspace` exits 0
- `cargo clippy --workspace --all-targets` exits 0

---
*Phase: 16-service-rest-teil-rueckgabe-auto-anlegen-phase*
*Completed: 2026-06-05*
