---
phase: 25-application-file-upload-audited-carryover
plan: 04
subsystem: rest+service+bin
tags: [rest, service, application-document, member-document, cr-02, carryover, audited-move, di-wiring]

requires:
  - phase: 25-application-file-upload-audited-carryover
    provides: "Wave 2 ApplicationDocumentService trait + impl (Plan 25-03)"
provides:
  - "ApplicationDocumentTO in genossi_rest_types (trimmed shape per CONTEXT #5)"
  - "3 admin-only REST endpoints under /api/applications/{application_id}/document (POST/GET/DELETE) with GET ?meta=1 metadata mode"
  - "ApplicationServiceImpl::confirm() CR-02 fix + audited Move-transfer to MemberDocument"
  - "Missing-file rollback of the confirm() cascade (APDOC-04)"
  - "genossi_bin DI wiring: ApplicationDocumentServiceImpl in RestStateImpl"
affects: [25-05 frontend Application-detail slot component]

tech-stack:
  added: []
  patterns:
    - "REST multipart handler mirroring member_document.rs: reuses lookup_allowed_mime + allowed_extensions + DefaultBodyLimit(50 MB) — single maintenance point"
    - "GET ?meta=1 branch returns the ApplicationDocumentTO instead of the file bytes; used by the frontend slot to decide empty-vs-filled without a HEAD request"
    - "confirm() cascade extended with audited MemberDocument create + non-audited soft-delete of the application_document row, all inside the same use_transaction. Best-effort storage.delete of the old path runs AFTER commit."
    - "Test D pins the APDOC-04 rollback guarantee via tx_dao.expect_commit().times(0) when storage.load returns NotFound"

key-files:
  created:
    - genossi_rest/src/application_document.rs
    - .planning/phases/25-application-file-upload-audited-carryover/25-04-SUMMARY.md
  modified:
    - genossi_rest_types/src/lib.rs
    - genossi_rest/src/lib.rs
    - genossi_service_impl/src/application.rs
    - genossi_bin/src/lib.rs

key-decisions:
  - "CR-02 fix applied at the existing confirm() site (application.rs) rather than the generic gen_auth_admin! helper. Rationale: the helper doesn't exist yet (v1.3-milestone audit techdebt), the two-line swap is surgical, and the awk gate in the plan pins the ordering going forward."
  - "GET ?meta=1 uses a String param (not bool). Query strings arrive as text; matching against Some(\"1\") avoids Serde's boolean-as-\"true\"/\"false\" strictness and is what frontends produce naturally."
  - "The 4 confirm() cascade tests hand-roll mocks via mockall::mock! (not automock) because the automocked PermissionService uses Context=() while our Deps require Context=MockContext. Mirrors the pattern already in place at genossi_service_impl/src/application_document.rs."
  - "Config/Mail service mocks reuse the automock-generated MockConfigService / MockMailService — never invoked in confirm() so we only need them to satisfy the assoc-type constraint on Deps."

requirements-completed:
  - APDOC-01
  - APDOC-02
  - APDOC-03
  - APDOC-04

coverage:
  - id: R1
    description: "3 REST endpoints reachable at /api/applications/{application_id}/document (POST/GET/DELETE) + GET ?meta=1"
    requirement: APDOC-01
    verification:
      - kind: build
        ref: "cargo check -p genossi_rest -p genossi_rest_types"
        status: pass
      - kind: build
        ref: "cargo build -p genossi_bin"
        status: pass
    human_judgment: false
  - id: R2
    description: "CR-02 fix at existing confirm() site — check_permission runs before current_user_id"
    requirement: APDOC-02
    verification:
      - kind: unit
        ref: "genossi_service_impl::application::tests::test_confirm_cr02_permission_denied_has_no_side_effects"
        status: pass
      - kind: awk-gate
        ref: "25-04-PLAN.md <verify> section (CR-02 awk gate)"
        status: pass
    human_judgment: false
  - id: R3
    description: "confirm() cascade Move-transfers the attached application_document to an audited MemberDocument (document_type='other', DE-formatted description)"
    requirement: APDOC-03
    verification:
      - kind: unit
        ref: "genossi_service_impl::application::tests::test_confirm_with_document_creates_audited_member_doc_and_soft_deletes"
        status: pass
    human_judgment: false
  - id: R4
    description: "Missing/corrupt application-document file rolls back the whole confirm() transaction (Member/Actions never committed) — tx.commit is never reached"
    requirement: APDOC-04
    verification:
      - kind: unit
        ref: "genossi_service_impl::application::tests::test_confirm_missing_file_rolls_back_full_transaction"
        status: pass
    human_judgment: false
  - id: R5
    description: "Confirm without an attached document still succeeds; no storage or MemberDocument DAO calls"
    requirement: APDOC-03
    verification:
      - kind: unit
        ref: "genossi_service_impl::application::tests::test_confirm_without_document_skips_carryover"
        status: pass
    human_judgment: false

