---
phase: 15-service-rest-kuendigung-aufstockung
verified: 2026-06-04T12:00:00Z
status: passed
score: 11/11 must-haves verified
overrides_applied: 0
---

# Phase 15: Service+REST Kündigung + Aufstockung Verification Report

**Phase Goal:** Implement v1.2 Mitgliedschaft-Anpassungen "Kündigung" + "Aufstockung" as REST endpoints with full audit-pattern, atomic-write-transactions, ADMIN_PRIVILEGE permission funnel, and 9+ E2E tests including audit-chain verification.
**Verified:** 2026-06-04T12:00:00Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | MembershipAdjustService trait exists with cancel_membership + increase_shares | VERIFIED | `genossi_service/src/membership_adjust.rs` — `pub trait MembershipAdjustService` with both methods; `#[automock]` present; module registered in `genossi_service/src/lib.rs:15` |
| 2 | validate_willensbekundung_date pure function defined as pub(crate) free-function | VERIFIED | `genossi_service_impl/src/membership_adjust.rs:284` — `pub(crate) fn validate_willensbekundung_date(date: Date, today: Date) -> Vec<ValidationFailureItem>` — no `now_utc()` call inside |
| 3 | recalc_dates refactored to pub(crate) free-function; existing method delegates | VERIFIED | `genossi_service_impl/src/member_action.rs:184` — `pub(crate) async fn recalc_dates<Md, Mad, Tx>(...)`; wrapper method delegates via `recalc_dates(&*self.member_dao, ...)` |
| 4 | 6+ Edge-Case-Unit-Tests for validate_willensbekundung_date are green | VERIFIED | Lines 375-420 in `membership_adjust.rs` — 6 tests: aktuelles_jahr_valid, naechstes_jahr_valid, vorjahr_invalid, uebernaechstes_jahr_invalid, today_31_dezember, schaltjahr. All in `cargo test -p genossi_service_impl --lib membership_adjust` → 20 passed |
| 5 | cancel_membership: ADMIN_PRIVILEGE check, date validation, EntityNotFound, Already-Cancelled (409), audited_create! with CANCEL_PROCESS, recalc_dates, returns (MemberAction, Member) | VERIFIED | `membership_adjust.rs:45-132` — all steps present in correct order; `CANCEL_PROCESS = "member-adjust.cancel"` at line 24; `crate::audited_create!` at line 104; `crate::member_action::recalc_dates` at line 114 |
| 6 | cancel_membership: shares_change=0, effective_date=Some(compute_effective_date(...).effective_date) | VERIFIED | Lines 93-95: `shares_change: 0, effective_date: Some(effective.effective_date)` |
| 7 | AUDT-01 grep gate: no direct member_action_dao.create / member_dao.update outside macros | VERIFIED | `grep -v '^//' ... | grep -cE 'self\.member_action_dao\.create\(|self\.member_dao\.update\('` returns 0 |
| 8 | CANC-05: No ActionType::Verkauf or RepaymentEntry created | VERIFIED | `grep -c 'ActionType::Verkauf\|RepaymentEntry' membership_adjust.rs` returns 0 |
| 9 | CANC-04: No direct member_dao.update_dates / update_migrated in membership_adjust.rs | VERIFIED | `grep -c 'member_dao\.update_dates\|member_dao\.update_migrated' membership_adjust.rs` returns 0 |
| 10 | increase_shares: ADMIN_PRIVILEGE, shares>0 validation, date validation, UPGD-04 block for cancelled member, audited_create! + audited_update! both with UPGRADE_PROCESS | VERIFIED | Lines 134-239 — all steps present; `UPGRADE_PROCESS = "member-adjust.upgrade"` at line 27; `crate::audited_create!` at line 203; `crate::audited_update!` at line 226 |
| 11 | REST endpoints registered, DTOs defined, DI wired, 9 active E2E tests green (2 ignored with documented reason) | VERIFIED | Routes at `genossi_rest/src/member.rs:65,69`; DTOs in `genossi_rest_types/src/lib.rs:513,524,541`; DI at `genossi_bin/src/lib.rs:605,724,1107,1828`; 11 E2E test functions total, 2 `#[ignore]` for mock_auth admin-only context — 9 active pass |

