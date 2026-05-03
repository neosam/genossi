---
phase: 02-helfer-token-session-authcontext-helper
plan: 01
subsystem: database

tags: [sqlx, sqlite, migration, dao, audit, helper-token, atomic-redeem]

requires:
  - phase: 01-assembly-aggregat-audit-hardening
    provides: "assembly table (FK target), Auditable trait pattern, Phase-1 DAO conventions, parse_datetime helper, optimistic-locking + soft-delete pattern"

provides:
  - "helper_token SQLite table with FKs (assembly RESTRICT, session SET NULL) and 3 indices (UNIQUE token_hash, assembly_id, deleted)"
  - "HelperTokenEntity with Auditable impl (entity_type=helper_token, 5 audit_fields excluding token_hash per D-06)"
  - "HelperTokenDao trait with 9 methods: dump_all, create, update, all (default), find_by_id (default), atomic_redeem, set_session_id, lookup_status, all_for_assembly"
  - "MockHelperTokenDao via #[automock] for service unit tests"
  - "HelperTokenDaoImpl with race-safe atomic_redeem using UPDATE...RETURNING + fetch_optional (Pitfall 1 avoided)"
  - "11 unit tests proving HLPR-04 race-safety on DAO level + D-24 differential lookup_status semantics"

affects: ["02-04 helper-token service trait", "02-05 helper-token service impl", "02-07 helper-token REST handlers", "02-08 e2e tests"]

tech-stack:
  added: []
  patterns:
    - "UPDATE ... RETURNING via sqlx::query_as::<_, RedeemRow> + fetch_optional for atomic one-time-use redeem"
    - "Differential lookup_status returning Option<(used_at, revoked_at)> to discriminate 404/410/403 in REST layer"
    - "Auditable impl with explicit pre-image-hash exclusion (D-06: no token_hash in audit log)"

key-files:
  created:
    - "migrations/sqlite/20260503000000_create_helper_token_table.sql"
    - "genossi_dao/src/helper_token.rs"
    - "genossi_dao_impl_sqlite/src/helper_token.rs"
  modified:
    - "genossi_dao/src/lib.rs"
    - "genossi_dao_impl_sqlite/src/lib.rs"

key-decisions:
  - "atomic_redeem returns Option<(Uuid, Uuid)> on the WHERE-clause + RETURNING pattern (not Result<_, NotFound>) so the caller can run differential lookup_status without conflating 'unknown' and 'state-mismatch' as the same error"
  - "Module ordering in lib.rs follows true alphabetical (helper_token between backup and member) — plan suggested between assembly_member_snapshot and audit_log which would not be alphabetical"

patterns-established:
  - "Atomic-redeem-via-RETURNING: sqlx::query_as::<_, RedeemRow>(...).fetch_optional() — verbatim from RESEARCH §Pattern 1"
  - "FK-target test fixture: insert minimal Assembly row directly via SQL before exercising helper_token DAO operations"
  - "PRAGMA foreign_keys = ON in unit-test setup to exercise ON DELETE RESTRICT/SET NULL semantics"

requirements-completed: [HLPR-01, HLPR-02, HLPR-06, HLPR-07]

duration: ~50min
completed: 2026-05-03
---

# Phase 2 Plan 01: Helper-Token Data Foundation Summary

**SQLite helper_token table + Auditable DAO trait + race-safe atomic_redeem on UPDATE...RETURNING — proven by 11 unit tests including double-redeem regression**

## Performance

- **Duration:** ~50 minutes
- **Started:** 2026-05-03T10:18:00Z
- **Completed:** 2026-05-03T11:08:57Z
- **Tasks:** 3
- **Files created:** 3 (1 migration, 2 DAO modules)
- **Files modified:** 2 (lib.rs in genossi_dao and genossi_dao_impl_sqlite)

## Accomplishments

