---
phase: 03-attendance-aggregat-cascade-invalidation
plan: 06
subsystem: rest-e2e
tags: [rest, axum, di-wiring, e2e, cascade, pii-test, race-test, hash-chain]

# Dependency graph
requires:
  - phase: 03-attendance-aggregat-cascade-invalidation
    plan: 01
    provides: AttendanceDaoImpl + AttendanceMemberRow (consumed by AttendanceServiceImpl, mounted via genossi_bin DI)
  - phase: 03-attendance-aggregat-cascade-invalidation
    plan: 02
    provides: HelperTokenDao::list_session_ids_for_assembly (verified end-to-end via the cascade E2E test)
  - phase: 03-attendance-aggregat-cascade-invalidation
    plan: 03
    provides: ClaimContext::as_helper bridge — exercised by the admin-pathway tests in this plan (Mock-Auth = no helper claim => admin branch)
  - phase: 03-attendance-aggregat-cascade-invalidation
    plan: 04
    provides: AttendanceMemberTO + AttendanceStatsTO + ListMembersQuery (the JSON-Schema and serde contracts the REST handlers serialize against)
  - phase: 03-attendance-aggregat-cascade-invalidation
    plan: 05
    provides: AttendanceServiceImpl + check_assembly_access permission funnel + close_assembly cascade body — wired to HTTP via the 4 handlers added in this plan
provides:
  - "4 REST handlers in genossi_rest/src/attendance.rs (list_attendance_members, mark_attendance_present, mark_attendance_absent, get_assembly_stats)"
  - "AttendanceRestState trait + 2 router builders (generate_attendance_route + generate_stats_route) — D-21 endpoints registered in genossi_rest::create_app"
  - "Local map_attendance_error: ServiceError::PermissionDenied -> RestError::Forbidden(403) for attendance endpoints (D-26 / RESEARCH §DECISION CONFLICT 1)"
  - "OpenAPI doc registration: AttendanceMemberTO + AttendanceStatsTO + ListMembersQuery schemas, 4 paths under tag=Attendance, mounted at /api/attendance/{assembly_id} in the global ApiDoc"
  - "DI-wiring in genossi_bin/src/lib.rs: AttendanceServiceImpl with 6 deps (AttendanceDao + AssemblyDao + MemberDao + AssemblyMemberSnapshotDao + PermissionService + TransactionDao); RestStateImpl carries attendance_service: Arc<AttendanceService>; impl AttendanceRestState for RestStateImpl"
  - "6 grüne E2E-Tests gegen real-laufenden HTTP-Server mit in-memory SQLite — alle 9 Phase-3-Requirements (ASSY-04, ASSY-06, ATTN-01..06, SYNC-02) sowie SC#8-Cascade-DB direkt verifiziert"
affects:
  - "Phase 4 (Frontend) — REST-Schemas (AttendanceMemberTO, AttendanceStatsTO) + Endpoint-Pfade (/api/attendance/{aid}/members, /{mid} PUT/DELETE, /api/assembly/{aid}/stats) ARE NOW STABLE; the helper-page and live-counter implementations can type-bind against utoipa-generated OpenAPI"
  - "Phase 5 (Operations) — the 60-burst api_rate_layer cap was re-validated empirically by the toggle-burst test (40 toggles + 4 surrounding REST calls fit comfortably under the cap). Phase-5 generalprobe should re-test with realistic GV-Tag traffic"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Differential ServiceError mapping per endpoint family: a local `map_attendance_error` keeps the global `From<ServiceError>` (PermissionDenied -> 401) intact for Phase 1+2 endpoints while attendance-specific handlers map PermissionDenied -> 403. Pattern is reusable for future endpoint-families that need a different status-code policy without breaking existing semantics (RESEARCH §DECISION CONFLICT 1 resolution)."
    - "Multi-namespace Axum routing for one logical service: `/api/attendance/{aid}` covers list/toggle endpoints while `/api/assembly/{aid}/stats` lives under the assembly namespace — both mounted via `Router::nest` with different prefixes so the routes coexist with `assembly::generate_route()` (already at `/api/assembly`). Pattern allows REST-aspect-grouping at the URL level even when the service-layer implementation is consolidated (D-21 / D-23)."
    - "DI-shared snapshot DAO: `assembly_member_snapshot_dao` is now `clone()`d (instead of moved) into AssemblyServiceImpl, so AttendanceServiceImpl can share the same Arc — exactly one `AssemblyMemberSnapshotDaoImpl` per process, consistent with the `helper_token_dao` sharing pattern from Plan 05."
    - "PII-leak guard via JSON-key whitelist+blacklist iteration: the E2E test enumerates every JSON key against an explicit allow-list of 7 fields AND a defensive blacklist of 12 forbidden PII keys (`email`, `iban`, `bank_account`, `street`, `house_number`, `postal_code`, `city`, `comment`, `join_date`, `exit_date`, `birth_date`, `phone`). Pattern catches future MemberTO field additions that silently propagate through the conversion layer (T-03-06-01 mitigation)."
    - "Audit-stille verification via paged-listing diff: count audit entries with `?entity_type=attendance` BEFORE and AFTER a toggle burst — equality proves D-08 / ATTN-05. Combined with `/api/audit/verify` for hash-chain integrity. Pattern is reusable for any `entity_type` where audit-stille is a contract (T-03-06-04 mitigation)."
    - "Cascade DB-effect verification: direct `sqlx::query_scalar` against the in-memory SQLite from `setup_with_pool()` exposes the post-close session-table state. Pattern catches cascade regressions that pure HTTP-level testing would miss (D-11/D-12 verification, SC#8)."

