---
phase: 25-application-file-upload-audited-carryover
plan: 02
subsystem: database
tags: [sqlx, sqlite, dao, migration, application-documents, single-slot, partial-unique-index]

requires:
  - phase: 25-application-file-upload-audited-carryover
    provides: CONTEXT.md decision #5 (schema minimum + no-Auditable rule) and REQUIREMENTS.md APDOC-01 wording
provides:
  - application_documents SQLite table with partial unique index enforcing single-slot invariant
  - ApplicationDocumentDao trait with find_active_by_application_id helper (default impl)
  - ApplicationDocumentEntity struct (narrow schema — no document_type, no description)
  - ApplicationDocumentDaoImpl SQLite implementation with optimistic-locking update
affects: [25-03 service layer, 25-04 confirm carryover, 25-05 REST endpoints, 25-06 frontend, 25-07 tests]

tech-stack:
  added: []
  patterns:
    - "Partial unique index (`WHERE deleted IS NULL`) as belt-and-suspenders for a service-layer invariant"
    - "Fixture DAO in trait tests to exercise default method impls (mockall cannot cover default methods)"
    - "Test seed helper `seed_application` because FK `REFERENCES application(id)` requires a parent row in in-memory tests"

key-files:
  created:
    - migrations/sqlite/20260703000000_create_application_documents_table.sql
    - genossi_dao/src/application_document.rs
    - genossi_dao_impl_sqlite/src/application_document.rs
  modified:
    - genossi_dao/src/lib.rs
    - genossi_dao_impl_sqlite/src/lib.rs

key-decisions:
  - "Test fixture uses inline `impl ApplicationDocumentDao for FixtureDao` (mirrors the intent behind member_document tests) — chosen over mockall because default methods (find_active_by_application_id, all) can only be exercised through a real impl that provides `dump_all`."
  - "SQLite test DB embeds the migration file via `include_str!` and splits on `;` so the test suite fails if the migration diverges from the schema the DAO expects."
  - "Test setup seeds a stub `application(id)` parent table because SQLite enforces FK targets at INSERT time even when foreign_keys pragma default is off; without this, all roundtrip tests fail with `no such table: main.application`."

patterns-established:
  - "Pattern: minimal-DAO shape (create/update/dump_all required + all/find_by_id/find_active_by_application_id as default methods) for narrow-schema entities that do NOT implement Auditable."
  - "Pattern: partial-unique-index migration comment names the invariant it enforces (`one_active`) and links to the CONTEXT decision."

requirements-completed:
  - APDOC-01

coverage:
  - id: D1
    description: "application_documents SQLite migration with single-slot partial unique index"
    requirement: APDOC-01
    verification:
      - kind: unit
        ref: "genossi_dao_impl_sqlite/src/application_document.rs#test_application_document_partial_unique_index_enforces_single_slot"
        status: pass
    human_judgment: false
  - id: D2
    description: "ApplicationDocumentDao trait + entity in genossi_dao (no Auditable impl, no document_type/description fields)"
    requirement: APDOC-01
    verification:
      - kind: unit
        ref: "genossi_dao/src/application_document.rs#test_find_active_by_application_id_returns_active_row"
        status: pass
      - kind: unit
        ref: "genossi_dao/src/application_document.rs#test_find_active_by_application_id_ignores_soft_deleted_row"
        status: pass
      - kind: unit
        ref: "genossi_dao/src/application_document.rs#test_all_filters_soft_deleted"
        status: pass
    human_judgment: false
  - id: D3
    description: "SQLite implementation of ApplicationDocumentDao with roundtrip + optimistic-lock update"
    requirement: APDOC-01
    verification:
      - kind: integration
        ref: "genossi_dao_impl_sqlite/src/application_document.rs#test_application_document_roundtrip_create_find_softdelete"
        status: pass
      - kind: integration
        ref: "genossi_dao_impl_sqlite/src/application_document.rs#test_application_document_update_version_mismatch_conflict"
        status: pass
    human_judgment: false

duration: 9min
completed: 2026-07-03
status: complete
---

# Phase 25 Plan 02: application_documents DAO + SQL migration Summary

**SQLite `application_documents` table with single-slot partial unique index, plus a narrow-schema `ApplicationDocumentDao` trait (no Auditable, no document_type/description) and its SQLite implementation with optimistic-locking update.**

## Performance

- **Duration:** 9 min
- **Started:** 2026-07-02T23:59:33Z
- **Completed:** 2026-07-03T00:08:33Z
- **Tasks:** 3
- **Files created:** 3 (1 migration, 2 Rust modules)
- **Files modified:** 2 (both lib.rs re-exports)

## Accomplishments

