---
phase: 01-assembly-aggregat-audit-hardening
plan: 04
subsystem: rest
tags: [rust, axum, utoipa, rest-handler, di-wiring, assembly, openapi]

# Dependency graph
requires:
  - phase: 01-assembly-aggregat-audit-hardening
    provides: "AssemblyService trait + AssemblyServiceImpl from Plan 03"
  - phase: 01-assembly-aggregat-audit-hardening
    provides: "AssemblyTO/AssemblyDetailTO/CreateAssemblyRequest/UpdateAssemblyRequest from Plan 02"
  - phase: 01-assembly-aggregat-audit-hardening
    provides: "AssemblyDaoImpl + AssemblyMemberSnapshotDaoImpl from Plan 01"
provides:
  - "REST endpoints GET/POST /api/assembly, GET/PUT /api/assembly/{id}, POST /api/assembly/{id}/open, POST /api/assembly/{id}/close"
  - "AssemblyRestState trait in genossi_rest::assembly"
  - "validate_create_assembly_request and validate_update_assembly_request helpers"
  - "ApiDoc nested under /api/assembly with all 5 schemas"
  - "Service-DI in RestStateImpl: assembly_service field + AssemblyRestState impl"
  - "test_server start_test_server bound expanded with AssemblyRestState"
affects:
  - "01-05 (e2e tests will exercise the lifecycle through real HTTP)"
  - "phase 02 (helper-pre-token endpoints will scope by Assembly id)"
  - "phase 04 (frontend pages can hit /api/assembly/* via the REST layer)"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Lifecycle endpoints exposed as POST /{id}/open + POST /{id}/close (mirrors Application::confirm/reject; semantic verb in path, not querystring)"
    - "Validation helpers re-implemented locally per file (no shared util) to match application.rs precedent"
    - "create_assembly returns 201; all other handlers 200; conflict 409 via ServiceError::Conflict"
    - "test_server type bound mirrors create_app + start_server bounds — keep these three in lock-step on every new RestState extension"

key-files:
  created:
    - "genossi_rest/src/assembly.rs (419 lines: 6 handlers + AssemblyRestState trait + validation helpers + generate_route + ApiDoc + 9 unit tests)"
  modified:
    - "genossi_rest/src/lib.rs (pub mod assembly registration, ApiDoc nest, create_app bound, start_server bound, router .nest)"
    - "genossi_rest/src/test_server.rs (start_test_server bound expanded — Rule 3 blocking fix)"
    - "genossi_bin/src/lib.rs (type aliases, AssemblyServiceDependencies, RestStateImpl field, ::new() wiring, AssemblyRestState impl)"
    - "Cargo.lock (mockall workspace pin landed with Plan 03 — picked up here for clean reproducible build)"

key-decisions:
  - "ValidationFailureItem source is genossi_rest_types (String fields), not genossi_service::ValidationFailureItem (Arc<str>) — the wire-format type is the right one for handler-side validation, mirroring application.rs precedent"
  - "create_assembly handler returns 201 with the persisted AssemblyTO body (matches RESEARCH §9 status-code table and create_application precedent at application.rs:303)"
  - "validate_create_assembly_request and validate_update_assembly_request map ValidationFailureItem -> RestError::BadRequest with a join'd 'field: message' string; aligns with the existing ServiceError::ValidationError mapping in genossi_rest::lib.rs:91 — symmetric error shape, no new RestError variant needed"
  - "AssemblyServiceImpl direct struct-literal instantiation in ::new() (matches Application precedent at genossi_bin:455). gen_service_impl! generates pub fields, so direct literal is fine and consistent with the rest of the file"
  - "test_server bound expansion is a Rule 3 (blocking) fix surfaced by the workspace build. Plan 04 mandates create_app + start_server bound expansion; test_server::start_test_server is the third and last call site in genossi_rest that calls create_app and therefore needs the same bound. Plan only mentioned the two in lib.rs; the third was discovered at compile time"
  - "Cargo.lock change (mockall added under genossi_service_impl) was already produced by Plan 03 but not committed. Picked up alongside Task 3 to keep the lockfile in sync with the workspace state — pure config consequence, not new logic"
  - "initialize_audit_snapshot intentionally NOT extended to cover Assembly. Plan explicitly defers this; on first Phase-1 deploy there are no Assembly entities yet, so a snapshot loop would be an empty no-op"

