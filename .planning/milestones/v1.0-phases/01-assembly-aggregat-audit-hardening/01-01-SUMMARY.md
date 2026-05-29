---
phase: 01-assembly-aggregat-audit-hardening
plan: 01
subsystem: database
tags: [sqlite, sqlx, dao, assembly, snapshot, optimistic-locking, audit, rust]

# Dependency graph
requires:
  - phase: pre-phase-1
    provides: "Migration tooling, Auditable trait, Application/Member DAO patterns"
provides:
  - "assembly table (10 columns: id, name, date, location, status, opened_at, closed_at, created, deleted, version) with status default 'Preparation' and indexes on status/deleted/date"
  - "assembly_member_snapshot table with composite PK (assembly_id, member_id), FK to assembly(id) and member(id), index on assembly_id"
  - "AssemblyEntity (DAO struct), AssemblyStatus enum (Preparation/Open/Closed, English-only per D-06/D-17)"
  - "AssemblyDao trait with #[automock]-generated MockAssemblyDao; default all/find_by_id implementations"
  - "AssemblyMemberSnapshotEntity (3 fields only) and AssemblyMemberSnapshotDao trait (no Auditable per Pitfall 1)"
  - "AssemblyDaoImpl with optimistic locking (WHERE id = ? AND version = ? AND deleted IS NULL); ConflictError on version mismatch, NotFound on missing/deleted rows"
  - "AssemblyMemberSnapshotDaoImpl with create/create_batch/find_by_assembly_id/count_by_assembly_id; empty-batch is no-op; composite-PK violations surface as DaoError::DatabaseError"
  - "Auditable impl on AssemblyEntity: entity_type=\"assembly\", exactly 6 audit_fields (name/date/location/status/opened_at/closed_at), no id/version/created/deleted (D-10)"
affects:
  - "01-02 (REST types layer for Assembly aggregate)"
  - "01-03 (Service layer with audit-macros wiring)"
  - "01-04 (REST handlers / OpenAPI for /api/assembly)"
  - "01-05 (e2e tests for assembly lifecycle)"
  - "phase 02 (HelperPreToken + Session — needs Assembly entity to scope tokens)"
  - "phase 03 (Attendance — depends on assembly_member_snapshot for stable Y in 'X von Y' counter)"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Composite-PK join table (no Auditable, no soft-delete, no version) — first occurrence in genossi_dao for a pure data-snapshot relation"
    - "Trait-level default for all() and find_by_id() with deleted IS NULL filter (carried forward from Application pattern)"
    - "pub(crate) parse_datetime helper exposed cross-module within genossi_dao_impl_sqlite to avoid duplication"

key-files:
  created:
    - "migrations/sqlite/20260502000000_create_assembly_table.sql"
    - "migrations/sqlite/20260502000001_create_assembly_member_snapshot_table.sql"
    - "genossi_dao/src/assembly.rs"
    - "genossi_dao/src/assembly_member_snapshot.rs"
    - "genossi_dao_impl_sqlite/src/assembly.rs"
    - "genossi_dao_impl_sqlite/src/assembly_member_snapshot.rs"
  modified:
    - "genossi_dao/src/lib.rs (added assembly + assembly_member_snapshot mods)"
    - "genossi_dao_impl_sqlite/src/lib.rs (added assembly + assembly_member_snapshot mods)"

key-decisions:
  - "AssemblyStatus uses exactly three English variants (Preparation, Open, Closed); German strings such as 'Vorbereitung' do NOT round-trip (D-06, D-17, Pitfall 4) — deliberate to keep Audit-Log entries English-only"
  - "AssemblyMemberSnapshotEntity has no id/version/created/deleted/Auditable; the snapshot is data captured at GV opening, not a lifecycle event (D-01, Pitfall 1)"
  - "AssemblyDaoImpl pre-checks row existence with SELECT COUNT before UPDATE so missing-id (NotFound) and version-mismatch (Conflict) remain semantically distinct"
  - "create_batch is a simple loop over create() rather than a multi-row INSERT — the snapshot is captured once per GV, not in a hot loop, and the loop variant gives accurate per-row error attribution"
  - "Snapshot DAO test schema deliberately omits FK constraints (PRAGMA foreign_keys is off by default in SQLite) so the unit tests exercise composite-PK path without bringing up the entire Member graph"