key-files:
  created:
    - "genossi_rest/src/attendance.rs (252 lines: 4 handlers + AttendanceRestState trait + 2 router builders + map_attendance_error + ApiDoc + 4 unit tests)"
    - ".planning/phases/03-attendance-aggregat-cascade-invalidation/03-06-SUMMARY.md (this file)"
  modified:
    - "genossi_rest/src/lib.rs (pub mod attendance + ApiDoc nest entry + 2 .nest() calls in create_app + AttendanceRestState bound on create_app/start_server)"
    - "genossi_rest/src/test_server.rs (+ AttendanceRestState bound on start_test_server so RestStateImpl tests reach the new handlers)"
    - "genossi_bin/src/lib.rs (type alias AttendanceDao + AttendanceServiceDependencies + AttendanceService + RestStateImpl.attendance_service field + RestStateImpl::new() construction + impl AttendanceRestState for RestStateImpl)"
    - "genossi_bin/tests/e2e_tests.rs (+ 4 new TO imports + create_open_assembly_with_members helper + 6 E2E tests)"

key-decisions:
  - "Hash-chain test burst reduced from 100 to 40 toggles (Plan-spec said 100, but 100 + the 4 surrounding REST calls would exceed the global 60-burst api_rate_layer cap and produce a 429 mid-burst). 40 toggles is sufficient to prove ATTN-05 — what matters is the audit invariant (count_before == count_after), not the magnitude. Documented as inline comment + Deviation 1 below."
  - "Stats endpoint registered as a SEPARATE Router::nest under `/api/assembly/{assembly_id}` (rather than being merged into the attendance route). This avoids collision with assembly::generate_route() (which is already nested at /api/assembly) and keeps the URL semantics aligned with D-21 (stats is an assembly aspect even though the implementation lives in AttendanceService)."
  - "ListMembersQuery DTO is registered in OpenAPI as a `ToSchema` on top of `IntoParams` so the schema list in the ApiDoc is complete — `IntoParams` alone produces the parameter description but not a reusable schema reference."
  - "Module-level unit tests for map_attendance_error + ListMembersQuery deserialization stay in `genossi_rest/src/attendance.rs::tests` (not moved to E2E) because they exercise pure-Rust contracts that don't require an HTTP roundtrip. The E2E tests then verify the full HTTP stack."

patterns-established:
  - "Plan-Layer-Gate pattern for new aggregate-rollout: 6-plan sequence (DAO -> DAO-extension -> Auth-Bridge -> Wire-Types -> Service-Impl -> REST+E2E) is now proven end-to-end. Future aggregates can copy this structure verbatim; each plan delivers a clean check-point that the next plan consumes via `requires:`."
  - "End-to-end verification of every requirement in the final plan: every Phase-3 acceptance criterion (ASSY-04, ASSY-06, ATTN-01..06, SYNC-02) has at least one E2E test that exercises the full HTTP stack against a real test server. This catches integration gaps that pure unit/service-layer tests miss (e.g. an unwired DI dep would not surface until E2E)."

requirements-completed: [ASSY-04, ASSY-06, ATTN-01, ATTN-02, ATTN-03, ATTN-04, ATTN-05, ATTN-06, SYNC-02]

# Metrics
duration: ~15 min
completed: 2026-05-04
---

# Phase 3 Plan 06: REST Handlers + DI-Wiring + E2E Tests Summary

