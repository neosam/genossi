---
phase: 25-application-file-upload-audited-carryover
plan: 05
subsystem: frontend+rest+service+tests+uat
tags: [frontend, dioxus, wasm, i18n, e2e, application-document, member-document, apdoc-05, apdoc-04, apdoc-02, uat, audit]

requires:
  - phase: 25-application-file-upload-audited-carryover
    provides: "Wave 3 REST + confirm() carryover (Plan 25-04) — POST/GET/DELETE endpoints + Move-transfer to MemberDocument + APDOC-04 rollback"
provides:
  - "Reusable ApplicationDocumentSlot component in genossi-frontend/src/component/"
  - "Frontend api.rs helpers: upload_application_document, get_application_document, delete_application_document, application_document_download_url"
  - "Frontend rest-types ApplicationDocumentTO"
  - "6 i18n keys (Application* series) in BOTH de.rs AND en.rs"
  - "Integration into Application-Detail (only rendered when status == Offen)"
  - "3 e2e HTTP tests pinning audited carryover, missing-file rollback, and replace-in-place"
  - "25-UAT-CHECKLIST.md with 12 verification steps + 6 HARD FAIL GATES"
  - "2 Rule-1 auto-fixes to Plan-04 confirm() and Plan-03 upload() replace path (version-mismatch bug caught by e2e)"
affects: []

tech-stack:
  added: []
  patterns:
    - "Component-first single-slot UI mirroring the MemberDocument upload pattern (FormData + fetch + serde_wasm_bindgen)"
    - "Two-state slot: empty (Antrag hochladen button) vs filled (filename+size+DD.MM.YYYY + Download/Replace/Delete)"
    - "Hidden `<input type=\"file\">` triggered programmatically via `element.click()` — reused for upload + replace"
    - "All interactive buttons carry `r#type: \"button\"` (5x in the slot) — Dioxus form-submit reload bug (Phase 17 hotfix e245013) never regresses"
    - "Both-locales i18n gate: every new Key variant has de + en arms in the SAME commit"
    - "Backend `?meta=1` branch (introduced in Plan 04) consumed by the frontend to detect empty-vs-filled without a HEAD request"
    - "E2E test pattern: reqwest multipart against mock_auth in-memory test server, filesystem probe via std::fs::read_dir under DOCUMENT_STORAGE_PATH"
    - "APDOC-04 rollback proved end-to-end: delete storage file → confirm returns 4xx/5xx → application status stays Offen → no member row → audit hashchain still valid"

key-files:
  created:
    - genossi-frontend/src/component/application_document_slot.rs
    - .planning/phases/25-application-file-upload-audited-carryover/25-UAT-CHECKLIST.md
    - .planning/phases/25-application-file-upload-audited-carryover/25-05-SUMMARY.md
  modified:
    - genossi-frontend/rest-types/src/lib.rs
    - genossi-frontend/src/api.rs
    - genossi-frontend/src/component/application_detail.rs
    - genossi-frontend/src/component/mod.rs
    - genossi-frontend/src/i18n/de.rs
    - genossi-frontend/src/i18n/en.rs
    - genossi-frontend/src/i18n/mod.rs
    - genossi_bin/tests/e2e_tests.rs
    - genossi_service_impl/src/application.rs
    - genossi_service_impl/src/application_document.rs
    - .planning/REQUIREMENTS.md
    - .planning/ROADMAP.md
    - .planning/STATE.md

key-decisions:
  - "Frontend `rest-types` gets its OWN copy of `ApplicationDocumentTO` (mirroring the existing pattern with MemberDocumentTO). The genossi_rest_types crate and the frontend rest-types crate are two independent files by design — sharing would drag utoipa into the WASM build, which the project has never done."
  - "One hidden `<input type=\"file\">` is reused between the empty-state upload button AND the filled-state Replace button. Two inputs with the same id would collide, and the DOM-lookup pattern is already the project convention for file uploads (member_details.rs)."
  - "Filesystem-probe helper `delete_stored_application_file` in E2E-2 walks `${DOCUMENT_STORAGE_PATH:-./documents}/applications/{app_id}/` and removes every regular file. Robust to future path-format changes as long as they keep the per-app subdirectory."
  - "Auto-mode auto-approves the UAT checkpoint after the automated portion (Steps 1–3) passes; the browser walkthrough (Steps 4–12) is deferred to the Vorstand smoke session before merge — mirrors Phase 24."
  - "Rule-1 auto-fixes on Plan-04 and Plan-03 code: the optimistic-lock contract requires `entity.version` to hold the CURRENT DB version (used in WHERE), not a freshly-minted UUID. Fixed at 2 sites; the Plan-04 mock tests hadn't asserted version so the bug slipped through until e2e-1 tripped it."