duration: 34min
completed: 2026-07-03
status: complete
---

# Phase 25 Plan 04: Wave 3 REST + confirm() Carryover Summary

**Wave 3 shipped the outward-facing surface for the single-slot ApplicationDocument.** Three admin-only REST endpoints (`POST/GET/DELETE /api/applications/{application_id}/document`) are wired through the Wave-2 `ApplicationDocumentService`; `GET ?meta=1` returns the metadata JSON so the frontend slot can render empty-vs-filled without a HEAD request. On the audit-payload side, `ApplicationServiceImpl::confirm()` now performs the audited Move-transfer that turns an attached application-document into a `MemberDocument` under `APPLICATION_SERVICE_PROCESS`, inside the existing `use_transaction` block; CR-02 was fixed at the same site as a mandatory side task (APDOC-02). Missing-file rollback (APDOC-04) is pinned by a dedicated unit test.

## Performance

- **Duration:** 34 min
- **Started:** 2026-07-03T00:33:36Z
- **Completed:** 2026-07-03T01:07:53Z
- **Tasks:** 3
- **Files created:** 2 (REST handler + SUMMARY)
- **Files modified:** 4 (rest_types, rest lib, service_impl application, bin lib)
- **Tests added:** 4 (confirm() cascade: CR-02, happy carryover, no-doc skip, missing-file rollback)

## Accomplishments

