# Phase 24 UAT Checklist

**Phase:** 24-wysiwyg-frontend-editor
**Coverage:** EDIT-01, EDIT-02, EDIT-03, EDIT-04, EDIT-05 (Vorstand-facing browser behaviors that automated e2e cannot cover)
**Companion automated tests:** `preview_body_html_round_trips_to_response`, `inbox_reply_body_html_sanitized_and_persisted` (both in `genossi_bin/tests/e2e_tests.rs`)

This checklist walks a Vorstand-facing UAT run through the three compose flows (Massenmail, Inbox-Reply, Mail-Template editor) to prove the WYSIWYG editor behaves correctly in a real browser. Steps 3, 4, 5 are hard-fail gates — those pin the ammonia/D-06 invariants that make the sanitize-on-store gate work at all.

## Setup

Follow the project skill `run-rust-backend-and-frontend` — or manually:

1. **Backend** — from repo root:
   ```bash
   cargo run --features mock_auth --bin genossi
   ```
   Serves on `http://localhost:3000` with mock authentication (Context = DEVUSER, admin).

2. **Frontend** — from `genossi-frontend/`:
   ```bash
   npx tailwindcss -i ./input.css -o ./assets/tailwind.css --watch &
   dx serve
   ```
   Serves on `http://localhost:8080`. `assets/config.json` points at the backend URL.

3. **Test data** — seed at least one Member via the Members page so the compose flows have someone to send to. Do **not** click Send in the smoke test — the Dev-DB may contain real member email addresses (see `reference_frontend_smoke_test_setup.md`).

## Verification Steps

Tick the checkbox after each step you complete. For failing items, capture: (a) which EDIT requirement was hit, (b) a DevTools screenshot of the innerHTML or Network payload, (c) suggested fix location.

- [ ] **1. Editor mounts without console errors [EDIT-01].** Navigate to the Massenmail-Compose page. Open DevTools Console. Confirm the `WysiwygEditor` renders (a contenteditable div with a toolbar row above it) and no red errors appear on mount. Expected: the toolbar shows 13 buttons (B/I/U/S/UL/OL/H1/H2/H3/¶/❝/🔗/⊘) plus the paragraph/heading labels per the i18n `MailEditor*` keys.

- [ ] **2. Plain-text side of on_change tuple flows correctly [EDIT-01, EDIT-03].** Type "Hallo Welt" into the editor. Open DevTools Elements panel, find the `<div id="wysiwyg-editor" contenteditable="true">` element, and confirm its innerText matches what you typed. Then click Send (or just open Network tab, then click Send) and inspect the outgoing `/api/mail/send-bulk` POST payload — the `body` field must equal `"Hallo Welt"` (plain-text extraction via innerText). Cancel the send before it actually goes through if the DB has real member addresses.

- [ ] **3. styleWithCSS=false is enforced (Bold produces `<b>`, not span-style) [EDIT-02, ammonia gate invariant].** ⚠️ **HARD FAIL GATE.** Type "Test", select all, click the Bold button. Open DevTools Elements panel and inspect the editor's innerHTML. Expected: `<b>Test</b>` (or `<b>...Test...</b>`) — a semantic `<b>` tag. **FAIL:** `<span style="font-weight: bold">Test</span>` — that means `styleWithCSS` was not toggled to `false` on mount. This breaks the ammonia gate downstream (spans with inline style don't survive the ammonia allow-list). Fix location: `wysiwyg_editor.rs` `onmounted` closure must call `exec_command_bool(&doc, "styleWithCSS", false)`.

- [ ] **4. Paste from Word/browser is plain-text only [EDIT-04, ammonia gate invariant].** ⚠️ **HARD FAIL GATE.** Open a formatted document (e.g. Word, or copy a bold+italic paragraph from any web page), copy the formatted text, paste into the editor. Inspect the innerHTML in DevTools. Expected: only the plain text appears, wrapped by whatever markup the caret was already inside (paragraph or line). **FAIL:** `<span>`/`<b>`/`<p style="...">` from the source document showing up in the editor's DOM. Fix location: `wysiwyg_editor.rs` `onpaste` closure must call `evt.prevent_default()` FIRST, then insert via `exec_command_str("insertText", &plain_text)`.