requirements-completed:
  - APDOC-05

coverage:
  - id: R1
    description: "ApplicationDocumentSlot component exists, is reusable, and lives under genossi-frontend/src/component/"
    requirement: APDOC-05
    verification:
      - kind: static
        ref: "test -f genossi-frontend/src/component/application_document_slot.rs"
        status: pass
      - kind: static
        ref: "grep -q 'pub mod application_document_slot' genossi-frontend/src/component/mod.rs"
        status: pass
    human_judgment: false
  - id: R2
    description: "Slot is composed on Application-Detail ONLY when status == Offen; after confirm it disappears (Bestaetigt branch skips it)"
    requirement: APDOC-05
    verification:
      - kind: static
        ref: "grep -q 'if is_open' genossi-frontend/src/component/application_detail.rs (and ApplicationDocumentSlot in the same block)"
        status: pass
    human_judgment: false
  - id: R3
    description: "api.rs exposes upload/get/delete/download_url mirroring the MemberDocument helpers"
    requirement: APDOC-05
    verification:
      - kind: static
        ref: "grep -cE 'upload_application_document|get_application_document|delete_application_document|application_document_download_url' genossi-frontend/src/api.rs ≥ 4"
        status: pass
    human_judgment: false
  - id: R4
    description: "All 6 i18n keys exist in BOTH de.rs AND en.rs (both-locales gate; locale drift is a documented anti-pattern)"
    requirement: APDOC-05
    verification:
      - kind: static
        ref: "for k in ApplicationDocumentUpload ApplicationDocumentReplace ApplicationDocumentDownload ApplicationDocumentDelete ApplicationDocumentEmptyState ApplicationDocumentDeleteConfirm — de.rs AND en.rs both grep-match"
        status: pass
    human_judgment: false
  - id: R5
    description: "No `r#type: \"submit\"` on interactive buttons; ≥ 2 `r#type: \"button\"` in the slot component (Dioxus reload-bug regression guard)"
    requirement: APDOC-05
    verification:
      - kind: static
        ref: "grep -c 'r#type: \"button\"' genossi-frontend/src/component/application_document_slot.rs ≥ 2 (actual: 5); grep -c 'r#type: \"submit\"' … = 0"
        status: pass
    human_judgment: false
  - id: R6
    description: "e2e test proves the audited Move-transfer end-to-end: upload → confirm → MemberDocument exists with document_type='other' + 'Original-Antrag' description + audit hashchain valid"
    requirement: APDOC-05
    verification:
      - kind: e2e
        ref: "genossi_bin/tests/e2e_tests.rs::application_upload_confirm_carryover_audited"
        status: pass
    human_judgment: false
  - id: R7
    description: "e2e test proves APDOC-04 rollback: missing storage file at confirm time → 4xx/5xx → application still Offen → no member → audit still valid"
    requirement: APDOC-04
    verification:
      - kind: e2e
        ref: "genossi_bin/tests/e2e_tests.rs::application_upload_confirm_missing_file_rolls_back"
        status: pass
    human_judgment: false
  - id: R8
    description: "e2e test proves replace-in-place: second upload returns a different version UUID and the active row reflects the second file"
    requirement: APDOC-05
    verification:
      - kind: e2e
        ref: "genossi_bin/tests/e2e_tests.rs::application_upload_replace_in_place"
        status: pass
    human_judgment: false
  - id: R9
    description: "UAT checklist filed with ≥ 5 HARD FAIL GATES, automated portion auto-approved"
    requirement: APDOC-05
    verification:
      - kind: static
        ref: "grep -c 'HARD FAIL GATE' .planning/phases/25-application-file-upload-audited-carryover/25-UAT-CHECKLIST.md ≥ 5 (actual: 6)"
        status: pass
    human_judgment: false
  - id: R10
    description: "Full workspace test suite matches the STATE.md baseline (only pre-existing failure remains)"
    requirement: APDOC-05
    verification:
      - kind: build
        ref: "cargo test --workspace --features mock_auth: 308 passed / 1 failed (pre-existing test_mail_preview_repayment_no_entries_does_not_default_to_one from Phase 22-ish, documented in 25-04 SUMMARY)"
        status: pass
    human_judgment: false