**Phase 3 final integration: 4 attendance REST handlers, DI-wiring of AttendanceServiceImpl into the binary's RestStateImpl, OpenAPI doc registration, and 6 end-to-end tests against a real-running HTTP server with in-memory SQLite. All 9 Phase-3 requirements (ASSY-04, ASSY-06, ATTN-01..06, SYNC-02) plus the SC#8 cascade-DB invariant verified at the integration level.**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-05-04T08:40:07Z
- **Completed:** 2026-05-04T08:55:00Z
- **Tasks:** 3 (REST handlers + Router + ApiDoc; DI-Wiring; E2E tests)
- **Files created:** 1 (`genossi_rest/src/attendance.rs`)
- **Files modified:** 4 (`genossi_rest/src/lib.rs`, `genossi_rest/src/test_server.rs`, `genossi_bin/src/lib.rs`, `genossi_bin/tests/e2e_tests.rs`)
- **Tests added:** 4 unit tests in `genossi_rest::attendance::tests` + 6 E2E tests in `genossi_bin/tests/e2e_tests.rs` = **10 new green tests**
- **Workspace test count after Plan:** 234 E2E tests grün (was 228 before Plan 06); workspace test pass remains 100%

## Accomplishments

- **D-21 (Endpoint registration):** All 4 endpoints live and routed:
  - `GET    /api/attendance/{assembly_id}/members?q=<text>` — reduced list, ATTN-01/02/06.
  - `PUT    /api/attendance/{assembly_id}/{member_id}` — toggle-on, ATTN-03 idempotent.
  - `DELETE /api/attendance/{assembly_id}/{member_id}` — toggle-off, ATTN-04 idempotent.
  - `GET    /api/assembly/{assembly_id}/stats` — live counter, ASSY-04.
- **D-26 (Differential error mapping):** Local `map_attendance_error` maps `ServiceError::PermissionDenied -> RestError::Forbidden(403)` for attendance endpoints. Phase 1+2 endpoints stay on the global mapping (PermissionDenied -> Unauthorized 401). Verified by unit test `test_map_attendance_error_permission_denied_returns_forbidden`.
- **D-23 (DI deps):** AttendanceServiceImpl wired with exactly the 6 deps from Plan 05 (no UuidService, no AuditLogDao). `assembly_member_snapshot_dao` is shared via Arc::clone with AssemblyServiceImpl — exactly one DAO instance per process.
- **AttendanceRestState wiring:** `RestStateImpl` exposes `attendance_service: Arc<AttendanceService>` and implements `AttendanceRestState` so the handlers can resolve `rest_state.attendance_service()`.
- **OpenAPI registration:** `attendance::ApiDoc` is nested at `/api/attendance/{assembly_id}` in the global `ApiDoc`. The 3 schemas (`AttendanceMemberTO`, `AttendanceStatsTO`, `ListMembersQuery`) appear in the Swagger-UI schema list.
- **D-28 (English naming):** Every new identifier (`AttendanceRestState`, `generate_attendance_route`, `generate_stats_route`, `list_attendance_members`, `mark_attendance_present`, `mark_attendance_absent`, `get_assembly_stats`, `map_attendance_error`, `AttendanceServiceDependencies`, `ListMembersQuery`) and every URL path (`/api/attendance`, `/api/assembly/{aid}/stats`, `/members`, `/{member_id}`) is English — verified by `grep` returning zero non-English tokens.
- **6 E2E tests cover all 9 phase requirements + SC#8** — see test table below.
- **234/234 E2E tests grün:** The full `cargo test --test e2e_tests` suite passes — no Phase-1/Phase-2 regression.
- **Workspace builds & tests:** `cargo build --bin genossi` exits 0; `cargo test --workspace` all green.

## Task Commits

| # | Task | Commit | Type | Files |
|---|------|--------|------|-------|
| 1 | 4 REST handlers + AttendanceRestState trait + 2 router builders + ApiDoc + 4 unit tests | `a553b6a` | feat | `genossi_rest/src/attendance.rs` (new), `genossi_rest/src/lib.rs`, `genossi_rest/src/test_server.rs` |
| 2 | DI-Wiring: AttendanceServiceImpl in RestStateImpl + AttendanceRestState impl | `b72b72c` | feat | `genossi_bin/src/lib.rs` |
| 3 | 6 E2E tests covering all 9 Phase-3 requirements + SC#8 cascade DB-check | `e39af6b` | test | `genossi_bin/tests/e2e_tests.rs` |
| — | cargo fmt of plan-06 files | `e90bd33` | style | `genossi_rest/src/attendance.rs`, `genossi_bin/src/lib.rs` |

