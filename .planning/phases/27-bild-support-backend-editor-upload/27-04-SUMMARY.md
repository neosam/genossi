---
phase: 27-bild-support-backend-editor-upload
plan: 04
subsystem: frontend
tags: [dioxus, wasm, wysiwyg, contenteditable, multipart, drag-drop, i18n, grep-gate]

# Dependency graph
requires:
  - phase: 27-bild-support-backend-editor-upload
    provides: "POST /api/mail/assets upload endpoint + GET /api/mail/assets/{id}/bytes preview (27-01)"
  - phase: 24-wysiwyg-editor
    provides: "shared WysiwygEditor/WysiwygToolbar contenteditable component + exec_command_str facade + onmousedown grep-gate"
provides:
  - "upload_mail_asset(config, file) -> MailAssetTO frontend API client"
  - "image toolbar button (hidden PNG/JPEG/GIF picker → upload → insertHTML)"
  - "drag&drop image insert on the contenteditable editor (ondragover + ondrop)"
  - "image_insert_html(id) pure helper shared by toolbar button + drop handler"
  - "MailEditorImage + MailEditorImageUploadError i18n keys (both locales)"
affects: [phase-28 preview]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Hidden file input backing a toolbar button: button.onclick programmatically clicks a class=hidden <input type=file> to open the OS picker without a visible input"
    - "ondrop mirrors the proven onpaste prevent_default-first pattern; downcast Synthetic<web_sys::Event> -> web_sys::DragEvent -> data_transfer().files()"
    - "Additive web-sys feature (DragEvent) — no new crate, no package install (T-27-SC honoured)"
    - "Frontend-local rest-types crate is a SEPARATE copy from the backend genossi_rest_types — TOs must be mirrored into both"

key-files:
  created: []
  modified:
    - genossi-frontend/src/api.rs
    - genossi-frontend/rest-types/src/lib.rs
    - genossi-frontend/src/i18n/mod.rs
    - genossi-frontend/src/i18n/de.rs
    - genossi-frontend/src/i18n/en.rs
    - genossi-frontend/src/component/mail_compose/wysiwyg_toolbar.rs
    - genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs
    - genossi-frontend/Cargo.toml

key-decisions:
  - "Mirrored MailAssetTO into the frontend-local rest-types crate (Rule 3 blocking fix) — the frontend does NOT depend on the backend genossi_rest_types; 27-01 only added the TO backend-side"
  - "Toolbar image button uses a hidden <input type=file> clicked programmatically (keeps the onmousedown grep-gate satisfied on a real <button>, not a <label> wrapper)"
  - "image_insert_html is pub(crate) in the toolbar module and reused by the editor drop handler so the inserted <img> shape is byte-identical in both paths"
  - "Frontend performs NO trusted validation — accept= filter is a UX hint only; authoritative PNG/JPEG/GIF + 5 MB gate is server-side (T-27-16)"

requirements-completed: [IMG-03]

coverage:
  - id: D1
    description: "upload_mail_asset POSTs a single-field multipart 'file' to /api/mail/assets and parses MailAssetTO"
    requirement: "IMG-03"
    verification:
      - kind: compile
        ref: "cargo check -p genossi-frontend (append_with_blob_and_filename(\"file\"), MailAssetTO parse)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Image toolbar button preserves the onmousedown selection invariant and inserts the data-genossi-asset-id img"
    requirement: "IMG-03"
    verification:
      - kind: unit
        ref: "genossi-frontend/src/component/mail_compose/wysiwyg_toolbar.rs::grep_gate_tests::every_button_has_onmousedown_prevent_default"
        status: pass
      - kind: unit
        ref: "genossi-frontend/src/component/mail_compose/wysiwyg_toolbar.rs::image_insert_html_tests::produces_exact_asset_img_shape"
        status: pass
    human_judgment: false
  - id: D3
    description: "Drag&drop insert reuses the same img shape and prevents default browser file-open"
    requirement: "IMG-03"
    verification:
      - kind: unit
        ref: "genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs::grep_gate_tests::drop_handler_calls_prevent_default"
        status: pass
    human_judgment: false
  - id: D4
    description: "Functional/visual: picker + drag&drop insert an image that displays immediately via /bytes"
    requirement: "IMG-03"
    verification:
      - kind: manual
        ref: "Browser UAT deferred to Vorstand smoke session (success-criterion #1), per project convention"
        status: deferred
    human_judgment: true

# Metrics
duration: 30min
completed: 2026-07-23
status: complete
---

# Phase 27 Plan 04: WYSIWYG Editor Image Upload Summary

**The authoring half of image support (IMG-03): the Vorstand can insert an inline image into the shared WYSIWYG editor via a toolbar button (PNG/JPEG/GIF file picker) OR by dragging & dropping a file onto the editor — both upload through 27-01's admin-gated endpoint and insert an identical `<img data-genossi-asset-id …>` at the caret.**

## Performance

- **Duration:** ~30 min
- **Tasks:** 3 completed
- **Files modified:** 8 (0 created, 8 modified)