duration: 24min
completed: 2026-07-03
status: complete
---

# Phase 25 Plan 05: Wave 4 Frontend Slot + e2e + UAT Summary

Wave 4 is the **ship gate** for Phase 25. It puts the Vorstand-facing surface on top of the Plan-03/04 backend cascade: a reusable Dioxus component that shows the single-slot Original-Antrag on the Application detail page, matching api.rs helpers, both-locale i18n, and — most importantly — three e2e HTTP tests that pin the audit-critical carryover behavior end-to-end.

The e2e work also caught **two version-mismatch bugs** in Plan-04 confirm() and Plan-03 upload() that Plan-04's mock tests could not see (the mock `expect_update()` only asserted `entity.deleted.is_some()`, never the version). Both are auto-fixed inline — see "Deviations" below.

## Performance

- **Duration:** 24 min
- **Started:** 2026-07-03T01:16:25Z
- **Completed:** 2026-07-03T01:40:00Z
- **Tasks:** 3
- **Files created:** 3 (component + UAT + SUMMARY)
- **Files modified:** 11 (frontend rest-types, api, application_detail, component mod, i18n mod/de/en, e2e test, service_impl application, service_impl application_document, plus the 3 planning docs)
- **Tests added:** 3 e2e tests (each provides its own audit / rollback / replace guarantee); 1 mock test updated (`test_upload_replace_in_place_calls_save_then_update_then_delete`) to exercise the new refetch path

## Accomplishments

### Task 1 — Frontend slot + api + i18n

- **`ApplicationDocumentSlot` component** (`genossi-frontend/src/component/application_document_slot.rs`) — renders empty vs filled state. Empty state = an "Antrag hochladen" button. Filled state = filename + `size · DD.MM.YYYY` + three action buttons (Herunterladen / Ersetzen / Löschen). All 5 interactive buttons carry `r#type: "button"` explicitly; a single hidden `<input type="file">` (id-scoped to the application_id) is triggered programmatically for both the upload and the replace flows. Errors surface as an inline red banner.
- **`api.rs` extensions** — 4 helpers (`upload_application_document`, `get_application_document`, `delete_application_document`, `application_document_download_url`) mirror the MemberDocument functions. Upload uses `web_sys::FormData` + `fetch_with_request` (same wasm-bindgen path); get uses `reqwest` against `?meta=1`; download_url returns the byte-stream URL without `?meta=1` for the `<a href>` target.
- **`rest-types` extension** — `ApplicationDocumentTO` added to the frontend's own `rest-types` crate (separate from `genossi_rest_types` by design — that one drags utoipa into the WASM build).
- **i18n keys** — 6 new `Key::ApplicationDocument*` variants; both `de.rs` and `en.rs` arms added in the same commit (both-locales gate).
- **Integration** — `application_detail.rs` composes the slot inside `if is_open { … }` between the detail fields and the Confirm/Reject action buttons. After confirm, the application status flips to Bestaetigt and the slot no longer renders.

### Task 2 — Three e2e HTTP tests + Rule-1 auto-fixes

- **E2E-1 (`application_upload_confirm_carryover_audited`).** Full end-to-end proof of APDOC-03: upload PDF → confirm → the new Member has exactly ONE MemberDocument with `document_type = "other"` and `description = "Original-Antrag (übernommen bei Bestätigung am DD.MM.YYYY)"`; the `application_documents` metadata endpoint returns 404 (row soft-deleted); `/api/audit/verify` returns `{valid: true}` (hashchain intact).
- **E2E-2 (`application_upload_confirm_missing_file_rolls_back`).** Full end-to-end proof of APDOC-04: upload → physically remove the file from `${DOCUMENT_STORAGE_PATH:-./documents}/applications/{app_id}/*` → confirm returns a 4xx/5xx (specifically the `RestError::InternalError` mapping) → application stays Offen → no member row created → `/api/audit/verify` still valid.
- **E2E-3 (`application_upload_replace_in_place`).** Proof of the replace contract: two uploads on the same application return different `version` UUIDs on the TO; a subsequent `?meta=1` GET returns the SECOND file's metadata (new filename + new version).