- Migration `20260703000000_create_application_documents_table.sql` establishes the `application_documents` table with the CONTEXT decision-#5 minimum columns (`id`, `application_id`, `file_name`, `mime_type`, `relative_path`, `size`, `created`, `deleted`, `version`) — deliberately WITHOUT `document_type` and `description`. Belt-and-suspenders partial unique index (`WHERE deleted IS NULL`) enforces the single-slot invariant at the storage boundary, mirroring the service-layer `find_active_by_application_id + branch` pattern that Wave 2 will use.
- `ApplicationDocumentEntity` and `ApplicationDocumentDao` in `genossi_dao/src/application_document.rs` provide the minimal-DAO shape (`create`/`update`/`dump_all` required, plus `all`/`find_by_id`/`find_active_by_application_id` as default methods). The entity intentionally does **NOT** implement `Auditable` — confirmed by the grep gate. Unit tests cover the default `find_active_by_application_id` filter with a hand-rolled fixture DAO (mockall cannot exercise default methods).
- `ApplicationDocumentDaoImpl` in `genossi_dao_impl_sqlite/src/application_document.rs` uses `sqlx::query`/`query_as` string queries (no `sqlx::query!` macros, per CONTEXT Pitfall 2 — no `.sqlx/` regen required). The update path uses `WHERE id = ? AND version = ?` and returns `DaoError::ConflictError` on `rows_affected() == 0`. Three tokio tests cover create → find → soft-delete roundtrip, version-mismatch conflict, and the partial-unique-index enforcement of the single-slot invariant.

## Task Commits

Each task was committed atomically via jj:

1. **Task 1: Migration** — `38fd1094` (feat) — `feat(25-02): add application_documents migration (single-slot partial unique index)`
2. **Task 2: DAO trait + entity** — `183076c8` (feat) — `feat(25-02): add ApplicationDocumentDao trait + entity (no Auditable, single-slot)`
3. **Task 3: SQLite impl** — `97331396` (feat) — `feat(25-02): add SQLite impl for ApplicationDocumentDao (in-memory roundtrip test)`

## Files Created/Modified

- **CREATED** `migrations/sqlite/20260703000000_create_application_documents_table.sql` — Table, partial unique index (`WHERE deleted IS NULL`), and `deleted`-lookup index.
- **CREATED** `genossi_dao/src/application_document.rs` — Entity struct + trait + 3 unit tests using a fixture DAO.
- **CREATED** `genossi_dao_impl_sqlite/src/application_document.rs` — `sqlx::FromRow` mirror struct + `TryFrom` + DAO impl + 3 tokio integration tests.
- **MODIFIED** `genossi_dao/src/lib.rs` — added `pub mod application_document;` between `application` and `assembly`.
- **MODIFIED** `genossi_dao_impl_sqlite/src/lib.rs` — same module registration.

## Decisions Made

- **Test fixture pattern over mockall for trait defaults.** The trait's `find_active_by_application_id` is a default method; mockall generates a mock of the *methods*, not the defaults, so the test uses a small `FixtureDao` that only implements `dump_all` (returning a hard-coded Vec) and lets the default filter run. This tests the *contract*, not the mock's own implementation.
- **Embed migration file in the SQLite test setup via `include_str!`.** If a future maintainer edits the migration without touching the tests, the roundtrip test fails immediately — the schema is exercised by the same DDL that ships to production. Splits on `;` so both the `CREATE TABLE` and both index creations execute.
- **Seed a stub `application(id)` row before every roundtrip test.** SQLite enforces FK target existence at INSERT time even when `foreign_keys` pragma is on-by-default in newer builds; the alternative (turning FKs off) would hide a class of production bugs. The stub table has only `id BLOB PRIMARY KEY` — enough to satisfy the FK, nothing else.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 — Blocking] Added stub `application` parent table + seeder in test setup**
- **Found during:** Task 3 (SQLite impl tests)
- **Issue:** First test run failed with `no such table: main.application` on INSERT — SQLite defers FK-target existence checking to statement execution. Every roundtrip test panicked before exercising the DAO under test.
- **Fix:** `setup_db()` now creates a minimal `application(id BLOB PRIMARY KEY NOT NULL)` table before applying the migration; a new `seed_application(&pool, app_id)` helper inserts a parent row before each test. This is scoped to test-only code and does not affect production migrations, which run in dependency order.
- **Files modified:** `genossi_dao_impl_sqlite/src/application_document.rs` (test module only).
- **Verification:** All 3 tokio tests now pass; the FK constraint is respected end-to-end.
- **Committed in:** `97331396` (Task 3 commit).

