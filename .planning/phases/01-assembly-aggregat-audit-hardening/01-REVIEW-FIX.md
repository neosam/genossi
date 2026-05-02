---
phase: 01-assembly-aggregat-audit-hardening
fixed_at: 2026-05-02T00:00:00Z
review_path: .planning/phases/01-assembly-aggregat-audit-hardening/01-REVIEW.md
iteration: 1
findings_in_scope: 10
fixed: 10
skipped: 0
status: all_fixed
---

# Phase 01: Code Review Fix Report — Assembly-Aggregat & Audit-Hardening

**Fixed at:** 2026-05-02
**Source review:** `.planning/phases/01-assembly-aggregat-audit-hardening/01-REVIEW.md`
**Iteration:** 1

**Summary:**
- Findings in scope: 10 (1 Critical + 9 Warning, fix_scope=critical_warning)
- Fixed: 10
- Skipped: 0

All ten in-scope findings were applied. Workspace builds cleanly with `SQLX_OFFLINE=true cargo build --workspace`. All workspace lib tests (167) and all e2e tests (218) pass after the fixes. One follow-up commit (`072f31e`) corrects an e2e test (`test_action_update_version_conflict`) that previously asserted the buggy 500 response — CR-01 now correctly produces 409 there.

## Fixed Issues

### CR-01: DAO-Versions-Konflikt wird zu HTTP 500 (statt 409) bei Race zwischen find_by_id und update

**Files modified:** `genossi_service/src/lib.rs`
**Commit:** `3f7eb1f`
**Applied fix:** Extended the global `From<genossi_dao::DaoError> for ServiceError` mapper with an explicit arm for `DaoError::ConflictError(msg) => ServiceError::Conflict(msg)`, so the DAO-level version-mismatch guard surfaces as the documented HTTP 409. Added three unit tests covering the NotFound, ConflictError, and DatabaseError mapping paths. Verified no existing service code branched on `ServiceError::DataAccess` for what was actually a `ConflictError`. The follow-up commit `072f31e` flips `test_action_update_version_conflict` from asserting 500 to asserting 409 -- the test was previously locking in the buggy behavior.

**Note (logic correctness):** This finding is a routing/dispatch fix, not a logic change. The mapping is exhaustive and behavior is verified by both unit tests and the (previously bug-for-bug) e2e test. No human verification flag needed.

### WR-01: Tautologische Assertion in Test verschleiert Test-Intent

**Files modified:** `genossi_rest/src/assembly.rs`
**Commit:** `b26c75b`
**Applied fix:** Replaced `assert!(req.version != Uuid::nil() || req.version == Uuid::nil())` with `assert_ne!(req.version, Uuid::nil(), "version must be a real UUID, not nil")`. Now the assertion has actual diagnostic value -- a future fixture refactor that accidentally produces a nil UUID will fail.

### WR-02: Snapshot-Logik filtert nicht auf `join_date <= opened_date`

**Files modified:** `genossi_service_impl/src/assembly.rs`
**Commit:** `593e736`
**Applied fix:** Added `.filter(|m| m.join_date <= opened_date)` to the open_assembly snapshot loop, alongside the existing exit_date filter. Updated the inline comment to document the intentional divergence from `member_dao.count_active` (which deliberately ignores join_date for total membership counting). Added unit test `test_open_assembly_excludes_future_joiner_from_snapshot` that constructs a member with `join_date = today + 180 days` and verifies it is excluded from the snapshot batch.

**Note (logic correctness):** The new filter is a literal `<=` comparison; behavior is verified by the new unit test plus the existing `test_open_assembly_filters_inactive_members`. No human verification flag needed.

### WR-03: FK-Constraint in Migration ohne `PRAGMA foreign_keys=ON` wirkungslos

**Files modified:** `migrations/sqlite/20260502000001_create_assembly_member_snapshot_table.sql`, `genossi_dao_impl_sqlite/src/assembly_member_snapshot.rs`
**Commit:** `bdd1375`
**Applied fix:** Per phase guidance ("prefer the documentation route unless verifying the PRAGMA change is feasible without breaking other tests"), this is a documentation-only fix:
  - Migration now starts with a NOTE explaining that the `FOREIGN KEY` clauses are documentary only because `PRAGMA foreign_keys=ON` is not set on the SqlitePool. The clauses are kept so the schema clearly expresses intent.
  - `AssemblyMemberSnapshotDaoImpl` carries a doc-comment that ties the same caveat to the service-layer invariant in `open_assembly` (assembly_id is created in the same transaction, member_ids come from `member_dao.all()`).
Enabling the PRAGMA workspace-wide is intentionally deferred to Phase 2/3 -- ripple effects on existing DAOs/tests would need their own verification pass.

### WR-04: Doppelte `find_by_id`-Query in `update_assembly` und `open_assembly`/`close_assembly`

**Files modified:** `genossi_service_impl/src/assembly.rs`
**Commit:** `8b66456`
**Applied fix:** Per the review's pragmatic recommendation, added a comment at all three sites explaining that the duplicate read against `audited_update!` is intentional and required for the state-transition guard and optimistic-locking version check. Future reviewers are warned not to "optimize" the duplication away because that would break the audit trail. No code refactor; behavior unchanged.

### WR-05: Validation laesst Byte-Laengen statt Zeichen-Laengen pruefen