- [ ] **5. Link toolbar button opens in-app modal (not native prompt) [EDIT-01, D-06].** ⚠️ **HARD FAIL GATE.** Click the 🔗 (Link) toolbar button. Expected: an in-app modal opens with URL field + optional display-text field + Insert/Cancel buttons — styled like the rest of the app. **FAIL:** a native browser `window.prompt()` dialog opens (looks like an OS-level prompt). Fix location: `wysiwyg_link_dialog.rs` must render inside the shared `Modal` component; no `window.prompt` in the source. Verify with: `grep -rn 'window.prompt' genossi-frontend/src/component/mail_compose/` returns 0.

- [ ] **6. Link wraps selection (selection preservation across modal) [EDIT-01, D-06, Pitfall 6].** Type "Klick hier", select "hier" with the cursor, click 🔗, enter URL `https://example.com`, click Insert. Inspect innerHTML. Expected: `Klick <a href="https://example.com">hier</a>`. **FAIL:** the anchor is inserted somewhere else (e.g. at the end of the paragraph) — that means the Selection Range wasn't preserved across the modal.

- [ ] **7. Invalid link URL is rejected before insert [EDIT-02 defense-in-depth].** Click 🔗, enter `javascript:alert(1)` in the URL field. Expected: the Insert button stays **disabled** (no click possible). Also verify with `data:text/html,<script>` and empty string — all three must keep Insert disabled. Fix location: `is_valid_link_url` in `wysiwyg_link_dialog.rs` (whitelist: `http://` | `https://` only).

- [ ] **8. TemplatePreview renders live HTML with member variables substituted [EDIT-05].** In the Massenmail-Compose page, type into the editor: `Hallo {{ first_name }}, dein Beitrag: **fett** kursiv`. Click the Bold button on the word "fett" to make it a `<b>`. Select a member in the TemplatePreview dropdown. Expected: the preview panel renders the HTML section (labeled per `Key::MailEditorPreviewHtml`) showing "Hallo <first_name>, dein Beitrag: **fett** kursiv" — with **fett** visually bold (not literal `<b>` text). The plain-text `<pre>` block above it shows the plain rendered text.

- [ ] **9. Sent bulk-mail is multipart/alternative with plain + HTML parts [EDIT-03, Phase 23 HTML-01].** ⚠️ **Only run against a test SMTP inbox** — never against real member emails. Configure a test recipient (e.g. Mailhog, Mailtrap, or a personal test address) via the SMTP config UI. Compose with formatted body ("Hallo **Welt**"), click Send, receive the mail in the test MUA, view raw source. Expected: `Content-Type: multipart/alternative`, with both `text/plain` and `text/html` parts, HTML part contains `<b>Welt</b>`, plain part contains `Welt` (without markup).

- [ ] **10. Inbox-Reply form matches Compose behavior [EDIT-01, EDIT-02, EDIT-04, EDIT-05].** Navigate to Inbox, open any inbound mail, click Reply. Repeat steps 3, 4, 5 (styleWithCSS Bold check, paste-plain check, in-app modal check) inside the reply form. All three must pass identically to the Compose flow — the `WysiwygEditor` is the same component instance.

- [ ] **11. Mail-Template edit loads existing body_html and toolbar works on it [EDIT-01, EDIT-03].** Navigate to Mail Templates, open a template that has a saved `body_html` (or create one first via step 12 and reload). Expected: the editor loads with the saved HTML rendered (bold text is visually bold, not literal `<b>` tags), and clicking toolbar buttons modifies the loaded content correctly. Inspect innerHTML in DevTools to confirm the content matches what was persisted.

