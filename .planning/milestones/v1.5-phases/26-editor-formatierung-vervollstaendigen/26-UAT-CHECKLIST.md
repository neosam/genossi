# Phase 26 UAT Checklist

**Phase:** 26-editor-formatierung-vervollstaendigen
**Coverage:** EDIT-06, EDIT-07, EDIT-08, EDIT-09, EDIT-10 plus alle Phase-24-UAT-Punkte (deferred aus v1.4)
**Companion automated tests:** `sanitize_preserves_unordered_list`, `sanitize_preserves_ordered_list`, `sanitize_preserves_headings_h1_h2_h3` (in `genossi_mail/src/sanitize.rs`), `create_template_body_html_lists_and_headings_round_trip` (in `genossi_bin/tests/e2e_tests.rs`), `style_with_css_false_guard_present` + `paste_handler_calls_prevent_default_before_read` (in `genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs`), plus die Phase-24-Companions `preview_body_html_round_trips_to_response` + `inbox_reply_body_html_sanitized_and_persisted` (beide in `genossi_bin/tests/e2e_tests.rs`)

This checklist walks a Vorstand-facing UAT run through the three compose flows (Massenmail, Inbox-Reply, Mail-Template editor) to prove the WYSIWYG editor behaves correctly in a real browser. Steps 3, 4, 5 are hard-fail gates (Phase-24-Invarianten). Steps 13-16 verify the Phase-26 additions: UL, OL, H2, H3 round-trip through Save + Reload. Per D-06 this checklist is the SHIP-Gate before `/gsd-complete-milestone` (v1.5-milestone-close), NOT a merge-gate inside Phase 26 (jj-WIP-Changes make classic PR-gating irrelevant).

## Setup

*Updated 2026-07-17.* Follow the project skill `run-rust-backend-and-frontend` — or manually:

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

- [ ] **1. Editor mounts without console errors [EDIT-01].** Navigate to the Massenmail-Compose page. Open DevTools Console. Confirm the `WysiwygEditor` renders (a contenteditable div with a toolbar row above it) and no red errors appear on mount. Expected: the toolbar shows 13 buttons (B/I/U/S/UL/OL/H1/H2/H3/¶/❝/🔗/⊘) plus the paragraph/heading labels per the i18n `MailEditor*` keys. *(H1 bleibt in Toolbar per D-01 — auch wenn EDIT-08 nur H2/H3 fordert; die 13-Buttons-Ziffer zählt H1 mit.)*

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

- [ ] **13. Unordered List Toolbar-Button erzeugt `<ul><li>` [EDIT-06].** Navigate to Massenmail-Compose. Type „Erstens" ↵ „Zweitens" ↵ „Drittens". Select alle drei Zeilen. Click UL-Button (`•`) in der Toolbar. Open DevTools Elements panel, inspect innerHTML des `<div id="wysiwyg-editor">`. Expected: `<ul><li>Erstens</li><li>Zweitens</li><li>Drittens</li></ul>` — leere `<br>`-Filler in `<li>` sind akzeptiert (execCommand-Cross-Browser-Toleranz, Pitfall 5 in 26-RESEARCH.md). Save-as-Template → Templates-Page → Reload → Template öffnen → innerHTML enthält weiterhin `<ul>` und `<li>`-Tags mit den drei Texten. **FAIL:** UL-Struktur ist verloren oder als `<div>`/`<p>`-Fallback vorhanden — Fix: `wysiwyg_toolbar.rs` UL-Button-Handler prüfen.

- [ ] **14. Ordered List Toolbar-Button erzeugt `<ol><li>` [EDIT-07].** Wie Step 13, aber mit dem OL-Button (`1.`). Expected: `<ol><li>Erstens</li><li>Zweitens</li><li>Drittens</li></ol>`. Save → Reload → OL-Struktur intakt. **FAIL:** OL fehlt oder wird zu UL. Fix: `wysiwyg_toolbar.rs` OL-Button-Handler prüfen.

- [ ] **15. H2 Toolbar-Button erzeugt `<h2>` und überlebt Reload [EDIT-08].** Type „Kapitel-Titel", select all, click H2-Button (`H2`). Expected: innerHTML enthält `<h2>Kapitel-Titel</h2>`. Save-as-Template → Reload → Template neu öffnen → innerHTML enthält weiterhin `<h2>`. **FAIL:** `<div>`/`<p>` statt `<h2>` oder `<h2>`-Tag wird beim Reload zu Plain-Text. Fix: `wysiwyg_toolbar.rs` H2-Button-Handler (`formatBlock <h2>`) + ammonia-Whitelist prüfen.

- [ ] **16. H3 Toolbar-Button erzeugt `<h3>` und überlebt Reload [EDIT-08].** Wie Step 15, aber mit H3-Button (`H3`) und Text „Sub-Titel". Expected: `<h3>Sub-Titel</h3>` überlebt Save → Reload byte-identisch (modulo ammonia-Whitespace-Normalisierung, Pitfall 4). **FAIL:** `<div>`/`<p>` statt `<h3>` oder `<h3>` verschwindet nach Reload. Fix: `wysiwyg_toolbar.rs` H3-Button-Handler (`formatBlock <h3>`) + ammonia-Whitelist prüfen.

## Known limitations