## Accomplishments
- Added `upload_mail_asset(config, file) -> MailAssetTO` to the frontend API client, copying the proven `upload_member_document` multipart flow but reduced to a single `file` field POSTing to 27-01's `/api/mail/assets`.
- Extended the EXISTING shared `WysiwygToolbar` with an image button (hidden PNG/JPEG/GIF file input, clicked programmatically) that uploads and inserts via the existing `exec_command_str("insertHTML", …)` facade — no inline-RSX duplication, and the mandatory `onmousedown`+`prevent_default` selection-preserve invariant is honoured (grep-gate stays green at 14 buttons).
- Extended the EXISTING `WysiwygEditor` contenteditable with `ondragover`(prevent_default) + `ondrop` handlers mirroring the proven `onpaste` prevent_default-first pattern; the drop path reuses the shared `image_insert_html` helper so the inserted `<img>` is byte-identical to the toolbar path.
- Added `MailEditorImage` + `MailEditorImageUploadError` i18n keys to both locales in the same commit (no locale drift), and the additive `DragEvent` web-sys feature (no new crate).

## Task Commits

1. **Task 1: upload_mail_asset API client + image i18n keys** - `6ed5538` (feat)
2. **Task 2: image toolbar button (picker → upload → insertHTML) with onmousedown invariant** - `0f4ad94` (feat)
3. **Task 3: drag&drop image insert on the WYSIWYG editor** - `417b431` (feat)

## Files Created/Modified
- `genossi-frontend/src/api.rs` - `upload_mail_asset` (single-field FormData `file` POST + `map_web_response_error` + `serde_wasm_bindgen::from_value`), import `MailAssetTO`
- `genossi-frontend/rest-types/src/lib.rs` - mirrored `MailAssetTO` into the frontend-local rest-types crate (backend TO shape: id/filename/mime_type/size_bytes/created)
- `genossi-frontend/src/i18n/{mod,de,en}.rs` - `MailEditorImage` ("Bild einfügen"/"Insert image") + `MailEditorImageUploadError`
- `genossi-frontend/src/component/mail_compose/wysiwyg_toolbar.rs` - image button + hidden file input, `pub(crate) fn image_insert_html(id)` helper + unit test, `editor_id_m` clone
- `genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs` - `ondragover` + `ondrop` handlers, reuse `image_insert_html`, new `drop_handler_calls_prevent_default` grep-gate
- `genossi-frontend/Cargo.toml` - added `DragEvent` web-sys feature

## Decisions Made
- Followed the PLAN's "keep the onmousedown grep-gate satisfied" guidance by rendering a real `<button>` (with onmousedown+prevent_default) that programmatically clicks a separate hidden `<input type=file class=hidden>`, rather than a `<label>` wrapper. The input carries `r#type: "file"` so it is NOT counted by the button grep-gate.
- Reused the toolbar's `image_insert_html` from the editor drop handler (made `pub(crate)`) so there is a single source of truth for the inserted `<img>` shape.
- Config is read via `crate::service::config::CONFIG.read().clone()` inside the spawned async task (same pattern as `template_tester.rs`).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Mirrored `MailAssetTO` into the frontend-local rest-types crate**
- **Found during:** Task 1
- **Issue:** The plan/PATTERNS assumed `MailAssetTO` was importable from the frontend's `rest_types` dependency, but the frontend uses a SEPARATE local crate at `genossi-frontend/rest-types` (not the backend `genossi_rest_types` that 27-01 modified). `cargo check` failed with `E0432: no MailAssetTO in the root`.
- **Fix:** Added a `MailAssetTO` struct to `genossi-frontend/rest-types/src/lib.rs` mirroring the backend TO's serialized shape (id, filename, mime_type, size_bytes, optional ISO8601 created). No `ToSchema`/`genossi_service` deps needed frontend-side.
- **Files modified:** `genossi-frontend/rest-types/src/lib.rs`
- **Commit:** `6ed5538`

No architectural changes (Rule 4) were needed.

## Authentication Gates

None. No package installs were attempted (T-27-SC honoured — only the additive `DragEvent` web-sys feature).

## Known Stubs

None. Both insert paths (toolbar + drop) are fully wired to the real 27-01 upload endpoint and the live `/bytes` preview src.

## Verification
- `cargo check` (from `genossi-frontend/`) — compiles with `upload_mail_asset`, the local `MailAssetTO`, the image button, ondrop, and the `DragEvent` feature.
- `cargo test wysiwyg_toolbar` — 3 passed (grep-gate `every_button_has_onmousedown_prevent_default` green with the new button counted; `image_insert_html` exact-shape unit test; production-region meta-test).
- `cargo test wysiwyg_editor` — 11 passed (incl. the new `drop_handler_calls_prevent_default` and the pre-existing paste/styleWithCSS/scope grep-gates).
- Full `cargo test` (frontend) — 300 passed, 0 failed (no regressions).
- Acceptance greps: `append_with_blob_and_filename("file"` present in api.rs; `MailEditorImage` in mod.rs+de.rs+en.rs; `insertHTML` via `exec_command_str` in the toolbar; `ondragover`/`ondrop` in the editor.

## Self-Check: PASSED

- All 8 modified files present on disk and in HEAD tree; top-level `CLAUDE.md` survives every commit tree (VCS index-desync guard: `git read-tree HEAD` before staging + `git write-tree` verification).
- All task commits present: `6ed5538`, `0f4ad94`, `417b431`.
- No unintended file deletions in any commit (`git diff --diff-filter=D HEAD~1 HEAD` empty per task).