- **`genossi_rest_types/src/lib.rs` — `ApplicationDocumentTO` added.** Fields: `id`, `application_id`, `file_name`, `mime_type`, `size`, `created`, `version` — deliberately no `document_type` / `description` (CONTEXT #5). `From<&ApplicationDocument>` conversion mirrors the entity view field-for-field.
- **`genossi_rest/src/application_document.rs` — new REST module.** Three handlers (`upload_application_document`, `download_application_document`, `delete_application_document`) mounted on `POST/GET/DELETE /`. Reuses `lookup_allowed_mime` + `allowed_extensions` (single maintenance point) and `DefaultBodyLimit::max(50 MB)`. Multipart-upload discards client-declared MIME and derives it server-side from the filename extension (T-25-04-04 mitigation). `GET ?meta=1` returns the TO JSON instead of the file bytes.
- **`genossi_rest/src/lib.rs` — trait + router.** `pub mod application_document;` registered; `RestStateDef` exposes `type ApplicationDocumentService` + `fn application_document_service`; `ApiDoc` nests the new sub-router at `/api/applications/{application_id}/document`; main router chain nests the sub-router at the same path.
- **`genossi_service_impl/src/application.rs` — confirm() cascade extended.**
  - DI block gained `ApplicationDocumentDao`, `MemberDocumentDao`, `DocumentStorage`.
  - **CR-02 fix (APDOC-02):** `check_permission(MANAGE_MEMBERS_PRIVILEGE, context.clone())` now runs BEFORE `current_user_id(context)` at the existing site. Awk gate in the plan pins the ordering going forward.
  - **APDOC-03 Move-transfer:** when `find_active_by_application_id` returns `Some(app_doc)`, the service loads bytes from storage, computes a member-doc path (`{uuid}.{ext}`), saves under the new path, calls `audited_create!(MemberDocument)` with `document_type = "other"` and the DE-formatted description `"Original-Antrag (übernommen bei Bestätigung am DD.MM.YYYY)"`, and soft-deletes the `application_document` row (non-audited DAO update).
  - **APDOC-04 rollback:** `storage.load` failure returns `ServiceError::InternalError` via `?` propagation → `use_transaction` rolls back the whole cascade. Test D asserts `tx_dao.expect_commit().times(0)` on this path.
  - **Best-effort delete-after-commit:** old application-doc file is deleted AFTER the transaction has committed; a delete failure is `tracing::warn!`-logged only (Member is already active).
- **`genossi_bin/src/lib.rs` — DI wiring.** New `ApplicationDocumentDao` alias, extended `ApplicationServiceDeps` impl with the three carryover deps, new `ApplicationDocumentServiceDependencies` struct + service type alias, new `application_document_service` field on `RestStateImpl` constructed in `new()` alongside `application_service` (single DAO/storage Arc per process), and `RestStateDef` impl exposes the getter to the REST layer.
- **Tests.** 4 new mock-based unit tests in `application::tests`. All 4 pass alongside the existing 426 `genossi_service_impl` tests (`428 passed / 0 failed / 2 ignored` — the 2 ignored pre-date this plan). Wave 2's 7 `application_document::tests` still pass. Full workspace `cargo build` succeeds.

## Task Commits

Each task committed atomically via jj:

1. **Task 1: REST layer** — `45aa3620` — `feat(25-04): add ApplicationDocumentTO + 3 REST endpoints (POST/GET/DELETE /api/applications/{id}/document)`
2. **Task 2: confirm() cascade + CR-02 fix + tests** — `e4c959d6` — `feat(25-04): extend ApplicationServiceImpl::confirm() with CR-02 fix + audited Move-transfer to MemberDocument (APDOC-02/03/04)`
3. **Task 3: DI wiring in genossi_bin** — `b3345772` — `feat(25-04): wire ApplicationDocumentService into RestStateImpl (genossi_bin DI)`

## Files Created/Modified

- **CREATED** `/home/neosam/programming/rust/projects/genossi3/genossi_rest/src/application_document.rs` — REST module with 3 handlers, `DownloadQuery` for `?meta=1`, `ApiDoc` derive.
- **CREATED** `/home/neosam/programming/rust/projects/genossi3/.planning/phases/25-application-file-upload-audited-carryover/25-04-SUMMARY.md` — this file.
- **MODIFIED** `/home/neosam/programming/rust/projects/genossi3/genossi_rest_types/src/lib.rs` — added `ApplicationDocumentTO` struct + `From<&ApplicationDocument>` impl.
- **MODIFIED** `/home/neosam/programming/rust/projects/genossi3/genossi_rest/src/lib.rs` — `pub mod application_document;`, `RestStateDef` extensions (assoc type + getter), `ApiDoc` nest, router nest.
- **MODIFIED** `/home/neosam/programming/rust/projects/genossi3/genossi_service_impl/src/application.rs` — DI block extended, confirm() rewritten (CR-02 fix + Move-transfer + best-effort delete-after-commit), 4 unit tests appended.
- **MODIFIED** `/home/neosam/programming/rust/projects/genossi3/genossi_bin/src/lib.rs` — `ApplicationDocumentDao` alias, extended `ApplicationServiceDeps` impl, new `ApplicationDocumentServiceDependencies` + service type alias, new `RestStateImpl` field and getter.

## Decisions Made

- **CR-02 fix applied inline at the existing confirm() site.** The v1.3-milestone-audit techdebt (extract into `gen_auth_admin!` helper) remains deferred; the plan-checker gate confirms the two-line swap is sufficient regression-guarded via the awk pattern. Any future site adding a new admin-scoped method will need the same review — the milestone audit tracks this as a project-wide item.
- **`GET ?meta=1` returns the full TO, not just size/filename.** Frontend needs the `id`, `application_id`, `file_name`, `size`, `created` at once to render the slot; splitting into two round-trips is worse UX and adds latency. Metadata is small enough that returning the full TO is cheap.
- **`String` query param for `?meta=1`, not `bool`.** Query strings arrive as text; matching against `Some("1")` sidesteps Serde's boolean deserialization quirks (`"true"` vs `"1"` vs `""` etc.) and is what frontends emit naturally with `?meta=1`.
- **Config/Mail deps use the pre-generated `MockConfigService` / `MockMailService`.** They are never invoked in confirm() but the `Deps` trait requires the assoc types. Reusing the automock-generated mocks avoids maintaining ~150 lines of hand-rolled trait scaffolding that would drift out of sync with the real trait signatures.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 — Blocking] Original hand-rolled Config/Mail service mocks failed to compile (out-of-date signatures).**
- **Found during:** Task 2 test compile — the manually-written `mock! { pub CfgSvc { ... } }` and `mock! { pub MailSvc { ... } }` blocks had signatures that no longer matched the real traits (missing `delete`, wrong `set` arity, missing 6 methods on `MailService` including `send_test_mail_with_body`).
- **Issue:** Manual mock definitions were an unnecessary maintenance liability; both traits already have `#[automock]` attributes.
- **Fix:** Removed both hand-rolled mocks; switched to the automock-generated `MockConfigService` (from `genossi_config::service`) and `MockMailService` (from `genossi_mail::service`).
- **Files modified:** `genossi_service_impl/src/application.rs` (tests module only).
- **Verification:** `cargo check -p genossi_service_impl --tests` clean; all 4 new tests pass.
- **Committed in:** `e4c959d6` (Task 2).

