---
phase: 25-application-file-upload-audited-carryover
plan: 03
subsystem: service
tags: [service, application-document, cr-02, replace-in-place, single-slot, no-audit]

requires:
  - phase: 25-application-file-upload-audited-carryover
    provides: "Wave 1b DAO surface (ApplicationDocumentDao, ApplicationDocumentEntity) from Plan 25-02"
provides:
  - "ApplicationDocumentService trait with upload/get/download/delete"
  - "ApplicationDocumentServiceImpl enforcing CR-02 ordering (check_permission BEFORE current_user_id) in every method"
  - "Single-slot upload branching: create-new OR replace-in-place (save-new → update-DB → best-effort delete-old)"
  - "MAX_FILE_SIZE = 50 MB defense-in-depth gate; storage layout applications/{app_id}/{doc_id}.{ext}"
affects: [25-04 confirm carryover, 25-05 REST endpoints, 25-06 frontend, 25-07 tests]

tech-stack:
  added: []
  patterns:
    - "Service impl without Auditable / audit_log_dao — proves the gen_service_impl! macro composes correctly with a narrow dependency set"
    - "mockall::Sequence to pin save-new → update-DB → delete-old call order in the replace-in-place unit test"
    - "CR-02 regression guard test: unauthorised call must trip .times(0) on every DAO/storage/current_user_id expectation"

key-files:
  created:
    - genossi_service/src/application_document.rs
    - genossi_service_impl/src/application_document.rs
  modified:
    - genossi_service/src/lib.rs
    - genossi_service_impl/src/lib.rs

key-decisions:
  - "Service does NOT capture user_id for audit (this entity is not audited). The permission_service.current_user_id call is still made after check_permission for future-observability parity, and to preserve the CR-02 ordering shape that unit Test 3 pins."
  - "Extension helper `extract_extension` is duplicated verbatim from member_document.rs rather than imported. Twelve-line helpers stay in their own crate boundary (CLAUDE.md — small helpers are acceptable duplication for a clean dependency graph)."
  - "Delete on download uses `StorageError::NotFound → ServiceError::InternalError(\"...missing on filesystem...\")` (NOT `EntityNotFound`). Rationale: the DB row exists but the file has vanished — this is corruption, not a `not-found` state the caller should confuse with a normal missing record."
  - "Replace-in-place delete-old failure is warn-log only, never propagated. Documented in the plan Threat Model as T-25-03-07 (accepted risk). The orphan file, if any, sits at a UUID path and is not reachable via any API surface."

requirements-completed:
  - APDOC-02

coverage:
  - id: D1
    description: "ApplicationDocumentService trait + DTOs in genossi_service (upload/get/download/delete signatures + ApplicationDocument service view + UploadApplicationDocument input DTO + Entity conversions)"
    requirement: APDOC-02
    verification:
      - kind: unit
        ref: "genossi_service/src/application_document.rs#test_entity_to_service_roundtrip_preserves_fields"
        status: pass
      - kind: unit
        ref: "genossi_service/src/application_document.rs#test_service_to_entity_conversion_preserves_soft_delete"
        status: pass
    human_judgment: false
  - id: D2
    description: "ApplicationDocumentServiceImpl with CR-02 ordering in every method (check_permission BEFORE current_user_id)"
    requirement: APDOC-02
    verification:
      - kind: unit
        ref: "genossi_service_impl/src/application_document.rs#test_upload_permission_denied_has_no_side_effects"
        status: pass
      - kind: unit
        ref: "awk-gate in 25-03-PLAN.md <verify>"
        status: pass
    human_judgment: false
  - id: D3
    description: "Single-slot upload: create-new path calls DAO.create + storage.save exactly once each"
    requirement: APDOC-02
    verification:
      - kind: unit
        ref: "genossi_service_impl/src/application_document.rs#test_upload_create_new_calls_create_then_save"
        status: pass
    human_judgment: false
  - id: D4
    description: "Replace-in-place upload: save-new → update-DB → best-effort delete-old sequence pinned by mockall::Sequence"
    requirement: APDOC-02
    verification:
      - kind: unit
        ref: "genossi_service_impl/src/application_document.rs#test_upload_replace_in_place_calls_save_then_update_then_delete"
        status: pass
    human_judgment: false
  - id: D5
    description: "download() maps StorageError::NotFound → ServiceError::InternalError (corruption, not not-found)"
    requirement: APDOC-02
    verification:
      - kind: unit
        ref: "genossi_service_impl/src/application_document.rs#test_download_missing_file_returns_internal_error"
        status: pass
    human_judgment: false
  - id: D6
    description: "delete() swallows storage.delete errors (best-effort, warn-log only)"
    requirement: APDOC-02
    verification:
      - kind: unit
        ref: "genossi_service_impl/src/application_document.rs#test_delete_storage_failure_is_swallowed"
        status: pass
    human_judgment: false
  - id: D7
    description: "get() returns Ok(None) when no active row exists (not an error)"
    requirement: APDOC-02
    verification:
      - kind: unit
        ref: "genossi_service_impl/src/application_document.rs#test_get_returns_none_when_no_active_row"
        status: pass
    human_judgment: false