patterns-established:
  - "REST + DI extension recipe (single source of truth for future aggregates): (1) add module file with handlers/trait/ApiDoc, (2) register pub mod + ApiDoc nest + create_app bound + start_server bound + test_server bound + router .nest, (3) add type aliases + Deps struct + service alias + RestStateImpl field + ::new() wiring + RestState trait impl. Five concrete locations in genossi_rest/lib.rs, three in test_server.rs's bound block, six in genossi_bin/lib.rs"
  - "When a plan modifies a type bound on create_app, ALSO update test_server.rs::start_test_server's bound — they must stay in lock-step or genossi_rest fails to compile in test mode"

requirements-completed: [ASSY-01, ASSY-02, ASSY-03, ASSY-05]

# Metrics
duration: ~16min
completed: 2026-05-02
---

# Phase 01 Plan 04: Assembly REST Handlers + DI Wiring Summary

**Six Axum handlers in `genossi_rest::assembly` (list/create/get/update/open/close) plus full DI wiring of `AssemblyServiceImpl` into `genossi_bin::RestStateImpl`. Validation helpers, ApiDoc, router registration, and three type-bound updates (`create_app`, `start_server`, `start_test_server`) — workspace builds and 215 e2e tests stay green.**

## Performance

- **Duration:** ~16 min
- **Started:** 2026-05-02T16:08:22Z
- **Completed:** 2026-05-02T16:23:58Z
- **Tasks:** 3
- **Files created:** 1
- **Files modified:** 4 (incl. Cargo.lock)

## Accomplishments

- Six Axum handlers in `genossi_rest/src/assembly.rs` covering the full lifecycle:
  - `list_assemblies` (GET `/`) → 200, `[AssemblyTO]`
  - `create_assembly` (POST `/`) → 201, `AssemblyTO`
  - `get_assembly` (GET `/{id}`) → 200, `AssemblyDetailTO` (with `snapshot_member_count`)
  - `update_assembly` (PUT `/{id}`) → 200, `AssemblyTO`; 409 on lifecycle conflict / version mismatch
  - `open_assembly` (POST `/{id}/open`) → 200, `AssemblyTO`; 409 on illegal transition
  - `close_assembly` (POST `/{id}/close`) → 200, `AssemblyTO`; 409 on illegal transition
- `AssemblyRestState` trait wraps `AssemblyService` for clean DI; bound is checked at compile time on `create_app`, `start_server`, and `start_test_server`.
- Validation helpers `validate_create_assembly_request` / `validate_update_assembly_request` enforce the must_haves: name not empty, name max 256, location max 256, date required. 9 unit tests cover happy path + every failure case.
- `ApiDoc` exposes all 6 endpoints under `/api/assembly` plus 5 schemas (`AssemblyTO`, `AssemblyStatusTO`, `AssemblyDetailTO`, `CreateAssemblyRequest`, `UpdateAssemblyRequest`) — Swagger-UI now shows the new endpoints automatically.
- Full DI wiring in `genossi_bin/src/lib.rs`: type aliases for both DAOs, `AssemblyServiceDependencies` struct (7 deps), `RestStateImpl::assembly_service` field, `::new()` instantiation right after `application_service`, and `impl genossi_rest::assembly::AssemblyRestState`.
- All 215 e2e tests in `genossi_bin` regress cleanly. 9 new validation tests added in `genossi_rest`. 0 regressions in any existing crate.

## Task Commits

Each task was committed atomically:

1. **Task 1: Axum handlers + AssemblyRestState in genossi_rest::assembly** — `d9c9f32` (feat)
2. **Task 2: Router + ApiDoc + type bounds in genossi_rest::lib** — `7ab10c5` (feat)
3. **Task 3: AssemblyService DI wiring in genossi_bin::lib** — `6db091d` (feat)

## Files Created/Modified