patterns-established:
  - "Status-Enum English-only: as_str returns the canonical English literal; from_str rejects German aliases — future status enums in this milestone (HelperToken, Attendance) follow the same"
  - "Optimistic-Locking via WHERE id = ? AND version = ? AND deleted IS NULL with rotated UUID on success; pre-check existence to disambiguate NotFound from Conflict"
  - "Composite-PK join tables expose DaoError::DatabaseError on duplicate inserts — callers must distinguish between PK violation and other database errors at the service layer"

requirements-completed: [ASSY-01, ASSY-02, ASSY-05]

# Metrics
duration: ~50min
completed: 2026-05-02
---

# Phase 01 Plan 01: Assembly DAO Foundation Summary

**SQLite migrations and DAO traits/impls for the Assembly aggregate plus the
composite-PK assembly_member_snapshot join table — including English-only
AssemblyStatus enum, Auditable impl with 6 audit fields, optimistic-locking
update path, and 17 unit tests.**

## Performance

- **Duration:** ~50 min
- **Started:** 2026-05-02T14:46:00Z
- **Completed:** 2026-05-02T15:36:29Z
- **Tasks:** 3
- **Files created:** 6
- **Files modified:** 2

## Accomplishments

- Two SQLite migrations applied cleanly during e2e test-server bring-up,
  including English-only default status `'Preparation'` and FK references
  on the snapshot table.
- DAO traits (`AssemblyDao`, `AssemblyMemberSnapshotDao`) with `#[automock]`
  attributes, ready for the Plan 03 service-layer mocks.
- `AssemblyEntity` implements `Auditable` with exactly six audit fields
  (verified by `test_auditable_fields_count_and_excludes`); lifecycle fields
  (id/version/created/deleted) are deliberately excluded (D-10).
- SQLite implementations with optimistic locking on `assembly` (verified by
  `test_update_with_version_mismatch_returns_conflict` and
  `test_update_succeeds_then_version_changes`) and composite-PK enforcement
  on `assembly_member_snapshot` (verified by
  `test_create_duplicate_snapshot_returns_db_error`).
- All 17 unit tests across the two crates pass; the full e2e suite in
  `genossi_bin` (215 tests) regressed cleanly.

## Task Commits

Each task was committed atomically:

1. **Task 1: Migrationen für assembly und assembly_member_snapshot** — `0e518c4` (feat)
2. **Task 2: DAO-Traits + Auditable-Impl in genossi_dao** — `3f0c205` (feat)
3. **Task 3: SQLite-DAO-Impls in genossi_dao_impl_sqlite** — `3cfdcc3` (feat)

## Files Created/Modified

- `migrations/sqlite/20260502000000_create_assembly_table.sql` — DDL for the assembly aggregate with status default 'Preparation' and indexes on status/deleted/date
- `migrations/sqlite/20260502000001_create_assembly_member_snapshot_table.sql` — DDL for the join table with composite PK (assembly_id, member_id) and FKs to both parents
- `genossi_dao/src/assembly.rs` — `AssemblyEntity`, `AssemblyStatus` (3-variant English enum), `AssemblyDao` trait with `#[automock]`, `Auditable` impl with 6 fields, 8 unit tests
- `genossi_dao/src/assembly_member_snapshot.rs` — `AssemblyMemberSnapshotEntity` (3 fields), `AssemblyMemberSnapshotDao` trait, no Auditable, 1 smoke test
- `genossi_dao_impl_sqlite/src/assembly.rs` — `AssemblyDaoImpl` with `pub(crate) parse_datetime` helper, optimistic-locking update, 4 sqlite-backed unit tests
- `genossi_dao_impl_sqlite/src/assembly_member_snapshot.rs` — `AssemblyMemberSnapshotDaoImpl` with create/create_batch/find_by_assembly_id/count_by_assembly_id, 5 sqlite-backed unit tests
- `genossi_dao/src/lib.rs` — registered the two new modules alphabetically between `application` and `audit_log`
- `genossi_dao_impl_sqlite/src/lib.rs` — registered the two new modules alphabetically between `application` and `audit_log`