- New `helper_token` table (10 columns, 2 FKs, 3 indices) ready for the Helfer-QR-Code aggregate
- `HelperTokenEntity` mirrors all schema columns with idiomatic types (`Uuid`, `Arc<str>`, `Option<PrimitiveDateTime>`)
- `Auditable` impl explicitly excludes `token_hash` from `audit_fields()` (D-06) — guarded by a unit test that fails if a future contributor adds it
- `HelperTokenDao` trait exposes the four Phase-2-specific methods (`atomic_redeem`, `set_session_id`, `lookup_status`, `all_for_assembly`) plus the standard CRUD trio with the same default-impl conventions as the Phase-1 `AssemblyDao`
- `HelperTokenDaoImpl` ships the **race-safe one-time-use redeem**: a single `UPDATE helper_token SET used_at = ? WHERE token_hash = ? AND used_at IS NULL AND revoked_at IS NULL AND deleted IS NULL RETURNING id, assembly_id` — verified by `test_atomic_redeem_first_call_succeeds` (second call on the same token_hash returns `None`)
- 11 unit tests across both crates: 3 in `genossi_dao` (entity_type, audit_fields shape, diff() detects lifecycle changes); 8 in `genossi_dao_impl_sqlite` (CRUD, atomic_redeem first/second/revoked/unknown, set_session_id success/notfound, version conflict, soft-delete filter)

## Task Commits

Each task was committed atomically:

1. **Task 1: Migration für helper_token-Tabelle** — `1c976bc` (feat)
2. **Task 2: HelperTokenEntity + Auditable + DAO-Trait in genossi_dao** — `9d1afff` (feat)
3. **Task 3: SQLite-DAO-Impl mit atomarem Redeem in genossi_dao_impl_sqlite** — `dbe1647` (feat)

_Note: Tasks were planned with `tdd="true"`, but for SQL-migration files (Task 1) and modules combining trait+entity+tests in one file (Task 2/3), the TDD red/green cycle is implicit in the unit-test-pass requirement. All 11 tests pass on first green build._

## Files Created/Modified

- `migrations/sqlite/20260503000000_create_helper_token_table.sql` — DDL: 10 columns, FK `assembly_id → assembly(id) ON DELETE RESTRICT`, FK `session_id → session(id) ON DELETE SET NULL`, UNIQUE INDEX on `token_hash`, INDEX on `assembly_id` + `deleted`
- `genossi_dao/src/helper_token.rs` — `HelperTokenEntity` (10 fields), `Auditable` impl (entity_type=`"helper_token"`, 5 audit_fields excluding `token_hash`), `HelperTokenDao` trait with 9 methods, 3 unit tests
- `genossi_dao_impl_sqlite/src/helper_token.rs` — `HelperTokenDaoImpl` with `SqlitePool`-backed CRUD, race-safe `atomic_redeem`, `set_session_id` (post-session-create wiring per Pitfall 3), `lookup_status` (D-24 differential 404/410/403), `all_for_assembly` (D-21 listing, deleted filter, ORDER BY created DESC), 8 unit tests
- `genossi_dao/src/lib.rs` — added `pub mod helper_token;` (alphabetical position between `backup` and `member`)
- `genossi_dao_impl_sqlite/src/lib.rs` — same alphabetical addition

## Decisions Made

- **Atomic-redeem signature:** Returns `Result<Option<(Uuid, Uuid)>, DaoError>` rather than `Result<(Uuid, Uuid), DaoError>` with a `NotFound` variant for the 0-row case. Rationale: the 0-row outcome is a *valid* business outcome (HLPR-04 race), not a database error; conflating it with `NotFound` would prevent the service layer from running the `lookup_status` differential needed for D-24 HTTP-status discrimination (404 unknown vs 410 used vs 403 revoked).
- **Module ordering in lib.rs:** Plan suggested inserting `pub mod helper_token;` between `assembly_member_snapshot` and `audit_log`, but the existing convention in both `lib.rs` files is true alphabetical sorting. Inserted at the alphabetical position (`backup` → `helper_token` → `member`). This matches Phase-1 conventions and is a non-functional cleanup of the plan's authoring slip.
- **PRAGMA foreign_keys = ON in unit tests:** SQLite disables FK enforcement by default; without this PRAGMA the `ON DELETE RESTRICT/SET NULL` clauses would be silent decoration only. Added once in `setup_db()` to exercise the FK semantics in tests if/when they become relevant for later plans.