**Files modified:** `genossi_rest/src/assembly.rs`
**Commit:** `1d69aa6`
**Applied fix:** Switched both `validate_required_field` and `validate_optional_max_len` from `value.len()` (bytes) to `value.chars().count()` (Unicode scalar values). Added two unit tests:
  - `test_validate_create_assembly_request_unicode_counts_chars_not_bytes` (256 "ä" chars accepted, 257 rejected)
  - `test_validate_create_assembly_request_unicode_optional_location_chars` (256 "ü" chars in optional location accepted)

The `application.rs` validators carry the same byte-count pattern and are out of scope for Phase 1 (they were not flagged by the reviewer). Flagged in this report as a candidate for a follow-up sweep.

### WR-06: Fehlender Test fuer `get_assembly` mit korrekter Snapshot-Count

**Files modified:** `genossi_service_impl/src/assembly.rs`, `genossi_bin/tests/e2e_tests.rs`
**Commit:** `d2ed7f2`
**Applied fix:** Two tests as recommended by the review:
  - Unit test `test_get_assembly_returns_snapshot_member_count` mocks `count_by_assembly_id => 7` and asserts `AssemblyDetail.snapshot_member_count == 7`.
  - Extended `test_assembly_lifecycle_audit_chain_intact` to create two active members BEFORE `open_assembly`, then GET `/api/assembly/{id}` after open and assert `AssemblyDetailTO.snapshot_member_count == 2`. The first occurrence of the e2e date string was also normalized to the `Z` suffix as a side-effect (the remaining two were normalized in the dedicated WR-09 commit).
Imported `AssemblyDetailTO` into the e2e test module.

### WR-07: `assembly` Entity hat `deleted`-Feld, aber kein Code-Pfad setzt/liest es

**Files modified:** `genossi_service_impl/src/assembly.rs`
**Commit:** `3505182`
**Applied fix:** Documentation-only. Added a module-level doc comment explaining:
  1. Phase 1 deliberately implements no delete path (no REST endpoint, no `audited_delete!` call).
  2. The schema column is reserved for a future Phase 2/3 soft-delete that MUST add an `audited_delete!` invocation and a DELETE handler with lifecycle guards.
  3. Reviewers must NOT remove the field "because it is unused" -- removal would force an avoidable Phase 2/3 migration.

### WR-08: `format_dt` in `audit_fields()` liefert leeren String bei Format-Fehler

**Files modified:** `genossi_dao/Cargo.toml`, `genossi_dao/src/assembly.rs`
**Commit:** `cae6c0c`
**Applied fix:** Replaced `unwrap_or_default()` with `unwrap_or_else(|err| { tracing::error!(...); "<invalid datetime>".to_string() })`. Now a formatting failure is logged via `tracing::error!` (with `entity = "assembly"` for filtering) and substitutes a visible sentinel string, so an auditor reading the audit log directly will spot the failure mode rather than seeing a blank field.

Added `tracing` (already in `[workspace.dependencies]`) as a dependency of `genossi_dao`.

### WR-09: e2e-Datum-String benutzt PrimitiveDateTime ohne TZ-Suffix; brittle gegen Iso8601-Strict-Parser-Updates

**Files modified:** `genossi_bin/tests/e2e_tests.rs`
**Commit:** `2121f7d`
**Applied fix:** Suffixed all three e2e date strings (`"2026-06-15T18:00:00.000000000"` -> `"2026-06-15T18:00:00.000000000Z"`) so the wire form matches the unit-test fixture in `genossi_rest_types/src/lib.rs`. The first occurrence was already corrected as a side-effect of the WR-06 commit (`d2ed7f2`); this commit harmonizes the two negative-path tests (`test_close_assembly_from_preparation_returns_conflict`, `test_open_assembly_from_closed_returns_conflict`).

## Skipped Issues

None — all in-scope findings were fixed.

## Follow-up Notes

- **WR-05 sweep candidate:** `validate_required_field` / `validate_optional_max_len` in `genossi_rest/src/application.rs` (and similar helpers across the REST layer) carry the same byte-count pattern. Out of scope for Phase 1 (only the assembly module was reviewed) but worth a project-wide normalization pass before Phase 2.
- **WR-03 future work:** Enabling `PRAGMA foreign_keys=ON` workspace-wide via `SqlitePoolOptions::after_connect` is the structurally correct fix. Phase 2/3 should pick it up under its own change with full e2e coverage.
- **CR-01 ripple:** The mapper change benefits not only Assembly but also Member, Application, MemberAction, MemberDocument, and UserPreference DAOs (all of which return `DaoError::ConflictError` for version mismatches). The follow-up commit `072f31e` flipped one previously-buggy e2e assertion (`test_action_update_version_conflict`) from 500 to 409. Other version-conflict e2e tests already expected 409 because their service-layer guards (Application, MailTemplate) caught the conflict before reaching the DAO; they remain green.

## Verification Summary

| Gate | Result |
|------|--------|
| `SQLX_OFFLINE=true cargo build --workspace` | Pass |
| `cargo test -p genossi_service --features utoipa` | 16/16 (3 new) |
| `cargo test -p genossi_dao` | 46/46 |
| `cargo test -p genossi_service_impl --lib` | 167/167 (2 new on assembly path) |
| `cargo test -p genossi_rest --lib` | 39/39 (2 new on assembly validation) |
| `cargo test --test e2e_tests` (genossi_bin) | 218/218 |

---

_Fixed: 2026-05-02_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
