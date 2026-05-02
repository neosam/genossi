---
phase: 01-assembly-aggregat-audit-hardening
plan: 02
subsystem: api
tags: [rust, axum, utoipa, serde, iso8601, openapi, dto, transfer-objects]

requires:
  - phase: 01-assembly-aggregat-audit-hardening
    provides: "genossi_dao::assembly::{AssemblyEntity, AssemblyStatus} from Plan 01"
provides:
  - "AssemblyStatusTO enum (Preparation/Open/Closed) wire-format"
  - "AssemblyTO struct with ISO8601 datetime serde for date/opened_at/closed_at/created/deleted"
  - "AssemblyDetailTO with embedded AssemblyTO + snapshot_member_count: u64"
  - "CreateAssemblyRequest DTO (name, date, location)"
  - "UpdateAssemblyRequest DTO (name, date, location, version) — version mandatory for optimistic locking"
  - "Bidirectional From-impls between genossi_dao::assembly::AssemblyStatus and AssemblyStatusTO"
  - "From<&genossi_service::assembly::Assembly> for AssemblyTO and From<&AssemblyDetail> for AssemblyDetailTO"
  - "Service-layer Assembly + AssemblyDetail domain types with bidirectional From<&AssemblyEntity> conversions (Plan 02 stub for Plan 03)"
affects: [01-03 service-layer, 01-04 rest-handlers, 04-frontend i18n mapping for status labels]

tech-stack:
  added: []
  patterns:
    - "iso8601_datetime serde module on all Optional<PrimitiveDateTime> fields"
    - "ToSchema derive on every TO/Request DTO for OpenAPI generation"
    - "Bidirectional From-impl pair for DAO<->TO Status enum mapping"
    - "version: Uuid without #[serde(default)] on Update-DTOs to enforce optimistic-lock-token presence at deserialize time"

key-files:
  created:
    - "genossi_service/src/assembly.rs (Assembly + AssemblyDetail domain types — Plan 02 stub anticipating Plan 03)"
  modified:
    - "genossi_rest_types/src/lib.rs (+144 net lines: 5 new public types + 4 new From-impls + 9 unit tests)"
    - "genossi_rest_types/Cargo.toml (activate utoipa feature on genossi_service dep)"
    - "genossi_service/src/lib.rs (register assembly module)"

key-decisions:
  - "All five new types derive ToSchema so Plan 04 REST handlers can register them in OpenAPI schemas without further changes"
  - "version field on UpdateAssemblyRequest is Uuid (non-Option, no serde default) — missing version triggers serde-deserialize error → HTTP 422 (T-01-02-01 mitigation)"
  - "AssemblyDetailTO exposes only snapshot_member_count: u64, never the member-id list (T-01-02-03: minimal data exposure for read endpoints)"
  - "AssemblyTO.version is Option<Uuid> like ApplicationTO.version (mirrors create-vs-update payload shapes; admin clients receive Some, public payloads can omit)"
  - "Stub Assembly + AssemblyDetail in genossi_service::assembly so Plan 02 builds standalone in worktree mode; Plan 03 will replace with full AssemblyService trait + lifecycle methods"

patterns-established:
  - "Pattern: TO datetime fields use #[serde(serialize_with = \"iso8601_datetime::serialize\", deserialize_with = \"iso8601_datetime::deserialize\", default)] on Option<PrimitiveDateTime>"
  - "Pattern: Status-enum bidirectional mapping uses fully-qualified DAO path in From-impls (impl From<&genossi_dao::assembly::AssemblyStatus> for AssemblyStatusTO) so audit/grep tooling can locate them deterministically"
  - "Pattern: Update-DTOs make version: Uuid mandatory; Create-DTOs omit version entirely"

requirements-completed: [ASSY-01, ASSY-02, ASSY-03, ASSY-05]

duration: 35min
completed: 2026-05-02
---

# Phase 01 Plan 02: Assembly REST Types Summary