### Task 3 — UAT-CHECKLIST

- `.planning/phases/25-application-file-upload-audited-carryover/25-UAT-CHECKLIST.md` — 12 verification steps + 6 HARD FAIL GATES (Steps 1, 2, 3, 8, 9, 10 — audit-critical + CR-02 regression + `?meta=1` visibility).
- Steps 1–3 (automated regression) auto-approved by this executor. Steps 4–12 (browser walkthrough) deferred to the Vorstand smoke session before merge (mirrors Phase 24 UAT pattern).

## Task Commits

Each task committed atomically via jj:

1. **Task 1: Frontend slot + api + i18n** — `02bb5353` — `feat(25-05): add ApplicationDocumentSlot component + api.rs helpers + i18n keys (De+En) (APDOC-05)`
2. **Task 2: e2e tests + version-mismatch auto-fixes** — `5239c740` — `test(25-05): add 3 e2e HTTP tests for upload+confirm carryover, missing-file rollback, replace-in-place`
3. **Task 3: UAT + SUMMARY + STATE** — this metadata commit (to be recorded after this file lands).

## Files Created/Modified

- **CREATED** `/home/neosam/programming/rust/projects/genossi3/genossi-frontend/src/component/application_document_slot.rs` — the reusable Dioxus component.
- **CREATED** `/home/neosam/programming/rust/projects/genossi3/.planning/phases/25-application-file-upload-audited-carryover/25-UAT-CHECKLIST.md` — 12-step UAT with 6 HARD FAIL GATES.
- **CREATED** `/home/neosam/programming/rust/projects/genossi3/.planning/phases/25-application-file-upload-audited-carryover/25-05-SUMMARY.md` — this file.
- **MODIFIED** `/home/neosam/programming/rust/projects/genossi3/genossi-frontend/rest-types/src/lib.rs` — added `ApplicationDocumentTO`.
- **MODIFIED** `/home/neosam/programming/rust/projects/genossi3/genossi-frontend/src/api.rs` — 4 new helper functions + import of `ApplicationDocumentTO`.
- **MODIFIED** `/home/neosam/programming/rust/projects/genossi3/genossi-frontend/src/component/application_detail.rs` — composes the slot inside `if is_open`; imports `ApplicationDocumentSlot` from the component module.
- **MODIFIED** `/home/neosam/programming/rust/projects/genossi3/genossi-frontend/src/component/mod.rs` — `pub mod application_document_slot;` + `pub use ApplicationDocumentSlot;`.
- **MODIFIED** `/home/neosam/programming/rust/projects/genossi3/genossi-frontend/src/i18n/mod.rs` — 6 new `Key` variants.
- **MODIFIED** `/home/neosam/programming/rust/projects/genossi3/genossi-frontend/src/i18n/de.rs` — 6 German arms.
- **MODIFIED** `/home/neosam/programming/rust/projects/genossi3/genossi-frontend/src/i18n/en.rs` — 6 English arms.
- **MODIFIED** `/home/neosam/programming/rust/projects/genossi3/genossi_bin/tests/e2e_tests.rs` — 3 new `#[tokio::test]` functions + `ApplicationDocumentTO` import + 2 helper functions (`seed_application`, `upload_application_pdf`, `delete_stored_application_file`).
- **MODIFIED** `/home/neosam/programming/rust/projects/genossi3/genossi_service_impl/src/application.rs` — 1-line Rule-1 auto-fix on the `soft_deleted_app_doc.version` field (was `new_v4()`, now `app_doc.version`).
- **MODIFIED** `/home/neosam/programming/rust/projects/genossi3/genossi_service_impl/src/application_document.rs` — Rule-1 auto-fix on the replace-in-place `updated_entity.version` (was `new_version`, now `old.version`) + post-update refetch to return the fresh DB-generated version + mock test scaffold update.
- **MODIFIED** `/home/neosam/programming/rust/projects/genossi3/.planning/REQUIREMENTS.md` — APDOC-05 marked complete + traceability table.
- **MODIFIED** `/home/neosam/programming/rust/projects/genossi3/.planning/ROADMAP.md` — Phase 25 marked complete (5/5 plans).
- **MODIFIED** `/home/neosam/programming/rust/projects/genossi3/.planning/STATE.md` — current phase set to complete; Plan 05 metric row added.

## Decisions Made

