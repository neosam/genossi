---
phase: 24-wysiwyg-frontend-editor
plan: 02
subsystem: frontend
tags: [dioxus, wasm, contenteditable, execCommand, ammonia-safe, i18n, wysiwyg, component-first, in-app-modal]

# Dependency graph
requires:
  - phase: 24-wysiwyg-frontend-editor plan 01
    provides: 19 Key::MailEditor* i18n arms + web-sys ClipboardEvent + DataTransfer features
provides:
  - WysiwygEditor Dioxus component (contenteditable host with on_change: EventHandler<(String, String)> -> (plain, html))
  - WysiwygToolbar sub-component (13 ammonia-safe buttons: B/I/U/S/UL/OL/H1/H2/H3/¶/❝/🔗/⊘)
  - WysiwygLinkDialog sub-component (in-app Modal for URL entry, per D-06)
  - is_valid_link_url pure helper (http:// | https:// only)
  - genossi-frontend/src/js.rs::exec_command_bool + exec_command_str + exec_command_simple execCommand facade
  - genossi-frontend Cargo.toml web-sys Selection + Range features
  - pub use WysiwygEditor from crate::component::mail_compose
affects: [24-03-migration]

# Tech tracking
tech-stack:
  added: [web-sys Selection feature, web-sys Range feature]
  patterns: [execCommand-via-js_sys::Reflect facade, contenteditable + onmounted styleWithCSS-off, plain-text paste via preventDefault + insertText, Selection Range preservation across in-app Modal, DOM-sync-after-every-command tuple push]

key-files:
  created:
    - genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs
    - genossi-frontend/src/component/mail_compose/wysiwyg_toolbar.rs
    - genossi-frontend/src/component/mail_compose/wysiwyg_link_dialog.rs
  modified:
    - genossi-frontend/src/js.rs
    - genossi-frontend/src/component/mail_compose/mod.rs
    - genossi-frontend/Cargo.toml

key-decisions:
  - execCommand facade via js_sys::Reflect (no new JS bundle, no extern "C" binding) mirrors the existing copy_with_exec_command pattern — EDIT-02 (no new frontend deps) honored
  - styleWithCSS=false is invoked exactly once at mount via onmounted so bold/italic emit semantic <b>/<i> instead of <span style=…> (Pitfall 1 of 24-RESEARCH.md, D-05)
  - is_valid_link_url is a whitelist: only http:// and https:// pass (rejects javascript:, data:, ftp:, relative paths) — UX gate before ammonia stripping at store boundary
  - onpaste ClipboardData path: Dioxus 0.6.3 ClipboardData has NO get_data; use downcast::<web_sys::Event> -> ClipboardEvent -> clipboard_data() -> get_data("text/plain") -> exec_command_str("insertText", …). preventDefault FIRST (Pitfall 3)
  - Every non-Link toolbar button triggers on_command -> sync_from_dom to push (inner_text, inner_html) tuple into on_change (Pitfall 5 — DOM-sync-race)
  - Link button captures Selection Range BEFORE opening WysiwygLinkDialog (Pitfall 6); on Insert restore focus + range before dispatching createLink so the anchor lands at the original caret
  - Only WysiwygEditor is re-exported from mail_compose/mod.rs — Toolbar and LinkDialog stay internal (they are strictly composed by Editor); Plan 24-03 rewires the 3 MailBodyEditor call sites purely by import swap

patterns-established:
  - "execCommand helpers: pub fn exec_command_bool/str/simple(&web_sys::Document, cmd, arg) -> Result<bool, JsValue> — canonical Reflect-based dispatch, mirrored from copy_with_exec_command"
  - "Dioxus button-reload-bug workaround: EVERY <button> in the new components carries r#type: 'button' + evt.prevent_default() first line in onclick (feedback_dioxus_button_type.md; 13 in toolbar + 2 in link dialog)"
  - "In-app Modal for link URL entry — no window.prompt anywhere (D-06)"
  - "Constant DOM id (EDITOR_ID = 'wysiwyg-editor') for the contenteditable div — passed to WysiwygToolbar so focus-before-command wiring is O(1) getElementById"
  - "sync_from_dom(on_change: &EventHandler<(String, String)>) helper: called after every DOM mutation (oninput, toolbar command, link insert, paste) — pushes (inner_text, inner_html) tuple through the EventHandler"

# Metrics
duration: ~35min
completed_date: 2026-07-03

status: complete
---

# Phase 24 Plan 02: WYSIWYG Editor Component Wave 2 Summary

Reusable Dioxus WYSIWYG editor component set — contenteditable host + 13-button toolbar + in-app link modal + execCommand facade — landed as three new files in `genossi-frontend/src/component/mail_compose/` plus three helpers in `genossi-frontend/src/js.rs`. Plan 24-03 will drop `WysiwygEditor` into the three `MailBodyEditor` call sites (mail_page, reply_form, mail_templates) by import swap alone.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Add exec_command_{bool,str,simple} helpers to js.rs | 7344d179 | genossi-frontend/src/js.rs |
| 2 | Create wysiwyg_link_dialog.rs (in-app link URL modal) | 34ddebda | wysiwyg_link_dialog.rs + mod.rs |
| 3 | Create wysiwyg_toolbar.rs (13 ammonia-safe buttons) | f4b04ee7 | wysiwyg_toolbar.rs + mod.rs |
| 4 | Create wysiwyg_editor.rs (contenteditable host) | b411c8fa | wysiwyg_editor.rs + Cargo.toml + mod.rs |
| 5 | Export WysiwygEditor from mod.rs | f2a3adb7 | mod.rs |

## Contract Guarantees (must_haves cross-check)

- [x] `WysiwygEditor` at `genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs` exposes `on_change: EventHandler<(String, String)>` where the tuple is `(plain: innerText, html: innerHTML)`. — D-01/D-02.
- [x] On mount, the editor calls `exec_command_bool(&doc, "styleWithCSS", false)` exactly once in the `onmounted` closure. — D-05, Pitfall 1.
- [x] Toolbar buttons emit only ammonia-safe commands: `bold, italic, underline, strikeThrough, insertUnorderedList, insertOrderedList, formatBlock(<h1>/<h2>/<h3>/<p>/<blockquote>), createLink, unlink`. No `foreColor`, `hiliteColor`, `insertImage`, `insertHorizontalRule`.
- [x] Every button uses `r#type: "button"` + `evt.prevent_default()` first line in onclick. Grep: 13 in toolbar, 2 in link dialog. — feedback_dioxus_button_type.md.
- [x] `onpaste` handler calls `evt.prevent_default()` FIRST, downcasts to `web_sys::ClipboardEvent`, reads `text/plain`, inserts via `exec_command_str("insertText", …)`. — Pitfall 3, D-07.
- [x] Every non-Link toolbar-button click triggers `on_command` → `sync_from_dom` → pushes tuple through `on_change`. — Pitfall 5.
- [x] `WysiwygLinkDialog` renders inside shared `Modal` component; no `window.prompt` anywhere. — D-06.

## Artifacts

- `genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs` (new — 160 lines, hosts contenteditable div + Toolbar + LinkDialog wiring + Selection Range preservation across the modal)
- `genossi-frontend/src/component/mail_compose/wysiwyg_toolbar.rs` (new — 260 lines, button row with 13 execCommand dispatches, focus-editor-before-command)
- `genossi-frontend/src/component/mail_compose/wysiwyg_link_dialog.rs` (new — 145 lines, URL + display-text form inside Modal with `is_valid_link_url` gate)
- `genossi-frontend/src/js.rs` — appended `exec_command_bool`, `exec_command_str`, `exec_command_simple` after `copy_with_exec_command`
- `genossi-frontend/src/component/mail_compose/mod.rs` — `pub mod wysiwyg_editor|toolbar|link_dialog` + `pub use wysiwyg_editor::WysiwygEditor`
- `genossi-frontend/Cargo.toml` — web-sys `Selection` + `Range` features added (Phase 24 P02 comments)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 — Blocker] Dioxus 0.6.3 ClipboardData has no `.get_data()` method**