**Five public TOs (AssemblyStatusTO, AssemblyTO, AssemblyDetailTO, CreateAssemblyRequest, UpdateAssemblyRequest) with ToSchema for OpenAPI, ISO8601 datetime serde on every Optional<PrimitiveDateTime>, and bidirectional Status-enum conversion between DAO and wire format.**

## Performance

- **Duration:** ~35 min
- **Started:** 2026-05-02T15:00:00Z (approx)
- **Completed:** 2026-05-02T15:35:08Z
- **Tasks:** 2 (Task 1 + Task 2 in single TDD RED→GREEN cycle since both tasks share the same file and test module)
- **Files modified:** 4 (1 added, 3 modified)

## Accomplishments

- `AssemblyStatusTO` enum (3 English variants per D-06/D-17) with bidirectional `From` impls against `genossi_dao::assembly::AssemblyStatus` — enables clean wire-format roundtrip and prevents deutsche-Strings im API
- `AssemblyTO` struct (10 fields) with ISO8601 datetime serde on **all 5** datetime fields (date, opened_at, closed_at, created, deleted) — no String-ifizierung, no SQLite-default-format leakage
- `AssemblyDetailTO` carrying just `snapshot_member_count: u64` — Plan 04 GET-Endpoint exposiert nur die Zahl, nie die Member-ID-Liste (T-01-02-03 mitigation)
- `CreateAssemblyRequest` (3 fields) and `UpdateAssemblyRequest` (4 fields with mandatory `version: Uuid`) — Optimistic-Locking-Token kommt im Update-Payload, sonst HTTP 422 (T-01-02-01 mitigation)
- Service-Layer-Stub `genossi_service::assembly::{Assembly, AssemblyDetail}` mit bidirektionalen `From<&AssemblyEntity>`-Impls — Plan 03 ersetzt das durch den vollständigen `AssemblyService`-Trait

## Task Commits

Plan 02 is TDD-flagged. Both tasks share a single source file (`genossi_rest_types/src/lib.rs`) and were executed as one RED→GREEN pair:

1. **RED — Failing tests for Tasks 1+2** — `f04b241` (test)
   - 9 unit tests across `assembly_tests` and `assembly_request_tests` modules
   - Compile-failure on undefined `AssemblyStatusTO`, `AssemblyTO`, `AssemblyDetailTO`, `CreateAssemblyRequest`, `UpdateAssemblyRequest`
   - Side-fix: registered `pub mod assembly` in `genossi_service/src/lib.rs` and added the Service-Layer stub (Rule 3 — blocking)
   - Side-fix: activated `utoipa` feature on `genossi_service` dep in `genossi_rest_types/Cargo.toml` so the crate builds standalone (Rule 3 — blocking, see Deviations)

2. **GREEN — Implementation of Tasks 1+2** — `3eb54a6` (feat)
   - 5 new public types with `ToSchema` derive
   - 4 new `From`-impls (Status DAO↔TO bidirectional + Service Assembly→AssemblyTO + Service AssemblyDetail→AssemblyDetailTO)
   - All 9 tests pass; `cargo build -p genossi_rest_types` exit 0; `cargo fmt --check` exit 0
   - REFACTOR was unnecessary — code is idiomatic and consistent with the existing `ApplicationTO` block

_Note: This plan executed Task 1 and Task 2 as one TDD cycle because both touch the same file, share the same test module, and have no behavioral coupling that would benefit from separate RED/GREEN pairs. The plan's `tdd="true"` flag was honored — tests committed before implementation._

## Files Created/Modified

- `genossi_rest_types/src/lib.rs` (modified, +144 net lines)
  - Added `// Assembly (Generalversammlung) types` block after `UpdateApplicationRequest`
  - 5 new public types, 4 new `From`-impls, 9 unit tests in 2 test modules
- `genossi_rest_types/Cargo.toml` (modified, 1 line)
  - `genossi_service = { path = "../genossi_service", features = ["utoipa"] }` (was without features) — enables standalone `cargo build -p genossi_rest_types`