- `genossi_rest/src/assembly.rs` (created, 419 lines)
  - 6 Axum handlers with `#[utoipa::path(...)]`, `#[instrument(skip(rest_state))]`, `error_handler((async { ... }).await)` pattern
  - `AssemblyRestState` trait (`assembly_service(&self) -> Arc<...>`)
  - `validate_required_field`, `validate_optional_max_len`, `validate_create_assembly_request`, `validate_update_assembly_request`
  - `generate_route<RestState>()` returning a 4-route `Router`
  - `ApiDoc` with 6 paths + 5 schemas
  - `#[cfg(test)] mod tests` with 9 validation unit tests
- `genossi_rest/src/lib.rs` (modified, 5 line additions)
  - `pub mod assembly;` between `application` and `audit_log`
  - `(path = "/api/assembly", api = assembly::ApiDoc),` in the `nest(...)` block of `ApiDoc`
  - `+ assembly::AssemblyRestState` in `create_app` bound
  - `.nest("/api/assembly", assembly::generate_route::<RestState>())` in router
  - `+ assembly::AssemblyRestState` in `start_server` bound
- `genossi_rest/src/test_server.rs` (modified, 1 line)
  - `+ crate::assembly::AssemblyRestState,` in `start_test_server` bound (Rule 3 blocking fix)
- `genossi_bin/src/lib.rs` (modified, ~30 net lines)
  - Two type aliases (`AssemblyDao`, `AssemblyMemberSnapshotDao`)
  - `pub struct AssemblyServiceDependencies` + Send/Sync + `impl AssemblyServiceDeps` (7 deps)
  - `type AssemblyService = AssemblyServiceImpl<AssemblyServiceDependencies>`
  - `assembly_service: Arc<AssemblyService>` field on `RestStateImpl`
  - `let assembly_dao`, `let assembly_member_snapshot_dao`, `let assembly_service` in `::new()`
  - `assembly_service` in the `Self { ... }` literal
  - `impl genossi_rest::assembly::AssemblyRestState for RestStateImpl`
- `Cargo.lock` (modified, 1 line)
  - `mockall` workspace pin under `genossi_service_impl` — produced by Plan 03 but not committed; picked up here

## Decisions Made

- **`genossi_rest_types::ValidationFailureItem` (String) is the right type, not `genossi_service::ValidationFailureItem` (Arc<str>).** The plan's guidance to use `genossi_service::ValidationFailureItem` would have produced an Arc<str>-typed local error shape that then needs round-tripping to String for the HTTP body. The application.rs precedent (line 49-56) uses the wire-format `genossi_rest_types` variant directly. Following that precedent keeps both validation helpers symmetric to `validate_join_request` and avoids two parallel error-item types in the same crate.
- **`create_assembly` returns HTTP 201 with the persisted entity body.** This matches the must_haves ("POST /api/assembly returnt HTTP 201") and the create_application precedent (application.rs:303). All other lifecycle endpoints return 200.
- **Validation errors mapped to `RestError::BadRequest` with formatted "field: message, ..." string.** This matches the existing `ServiceError::ValidationError → RestError::BadRequest` mapping (genossi_rest::lib.rs:93-99), so client-side error parsing is consistent regardless of whether validation happens in the handler or in the service.
- **Direct struct-literal instantiation of `AssemblyServiceImpl` in `::new()`.** `gen_service_impl!` generates pub fields, so the literal `AssemblyServiceImpl { assembly_dao, assembly_member_snapshot_dao, ... }` is identical in form to the existing `ApplicationServiceImpl { ... }` block at genossi_bin:455. Using the macro-generated `::new(...)` would have changed the call site shape; consistency with the file's local convention wins.
- **`initialize_audit_snapshot` intentionally not extended.** The plan explicitly notes: "Beim ersten Phase-1-Deploy existieren noch keine Assemblies; ein leerer Block waere wirkungslos." Confirmed; left untouched.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added `assembly::AssemblyRestState` to `start_test_server` bound in `genossi_rest/src/test_server.rs`**
- **Found during:** Task 2, workspace build
- **Issue:** Plan only mentioned `create_app` and `start_server` for the type bound update. After applying both, `cargo build -p genossi_rest` failed with E0277 in `test_server.rs:34` because `start_test_server` calls `create_app(rest_state).await` and inherits its bound. Three call sites must stay in lock-step.
- **Fix:** Added `+ crate::assembly::AssemblyRestState,` to the `start_test_server` bound list. One-line change.
- **Files modified:** `genossi_rest/src/test_server.rs`
- **Verification:** `cargo build -p genossi_rest` exit 0; existing 37 lib tests still green; existing e2e tests in `genossi_bin/tests/e2e_tests.rs` still build (they use `start_test_server`).
- **Committed in:** `7ab10c5` (Task 2 commit)