**Plan metadata commit:** follows after this SUMMARY.

## Endpoint Reference

| Method | Path | Handler | Status Codes |
|--------|------|---------|--------------|
| GET    | `/api/attendance/{assembly_id}/members?q=<text>` | `list_attendance_members` | 200, 401, 403, 404 |
| PUT    | `/api/attendance/{assembly_id}/{member_id}`     | `mark_attendance_present`  | 200, 401, 403, 404 |
| DELETE | `/api/attendance/{assembly_id}/{member_id}`     | `mark_attendance_absent`   | 200, 401, 403, 404 |
| GET    | `/api/assembly/{assembly_id}/stats`             | `get_assembly_stats`       | 200, 401, 403, 404 |

All four handlers route through `map_attendance_error` so PermissionDenied -> 403 (D-26).

## DI-Wiring Diff (genossi_bin/src/lib.rs)

```text
+ type AttendanceDao = genossi_dao_impl_sqlite::attendance::AttendanceDaoImpl;
+
+ pub struct AttendanceServiceDependencies;
+ unsafe impl Send for AttendanceServiceDependencies {}
+ unsafe impl Sync for AttendanceServiceDependencies {}
+
+ impl genossi_service_impl::attendance::AttendanceServiceDeps for AttendanceServiceDependencies {
+     type Context = Context;
+     type Transaction = Transaction;
+     type AttendanceDao = AttendanceDao;
+     type AssemblyDao = AssemblyDao;
+     type MemberDao = MemberDao;
+     type AssemblyMemberSnapshotDao = AssemblyMemberSnapshotDao;
+     type PermissionService = PermissionService;
+     type TransactionDao = TransactionDao;
+ }
+
+ type AttendanceService = AttendanceServiceImpl<AttendanceServiceDependencies>;

  pub struct RestStateImpl {
      // ... existing fields ...
+     attendance_service: Arc<AttendanceService>,
  }

  impl RestStateImpl {
      pub fn new(pool: Arc<SqlitePool>) -> Self {
-         let assembly_member_snapshot_dao = Arc::new(AssemblyMemberSnapshotDao::new(pool.clone()));
+         let assembly_member_snapshot_dao = Arc::new(AssemblyMemberSnapshotDao::new(pool.clone()));
          // ... helper_token_dao + assembly_service ...
-         assembly_member_snapshot_dao,        // moved
+         assembly_member_snapshot_dao: assembly_member_snapshot_dao.clone(),  // shared

+         let attendance_dao = Arc::new(AttendanceDao::new(pool.clone()));
+         let attendance_service = Arc::new(AttendanceServiceImpl {
+             attendance_dao,
+             assembly_dao: assembly_dao.clone(),
+             member_dao: member_dao.clone(),
+             assembly_member_snapshot_dao,    // moved here (final consumer)
+             permission_service: permission_service.clone(),
+             transaction_dao: transaction_dao.clone(),
+         });
      }
  }

+ impl genossi_rest::attendance::AttendanceRestState for RestStateImpl {
+     type AttendanceService = AttendanceService;
+     fn attendance_service(&self) -> Arc<Self::AttendanceService> {
+         self.attendance_service.clone()
+     }
+ }
```

## Test Suite

### genossi_rest/src/attendance.rs::tests (4 unit tests)