- **`execCommand` is deprecated but stable for our command subset.** The subset we dispatch (`bold`, `italic`, `underline`, `strikeThrough`, `insertUnorderedList`, `insertOrderedList`, `formatBlock`, `createLink`, `unlink`, `insertText`, `styleWithCSS`) is supported in all current browsers. If a future Chromium/Firefox release removes support, the fallback path is a `Range.surroundContents`-based tag wrapper in `wysiwyg_toolbar.rs` — not scoped for Phase 24.

- **TemplateVarButtons signal-sync lag (Plan 24-03 Task 3 comment).** When you click a template-var button (e.g. `{{ payout_amount }}`) it appears immediately in the plain-text body signal AND is mirrored (HTML-escaped) into body_html. But the editor's contenteditable DOM state does NOT visually update until the next keystroke (Dioxus's `value` prop does not push into contenteditable once mounted). This is a known 1-render lag documented in Plan 24-03 §Decisions Made and Plan 24-03 §Task Handoff. Not a bug; documented UX quirk. On the next user keystroke, `oninput` triggers `sync_from_dom` and both signals resync from the actual DOM.

- **TemplateSelector clears body_html on select.** Templates surface plain-text `body` only in this milestone. When a template is selected, the WysiwygEditor's body_html signal is reset to empty. Any HTML the user then adds via the toolbar becomes the new HTML surface. See Plan 24-03 §Decisions Made for the reasoning.

- **The initial-body-with-footer path (mail_page.rs) and initial-body-with-quote path (reply_form.rs) leave body_html empty on mount.** The footer/quote sits in plain body only. HTML surface only appears when the user types in the WysiwygEditor. This is intentional Wave 3 UX — the smoke tester should not expect a formatted footer on a fresh compose.

- **execCommand emitiert manchmal leere `<br>`-Filler in `<li>`.** Chromium fügt `<br>` in leere neue Listeneinträge ein, Firefox nicht (Pitfall 5 in 26-RESEARCH.md). Der Round-Trip-Test in `e2e_tests.rs` und die ammonia-Grenze lassen `<br>` beide durch — daher ist der Filler kein UAT-Fail, auch wenn er im innerHTML sichtbar ist.

- **H1-Button bleibt in der Toolbar (D-01).** Auch wenn EDIT-08 nur H2/H3 fordert, wird H1 NICHT entfernt — die 13-Buttons-Ziffer in Step 1 zählt H1 mit. Ein Round-Trip-Test in `genossi_mail/src/sanitize.rs::sanitize_preserves_headings_h1_h2_h3` deckt H1 zusätzlich automatisiert ab.

## Regression check

Before declaring UAT complete, run the backend + workspace test suites and confirm no regression:

```bash
cargo test -p genossi_mail --lib
cargo test -p genossi_bin --test e2e_tests
cargo build
```

Expected results:
- `genossi_mail --lib`: 255+ tests pass (252 baseline aus Phase 24 + 3 neue aus Plan 26-01).
- `genossi_bin --test e2e_tests`: 309 tests pass (306 baseline aus Phase 24 + 3 neue Phase-26-Additions), davon 1 pre-existing failure `test_mail_preview_repayment_no_entries_does_not_default_to_one` aus Phase 22 — dokumentiert in `.planning/STATE.md`, NICHT eine Phase-26-Regression.
- `cargo build` (workspace): clean, no warnings escalated.

Frontend:
```bash
cd genossi-frontend
cargo check --target wasm32-unknown-unknown
cargo test --bin genossi-frontend
```

Expected:
- `cargo check --target wasm32-unknown-unknown`: clean (pre-existing dead_code warnings on unrelated legacy variants OK).
- `cargo test --bin genossi-frontend`: 286+ pass (284 baseline aus Phase 24 + 2 neue Grep-Gate-Tests aus Plan 26-02).

## Sign-off

Per D-05 („ein Sign-Off-Termin für alle 16 Punkte") deckt der folgende Block alle Verifikations-Steps mit einem Vorstands-Termin ab:

- **Vorstand smoke tester:** _______________
- **Date:** _______________
- **All 16 steps checked:** ☐ Yes  ☐ No — see notes
- **Hard-fail gates (3, 4, 5) passed:** ☐ Yes  ☐ No — MUST fix before v1.5 milestone close
- **New Phase-26 steps (13, 14, 15, 16) passed:** ☐ Yes  ☐ No — MUST fix before v1.5 milestone close

If any hard-fail gate (3, 4, 5) or new Phase-26 step (13, 14, 15, 16) failed, block v1.5-milestone-close (`/gsd-complete-milestone`) and file a bug ticket referencing the specific EDIT-XX requirement plus DevTools evidence. Non-critical failures on Phase-24 steps (11, 12) may be deferred — but Phase-24 was already deferred once; the milestone-audit skill checks that this checklist is signed off before v1.5 archives.

Per D-06: dieser UAT-Smoke ist **Ship-Gate vor `/gsd-complete-milestone`** (v1.5-Milestone-Close), NICHT Merge-Gate innerhalb Phase 26. Die Phase gilt code-fertig, sobald 26-01 (Round-Trip-Tests) und 26-02 (Grep-Gate) grün sind — der Vorstands-Smoke läuft parallel/nachgelagert und muss vor dem Milestone-Archive abgehakt sein.