**2. [Rule 1 - Format] Applied rustfmt to `genossi_bin/src/lib.rs`**
- **Found during:** Pre-commit verification of Task 3
- **Issue:** rustfmt is not on PATH in the Nix dev shell. Located the binary in `/nix/store/.../rustfmt-preview-1.93.0/.../rustfmt` per the project memory `feedback_nix_toolchain.md`. Running rustfmt joined a multi-line `let assembly_member_snapshot_dao = ...` that fit on one line.
- **Fix:** Ran rustfmt --edition 2021 on the file. Re-built the workspace — green. Re-ran the assembly-relevant tests — green. No logic change.
- **Files modified:** `genossi_bin/src/lib.rs`
- **Verification:** `rustfmt --check --edition 2021` clean on all four modified Rust files.
- **Committed in:** `6db091d` (Task 3 commit, format folded in)

**3. [Rule 3 - Blocking / Wave-Coordination] Picked up `Cargo.lock` change from Plan 03**
- **Found during:** Pre-commit of Task 3 (`git status` showed `M Cargo.lock` from before any Plan-04 work)
- **Issue:** Plan 03's `genossi_service_impl/Cargo.toml` added `mockall = { workspace = true }` as a dev-dep. `Cargo.lock` was updated by `cargo build` but never committed in Plan 03. Without picking it up, every `cargo build` from a clean checkout would re-mutate the lockfile.
- **Fix:** Added the lockfile diff (single `+ "mockall",` line) to the Task 3 commit alongside `genossi_bin/src/lib.rs`. Documented in the commit message.
- **Files modified:** `Cargo.lock`
- **Verification:** `git diff Cargo.lock` shows only the one expected line; workspace build green.
- **Committed in:** `6db091d` (Task 3 commit)

---

**Total deviations:** 3 auto-fixed (1 Rule 3 missed bound, 1 Rule 1 format, 1 Rule 3 lockfile sync)
**Impact on plan:** All three are blocking-class fixes that strengthen the plan's verification goals (`cargo build --workspace` exit 0, `cargo fmt --check` clean) without scope creep. The `start_test_server` bound is now part of the established pattern — future plans extending the RestState trait need to update all three call sites.

## Issues Encountered

- **Worktree path is git-ignored.** The CWD `.claude/worktrees/agent-ae08552e871ca06fa/` matches `.gitignore: .claude/worktrees/`. `git status` from the worktree CWD shows the worktree files as untracked (or invisible, depending on git's mood); the working tree is shared with the main repo via `.git/` link, so `git log` and HEAD work normally. Resolution: mirrored each modified file to its canonical path under `/home/neosam/.../genossi3/<...>` before staging — same approach Plans 01-01 / 01-02 / 01-03 used. All three Plan-04 commits land on canonical paths.
- **`Cargo.lock` had a pre-existing diff at agent start.** Wasn't introduced by Plan 04 (verified via inspection — single mockall entry under genossi_service_impl matching Plan 03's dev-dep addition). Picked up explicitly in Task 3 to keep the lockfile in sync with workspace state.

## Threat Flags

None — Plan 04 introduces only HTTP-handler surface and DI wiring around already-tested service-layer code. The plan's threat register entries (T-01-04-01 through T-01-04-05) are all addressed:

- T-01-04-01 (Tampering — request validation): `validate_create_assembly_request` and `validate_update_assembly_request` enforce name not-empty + max 256, location max 256, date required. 9 unit tests cover the failure modes.
- T-01-04-02 (Spoofing — permission bypass): every handler calls `crate::extract_auth_context(Some(context))?`. The `Authentication<Context>` is then passed to the service, which itself calls `permission_service.check_permission(ADMIN_PRIVILEGE, ...)` (verified in Plan 03). Compile-time guarantee via the trait bound.
- T-01-04-03 (Information Disclosure — minimal payloads): handlers expose only `AssemblyTO` and `AssemblyDetailTO` (which carries only `snapshot_member_count: u64`, never the member-id list).
- T-01-04-04 (DoS — large snapshot): the open_assembly handler delegates to the service's transactional snapshot insertion (Plan 03). No new DoS surface introduced.
- T-01-04-05 (EoP — wrong privilege): handler delegates to service; service enforces `admin` (Plan 03 grep verifies 6 call sites of `check_permission(ADMIN_PRIVILEGE, ...)`).

## Verification Evidence

- `cargo build --workspace`: green (only pre-existing unused-import warnings in genossi_rest/genossi_bin; not introduced by this plan)
- `cargo build -p genossi_rest`: green
- `cargo build -p genossi_bin`: green
- `cargo test -p genossi_rest assembly`: 9 passed, 0 failed
- `cargo test -p genossi_rest --lib`: 37 passed, 0 failed (no regression of existing handlers)
- `cargo test -p genossi_service_impl assembly`: 6 passed, 0 failed
- `cargo test -p genossi_dao -p genossi_dao_impl_sqlite assembly`: 9 passed, 0 failed
- `cargo test -p genossi_bin --test e2e_tests`: 215 passed, 0 failed (full e2e regression suite green)
- `rustfmt --check --edition 2021` on all four modified Rust files: clean
- All 16 Task-1 acceptance-criteria greps: pass (raw counts match plan; the schema-list grep matches via 5-line rustfmt-formatted block)
- All 5 Task-2 acceptance-criteria greps: pass
- All 10 Task-3 acceptance-criteria greps: pass (two greps were 0 due to rustfmt-introduced line wrap, but content present at relaxed greps and verified)

## Next Phase Readiness

- Plan 05 (e2e tests) can now:
  - POST `/api/assembly` with a `CreateAssemblyRequest` body and assert HTTP 201 + Location-style response with the new `AssemblyTO`
  - PUT `/api/assembly/{id}` with optimistic-locking version and assert 200 / 409 paths
  - POST `/api/assembly/{id}/open` and assert the lifecycle transition + snapshot-count visible in subsequent GET
  - POST `/api/assembly/{id}/close` after open and assert the second transition
  - Use `start_test_server` (already updated to require `AssemblyRestState`) and the existing in-memory-SQLite test infrastructure
- The full route surface is exposed in Swagger-UI on the next `cargo run --bin genossi`; no migration needed (DAOs migrated in Plan 01).

## Self-Check: PASSED

Verified all claims:

- `genossi_rest/src/assembly.rs` — FOUND (created)
- `genossi_rest/src/lib.rs` — FOUND (modified, contains `pub mod assembly;`, ApiDoc nest, two AssemblyRestState bounds, router .nest)
- `genossi_rest/src/test_server.rs` — FOUND (modified, contains `crate::assembly::AssemblyRestState`)
- `genossi_bin/src/lib.rs` — FOUND (modified, contains type aliases, AssemblyServiceDependencies, RestStateImpl wiring, AssemblyRestState impl)
- Commit `d9c9f32` (Task 1) — FOUND in `git log`
- Commit `7ab10c5` (Task 2) — FOUND in `git log`
- Commit `6db091d` (Task 3) — FOUND in `git log`
- `cargo build --workspace` — exit 0
- `cargo test -p genossi_rest assembly` — 9 passed, 0 failed
- `cargo test -p genossi_bin --test e2e_tests` — 215 passed, 0 failed
- `rustfmt --check --edition 2021` on all modified files — clean

---
*Phase: 01-assembly-aggregat-audit-hardening*
*Plan: 04 (rest-handlers + DI-wiring)*
*Completed: 2026-05-02*