- [ ] **12. Mail-Template body_html round-trips through save + reload [EDIT-03, wire integrity].** Create a new Mail Template with a formatted body ("Hallo **Welt**"). Save it. Reload the templates page and reopen the template. Expected: the editor loads with the same formatting intact — `<b>Welt</b>` still bold. Verify via DevTools that the innerHTML matches the sanitized-ammonia allow-list (only safe tags survived). Also check the Network response of the template GET: the JSON body must contain `body_html: "<p>Hallo <b>Welt</b></p>"` (or similar, per ammonia normalization).

## Known limitations

- **`execCommand` is deprecated but stable for our command subset.** The subset we dispatch (`bold`, `italic`, `underline`, `strikeThrough`, `insertUnorderedList`, `insertOrderedList`, `formatBlock`, `createLink`, `unlink`, `insertText`, `styleWithCSS`) is supported in all current browsers. If a future Chromium/Firefox release removes support, the fallback path is a `Range.surroundContents`-based tag wrapper in `wysiwyg_toolbar.rs` — not scoped for Phase 24.

- **TemplateVarButtons signal-sync lag (Plan 24-03 Task 3 comment).** When you click a template-var button (e.g. `{{ payout_amount }}`) it appears immediately in the plain-text body signal AND is mirrored (HTML-escaped) into body_html. But the editor's contenteditable DOM state does NOT visually update until the next keystroke (Dioxus's `value` prop does not push into contenteditable once mounted). This is a known 1-render lag documented in Plan 24-03 §Decisions Made and Plan 24-03 §Task Handoff. Not a bug; documented UX quirk. On the next user keystroke, `oninput` triggers `sync_from_dom` and both signals resync from the actual DOM.

- **TemplateSelector clears body_html on select.** Templates surface plain-text `body` only in this milestone. When a template is selected, the WysiwygEditor's body_html signal is reset to empty. Any HTML the user then adds via the toolbar becomes the new HTML surface. See Plan 24-03 §Decisions Made for the reasoning.

- **The initial-body-with-footer path (mail_page.rs) and initial-body-with-quote path (reply_form.rs) leave body_html empty on mount.** The footer/quote sits in plain body only. HTML surface only appears when the user types in the WysiwygEditor. This is intentional Wave 3 UX — the smoke tester should not expect a formatted footer on a fresh compose.

## Regression check

Before declaring UAT complete, run the backend + workspace test suites and confirm no regression:

```bash
cargo test -p genossi_mail --lib
cargo test -p genossi_bin --test e2e_tests
cargo build
```

Expected results:
- `genossi_mail --lib`: 252+ tests pass (baseline from Plan 24-01 self-check).
- `genossi_bin --test e2e_tests`: 306 tests pass (304 baseline + 2 new from Plan 24-04).
- One pre-existing failure `test_mail_preview_repayment_no_entries_does_not_default_to_one` is carried over from Phase 22 and documented in `.planning/STATE.md` — that is NOT a Phase 24 regression.
- `cargo build` (workspace): clean, no warnings escalated.

Frontend:
```bash
cd genossi-frontend
cargo check --target wasm32-unknown-unknown
cargo test --bin genossi-frontend
```

Expected:
- `cargo check --target wasm32-unknown-unknown`: clean (pre-existing dead_code warnings on unrelated legacy variants OK).
- `cargo test --bin genossi-frontend`: 284+ pass (baseline from Plan 24-03 self-check).

## Sign-off

- **Vorstand smoke tester:** _______________
- **Date:** _______________
- **All 12 steps checked:** ☐ Yes  ☐ No — see notes
- **Hard-fail gates (3, 4, 5) passed:** ☐ Yes  ☐ No — MUST fix before merge

If any hard-fail gate (3, 4, 5) failed, block the phase completion and file a bug ticket referencing the specific EDIT-XX requirement plus the DevTools evidence. Non-critical failures (e.g. step 11 or 12) may be deferred to a follow-up plan with human sign-off.