**2. [Rule 1 — Bug] `MemberActionDao::get_by_member` and `MemberDocumentDao::count_by_type_grouped` do not exist.**
- **Found during:** Task 2 test compile.
- **Issue:** Initial mock scaffolding used method names that didn't match the actual traits (`get_by_member` should be `find_by_member_id`; `count_by_type_grouped(DocumentType, ...)` should be `count_by_type(&str, ...)`).
- **Fix:** Corrected the mock signatures to match the real traits (verified by inspecting `genossi_dao/src/member_action.rs` and `genossi_dao/src/member_document.rs`).
- **Files modified:** `genossi_service_impl/src/application.rs` (tests module only).
- **Verification:** `cargo check -p genossi_service_impl --tests` clean.
- **Committed in:** `e4c959d6` (Task 2).

---

**Total deviations:** 2 auto-fixed (both are test-scaffolding fixes; production code was untouched by these deviations).

## Issues Encountered

- **Pre-existing e2e_tests failure `test_mail_preview_repayment_no_entries_does_not_default_to_one`.** This test fails on the pre-25-04 baseline commit `b1d75981` as well as on the post-25-04 head. Not caused by this plan — a Phase-22 or earlier regression in the mail-template-preview render pipeline (`errors must be array` panic at `e2e_tests.rs:14377`). Deferred to a separate quick/investigation. All other 305 e2e tests pass.
- **jj workflow reminder:** working from the current change onto a prior commit via `jj new <prev>` performs a **checkout-in-place**, which surfaced my working-copy modifications as "reverted" (they lived on the old branch). Re-issued `jj new b3345772` to reattach onto the 25-04 head — code was intact on that branch, no work lost.

## User Setup Required

None. No new packages, no schema migrations, no config changes. The three REST endpoints are live at `/api/applications/{application_id}/document` as soon as the binary is redeployed. Existing `manage_members` admins have permission automatically.

## Next Phase Readiness

- **Wave 4 (Plan 25-05) is unblocked.** Frontend can now build `ApplicationDocumentSlot` in `genossi-frontend/src/component/` against the three REST endpoints. Use `GET ?meta=1` to detect empty-vs-filled slot without needing a HEAD request.
- **APDOC-05 (frontend slot) is the only remaining requirement in Phase 25.** All backend concerns are shipped.
- **No blockers.** No new deps, no migrations, no schema changes.

## Threat Model Compliance

| Threat ID | Mitigation | Where |
|---|---|---|
| T-25-04-01 (InfoDisclosure: CR-02 in confirm()) | Two-line swap: `check_permission` runs before `current_user_id`. Awk gate in the plan pins the ordering. Test A asserts unauthorised call touches zero DAOs/storage. | `genossi_service_impl/src/application.rs` `confirm()` body; test at `test_confirm_cr02_permission_denied_has_no_side_effects`. |
| T-25-04-02 (InfoDisclosure: CR-02 in REST) | REST handlers extract `Authentication<Context>` and delegate; Service enforces the ordering (Plan 03 CR-02 tests pin this). | `genossi_rest/src/application_document.rs` handlers + Wave 2 tests. |
| T-25-04-03 (DoS: oversized upload) | `DefaultBodyLimit::max(50 MB)` on POST route + Service `MAX_FILE_SIZE` gate (defense in depth). | `application_document.rs:APPLICATION_DOCUMENT_BODY_LIMIT`; Wave 2 service limit. |
| T-25-04-04 (Tampering: client MIME spoofing) | REST discards client MIME; `lookup_allowed_mime(extension)` derives server-side; 415 with allow-list on unknown extension. | `upload_application_document` handler. |
| T-25-04-05 (DoS/Data loss: missing file at confirm) | `storage.load(...).map_err(...)?` inside `use_transaction` — automatic rollback of Member/Actions/Application status. Test D pins `tx.commit.times(0)` on this path. | `confirm()` cascade + `test_confirm_missing_file_rolls_back_full_transaction`. |
| T-25-04-06 (InfoDisclosure: orphan file) | Best-effort delete AFTER commit; warn-log on failure. Orphan file lives at a UUID path unreachable via API. Deferred housekeeping-job in v1.5+. | `confirm()` post-commit block. |
| T-25-04-07 (Repudiation: audit trail) | `audited_create!(MemberDocument)` runs with `APPLICATION_SERVICE_PROCESS` + the same `user_id` as the Member/Actions creates — one hash-chain link across the whole confirm event. | `confirm()` cascade `audited_create!` for MemberDocument. |
| T-25-04-08 (Tampering: cross-application access) | REST handler uses `application_id` from Path; Service looks up doc by `application_id` (not doc_id) so URL substitution can't leak. `check_permission(MANAGE_MEMBERS_PRIVILEGE)` covers admin-only. | `application_document.rs` handlers + Wave 2 service. |
| T-25-04-SC (Tampering: package installs) | No new packages. Cargo.lock unchanged. | Verified. |