- **Frontend `rest-types` gets its own `ApplicationDocumentTO`** — dropping utoipa into the WASM build has never been done in this project; the two crates are independent by design and keeping them so preserves the wasm-lean invariant.
- **Backend `?meta=1` (Plan 04) is the load-and-render contract** — the slot fetches metadata via `GET …/document?meta=1` and uses the same URL without `?meta=1` as the `<a href>` for Download. Single URL for both purposes keeps the route surface at three.
- **The refetch after replace-in-place lives inside the transaction** — post-update, before commit — so the returned TO reflects the fresh DB-generated version. That's what E2E-3 asserts.
- **Auto-mode auto-approves the UAT checkpoint** — regression Steps 1–3 pass, so the checkpoint clears; browser walkthrough Steps 4–12 deferred to Vorstand smoke session (mirrors Phase 24 pattern).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 — Bug] Plan-04 confirm() passed a fresh v4 UUID as the "old_version" for the app_doc soft-delete WHERE clause.**
- **Found during:** Task 2, e2e-1 first run — `POST /api/applications/{id}/confirm` returned 409 with body "Version mismatch".
- **Issue:** `soft_deleted_app_doc.version: self.uuid_service.new_v4().await` — the `ApplicationDocumentDao::update` contract uses `entity.version` as the OLD version for the optimistic-lock WHERE clause; the DAO generates the NEW version internally. Passing a fresh UUID means `WHERE id = X AND version = <random>` matches zero rows, `rows_affected == 0`, `DaoError::ConflictError("Version mismatch")`, cascades to 409 out of confirm().
- **Fix:** Pass `app_doc.version` (the just-loaded value from `find_active_by_application_id`) instead of a fresh UUID. Added a 6-line block comment at the site explaining the contract so it doesn't regress.
- **Files modified:** `genossi_service_impl/src/application.rs`.
- **Verification:** E2E-1 flips from `409 Version mismatch` to `200 OK` after the fix. All 4 Plan-04 mock tests still pass (they never checked the version).
- **Committed in:** `5239c740` (Task 2).

**2. [Rule 1 — Bug] Plan-03 upload() replace-in-place path had the SAME bug + returned a stale version to the caller.**
- **Found during:** Task 2, e2e-3 first run — second upload succeeded but the returned TO carried the pre-update version, tripping `assert_ne!(first.version, second.version)`.
- **Issue:** Same optimistic-lock contract violation (`updated_entity.version: new_version` — a fresh v4 — instead of `old.version`). Additionally the returned TO was built from `updated_entity`, which never sees the DB-generated new version.
- **Fix:** (a) Pass `old.version` in the update — the DAO generates the fresh new version internally. (b) After the update, `find_active_by_application_id` (still inside the transaction) to load the fresh row with its new version; return that. Added a block comment at both sites explaining the contract. Updated `test_upload_replace_in_place_calls_save_then_update_then_delete` mock so `find_active_by_application_id` returns the pre-update row on the FIRST call and a synthesised post-update row (new filename + new path + new version) on subsequent calls.
- **Files modified:** `genossi_service_impl/src/application_document.rs` (production + tests).
- **Verification:** All 7 `application_document::tests` pass. E2E-3 flips from red to green.
- **Committed in:** `5239c740` (Task 2).

**Total deviations:** 2 auto-fixed (both Rule 1 correctness bugs surfaced by e2e proof; the mock tests failed to catch either).

## Issues Encountered

- **Pre-existing `test_mail_preview_repayment_no_entries_does_not_default_to_one` failure.** Same failure documented in Plan 25-04 SUMMARY under "Issues Encountered". Baseline `cargo test --workspace` shows 308 passed / 1 failed. Not related to Plan 25-05 — a Phase-22-or-earlier regression in the mail-template-preview render pipeline. Deferred to a separate quick/investigation. All 308 other tests (including the 3 new e2e-25-05 tests) pass.
- **rustfmt on the workspace shows drift in pre-existing files** (`genossi_service_impl/src/pdf_generation.rs`, `genossi-frontend/src/component/tsa_config.rs`, others). None of the files touched by this plan carry any formatting diff. Not fixed here — out of scope per the plan's scope-boundary rule.

## User Setup Required

None. No new packages, no schema migrations, no config changes. The frontend slot is live on the Application detail page for Offen applications as soon as the frontend rebuild lands (`dx serve` picks it up on hot-reload).