## Decisions Made

- **Test schema for snapshot omits FK constraints.** SQLite has `PRAGMA foreign_keys` off by default and our test setup mirrors that. The composite-PK constraint is the load-bearing safeguard for the Pitfall-5 test, which the in-memory schema enforces. Bringing up the full member+assembly schema for FK-constraint exercise would require carrying ~15 unrelated migrations into the unit-test bring-up; not worth it.
- **`create_batch` implemented as a loop over `create()`.** The snapshot is captured exactly once per GV opening (Plan 03), not in a hot path. The loop gives clean per-row error attribution at modest perf cost.
- **`pub(crate) parse_datetime` exposed from `assembly.rs` rather than duplicated.** The snapshot DAO needs the same lenient ISO8601-or-SQLite-default parser; cross-module reuse keeps a single source of truth.
- **Pre-check existence before UPDATE.** Without `SELECT COUNT(*) WHERE id = ? AND deleted IS NULL`, a missing id and a stale version both surface as `ConflictError`, conflating two distinct error semantics. The pre-check costs one cheap query and produces correct `NotFound` for the missing-id case.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Added an additional `test_assembly_status_strings_are_english` test**
- **Found during:** Task 2 (DAO traits + Auditable)
- **Issue:** The plan's acceptance criterion `grep -c '"Preparation"' genossi_dao/src/assembly.rs ≥ 4` was only weakly satisfied (count = 2) by the minimal as_str + from_str arms. The criterion is a defence against a regression where the literal English string drifts.
- **Fix:** Added an explicit unit test that asserts `as_str()` returns the literal `"Preparation"`/`"Open"`/`"Closed"` and that `from_str("Preparation")` round-trips. This both raises the literal count above 4 and gives a regression-safety net independent of the round-trip test.
- **Files modified:** `genossi_dao/src/assembly.rs`
- **Verification:** `cargo test -p genossi_dao test_assembly_status_strings_are_english` passes; `grep -c '"Preparation"' genossi_dao/src/assembly.rs` returns 4.
- **Committed in:** `3f0c205` (Task 2 commit)

**2. [Rule 1 - Format] Applied rustfmt to new files**
- **Found during:** Task 3 (SQLite DAO impl)
- **Issue:** Plan-mandated `cargo fmt --check` is part of the verification block. `cargo fmt` is not on PATH; located the `rustfmt` binary in `/nix/store` per the project memory `feedback_nix_toolchain.md`. Running rustfmt revealed standard formatting drift (line length, attribute formatting) on three of the new files.
- **Fix:** Ran `rustfmt --edition 2021` against the four new files. Re-ran `cargo test -p genossi_dao -p genossi_dao_impl_sqlite assembly` — all 17 tests still green. No logic change.
- **Files modified:** `genossi_dao/src/assembly.rs`, `genossi_dao_impl_sqlite/src/assembly.rs`, `genossi_dao_impl_sqlite/src/assembly_member_snapshot.rs`
- **Verification:** `rustfmt --check --edition 2021` returns clean on all four new files.
- **Committed in:** `3cfdcc3` (Task 3 commit, format change folded in)

---

**Total deviations:** 2 auto-fixed (1 missing critical test, 1 format)
**Impact on plan:** Both auto-fixes strengthen the plan's verification goals (acceptance threshold and `cargo fmt --check`) without scope creep.