**Score:** 11/11 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `genossi_service/src/membership_adjust.rs` | MembershipAdjustService trait + Mock | VERIFIED | `pub trait MembershipAdjustService` with `#[automock]` |
| `genossi_service/src/lib.rs` | Module registration | VERIFIED | `pub mod membership_adjust;` at line 15 |
| `genossi_service_impl/src/membership_adjust.rs` | Full impl + 20 tests | VERIFIED | MembershipAdjustServiceImpl via gen_service_impl!, 12 pure-function tests + 8 service tests |
| `genossi_service_impl/src/member_action.rs` | recalc_dates as free-function | VERIFIED | `pub(crate) async fn recalc_dates<Md, Mad, Tx>` at line 184 |
| `genossi_rest_types/src/lib.rs` | 3 new TOs | VERIFIED | CancelMembershipRequestTO, IncreaseSharesRequestTO, MembershipAdjustResponseTO |
| `genossi_rest/src/membership_adjust.rs` | 2 Axum handlers | VERIFIED | `pub async fn cancel_membership` + `pub async fn increase_shares` with utoipa annotations |
| `genossi_rest/src/member.rs` | Sub-route registration | VERIFIED | `/{id}/cancel` at line 65, `/{id}/increase-shares` at line 69, before `/{id}` catch-all |
| `genossi_rest/src/lib.rs` | RestStateDef::MembershipAdjustService | VERIFIED | `type MembershipAdjustService` at line 230 + trait method at line 1828 |
| `genossi_bin/src/lib.rs` | DI wiring | VERIFIED | `membership_adjust_service: Arc<MembershipAdjustService>` at line 605; construction at 724; Self-init at 1107 |
| `genossi_bin/tests/membership_adjust_e2e.rs` | 9+ active E2E tests | VERIFIED | 11 total test functions; 2 `#[ignore]` (documented: mock_auth always-admin, 401-path covered by service unit tests); 9 active |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `member::generate_route` | `membership_adjust::cancel_membership + increase_shares` | `.route("/{id}/cancel", ...)` | WIRED | `genossi_rest/src/member.rs:65-73` |
| `cancel_membership` handler | `rest_state.membership_adjust_service()` | `membership_adjust_service()` call | WIRED | `genossi_rest/src/membership_adjust.rs:59` |
| `increase_shares` handler | `rest_state.membership_adjust_service()` | `membership_adjust_service()` call | WIRED | `genossi_rest/src/membership_adjust.rs:104` |
| `RestStateImpl::new` | `MembershipAdjustServiceImpl::new` | `Arc::new(...)` with 6 deps | WIRED | `genossi_bin/src/lib.rs:724` |
| `cancel_membership` impl | `audited_create!` with CANCEL_PROCESS | macro invocation | WIRED | `membership_adjust.rs:104-111` |
| `cancel_membership` impl | `crate::member_action::recalc_dates` | free-function call | WIRED | `membership_adjust.rs:114-120` |
| `increase_shares` impl | `audited_create!` + `audited_update!` with UPGRADE_PROCESS | macro invocations | WIRED | `membership_adjust.rs:203-234` |
| `cancel_membership` impl | `check_permission(ADMIN_PRIVILEGE, ...)` | permission funnel | WIRED | `membership_adjust.rs:62` |

### Data-Flow Trace (Level 4)

Not applicable for this phase — all artifacts are service/REST logic, not frontend components rendering dynamic data. The E2E tests verify end-to-end data flow through real HTTP+DB.

### Behavioral Spot-Checks