## Self-Check: PASSED

Verified via absolute-path checks and command output:

- `[ -f /home/neosam/programming/rust/projects/genossi3/genossi_rest/src/application_document.rs ]` → FOUND
- `grep -c "ApplicationDocumentTO" /home/neosam/programming/rust/projects/genossi3/genossi_rest_types/src/lib.rs` → 2 (struct + From impl)
- `grep -c "pub mod application_document" /home/neosam/programming/rust/projects/genossi3/genossi_rest/src/lib.rs` → 1
- `grep -c "/api/applications/{application_id}/document" /home/neosam/programming/rust/projects/genossi3/genossi_rest/src/lib.rs` → 2 (ApiDoc nest + router nest)
- `grep -c "DefaultBodyLimit::max" /home/neosam/programming/rust/projects/genossi3/genossi_rest/src/application_document.rs` → 1
- `grep -c "lookup_allowed_mime" /home/neosam/programming/rust/projects/genossi3/genossi_rest/src/application_document.rs` → 2
- CR-02 awk gate on confirm() → `CR-02 OK`
- `grep -c "audited_create!" /home/neosam/programming/rust/projects/genossi3/genossi_service_impl/src/application.rs` → 6 (submit + confirm[member/eintritt/aufstockung/member_doc] + update via audited_update! separately)
- `grep -c "find_active_by_application_id" /home/neosam/programming/rust/projects/genossi3/genossi_service_impl/src/application.rs` → 7 (impl call + 6 test mock references)
- `grep -c "Original-Antrag (übernommen bei Bestätigung am" /home/neosam/programming/rust/projects/genossi3/genossi_service_impl/src/application.rs` → 3 (format string + 2 test assertions)
- `grep -c "old_app_doc_path_for_cleanup" /home/neosam/programming/rust/projects/genossi3/genossi_service_impl/src/application.rs` → 3 (declared + assigned + read after commit)
- `grep -c "ApplicationDocumentServiceDependencies" /home/neosam/programming/rust/projects/genossi3/genossi_bin/src/lib.rs` → 5
- `grep -c "type ApplicationDocumentService" /home/neosam/programming/rust/projects/genossi3/genossi_bin/src/lib.rs` → 2 (type alias + RestStateDef impl)
- `grep -c "application_document_service" /home/neosam/programming/rust/projects/genossi3/genossi_bin/src/lib.rs` → 5 (field decl + construction + return-struct shorthand + getter + let binding)
- Commits `45aa3620`, `e4c959d6`, `b3345772` present in jj log
- `cargo build -p genossi_bin` → clean
- `cargo test -p genossi_service_impl --lib` → 428 passed / 0 failed / 2 ignored (2 ignored pre-date this plan)
- `cargo test -p genossi_service_impl --lib application::tests` → 4 passed / 0 failed (all new tests)
- All other workspace crate tests pass. `genossi_bin --test e2e_tests` has 1 pre-existing failure (`test_mail_preview_repayment_no_entries_does_not_default_to_one`) that also fails on the pre-25-04 baseline; documented in "Issues Encountered".

---
*Phase: 25-application-file-upload-audited-carryover*
*Completed: 2026-07-03*