| # | Test | Purpose | Status |
|---|------|---------|--------|
| 1 | `test_map_attendance_error_permission_denied_returns_forbidden` | D-26: PermissionDenied -> RestError::Forbidden(403), NOT Unauthorized | green |
| 2 | `test_map_attendance_error_entity_not_found_delegates_to_global` | Other variants delegate to global From<ServiceError> -> NotFound(404) | green |
| 3 | `test_list_members_query_with_q_serializes_via_serde_json` | Field name `q` and serde-deserializability of ListMembersQuery | green |
| 4 | `test_list_members_query_without_q_defaults_to_none` | `q` defaults to None when absent (#[serde(default)] contract) | green |

### genossi_bin/tests/e2e_tests.rs (6 E2E tests, real HTTP server + in-memory SQLite)

| # | Test | Phase Requirement | Status |
|---|------|-------------------|--------|
| 1 | `test_attendance_upsert_race_one_row_two_200ok` | SYNC-02 + ATTN-03 | green |
| 2 | `test_close_assembly_cascade_invalidates_helper_sessions` | SC#8 + D-11..D-13 (Cascade DB-effect) | green |
| 3 | `test_attendance_members_response_has_no_pii_fields` | ATTN-01 + T-03-06-01 (PII whitelist+blacklist guard) | green |
| 4 | `test_attendance_toggle_burst_does_not_pollute_audit_chain` | ATTN-05 + T-03-06-04 (audit silence + hash-chain integrity) | green |
| 5 | `test_vorstand_can_edit_attendance_after_close` | ASSY-06 + D-20 + SC#9 (admin post-close edit, status stays Closed) | green |
| 6 | `test_attendance_members_substring_search_filters_by_query_param` | ATTN-02 (?q= LIKE filter, 1-of-2 hit on Müller, both without ?q) | green |

**Workspace summary:**

- `cargo build --bin genossi`: exit 0
- `cargo test --test e2e_tests`: 234 passed, 0 failed (228 pre-existing + 6 new)
- `cargo test -p genossi_rest --lib`: 48 passed (was 44 + 4 new)
- `cargo test --workspace`: all packages green

## Phase 3 Requirement Coverage Map

| Req | Source | E2E Test(s) |
|-----|--------|-------------|
| ASSY-04 (Live counter) | tests #1, #5 — both use `/api/assembly/{aid}/stats` |
| ASSY-06 (Vorstand-Post-Close-Edit) | test #5 — admin DELETE after close + status stays Closed |
| ATTN-01 (Reduced PII view) | test #3 — JSON-key whitelist + blacklist guard |
| ATTN-02 (Substring search) | test #6 — `?q=Müll` matches Müller, not Schmidt |
| ATTN-03 (Idempotent PUT) | test #1 — 2× parallel PUT = 1 row + 2× 200 OK |
| ATTN-04 (Idempotent DELETE) | tests #4, #5 — DELETE in alternation + DELETE post-close |
| ATTN-05 (No audit) | test #4 — audit-listing count_before == count_after, hash chain valid |
| ATTN-06 (Helper-View also for Vorstand) | tests #1, #5, #6 — all run in mock_auth admin context (no helper claim) |
| SYNC-02 (Idempotent Sync) | test #1 — race produces exactly 1 row |
| **SC#8 (Cascade-Invalidation)** | test #2 — direct `session` table query before/after close |

## Self-Check

```bash
[ -f /home/neosam/programming/rust/projects/genossi3/genossi_rest/src/attendance.rs ]   && echo "FOUND: attendance.rs"
[ -f /home/neosam/programming/rust/projects/genossi3/.planning/phases/03-attendance-aggregat-cascade-invalidation/03-06-SUMMARY.md ] && echo "FOUND: SUMMARY.md"
grep -c 'pub mod attendance'                  /home/neosam/programming/rust/projects/genossi3/genossi_rest/src/lib.rs
grep -c 'pub fn generate_attendance_route'    /home/neosam/programming/rust/projects/genossi3/genossi_rest/src/attendance.rs
grep -c 'pub fn generate_stats_route'         /home/neosam/programming/rust/projects/genossi3/genossi_rest/src/attendance.rs
grep -c 'fn map_attendance_error'             /home/neosam/programming/rust/projects/genossi3/genossi_rest/src/attendance.rs
grep -c 'ServiceError::PermissionDenied => RestError::Forbidden' /home/neosam/programming/rust/projects/genossi3/genossi_rest/src/attendance.rs
grep -c 'attendance::ApiDoc'                  /home/neosam/programming/rust/projects/genossi3/genossi_rest/src/lib.rs
grep -c 'AttendanceRestState'                 /home/neosam/programming/rust/projects/genossi3/genossi_bin/src/lib.rs
grep -c 'attendance_service'                  /home/neosam/programming/rust/projects/genossi3/genossi_bin/src/lib.rs
grep -c '^async fn test_attendance\|^async fn test_close_assembly_cascade\|^async fn test_vorstand_can_edit' /home/neosam/programming/rust/projects/genossi3/genossi_bin/tests/e2e_tests.rs
git log --oneline | grep -E 'a553b6a|b72b72c|e39af6b|e90bd33'
```

See `## Self-Check: PASSED` block at end.

## Decisions Made

- **Hash-chain test burst = 40 toggles (not 100):** The plan-spec called for 100 toggles but combined with the 4 surrounding REST calls would exceed the 60-burst api_rate_layer cap and produce a 429 mid-burst. 40 toggles is sufficient — what matters is that ANY count of toggles produces ZERO new audit entries, not the magnitude. Documented inline + flagged as Deviation 1.
- **Stats endpoint registered at a separate `.nest()`:** D-21 placed `/api/assembly/{aid}/stats` under the assembly namespace, but `assembly::generate_route()` already nests at `/api/assembly`. Solution: a dedicated `.nest("/api/assembly/{assembly_id}", attendance::generate_stats_route())` — Axum allows multiple nests with different prefix specificities. Pattern is reusable for future cross-namespace endpoints.
- **`assembly_member_snapshot_dao` is now Arc-shared:** Previously moved into AssemblyServiceImpl, now cloned so AttendanceServiceImpl can also consume it. Mirror of the helper_token_dao sharing pattern from Plan 05; keeps the "exactly one DAO instance per process" invariant.
- **`ListMembersQuery` is both `IntoParams` AND `ToSchema`:** `IntoParams` produces the OpenAPI parameter docs; `ToSchema` registers a reusable schema reference. Both are needed because the global ApiDoc lists `ListMembersQuery` in `components(schemas(...))`.
- **Local map_attendance_error stays in `attendance.rs` (not moved to a shared utility):** D-26 / RESEARCH §DECISION CONFLICT 1 explicitly resolved this. A shared mapping would require either (a) breaking the Phase-1+2 endpoints' existing `From<ServiceError>` semantics, or (b) introducing per-handler boilerplate everywhere. The local function is the smallest delta.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 — Blocking] Hash-chain test burst exceeded the api_rate_layer cap**

- **Found during:** Task 3 first run of `test_attendance_toggle_burst_does_not_pollute_audit_chain`.
- **Issue:** The plan-spec called for "100 toggles". With 100 toggles + 1 stats + 2 audit-listings + 1 verify = 104 REST calls in a single test, exceeding the global `api_rate_layer` cap (`burst_size=60`, `per_second=1` refill) — toggle 56 returned 429 Too Many Requests instead of 200 OK.
- **Fix:** Reduced the burst to 40 toggles. The audit invariant (count_before == count_after, hash chain valid) is independent of the burst magnitude — what matters is that MULTIPLE toggles produce ZERO new audit entries. The reduced burst still verifies ATTN-05 + T-03-06-04 fully. Inline comment in the test documents the rationale + the 60-burst-cap context.
- **Files modified:** `genossi_bin/tests/e2e_tests.rs` (toggle_count constant + comment block).
- **Verification:** `test_attendance_toggle_burst_does_not_pollute_audit_chain` exits 0 with `count_before == count_after == 0` and `verify.valid == true`.
- **Committed in:** `e39af6b` (Task 3 GREEN, since the rate-limit interaction is part of the integration verification).
- **Forward impact:** None — Phase 4 frontend will not toggle 40+ items in <1 second (the typical helper marks one member at a time after visual identification). Phase 5 should re-evaluate the rate-limit configuration under realistic GV-Tag traffic patterns (already captured in `tech-stack.affects` above).

**2. [Rule 1 — Bug avoided proactively] Defensive blacklist on top of whitelist for PII guard**

- **Found during:** Task 3 design of `test_attendance_members_response_has_no_pii_fields`.
- **Issue:** A pure whitelist test (assert each key is in the allowed set) would catch any leaked field, but a developer reading the test might wonder "what specifically is forbidden?" — making the test less informative as a contract document.
- **Fix:** The test runs BOTH a whitelist iteration AND an explicit blacklist of 12 PII keys (`email`, `iban`, `bank_account`, `street`, `house_number`, `postal_code`, `city`, `comment`, `join_date`, `exit_date`, `birth_date`, `phone`). The whitelist catches "any new field", the blacklist makes the contract explicit and survives future refactoring even if the whitelist is mistakenly relaxed.
- **Files modified:** `genossi_bin/tests/e2e_tests.rs` (test body).
- **Verification:** Test exits 0 against the current implementation. Defensive blacklist is documented in the inline comment.
- **Committed in:** `e39af6b`.
- **Forward impact:** None — pure test hardening.

---

**Total deviations:** 2 (1 Blocking auto-fix for the rate-limit interaction; 1 Test-hardening for the PII guard).
**Impact on plan:** None mechanical. Both deviations strengthen the verification without changing any production code.

## Issues Encountered

- **rustfmt + cargo-clippy not directly on PATH** (pre-existing in Nix-Setup, see Memory `feedback_nix_toolchain.md`). `cargo fmt` works via the explicit Nix-Store path `/nix/store/p5rylqcg4ddwnr87iiybmjrchidlld9m-rust-default-1.93.0/bin`. `cargo clippy` against the workspace fails with a tracing-core toolchain mismatch (rust-default-1.93 vs already-compiled artifacts) — pre-existing Nix issue, not Plan-06-introduced.
- **Pre-existing dead-code warning** in `genossi_bin/src/lib.rs:778` (`use genossi_dao::auditable::Auditable`) — out-of-scope, not Plan-06.
- **Pre-existing fmt diff** in `genossi_dao/src/helper_token.rs` — out-of-scope (touched in Plan 02), Plan 06 only formatted its own files.

## TDD Gate Compliance

Plan 03-06 has `tdd="true"` on all three tasks. Per the Plan-04/05 convention, RED + GREEN were committed as atomic pairs (separate RED commits would not have compiled meaningfully — e.g., the E2E tests cannot run without the REST handlers; the REST handlers cannot type-check without the AttendanceRestState trait):

- **Task 1 GREEN:** Commit `a553b6a` — 4 handlers + router + ApiDoc + 4 unit tests, all green on construction.
- **Task 2 GREEN:** Commit `b72b72c` — DI wiring; the impl-only delta is verified by the workspace build (an unwired dep would fail at compile time).
- **Task 3 GREEN:** Commit `e39af6b` — 6 E2E tests, 5 green on construction; 1 (hash-chain burst) red on first run due to the rate-limit issue, fixed in the same commit per Rule 3 (Blocking auto-fix).

A separate RED commit per task was elided because the structural tests (handler-deserialize unit tests, DI-wiring) are trivially green-on-construction, and the behavior-tests (E2E race, cascade) require both the production code path AND the test in one atomic state to be meaningful — a RED-commit with the E2E test alone would not compile (the handlers wouldn't exist yet).

**REFACTOR-Gate:** Skipped — code is minimal and idiomatic. `cargo fmt` was run + committed separately as `e90bd33` for cleanliness.

## Threat Flags

The six threats from the plan-06 frontmatter `threat_model:` are all addressed:

- **T-03-06-01 (Information Disclosure — PII Leak):** Mitigated. The whitelist+blacklist E2E test (`test_attendance_members_response_has_no_pii_fields`) iterates JSON keys against an explicit allowlist of 7 fields and a defensive blocklist of 12 PII keys. Any future MemberTO field that propagates through to AttendanceMemberTO will fail this test.
- **T-03-06-02 (Elevation of Privilege — 401-vs-403 leak):** Mitigated. `map_attendance_error` differential-maps PermissionDenied -> Forbidden(403) for attendance endpoints; verified by unit test `test_map_attendance_error_permission_denied_returns_forbidden`. Other endpoints' 401-mapping is preserved.
- **T-03-06-03 (Tampering — PUT/DELETE Idempotenz):** Mitigated. The race E2E test (`test_attendance_upsert_race_one_row_two_200ok`) verifies via stats that 2× parallel PUT produces exactly ONE present row, both 200 OK.
- **T-03-06-04 (Repudiation — Audit Hash-Chain Stability):** Mitigated. The toggle-burst test (`test_attendance_toggle_burst_does_not_pollute_audit_chain`) verifies count_before == count_after for entity_type=attendance AND hash chain validity via /api/audit/verify.
- **T-03-06-05 (DoS — Stats-Polling-Rate-Limit):** Accepted per plan. Existing api_rate_layer (60/min/IP) covers stats. The rate-limit interaction was empirically validated by Deviation 1 — a 40-toggle burst + audit-listing fits comfortably under the cap.
- **T-03-06-06 (Tampering — setup_with_pool exposes Pool):** Accepted per plan. The cascade test uses direct sqlx queries on the test pool to verify the post-close session-table state. Test-only risk; production code never exposes the pool to handlers.

No new threat flags discovered during execution.

## Next Phase Readiness

**Phase 3 is COMPLETE.** All 9 phase requirements (ASSY-04, ASSY-06, ATTN-01..06, SYNC-02) are verified at the integration level. ROADMAP-Phase-3-SC#8 (cascade invalidation) is verified by direct DB-state assertion in the E2E test.

**Direct consumers of Plan 03-06:**

- **Phase 4 (Frontend):** the REST contract is now stable. The Dioxus helper-page components can consume:
  - `GET /api/attendance/{aid}/members?q=...` with `Vec<AttendanceMemberTO>` JSON;
  - `PUT/DELETE /api/attendance/{aid}/{mid}` with empty body, 200 OK on success;
  - `GET /api/assembly/{aid}/stats` with `AttendanceStatsTO { present, total }` JSON for the live counter.
  - The OpenAPI doc at `/swagger-ui/` includes these endpoints under tag=Attendance, so utoipa-generated TypeScript bindings (if Phase 4 chooses that path) will be complete.
- **Phase 5 (Generalprobe):** the rate-limit interaction is now empirically anchored — a 40-toggle burst + 4 surrounding REST calls fits under the 60-burst cap with margin. Generalprobe should test under realistic GV-Tag traffic to confirm the cap is appropriate or needs tuning.

**Pitfall reminders for Phase 4:**

- The Phase-4 helper page must NOT expose any field beyond AttendanceMemberTO's 7 fields — the PII guard at the REST layer prevents leaks at the API level, but Frontend code must not duplicate `MemberTO` for the helper view.
- The `is_present` field is the single source of truth for the toggle UI — Frontend should track it from the most recent `GET /members` response, not from optimistic local state (per the "Sync only on Refresh, no Live Push" principle).
- The `?q=` search filter is server-side — Frontend should re-fetch on every search-input change (debounced) rather than client-side filtering.

---

## Self-Check: PASSED

- `genossi_rest/src/attendance.rs` — FOUND on disk (252 lines after fmt: 4 handlers + AttendanceRestState trait + 2 router builders + map_attendance_error + ApiDoc + 4 unit tests).
- `.planning/phases/03-attendance-aggregat-cascade-invalidation/03-06-SUMMARY.md` — FOUND on disk (this file).
- `genossi_rest/src/lib.rs` — `pub mod attendance` line present (1 occurrence); `attendance::ApiDoc` registered in the global ApiDoc (1 occurrence); `AttendanceRestState` bound on `create_app` and `start_server` (3 occurrences total including test_server.rs).
- `genossi_rest/src/attendance.rs` — `pub fn generate_attendance_route` (1), `pub fn generate_stats_route` (1), `fn map_attendance_error` (1), `ServiceError::PermissionDenied => RestError::Forbidden` (1), `AttendanceRestState` (7), `#[utoipa::path` (4).
- `genossi_bin/src/lib.rs` — `AttendanceServiceImpl` (6 occurrences: type alias + Deps + RestStateImpl construction + Arc<AttendanceService>); `AttendanceServiceDependencies` (5); `attendance_service` (5: struct field + Self {} + impl AttendanceRestState + 2 in construction); `AttendanceRestState` (2: import-path + impl-block); `helper_token_dao.clone()` (1: in AssemblyServiceImpl); `permission_dao.clone()` (5).
- `genossi_bin/tests/e2e_tests.rs` — 6 new test functions (`^async fn test_attendance|^async fn test_close_assembly_cascade|^async fn test_vorstand_can_edit` count = 6); `AttendanceStatsTO` and `AttendanceMemberTO` usage = 7 references.
- Commit `a553b6a` (Task 1) — FOUND in git log (subject: "feat(03-06): add 4 attendance REST handlers + router + ApiDoc").
- Commit `b72b72c` (Task 2) — FOUND in git log (subject: "feat(03-06): wire AttendanceServiceImpl into RestStateImpl + bin DI").
- Commit `e39af6b` (Task 3) — FOUND in git log (subject: "test(03-06): add 6 attendance E2E tests covering 9 phase requirements").
- Commit `e90bd33` (style) — FOUND in git log (subject: "style(03-06): apply cargo fmt to plan-06 files").
- 4 unit tests in `genossi_rest/src/attendance.rs::tests` green (`cargo test -p genossi_rest --lib`: 48 passed, was 44 before Plan 06).
- 6 E2E tests in `genossi_bin/tests/e2e_tests.rs` green (`cargo test --test e2e_tests`: 234 passed, was 228 before Plan 06; 0 failed).
- Workspace builds OK (`cargo build --bin genossi`: exit 0).
- All Phase-1 + Phase-2 + Phase-3 (Plan 01-05) E2E tests stay green — no regression.
- All 9 phase-3 requirements (ASSY-04, ASSY-06, ATTN-01..06, SYNC-02) covered by at least one E2E test (see Phase 3 Requirement Coverage Map above).
- ROADMAP-Phase-3-SC#8 (cascade invalidation) verified by direct DB-state E2E test.

---

*Phase: 03-attendance-aggregat-cascade-invalidation*
*Plan: 06 (Phase 3 final integration plan — Phase 3 COMPLETE)*
*Completed: 2026-05-04*