**2. [Rule 1 — Bug] rustfmt import ordering in genossi_dao_impl_sqlite/src/application_document.rs**
- **Found during:** post-Task-3 formatting check
- **Issue:** `use crate::TransactionImpl;` was placed before `use crate::datetime_utils::parse_datetime;`, violating rustfmt's ordering (nested paths sort before bare crate items — actually, the reverse: `crate::datetime_utils` sorts before `crate::TransactionImpl` because lowercase < uppercase in ASCII, which is what rustfmt enforces).
- **Fix:** Swapped the two lines.
- **Files modified:** `genossi_dao_impl_sqlite/src/application_document.rs`.
- **Verification:** `cargo fmt -p genossi_dao_impl_sqlite -- --check` no longer flags this file (widespread pre-existing repo formatting deviations in other files remain — out of scope per Scope Boundary rule).
- **Committed in:** `97331396` (Task 3 commit — same commit as the file introduction).

---

**Total deviations:** 2 auto-fixed (1 blocking test infrastructure, 1 minor formatting).
**Impact on plan:** Both auto-fixes stay strictly within Task 3's scope. No changes to trait shape, entity fields, or migration schema. No new dependencies. All original acceptance criteria still hold as specified.

## Issues Encountered

- **jj commit workflow:** Repository uses Jujutsu VCS (`.jj/` present, per project memory). All task commits used `jj describe -m "..." && jj new` — three change IDs `lkxvslll`, `kpwuplkt`, `smywrmvl`, resolving to commit hashes `38fd1094`, `183076c8`, `97331396`. Sequential wave (no parallel work), so no worktree isolation concerns.
- **Widespread pre-existing rustfmt deviations** across ~18 files in `genossi_dao_impl_sqlite/` and elsewhere. Not caused by this plan and out of scope (Scope Boundary rule). Deferred — should be handled by a repo-wide `cargo fmt` sweep in a dedicated cleanup phase.

## User Setup Required

None — no external service configuration required. The new migration will auto-apply on next server startup via the existing `sqlx::migrate!` in `genossi_bin`.

## Next Phase Readiness

- **Wave 2 (Service layer, Plan 25-03) is unblocked.** The compiling DAO surface exposes exactly what the service will call: `find_active_by_application_id` (for the create-vs-update branch in `upload/replace`), `create`/`update` (for the primary write paths), and the entity struct for the `MemberDocument` construction during `confirm()` carryover (Wave 3).
- **Migration ordering verified.** `ls migrations/sqlite/ | sort | tail -1` = `20260703000000_create_application_documents_table.sql` — lexicographically newest, safely after `20260702000002_mail_recipients_add_rendered_html_body.sql`.
- **No blockers.** No new dependencies, no `.sqlx/` cache regen needed (no `sqlx::query!` macros anywhere), no schema migration for existing data (fresh table).

## Threat Model Compliance

All four mitigations from PLAN.md `<threat_model>` are in place:

| Threat ID | Mitigation | Where |
|---|---|---|
| T-25-02-01 (Tampering: SQL injection) | All queries use `.bind()` — zero string interpolation, zero `sqlx::query!` macros. | `genossi_dao_impl_sqlite/src/application_document.rs` throughout. |
| T-25-02-02 (Tampering: single-slot invariant) | Partial unique index `WHERE deleted IS NULL` in migration; `find_active_by_application_id` default trait method for the service branch. | Migration + `genossi_dao/src/application_document.rs`. |
| T-25-02-03 (EoP: Auditable creep) | Grep gate `grep -c "impl crate::auditable::Auditable" genossi_dao/src/application_document.rs == 0` passes. | Explicitly NO `impl Auditable`. |
| T-25-02-04 (InfoDisclosure: optimistic-locking miss) | `UPDATE ... WHERE id = ? AND version = ?` + `rows_affected() == 0` → `ConflictError`. Test `test_application_document_update_version_mismatch_conflict` proves it fires. | `genossi_dao_impl_sqlite/src/application_document.rs` update fn. |

No new threat surface introduced beyond the plan's threat register.

## Self-Check: PASSED

Verified via absolute-path checks:

- `[ -f /home/neosam/programming/rust/projects/genossi3/migrations/sqlite/20260703000000_create_application_documents_table.sql ]` → FOUND
- `[ -f /home/neosam/programming/rust/projects/genossi3/genossi_dao/src/application_document.rs ]` → FOUND
- `[ -f /home/neosam/programming/rust/projects/genossi3/genossi_dao_impl_sqlite/src/application_document.rs ]` → FOUND
- Commits `38fd1094`, `183076c8`, `97331396` present in jj log
- `cargo test -p genossi_dao application_document::tests` → 3 passed, 0 failed
- `cargo test -p genossi_dao_impl_sqlite application_document::tests` → 3 passed, 0 failed
- `cargo build -p genossi_dao -p genossi_dao_impl_sqlite` → clean

---
*Phase: 25-application-file-upload-audited-carryover*
*Completed: 2026-07-03*