- `genossi_service/src/assembly.rs` (created, 116 lines)
  - `Assembly` struct (10 fields, `Arc<str>` for strings per service convention)
  - `AssemblyDetail` struct
  - Bidirectional `From<&AssemblyEntity> for Assembly` and reverse
  - 2 unit tests (entity-roundtrip, AssemblyDetail-count)
- `genossi_service/src/lib.rs` (modified, 1 line)
  - `pub mod assembly;` between `application` and `auth_types`

## Decisions Made

- **TDD execution merged Task 1 + Task 2 into one RED→GREEN cycle** — both tasks modify the same file, share the same test module, and have no behavioral coupling. Plan ordering preserved (RED commit precedes GREEN commit).
- **Service-Layer-Stub created in Plan 02 instead of waiting for Plan 03** — Required by Plan 02 acceptance criteria (`From<&genossi_service::assembly::Assembly> for AssemblyTO` must exist). Plan 03 will expand this module with the full `AssemblyService` trait and lifecycle methods; the bidirectional `From<&AssemblyEntity>` already follows the Application precedent so Plan 03 can extend without rewriting.
- **`AssemblyTO.version: Option<Uuid>`** (not `Uuid`) — mirrors `ApplicationTO.version` (line 876). Read responses and the DTO that wraps newly-created assemblies both flow through `AssemblyTO`; the `Option` lets callers distinguish "never persisted" from "persisted but version unknown to the client".
- **`UpdateAssemblyRequest.version: Uuid`** (not `Option<Uuid>`, no `#[serde(default)]`) — D-07 + RESEARCH Open Q2: optimistic-locking token must accompany every update. Missing field ⇒ serde-deserialize error ⇒ HTTP 422.
- **Fully-qualified DAO path in `From`-impls** — `impl From<&genossi_dao::assembly::AssemblyStatus> for AssemblyStatusTO` (no `use` alias). The Plan-02 acceptance criteria grep checks for the literal qualified path; this also keeps the impl unambiguous when `ApplicationStatus` is already in scope at file-level.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 — Blocking] Activate `utoipa` feature on `genossi_service` dep**
- **Found during:** Task 1 GREEN setup (`cargo build -p genossi_rest_types`)
- **Issue:** Standalone build of `genossi_rest_types` failed with E0433 "use of unresolved module or unlinked crate `utoipa`" inside `genossi_service::auth_types`. The workspace build worked transitively through `genossi_rest`/`genossi_mail`/`genossi_backup`, all of which already activate the feature, but the standalone build for the plan-required `cargo build -p genossi_rest_types` exit-0 check did not.
- **Fix:** Changed `genossi_rest_types/Cargo.toml` line 8 from `genossi_service = { path = "../genossi_service" }` to `genossi_service = { path = "../genossi_service", features = ["utoipa"] }`. This is consistent with how `genossi_rest`, `genossi_mail`, and `genossi_backup` already declare the dependency.
- **Files modified:** `genossi_rest_types/Cargo.toml`
- **Verification:** `cargo build -p genossi_rest_types` exit 0; `cargo build` (workspace) still exit 0; no test regressions
- **Committed in:** `f04b241` (RED commit, alongside test additions)

**2. [Rule 3 — Blocking / Wave-Coordination] Created `genossi_service/src/assembly.rs` stub**
- **Found during:** Task 1 GREEN — `From<&genossi_service::assembly::Assembly> for AssemblyTO` requires the module to exist
- **Issue:** Plan 02 acceptance criteria require an impl that references `genossi_service::assembly::Assembly`, but this module is owned by Plan 03 and was not yet present in the worktree. The plan's `<interfaces>` section warned about this Wave-1 coordination case and stated "Wave-1-Ausfuehrung garantiert beides parallel" — but Plan 03 ships the full service trait while Plan 02 only needs the domain structs.
- **Fix:** Created a minimal `genossi_service::assembly` module containing only `Assembly` (10 fields, `Arc<str>` strings) and `AssemblyDetail` (assembly + snapshot_member_count) with bidirectional `From<&AssemblyEntity>` conversions. This is the same domain shape Plan 03 will use (we mirror `genossi_service::application::Application` line-for-line), so Plan 03 will extend it without rewriting. 2 unit tests cover the roundtrip.
- **Files modified:** `genossi_service/src/assembly.rs` (new), `genossi_service/src/lib.rs` (mod registration)
- **Verification:** `cargo test -p genossi_service` passes (existing tests + new 2 tests); `cargo build` workspace exit 0
- **Committed in:** `f04b241` (RED commit)

