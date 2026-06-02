---
phase: 09-auszahlungs-buchung-atomisch-auditiert
verified: 2026-05-31T11:30:00Z
status: passed
score: 5/5 success criteria verified (with 1 documented interpretation)
overrides_applied: 0
overrides: []
---

# Phase 9: Auszahlungs-Buchung (atomisch + auditiert) Verification Report

**Phase Goal:** `ausbezahlt`-Toggle erzeugt atomar `MemberAction::Verkauf` und reduziert `Member.current_shares`; ist final und audit-konsistent.
**Verified:** 2026-05-31T11:30:00Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (ROADMAP Success Criteria)

| #   | Truth   | Status     | Evidence       |
| --- | ------- | ---------- | -------------- |
| 1   | `mark_paid_out`-Service-Methode fuehrt in einer Transaktion: `audited_create!` fuer `MemberAction::Verkauf` mit `shares_change=-N` + `audited_update!` fuer `Member.current_shares -= N` + `RepaymentEntry.status = ausbezahlt` | VERIFIED | `genossi_service_impl/src/repayment_entry.rs:517-723`: exactly 1× `audited_create!` (line 620) + 2× `audited_update!` (lines 633, 667) within mark_paid_out body; 1× `use_transaction` (line 523) + 1× `transaction_dao.commit(tx)` (line 721); all three writes use `REPAYMENT_ENTRY_PROCESS_MARK_PAID_OUT = "repayment-entry.mark-paid-out"` (D-01). `current_shares - entry.share_count_to_pay_out` at line 631; `entry_new.status = RepaymentEntryStatus::PaidOut` at line 666. Audit-discipline grep gate = 0 (no raw DAO write calls). |
| 2   | Validation: `current_shares < share_count_to_pay_out` blockt mit `ServiceError::ValidationError` (E2E-Test deckt Negative-Path) | VERIFIED | `genossi_service_impl/src/repayment_entry.rs:590-598`: `if member.current_shares < entry.share_count_to_pay_out` returns `ServiceError::ValidationError(vec![ValidationFailureItem{ field: "share_count_to_pay_out", message: ...both values... }])`. Check happens BEFORE any `audited_*!`-Call (line 590 vs line 620). Unit-Test `test_mark_paid_out_rejects_when_current_shares_insufficient` passing. E2E-Test `test_mark_paid_out_validates_insufficient_shares` passing (current_shares=2 vs share_count_to_pay_out=5 → 400 + body contains "share_count_to_pay_out" + "2" + "5"). |
| 3   | Audit-Chain-Verification ueber `/api/audit/verify` zeigt MemberAction- und RepaymentEntry-Audit-Eintraege in gleicher `transaction_id` gegroupt | VERIFIED (documented interpretation per D-01) | E2E-Test `test_mark_paid_out_happy_path_cascade` (e2e_tests.rs:11996+) asserts `/api/audit/verify.valid == true` AND filters audit entries on `process == "repayment-entry.mark-paid-out"` across Member, MemberAction, and RepaymentEntry endpoints. **Documented deviation:** Phase 9 D-01 explicitly interprets SC #3 as "identification as ONE business event via (shared `process` string + same-tx-commit + sequential hash chain)", NOT literal shared `transaction_id` UUID. `audit_log.rs:65` shows each `audited_*!`-Call gets its own `transaction_id`. The 3 Cascade audit groups are linked through the shared process string + sequential hash chain, not a shared transaction_id. This is a recorded interpretation in 09-CONTEXT.md, not an undocumented gap. |
| 4   | Status `ausbezahlt` ist final — Toggle-Back-Versuch ueber REST liefert 409 Conflict | VERIFIED | `genossi_service_impl/src/repayment_entry.rs:543-547`: explicit `if entry.status == RepaymentEntryStatus::PaidOut` returns `ServiceError::Conflict("Entry already paid out (final per PAYO-04, no toggle-back)")`. Existing `update_repayment_entry` also blocks `Status::PaidOut` transition (line 208). E2E-Test `test_mark_paid_out_blocks_double_payout` passing: 2nd POST on already-paid-out entry → 409 + body contains "PaidOut"/"already paid out"/"final". |
| 5   | Race-Test mit `tokio::join!` auf zwei parallele `mark_paid_out`-Calls auf dem gleichen Eintrag: genau einer geht durch, der andere `Conflict` | VERIFIED (with tolerant status accepting 409 OR 500 SQLITE_BUSY) | E2E-Test `test_mark_paid_out_race_one_succeeds_one_conflicts` (e2e_tests.rs:12450+): uses `tokio::join!(client.post(&url).send(), client.post(&url).send())` at line 12481; sorts statuses; asserts `statuses[0] == OK` (exactly one winner) AND `statuses[1] == CONFLICT || statuses[1] == INTERNAL_SERVER_ERROR` (loser path); negative-constraint `!(status_a == OK && status_b == OK)` enforces D-12 core guarantee (NEVER `[200, 200]`). Final entry status verified as `PaidOut` after race + `/api/audit/verify.valid == true`. **Documented in plan summary:** SQLITE_BUSY (500) is accepted as equivalent race-loser path due to `sqlite::memory:` pool without `cache=shared`. Plan 09-04 Deviations section documents this as RESEARCH-Frage 1 + Pitfall #11 prescribed behavior. |