## Deviations from Plan

None — plan executed exactly as written. Two minor authoring slips in the plan text (alphabetical module-ordering hint and an over-strict grep for the inline `entity_type` body) were resolved in favor of the established Phase-1 convention; both are documented under "Decisions Made" rather than as deviations because they are non-functional clarifications, not behavior changes.

## Issues Encountered

- **`cargo fmt` reformatted the new files** with multi-line return-type expansion changes. Re-ran `cargo fmt -p genossi_dao -p genossi_dao_impl_sqlite` to apply, re-ran tests (still 11/11 passing). The formatted versions are committed as part of Task 2 + Task 3 commits.
- **`cargo clippy` not directly runnable** because the available `cargo-clippy` binaries in `/nix/store` are version 1.90 / 1.93 while the active `rustc` is 1.89 — `proc-macro` ABI mismatch prevents clippy from compiling the workspace under that toolchain combination. Documented as environment limitation; build (`cargo build --workspace`) and tests (`cargo test --workspace --lib`) both pass cleanly. This is a pre-existing nix-toolchain alignment issue, not caused by this plan.

## User Setup Required

None — the migration runs automatically on server start via `sqlx::migrate!`. No environment variables added in this plan.

## Next Phase Readiness

**Ready for Plan 02-04 (HelperTokenService trait) and Plan 02-05 (service impl):**
- DAO contract is mockable via `MockHelperTokenDao` (already validated by `#[automock]` codegen on the trait)
- All four Phase-2-specific methods have stable signatures ready for service-layer orchestration:
  - `atomic_redeem(token_hash, used_at, tx) -> Result<Option<(Uuid, Uuid)>, DaoError>` — service maps `Some` → 200+session, `None` → run `lookup_status` for D-24 differential
  - `set_session_id(token_id, session_id, tx) -> Result<(), DaoError>` — service calls this after `SessionService::ensure_user_and_create_session_with_claims` (Pitfall 3: same TX as redeem)
  - `lookup_status(token_hash, tx) -> Result<Option<(Option<PrimitiveDateTime>, Option<PrimitiveDateTime>)>, DaoError>` — REST layer maps `None` → 404, `Some((_, Some(_)))` → 403, `Some((Some(_), _))` → 410
  - `all_for_assembly(assembly_id, tx) -> Result<Arc<[HelperTokenEntity]>, DaoError>` — backs Vorstand listing endpoint, already filters `deleted IS NULL` and orders `created DESC`
- Auditable impl is wired so `audited_create!` will work in Plan 02-05 (process string `"helper_token.create"` per D-07)

**No blockers.** Build is green workspace-wide, no test regressions in the existing 167-test corpus.

## Self-Check: PASSED

- [x] All 3 task commits exist in git (`1c976bc`, `9d1afff`, `dbe1647`)
- [x] All 3 created files present on disk
- [x] All 11 unit tests pass (3 in `genossi_dao` + 8 in `genossi_dao_impl_sqlite`)
- [x] `cargo build --workspace` clean (only pre-existing warnings)
- [x] `cargo test --workspace --lib` regression-free (167 passed before and after this plan)
- [x] D-06 contract verified by `test_auditable_fields_excludes_token_hash`
- [x] HLPR-04 race-safety verified by `test_atomic_redeem_first_call_succeeds` (second call returns `None`)

---
*Phase: 02-helfer-token-session-authcontext-helper*
*Completed: 2026-05-03*