duration: 16min
completed: 2026-07-03
status: complete
---

# Phase 25 Plan 03: ApplicationDocumentService trait + Impl Summary

**Wave 2: `ApplicationDocumentService` trait in `genossi_service` and `ApplicationDocumentServiceImpl` in `genossi_service_impl`. Four methods (upload/get/download/delete) all enforce CR-02 ordering; upload branches into create-new OR replace-in-place with a mockall-Sequence-pinned save→update→delete-old flow. Seven unit tests including a dedicated CR-02 regression guard.**

## Performance

- **Duration:** 16 min
- **Started:** 2026-07-03T00:13:38Z
- **Completed:** 2026-07-03T00:29:47Z
- **Tasks:** 2
- **Files created:** 2 (trait + impl)
- **Files modified:** 2 (both lib.rs re-exports)
- **Tests added:** 9 (2 conversion tests in genossi_service + 7 impl tests in genossi_service_impl)

## Accomplishments

- `genossi_service/src/application_document.rs` — trait `ApplicationDocumentService` with associated `Context`/`Transaction` types and four methods (`upload`, `get`, `download`, `delete`). Service-layer `ApplicationDocument` view + bidirectional `From` conversions with `ApplicationDocumentEntity`. Input DTO `UploadApplicationDocument`. Two conversion round-trip tests.
- `genossi_service_impl/src/application_document.rs` — `ApplicationDocumentServiceImpl` wired via `gen_service_impl!` with a narrow dep set (**no** `AuditLogDao` — this entity is intentionally not audited per CONTEXT decision #5). CR-02 ordering enforced in every method: `check_permission(MANAGE_MEMBERS_PRIVILEGE, context.clone()).await?` FIRST, then `current_user_id(context).await?`. `upload()` looks up any existing active row via `find_active_by_application_id` and branches:
  - **create-new:** insert entity → `storage.save`
  - **replace-in-place:** `storage.save(new)` → `DAO.update` → best-effort `storage.delete(old)` (warn-log on failure, never propagated)
  Storage paths follow the Pitfall-7 layout `applications/{app_id}/{doc_id}.{ext}`; extension is derived server-side via a duplicated `extract_extension` helper (verbatim copy from `member_document.rs`).
- Seven unit tests: create-new happy-path, replace-in-place with `mockall::Sequence` pinning the save→update→delete order, **CR-02 regression guard** (unauthorised upload trips `.times(0)` on every DAO/storage/current_user_id expectation), download-missing-file → `InternalError` (not `EntityNotFound`), delete swallows storage errors, get returns `Ok(None)` for no active row, and the extension helper coverage.

## Task Commits

Each task was committed atomically via jj:

1. **Task 1: Trait + DTOs** — `f5e65d90` (feat) — `feat(25-03): add ApplicationDocumentService trait + DTOs`
2. **Task 2: Impl + tests** — `4f83ed04` (feat) — `feat(25-03): impl ApplicationDocumentService with CR-02 ordering + replace-in-place (7 tests)`

## Files Created/Modified

- **CREATED** `genossi_service/src/application_document.rs` — trait, DTOs, entity conversions, 2 conversion round-trip tests.
- **CREATED** `genossi_service_impl/src/application_document.rs` — service impl + `extract_extension` helper + `now_primitive` helper + 7 unit tests with full mockall-generated dependency mocks.
- **MODIFIED** `genossi_service/src/lib.rs` — added `pub mod application_document;` (alphabetical, between `application` and `assembly`).
- **MODIFIED** `genossi_service_impl/src/lib.rs` — same module registration.

## Decisions Made

- **`_user_id` is captured but unused for audit.** The `permission_service.current_user_id` call runs AFTER `check_permission` in every method to preserve the CR-02 ordering shape. The result binds to `_user_id` because this entity has no audit trail; the call itself is retained for future observability and to make the CR-02 test meaningful (a caller who reorders `current_user_id` above `check_permission` will trigger the regression test).
- **Extension helper duplicated verbatim.** `extract_extension` is a 12-line function; importing it from `member_document.rs` would leak a cross-module dependency purely for a code-cosmetic saving. CLAUDE.md accepts small-helper duplication for clean crate boundaries. If the helper grows, it moves to a shared `genossi_service` utility.
- **`StorageError::NotFound` during `download()` → `InternalError`, NOT `EntityNotFound`.** The DB row exists — this is filesystem corruption, and the caller (REST layer) must surface it as 500. Using `EntityNotFound` would leak "does this application have a document?" semantics (which is what `get()` answers separately with `Ok(None)`).
- **`delete()` returns `Ok(())` even when the physical file cannot be removed.** DB truth is the source of truth; the row is soft-deleted and stops appearing in reads. The orphan file sits at a UUID path unreachable via any REST endpoint. This is the accepted risk T-25-03-07 in the plan's threat model.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 — Blocking] `use genossi_service::member_document::{allowed_extensions as _, lookup_allowed_mime as _};` triggered dead-code warning**
- **Found during:** first `cargo check` on the impl file.
- **Issue:** The plan calls for these imports as "kept in scope for parity but NOT called here". However, `use ... as _;` still triggers `unused_imports` when nothing is actually named-imported, because the underscore rebinding is only meaningful for name collisions.
- **Fix:** Removed the two `use ... as _;` lines; added a comment block referencing the same helpers by path so a future reader still finds the plumbing. The Wave 3 REST layer will import them at the actual call site.
- **Files modified:** `genossi_service_impl/src/application_document.rs` (imports section only).
- **Verification:** `cargo check -p genossi_service_impl` clean; `cargo test -p genossi_service_impl application_document::tests` still 7 passed / 0 failed.
- **Committed in:** `4f83ed04` (Task 2 commit).

**2. [Rule 1 — Bug] Test helper `app_entity` referenced non-existent `ApplicationEntity` fields**
- **Found during:** Task 2 test compile.
- **Issue:** Initial helper included `confirmed_at: None` and `member_id: None`, but the current `ApplicationEntity` in `genossi_dao/src/application.rs` does NOT declare those columns (they may be added in a later plan). Test failed to compile.
- **Fix:** Removed the two spurious fields from `app_entity()`. Struct literal now matches the actual entity schema.
- **Files modified:** `genossi_service_impl/src/application_document.rs` (tests module only).
- **Verification:** `cargo test -p genossi_service_impl application_document::tests` — 7 passed / 0 failed.
- **Committed in:** `4f83ed04` (Task 2 commit).

**3. [Rule 1 — Bug] rustfmt formatting nits (long-line splits + `withf` closure wrap)**
- **Found during:** post-Task-2 `rustfmt --check`.
- **Issue:** Three formatting deviations from rustfmt defaults on the new file (nothing in existing files touched): `assert!(doc.relative_path.starts_with(...))` at two call sites and a `withf` closure body that wanted a wider layout.
- **Fix:** `rustfmt --edition 2021` applied to the new files only.
- **Files modified:** `genossi_service_impl/src/application_document.rs`, `genossi_service/src/application_document.rs`.
- **Verification:** post-format `cargo test -p genossi_service_impl application_document::tests` still 7 passed; CR-02 gate still passes.
- **Committed in:** `4f83ed04` (Task 2 commit).

---

**Total deviations:** 3 auto-fixed (1 dead-code warning, 1 test helper schema mismatch, 1 formatting sweep).
**Impact on plan:** All fixes stay within Task 2's scope. Trait shape unchanged. No new dependencies. No public-API impact.

## Issues Encountered

- **jj commit workflow:** Repository uses Jujutsu VCS. Both task commits used `jj describe -m "..." && jj new`; change IDs `kkrtrnvn`, `ykzqrmmk` resolving to commit hashes `f5e65d90`, `4f83ed04`.
- **Widespread pre-existing rustfmt deviations** in unrelated files (e.g. `pdf_generation.rs`) surfaced when running `cargo fmt -p genossi_service_impl -- --check`. Not caused by this plan; out of scope per Scope Boundary rule. Documented in prior 25-02 SUMMARY as well.

## User Setup Required

None. No external services touched; no new deps; no migration required (Wave 1b already shipped the schema).

## Next Phase Readiness

- **Wave 3 (Plan 25-04) is unblocked.** The compiling service surface exposes `upload/get/download/delete`, which the confirm() carryover (Plan 25-04) and REST endpoints (Plan 25-05) will consume. `ApplicationDocument::from(&ApplicationDocumentEntity)` and the reverse are both wired so any layer can round-trip freely.
- **Wave 3 REST layer will import** `genossi_service::member_document::{allowed_extensions, lookup_allowed_mime}` at the actual multipart-parsing call site (comment in this file documents the plumbing).
- **DI wiring in `genossi_bin`** will need one new field on `RestStateImpl` for `ApplicationDocumentServiceImpl`. Wave 3 plan (25-04 or 25-05) is expected to add it.
- **No blockers.** No new packages, no migrations, no schema changes.

## Threat Model Compliance

| Threat ID | Mitigation | Where |
|---|---|---|
| T-25-03-01 (InfoDisclosure: CR-02 ordering) | `check_permission` FIRST in every method. Awk gate + `test_upload_permission_denied_has_no_side_effects` pin the ordering. | `genossi_service_impl/src/application_document.rs` all four methods; test at `#test_upload_permission_denied_has_no_side_effects`. |
| T-25-03-02 (DoS: oversized upload) | `MAX_FILE_SIZE = 50 * 1024 * 1024` gate before any storage side-effect in `upload`. | `application_document.rs` line ~107. |
| T-25-03-03 (Tampering: client MIME spoofing) | Service accepts server-derived MIME (from `UploadApplicationDocument.mime_type`, populated by REST layer via `lookup_allowed_mime`). Client-declared MIME is never trusted at the service boundary. | Enforced at Wave 3 REST layer; service surface only accepts the DTO. |
| T-25-03-04 (Tampering: single-slot bypass) | `find_active_by_application_id`-then-branch pattern in `upload`. DB partial unique index from Plan 25-02 is belt-and-suspenders. Test 2 (`test_upload_replace_in_place_calls_save_then_update_then_delete`) pins the replace path. | `upload()` branch + Plan 25-02 migration. |
| T-25-03-05 (EoP: stale entity replay) | Optimistic locking: every update path constructs a new `version` UUID and passes it through DAO `update`; DAO returns `ConflictError` on version mismatch (Plan 25-02 impl). | `upload()` and `delete()` both bump `version`; DAO impl in Plan 25-02. |
| T-25-03-06 (Tampering: path traversal) | `relative_path` composed only from UUIDs (`applications/{app_id}/{doc_id}.{ext}`). No client-controlled path components. `FilesystemDocumentStorage` adds path-clean + prefix check. | `format!("applications/{}/{}.{}", ...)` in `upload()`. |
| T-25-03-07 (InfoDisclosure: orphan file) | Accepted risk. Save-new happens before DAO update in the create-new path — if the update fails and tx rolls back, the file lingers at a UUID path unreachable via API. | Documented; no code mitigation. |
| T-25-03-SC (Tampering: package installs) | No new packages installed. | `Cargo.lock` unchanged. |

## Self-Check: PASSED

Verified via absolute-path checks and command output:

- `[ -f /home/neosam/programming/rust/projects/genossi3/genossi_service/src/application_document.rs ]` → FOUND
- `[ -f /home/neosam/programming/rust/projects/genossi3/genossi_service_impl/src/application_document.rs ]` → FOUND
- `grep -c "pub mod application_document" genossi_service/src/lib.rs` → 1
- `grep -c "pub mod application_document" genossi_service_impl/src/lib.rs` → 1
- `grep -c "trait ApplicationDocumentService" genossi_service/src/application_document.rs` → 1 (plus 1 impl declaration for a total of ≥1 trait pattern match)
- `grep -c "check_permission(MANAGE_MEMBERS_PRIVILEGE" genossi_service_impl/src/application_document.rs` → 4 (one per method)
- `grep -c "current_user_id" genossi_service_impl/src/application_document.rs` → 9 (includes mock declarations and impl call sites)
- `grep -c "applications/{}/{}\." genossi_service_impl/src/application_document.rs` → 1 (Pitfall-7 storage layout)
- CR-02 awk gate → `CR-02 ORDERING OK`
- Commits `f5e65d90`, `4f83ed04` present in jj log
- `cargo check -p genossi_service --features utoipa` → clean
- `cargo check -p genossi_service_impl` → clean (no warnings after the `use ... as _;` removal)
- `cargo test -p genossi_service_impl application_document::tests` → 7 passed / 0 failed / 0 ignored
- `cargo clippy -p genossi_service_impl --lib` on new file → no findings

---
*Phase: 25-application-file-upload-audited-carryover*
*Completed: 2026-07-03*