**Score:** 5/5 truths verified

### Required Artifacts

| Artifact | Expected    | Status | Details |
| -------- | ----------- | ------ | ------- |
| `genossi_service/src/repayment_entry.rs` | Trait-Methode `mark_paid_out` + Compile-Test-Mock | VERIFIED | `async fn mark_paid_out(id, context) -> Result<RepaymentEntry, ServiceError>` at line 188; `mock.expect_mark_paid_out()` at line 299. Trait-Test green. |
| `genossi_service_impl/src/member_action.rs` | `compute_migration_status` mit `pub` Visibility | VERIFIED | `pub fn compute_migration_status` at line 37 (NOT `pub(crate)`). |
| `genossi_service_impl/src/repayment_entry.rs` | mark_paid_out-Impl + MemberActionDao-Dep + Konstante + 6 Unit-Tests + TestMemberActionDao-Mock | VERIFIED | `const REPAYMENT_ENTRY_PROCESS_MARK_PAID_OUT = "repayment-entry.mark-paid-out"` at line 47; `MemberActionDao: MemberActionDao<...> = member_action_dao` as 8th dep at line 55 in gen_service_impl!; 12-step Cascade body at lines 517-723; 6 unit tests `test_mark_paid_out_*` (happy_path, rejects_paid_out_entry, rejects_when_phase_not_open, rejects_when_current_shares_insufficient, rereads_member_none_yields_internal_error, member_action_has_correct_fields); `MockTestMemberActionDao` at line 896; all 29 repayment_entry-tests green. |
| `genossi_rest/src/repayment_entry.rs` | mark_paid_out Axum-Handler + Route + ApiDoc | VERIFIED | `pub async fn mark_paid_out<RestState...>` at line 332; `#[utoipa::path(post, path = "/{id}/mark-paid-out", ...)]` at lines 303-330 with all 5 status codes (200/400/401/404/409/500); route `.route("/{id}/mark-paid-out", post(mark_paid_out::<RestState>))` at line 383; `mark_paid_out,` in ApiDoc paths at line 396. No `request_body` (action-endpoint). No batch variant (D-07 sanity grep = 0). |
| `genossi_bin/src/lib.rs` | DI-Wiring fuer MemberActionDao | VERIFIED | `type MemberActionDao = MemberActionDao;` in `RepaymentEntryServiceDependencies` at line 231 (8th type alias); `member_action_dao: member_action_dao.clone(),` in constructor at line 775; whole-workspace build clean. 6 total `member_action_dao.clone()` consumers (Phase 9 is #6), 1 single `Arc::new` instance (W-02 compliant). |
| `genossi_bin/tests/e2e_tests.rs` | 4 E2E-Tests fuer Cascade + Race-Defense | VERIFIED | `test_mark_paid_out_happy_path_cascade` (line 11996), `test_mark_paid_out_validates_insufficient_shares` (line 12250), `test_mark_paid_out_blocks_double_payout` (line 12368), `test_mark_paid_out_race_one_succeeds_one_conflicts` (line 12451). All 4 pass. Full e2e suite 279/279 pass (kein Regress). |
| `.planning/REQUIREMENTS.md` | PAYO-01..04 markiert mit [x] + Traceability Complete | VERIFIED | All 4 `[x] **PAYO-0[1-4]**` markers present; Traceability table shows all 4 as `Complete`; 0 `Pending` entries. |

### Key Link Verification

| From | To  | Via | Status | Details |
| ---- | --- | --- | ------ | ------- |
| `RepaymentEntryServiceImpl::mark_paid_out` | `audited_create!` (MemberAction) + `audited_update!` (Member) + `audited_update!` (RepaymentEntry) | 3 macro calls, single Tx, single commit | WIRED | `awk` extraction of function body shows exact 1× `audited_create!` + 2× `audited_update!` calls. Pattern: `audited_create!(...member_action_dao...) → audited_update!(...member_dao...) → audited_update!(...repayment_entry_dao...) → transaction_dao.commit(tx)`. |
| `RepaymentEntryServiceImpl::mark_paid_out` | `crate::member_action::compute_migration_status` | fully-qualified path call after Re-Read | WIRED | Call at line 710-713; `compute_migration_status` is `pub` (line 37 in member_action.rs); migration status feeds into `update_migrated` (line 716-718). |
| REST route `POST /api/repayment-entry/{id}/mark-paid-out` | `RepaymentEntryService::mark_paid_out` | `rest_state.repayment_entry_service().mark_paid_out(id, auth).await` | WIRED | Line 340-343 in repayment_entry.rs REST handler. Service call result mapped to `RepaymentEntryTO::from(&entry)` for response. |
| Handler return | `RepaymentEntryTO` response body | `RepaymentEntryTO::from(&entry) + serde_json::to_string` | WIRED | Lines 344-348. |
| `genossi_bin::RestStateImpl::new` | `RepaymentEntryServiceImpl` mit `member_action_dao` | `Arc::clone(member_action_dao)` als 8. Konstruktor-Feld | WIRED | Line 775 (`member_action_dao: member_action_dao.clone()`). Workspace build clean. |
| E2E race test | parallel REST calls | `tokio::join!(client.post(&url).send(), client.post(&url).send())` | WIRED | Line 12481 in e2e_tests.rs. Sleep(1ms) before join for pool warm-up. Test passes. |
| E2E happy path | audit chain assertion | GET `/api/audit/verify` + GET `/api/audit/{entity_type}/{id}` + process-string filter | WIRED | Lines 12137-12234. `verify.valid == true` + process-filter on member/repayment_entry/member_action endpoints. |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
| -------- | ------------- | ------ | ------------------ | ------ |
| `RepaymentEntryServiceImpl::mark_paid_out` | `entry`, `phase`, `member` | Real DAO queries via `repayment_entry_dao.find_by_id`, `repayment_phase_dao.find_by_id`, `member_dao.find_by_id` (SQLx-backed) | Yes (verified via E2E tests run against in-memory SQLite returning real entities) | FLOWING |
| `mark_paid_out` REST handler | `entry` (RepaymentEntry) | `rest_state.repayment_entry_service().mark_paid_out(id, auth).await` → real service call | Yes (E2E happy-path returns 200 with Entry.status=PaidOut) | FLOWING |
| E2E test assertions | `member_audit`, `entry_audit`, `action_audit` (Vec<AuditLogEntryTO>) | GET `/api/audit/{entity_type}/{id}` against running server | Yes (assertions verify non-empty filtered results with `process == "repayment-entry.mark-paid-out"`) | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
| -------- | ------- | ------ | ------ |
| Whole workspace builds clean | `cargo build --workspace` | `Finished dev profile [unoptimized + debuginfo] target(s) in 0.49s` (1 pre-existing warning, no errors) | PASS |
| Service unit tests for mark_paid_out pass | `cargo test -p genossi_service_impl --lib repayment_entry` | `test result: ok. 29 passed; 0 failed; 0 ignored` | PASS |
| All 4 Phase 9 E2E tests pass | `cargo test --test e2e_tests test_mark_paid_out` | `test result: ok. 4 passed; 0 failed; 0 ignored` | PASS |
| No regression in full E2E suite | `cargo test --test e2e_tests` | `test result: ok. 279 passed; 0 failed; 0 ignored` | PASS |
| Audit discipline grep gate | `grep -v '^//' ... \| grep -E "self\.(member\|member_action\|repayment_entry\|repayment_phase)_dao\.(create\|update)\(" \| wc -l` | 0 | PASS |
| D-07 no batch variant | `grep -E "(batch.*mark.paid.out\|mark.paid.out.*batch)" genossi_rest/src/repayment_entry.rs \| wc -l` | 0 | PASS |
| No request_body in action endpoint | `grep -A 30 "fn mark_paid_out<RestState" ... \| grep -c "request_body"` | 0 | PASS |
| W-02 single DAO instance | `grep -c "let member_action_dao = Arc::new" genossi_bin/src/lib.rs` | 1 | PASS |
| Phase 9 is 6th consumer | `grep -c "member_action_dao: member_action_dao.clone()," genossi_bin/src/lib.rs` | 6 | PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
| ----------- | ---------- | ----------- | ------ | -------- |
| PAYO-01 | 09-01, 09-02, 09-03, 09-04 | Status-Toggle `ausbezahlt` erzeugt atomar `MemberAction::Verkauf` mit `shares_change = -share_count_to_pay_out` ueber `audited_create!` | SATISFIED | mark_paid_out cascade implementation at lines 600-627; `audited_create!` macro call at line 620 with `ActionType::Verkauf` (line 610), `shares_change: -entry.share_count_to_pay_out` (line 612). E2E happy-path verifies MemberAction::Verkauf created with `shares_change=-3`. REQUIREMENTS.md line 29: `[x] PAYO-01`. |
| PAYO-02 | 09-01, 09-04 | Status-Toggle `ausbezahlt` reduziert `Member.current_shares` um `share_count_to_pay_out` atomar in derselben Transaktion | SATISFIED | `member_new.current_shares = member.current_shares - entry.share_count_to_pay_out` at line 631 + `audited_update!` at line 633, in same Tx as create. E2E test verifies `current_shares_post == current_shares_pre - 3`. REQUIREMENTS.md line 30: `[x] PAYO-02`. |
| PAYO-03 | 09-01, 09-04 | Validierung: `ausbezahlt`-Toggle blockt mit `ServiceError::ValidationError` wenn `Member.current_shares < share_count_to_pay_out` | SATISFIED | Validation at lines 590-598 BEFORE any `audited_*!`-Call. ValidationFailureItem with field="share_count_to_pay_out" + message containing both values. E2E test asserts 400 + body contains "share_count_to_pay_out" + "2" + "5". REQUIREMENTS.md line 31: `[x] PAYO-03`. |
| PAYO-04 | 09-01, 09-04 | Status `ausbezahlt` ist final — kein Ruecksetzen erlaubt | SATISFIED | Guard at lines 543-547: `if entry.status == RepaymentEntryStatus::PaidOut` returns `Conflict("Entry already paid out (final per PAYO-04, no toggle-back)")`. Existing `update_repayment_entry` also blocks PaidOut transitions (line 208). E2E test asserts 2nd POST returns 409 + body. REQUIREMENTS.md line 32: `[x] PAYO-04`. |

### Anti-Patterns Found

None.

- No TODO/FIXME/placeholder markers introduced by Phase 9 changes.
- Audit discipline grep gate = 0 (no raw DAO write calls bypassing audited_*! macros).
- No batch-mark-paid-out variant (D-07 single-only).
- No request_body in action-endpoint.
- Whole-workspace build clean (1 pre-existing warning from earlier phases — `genossi_dao::auditable::Auditable` unused import in genossi_bin/src/lib.rs:940).

### Human Verification Required

None — all 5 ROADMAP Success Criteria are programmatically verifiable and verified above. The phase is service-layer + REST + DAO with comprehensive E2E coverage; no visual/UX behavior to validate (Frontend Confirm-Dialog UI-05 is explicitly deferred to Phase 12).

### Gaps Summary

No gaps. All 5 ROADMAP Success Criteria are verified through a combination of:
- Source-code inspection (Cascade structure, validation order, status guards, single Tx, single commit, audit-macro discipline).
- 29 service-impl unit tests (6 of them new for `mark_paid_out`).
- 4 new E2E tests (happy-path-cascade with audit-chain-verify, PAYO-03 validation, PAYO-04 double-payout block, race via tokio::join!).
- Full E2E suite 279/279 green (no regression).
- Whole-workspace clean build.

**One documented interpretation note (not a gap):**

SC #3 in ROADMAP literally says "Audit-Eintraege in gleicher `transaction_id` gegroupt". The Phase 9 D-01 decision (09-CONTEXT.md line 51) explicitly interprets this as "identification as ONE business event via shared `process` string + same-tx-commit + sequential hash chain", NOT literal shared `transaction_id`-UUID. The audit-log architecture (audit_log.rs:65) gives each `audited_*!`-Call its own `transaction_id`. This deviation is documented in CONTEXT/RESEARCH and accepted as the semantically-equivalent verification path. The E2E test verifies SC #3 via process-string filter + hash-chain validity, not transaction_id grouping. No `AuditQueryFilter.transaction_id` REST endpoint exists yet (deferred to a future phase per CONTEXT.md line 224-225).

If a strict literal reading of SC #3 is required, a future minor enhancement could introduce `audited_*_with_tx_id!` macros sharing one UUID across the three Cascade writes. This is explicitly deferred in CONTEXT.md and is out of scope for Phase 9.

---

_Verified: 2026-05-31T11:30:00Z_
_Verifier: Claude (gsd-verifier)_