- **Found during:** Task 4
- **Issue:** The plan's `<action>` block flagged the exact `ClipboardData::get_data` shape as `[ASSUMED]` per Open Question §3 of 24-RESEARCH.md and instructed a probe. Confirmed via source inspection of `~/.cargo/registry/…/dioxus-html-0.6.3/src/events/clipboard.rs`: `ClipboardData` only exposes `.downcast::<T>()` — no `.get_data()`, no `.data()` method.
- **Fix:** Took the web-sys fallback path baked into the plan: `evt.downcast::<web_sys::Event>().cloned()` (dioxus-web's `HasClipboardData` impl stores the browser Event as its any) → cast to `web_sys::ClipboardEvent` → `clipboard_data()` → `get_data("text/plain")`. No new deps.
- **Files modified:** genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs
- **Commit:** b411c8fa (Task 4)

**2. [Rule 2 — Missing critical functionality] web-sys Selection + Range features missing**

- **Found during:** Task 4
- **Issue:** Selection Range preservation (Pitfall 6) requires `web_sys::Range`, which is behind the `Range` cargo feature. `get_selection()`/`add_range()` require `Selection`. Neither was on the existing feature list; without them the link-dialog Insert flow could not restore the caret.
- **Fix:** Added `"Selection"` and `"Range"` to `[dependencies.web-sys].features` in `genossi-frontend/Cargo.toml` (in-scope per Task 4 action — legitimate scope call-out).
- **Commit:** b411c8fa (Task 4)

**3. [Rule 1 — Bug] `title` attribute type coercion**

- **Found during:** Task 3
- **Issue:** `i18n.t(Key::…)` returns `Rc<str>`, which the Dioxus rsx `title:` attribute rejects with `trait bound Rc<str>: IntoAttributeValue<AnyFmtMarker> is not satisfied` on all 13 buttons.
- **Fix:** Wrap in Dioxus format-string syntax: `title: "{i18n.t(Key::…)}"` (matches existing pattern in `component/editable_share_count_cell.rs`).
- **Commit:** f4b04ee7 (Task 3)

### Scope Adjustments

- Task 5's `pub mod wysiwyg_*` lines had to be added incrementally during Tasks 2–4 (not deferred to Task 5) so each intermediate task's `cargo check` could see the file. Task 5 then added only the `pub use wysiwyg_editor::WysiwygEditor;` re-export. Net effect on `mod.rs` matches the plan.
- The `display_text` capture in `WysiwygLinkDialog` is passed through the `on_insert` tuple but the current editor implementation ignores it (variable prefixed `_display_text`). The plan explicitly allowed this: "Optional: rewrite the anchor text if the user supplied a distinct display_text — omit for now, users can type it themselves before opening the dialog". Users type the visible link text inside the editor before clicking 🔗; if they later want to override it, that becomes a Plan 24-04 UAT nit.

## Verification Evidence

- `cargo check` in `genossi-frontend` — clean; only pre-existing dead_code warnings on unrelated legacy variants.
- `cargo test is_valid_link_url` — 3 tests pass:
  - `is_valid_link_url_accepts_http_and_https` ✓
  - `is_valid_link_url_rejects_empty_and_whitespace` ✓
  - `is_valid_link_url_rejects_javascript_and_data_scheme` ✓
- `cargo build` (workspace) — clean.
- Grep hygiene:
  - `grep -c 'r#type: "button"' wysiwyg_toolbar.rs` = 13 ✓
  - `grep -c 'contenteditable' wysiwyg_editor.rs` = 4 (1 real attribute + 3 in doc comments explaining the pattern)
  - `grep -c 'styleWithCSS' wysiwyg_editor.rs` = 3 (1 real call + 2 in doc comments citing Pitfall 1)
  - `grep -c 'prevent_default' wysiwyg_editor.rs` = 1 (paste handler) ✓
  - `grep -rn 'window.prompt' genossi-frontend/src/component/mail_compose/wysiwyg_*.rs` = 0 ✓
  - `grep -rn 'onsubmit' genossi-frontend/src/component/mail_compose/wysiwyg_*.rs` = 0 ✓
- `grep -c 'pub mod wysiwyg' mail_compose/mod.rs` = 3, `grep -c 'pub use wysiwyg_editor::WysiwygEditor' mail_compose/mod.rs` = 1 ✓

## Threat Model Follow-up

Task-scoped threats T-24-04..T-24-06 from the plan's `<threat_model>` block are all `mitigate`-disposed and land inside the code:

| Threat | Mitigation in code | Where |
|--------|--------------------|-------|
| T-24-04 (paste tampering) | preventDefault + `exec_command_str("insertText", &text)` — no HTML paste | wysiwyg_editor.rs onpaste closure |
| T-24-05 (URL scheme tampering) | `is_valid_link_url` gates the Insert button; ammonia at store is defense-in-depth | wysiwyg_link_dialog.rs |
| T-24-06 (Range signal disclosure) | Signal is component-scoped, cleared to `None` after use | wysiwyg_editor.rs `saved_range.set(None)` after insert |

No new threat surface discovered — all execCommand dispatches produce tags on the existing ammonia allow-list (Phase 23 D-03).

## Known Stubs

None. The `display_text` field of `WysiwygLinkDialog::on_insert` is currently ignored by the editor (see Scope Adjustments) — this is a documented design decision, not a stub. All UI paths render live data via i18n + user input.

## Self-Check: PASSED

Files exist:
- `[FOUND] genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs`
- `[FOUND] genossi-frontend/src/component/mail_compose/wysiwyg_toolbar.rs`
- `[FOUND] genossi-frontend/src/component/mail_compose/wysiwyg_link_dialog.rs`

Commits exist in jj log:
- `[FOUND] 7344d179` (Task 1)
- `[FOUND] 34ddebda` (Task 2)
- `[FOUND] f4b04ee7` (Task 3)
- `[FOUND] b411c8fa` (Task 4)
- `[FOUND] f2a3adb7` (Task 5)

## Plan 24-03 Handoff

Plan 24-03 (Wave 3 — Migration) can now:

1. Import `WysiwygEditor` from `crate::component::mail_compose` (single-line import swap).
2. Wire the tuple `on_change: EventHandler<(String, String)>` — first tuple element is the plain-text body (backward-compat with existing `Signal<String>` body_text), second is the sanitized-server-side body_html for the new `Preview*.body_html` and `ReplyRequest.body_html` wire fields already delivered in Plan 24-01.
3. Delete `genossi-frontend/src/component/mail_compose/body_editor.rs` and its `pub use body_editor::MailBodyEditor;` re-export from `mod.rs` after all three call sites have been verified against the migration.

No further deps or seams needed — the migration in 24-03 is a pure edit-3-sites-mechanically operation.