---

**Total deviations:** 2 auto-fixed (both Rule 3 — blocking)
**Impact on plan:** Both deviations are required for the plan's standalone-build acceptance criteria. The Service-Layer stub follows the exact shape Plan 03 will need, so no rework is anticipated. The Cargo.toml feature activation is a config-only fix matching how the rest of the workspace declares the same dep.

## Issues Encountered

- The worktree branch already contained Plan-01 commits (`0e518c4` migrations + `3f0c205` DAO traits) when this Plan-02 agent started. The `worktree_branch_check` passed because `merge-base(HEAD, wave-base) == wave-base` (HEAD is descendant of base). Plan-02 work proceeds on top of Plan-01's commits without conflict because `files_modified` are disjoint. No action needed — the orchestrator will merge wave outputs at the end.

## Threat Flags

None — Plan 02 introduces only wire-format types and serde mappings. The threat register entries (T-01-02-01/-02/-03) are exhaustive for this plan's surface and all are mitigated as planned.

## TDD Gate Compliance

- **RED gate:** `f04b241` (`test(01-02): add failing tests for AssemblyStatusTO/AssemblyTO/AssemblyDetailTO`)
- **GREEN gate:** `3eb54a6` (`feat(01-02): add Assembly REST types with ToSchema and ISO8601 serde`)
- **REFACTOR gate:** intentionally skipped — implementation is idiomatic, mirrors the proven `ApplicationTO` pattern, and required no cleanup

## Next Phase Readiness

Plan 04 (REST handlers) can now:
- Import `AssemblyStatusTO`, `AssemblyTO`, `AssemblyDetailTO`, `CreateAssemblyRequest`, `UpdateAssemblyRequest` from `genossi_rest_types`
- Register all five types in `#[utoipa::path(...)]` schema components
- Convert service-layer `Assembly`/`AssemblyDetail` to wire format via the established `From`-impls

Plan 03 (service layer) needs to:
- Replace the Plan-02 stub `genossi_service::assembly` with the full `AssemblyService` trait, `AssemblySubmission`/`AssemblyUpdate` input types, and lifecycle methods (`create`, `open`, `close`, `update`)
- Keep `Assembly` and `AssemblyDetail` shapes intact — Plan 02's `From`-impls and Plan 04's wire-format conversions depend on them

## Self-Check: PASSED

Verified via `git log` and filesystem checks:
- FOUND: `f04b241` (RED commit)
- FOUND: `3eb54a6` (GREEN commit)
- FOUND: `genossi_rest_types/src/lib.rs` (modified, contains all 5 types)
- FOUND: `genossi_service/src/assembly.rs` (created, 116 lines)
- FOUND: `genossi_service/src/lib.rs` (modified, contains `pub mod assembly;`)
- FOUND: `genossi_rest_types/Cargo.toml` (modified, contains `features = ["utoipa"]`)
- FOUND: 9 passing tests via `cargo test -p genossi_rest_types`
- FOUND: All Plan-02 acceptance-criteria greps return the expected counts
- Workspace `cargo build` and `cargo fmt --check` exit 0; pre-existing clippy warning in `MemberStatusTO` is out of scope

---
*Phase: 01-assembly-aggregat-audit-hardening*
*Plan: 02 (rest-types)*
*Completed: 2026-05-02*