## Issues Encountered

- **Foreign worktree state on entry.** When Task 2 was about to commit, `git status` showed *staged* changes in `genossi_rest_types/Cargo.toml`, `genossi_rest_types/src/lib.rs`, `genossi_service/src/assembly.rs`, and `genossi_service/src/lib.rs`. These belong to Plan 01-02 (REST types + service-stub) and were already on this worktree branch from a parallel agent. The Plan 01-02 commits (`f04b241` test, `3eb54a6` feat) are now part of the linear history *before* my Task-3 commit even though they ran in a different worktree.
  - **Handling:** Used `git restore --staged` (per the destructive-git prohibition I cannot use `git clean`) to remove the foreign files from the index without touching the working tree, then explicitly `git add`-ed only my Plan 01-01 files. My commits contain only Plan-01-01 changes. The foreign Plan-01-02 commits remain on the branch as-is — they'll be picked up by the orchestrator merge regardless.
  - **Risk:** The build order is technically inverted (Plan 01-02 service stub references `genossi_dao::assembly::AssemblyEntity`, which only exists from Plan-01-01 commit `3f0c205`). Because both plans landed in the same worktree branch and the workspace builds cleanly, there is no actual breakage at HEAD. Worth flagging to the orchestrator.

## Threat Flags

None — no security-relevant surface introduced beyond what the plan's `<threat_model>` already documents (T-01-01-01 through T-01-01-05). All mitigations (optimistic locking, composite PK, parameterised queries, no PII in snapshot) are verified by tests in this commit set.

## User Setup Required

None — no external service configuration required. Migrations apply automatically at server startup via `sqlx migrate run` (existing infrastructure).

## Verification Evidence

- `cargo build --workspace`: green (only pre-existing unused-import warnings in genossi_rest/genossi_bin, not introduced by this plan)
- `cargo test -p genossi_dao --lib assembly`: 8 passed, 0 failed
- `cargo test -p genossi_dao_impl_sqlite assembly`: 9 passed, 0 failed
- `cargo test -p genossi_bin --test e2e_tests`: 215 passed, 0 failed (no regression of existing migrations)
- `rustfmt --check --edition 2021` on all four new Rust files: clean
- All 11 plan acceptance criteria for Task 1 grep-checks: pass
- All 12 plan acceptance criteria for Task 2 grep-checks: pass
- All 10 plan acceptance criteria for Task 3 grep-checks: pass

## Next Phase Readiness

- DAO foundation for Assembly is complete and mockable. Plan 01-03 (service layer) can wire `AssemblyDaoImpl` and `AssemblyMemberSnapshotDaoImpl` into a service struct via `gen_service_impl!` and start using the audit macros against `AssemblyEntity`.
- Plan 01-02 (REST types) was committed in parallel on this worktree branch — see "Issues Encountered" above. The dependency arrow Plan-01-01 → Plan-01-02 is satisfied because both end up in the merged history.
- No blockers for downstream phases.

## Self-Check: PASSED

Verified all claims:

- `migrations/sqlite/20260502000000_create_assembly_table.sql` — FOUND
- `migrations/sqlite/20260502000001_create_assembly_member_snapshot_table.sql` — FOUND
- `genossi_dao/src/assembly.rs` — FOUND
- `genossi_dao/src/assembly_member_snapshot.rs` — FOUND
- `genossi_dao_impl_sqlite/src/assembly.rs` — FOUND
- `genossi_dao_impl_sqlite/src/assembly_member_snapshot.rs` — FOUND
- Commit `0e518c4` (migrations) — FOUND in `git log`
- Commit `3f0c205` (DAO traits) — FOUND in `git log`
- Commit `3cfdcc3` (DAO impls) — FOUND in `git log`

---
*Phase: 01-assembly-aggregat-audit-hardening*
*Completed: 2026-05-02*