| Behavior | Result | Status |
|----------|--------|--------|
| `cargo build --workspace` | exit 0 (confirmed by orchestrator) | PASS |
| `cargo test -p genossi_bin --test membership_adjust_e2e` | 9 passed, 2 ignored, 0 failed (confirmed by orchestrator) | PASS |
| `cargo test -p genossi_service_impl --lib membership_adjust` | 20 passed, 0 failed (confirmed by orchestrator) | PASS |
| `cargo test --workspace --lib` | 370 passed, 0 failed (confirmed by orchestrator) | PASS |
| `/api/audit/verify` returns valid=true after cancel + increase ops | Asserted in E2E tests test_cancel_membership_audit_chain_verify and test_increase_shares_audit_chain_verify | PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|---------|
| CANC-01 | 15-02, 15-04 | cancel_membership erzeugt MemberAction::Austritt | SATISFIED | `action_type: ActionType::Austritt` in impl body + E2E asserts `action_type == "Austritt"` |
| CANC-03 | 15-02, 15-04 | shares_change=0, effective_date=H1/H2-Stichtag | SATISFIED | Lines 93-95 in impl; E2E asserts shares_change==0 and effective_date |
| CANC-04 | 15-02, 15-04 | exit_date via recalc_dates only | SATISFIED | No direct update_dates/update_migrated in membership_adjust.rs; recalc_dates called; E2E asserts member.exit_date set |
| CANC-05 | 15-02, 15-04 | No Verkauf/RepaymentEntry created | SATISFIED | grep count = 0 for both patterns |
| UPGD-01 | 15-03, 15-04 | increase_shares erzeugt MemberAction::Aufstockung | SATISFIED | `action_type: ActionType::Aufstockung` in impl body + E2E |
| UPGD-02 | 15-03, 15-04 | effective_date=None (sofort wirksam) | SATISFIED | `effective_date: None` in impl; E2E asserts field absent in JSON (skip_serializing_if) |
| UPGD-03 | 15-03, 15-04 | current_shares atomic bump via audited_update! | SATISFIED | `audited_update!` with member_dao at line 226; E2E asserts current_shares increases |
| UPGD-04 | 15-03, 15-04 | Block cancelled members (HTTP 400) | SATISFIED | exit_date.is_some() -> ValidationError -> 400; E2E test_increase_shares_cancelled_member_blocked asserts 400 |
| PERM-01 | 15-02, 15-03, 15-04 | Admin-only via ADMIN_PRIVILEGE | SATISFIED | `check_permission(ADMIN_PRIVILEGE, context)` in both methods; service unit tests cover PermissionDenied path |
| PERM-02 | 15-01, 15-02, 15-03, 15-04 | Server-layer date validation | SATISFIED | `validate_willensbekundung_date` pure function + 6 edge-case tests; E2E tests previous-year and overnext-year rejections |
| AUDT-01 | 15-02, 15-03, 15-04 | All ops via audited_*! macros, 0 direct DAO calls | SATISFIED | 2x audited_create! + 1x audited_update! in impl; AUDT-01 grep gate confirms 0 direct calls; audit-verify E2E tests pass |

### Anti-Patterns Found

None. Scan results:
- `TODO/FIXME/PLACEHOLDER` in `membership_adjust.rs`: 0 hits in non-test code
- `return null/{}` stubs: 0 hits in impl methods
- Direct DAO create/update outside macros: 0 hits (AUDT-01 grep gate passed)
- `increase_shares — Plan 03` stub string: 0 hits (replaced by full impl)
- `ActionType::Verkauf` or `RepaymentEntry`: 0 hits

### Human Verification Required

None. All must-haves are verifiable programmatically. The 2 permission-denied E2E tests are `#[ignore]` with a clear documented reason (mock_auth always injects admin context from the default migration). The 401 path is covered at service unit-test level where `ServiceError::PermissionDenied` is asserted, which maps to HTTP 401 via the global `From<ServiceError> for RestError` in `genossi_rest/src/lib.rs:115`. This is an acceptable and documented design decision.

### Gaps Summary

No gaps. All 11 ROADMAP success criteria are met:

1. cancel_membership produces MemberAction::Austritt with correct effective_date and shares_change=0; recalc_dates sets Member.exit_date; 5 E2E tests (H1, H2, permission-denied#[ignore], already-cancelled 409, audit-chain-verify).
2. increase_shares produces MemberAction::Aufstockung with shares_change=+n and atomic current_shares bump; 4 E2E tests (happy-path, cancelled-block 400, permission-denied#[ignore], audit-chain-verify).
3. Server-layer date validation rejects out-of-bounds dates (HTTP 400); 2 edge-case E2E tests.
4. cargo test --test e2e_tests passes (1 pre-existing failure in mail-preview, documented in deferred-items.md, predates Phase 15); v1.1 audit-hashchain verified valid by E2E tests.

The 2 ignored permission-denied E2E tests are a known infrastructure limitation of mock_auth (DEVUSER is always admin per migration) and are documented in the test file header. The service-layer unit tests `test_cancel_membership_permission_denied` and `test_increase_shares_permission_denied` cover this path. This was anticipated in the plan's must_haves and is not a gap.

---

_Verified: 2026-06-04T12:00:00Z_
_Verifier: Claude (gsd-verifier)_