## Next Phase Readiness

- **Phase 25 is fully complete.** All 5 APDOC requirements shipped; all 5 plans done.
- **Milestone v1.4** — Phase 25 complete, Phase 24 complete. Phase 22 is "Ready to verify" and Phase 23 is "In Progress" per ROADMAP. The v1.4 close audit can now start after those two clear.
- **Vorstand smoke session** — Steps 4-12 of 25-UAT-CHECKLIST are still open. That is the intended way (mirrors Phase 24 UAT). Do NOT hold this phase for the smoke session.
- **No blockers** identified.

## Threat Model Compliance

| Threat ID | Mitigation | Where |
|---|---|---|
| T-25-05-01 (Tampering: Dioxus button form-submit reload) | 5 explicit `r#type: "button"` on interactive buttons + acceptance grep gate that rejects `r#type: "submit"`. | `application_document_slot.rs`. |
| T-25-05-02 (InfoDisclosure: locale drift) | Both-locales gate: every new key exists in `de.rs` AND `en.rs` in the same commit. | `i18n/mod.rs` + `i18n/de.rs` + `i18n/en.rs`. |
| T-25-05-03 (DoS: confirm-time missing-file NOT rolling back) | E2E-2 pins the rollback contract — status stays Offen, no member, audit still valid. | `application_upload_confirm_missing_file_rolls_back` + `application.rs` confirm() `?` propagation. |
| T-25-05-04 (Repudiation: audit hashchain invalidated by carryover) | E2E-1 asserts `/api/audit/verify.valid == true` after confirm. | `application_upload_confirm_carryover_audited`. |
| T-25-05-05 (Elevation of Privilege: unauthenticated upload) | UAT Step 10 + service-layer CR-02 tests (Wave 2). | `application_document.rs::upload` + Wave 2 tests. |
| T-25-05-SC (Tampering: package installs) | No new packages. Cargo.lock unchanged. | Verified. |

## Self-Check: PASSED

Verified via absolute-path checks and command output:

- `[ -f /home/neosam/programming/rust/projects/genossi3/genossi-frontend/src/component/application_document_slot.rs ]` → FOUND
- `[ -f /home/neosam/programming/rust/projects/genossi3/.planning/phases/25-application-file-upload-audited-carryover/25-UAT-CHECKLIST.md ]` → FOUND
- `grep -c 'pub mod application_document_slot' /home/neosam/programming/rust/projects/genossi3/genossi-frontend/src/component/mod.rs` → 1
- `grep -c 'ApplicationDocumentSlot' /home/neosam/programming/rust/projects/genossi3/genossi-frontend/src/component/application_detail.rs` → 2 (import + composition site)
- `grep -cE 'upload_application_document|get_application_document|delete_application_document|application_document_download_url' /home/neosam/programming/rust/projects/genossi3/genossi-frontend/src/api.rs` → 5 (4 declarations + 1 import re-export in Component)
- Both-locales gate for all 6 keys → PASS (de + en each have all 6)
- `grep -c 'r#type: "button"' /home/neosam/programming/rust/projects/genossi3/genossi-frontend/src/component/application_document_slot.rs` → 5
- `grep -v '^//' /home/neosam/programming/rust/projects/genossi3/genossi-frontend/src/component/application_document_slot.rs | grep -c 'r#type: "submit"'` → 0
- `grep -c 'HARD FAIL GATE' /home/neosam/programming/rust/projects/genossi3/.planning/phases/25-application-file-upload-audited-carryover/25-UAT-CHECKLIST.md` → 6
- `cargo check --target wasm32-unknown-unknown -p genossi-frontend` → clean (37 pre-existing dead-code warnings, no errors)
- `cargo test --test e2e_tests application_upload --features mock_auth` → 3 passed / 0 failed
- `cargo test -p genossi_service_impl application::tests` → 4 passed / 0 failed
- `cargo test -p genossi_service_impl application_document::tests` → 7 passed / 0 failed
- `cargo test --workspace --features mock_auth` → 308 passed / 1 failed (pre-existing `test_mail_preview_repayment_no_entries_does_not_default_to_one`, documented in Plan 25-04 SUMMARY)
- Commits `02bb5353` and `5239c740` present in jj log

---
*Phase: 25-application-file-upload-audited-carryover*
*Completed: 2026-07-03*
