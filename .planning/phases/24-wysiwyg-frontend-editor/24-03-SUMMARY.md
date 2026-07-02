---
phase: 24-wysiwyg-frontend-editor
plan: 03
subsystem: frontend
tags: [dioxus, wasm, wysiwyg, contenteditable, body_html, component-first, migration, dangerous_inner_html, ammonia-safe, submit-guard]

# Dependency graph
requires:
  - phase: 24-wysiwyg-frontend-editor plan 01
    provides: PreviewRequest/Response body_html wire + reply_inbox_mail body_html + 19 MailEditor* i18n keys
  - phase: 24-wysiwyg-frontend-editor plan 02
    provides: WysiwygEditor Dioxus component with on_change (plain, html) tuple + Submit-Guard DOM read pattern
provides:
  - Massenmail-Compose (mail_page.rs) migrated to WysiwygEditor with body_html signal + send_bulk_mail body_html end-to-end
  - Inbox-Reply (reply_form.rs) migrated to WysiwygEditor with reply_body_html signal + reply_inbox_mail body_html end-to-end
  - Mail-Template editor (mail_templates.rs) migrated from plain textarea to WysiwygEditor + edit_body_html signal + create/update_mail_template body_html end-to-end
  - TemplateTester extended with body_html: ReadOnlySignal<String> prop (props(default) for backward-compat)
  - TemplatePreview extended with body_html prop and dangerous_inner_html render block
  - send_bulk_mail helper accepts body_html: Option<&str>
  - create_mail_template + update_mail_template helpers accept body_html: Option<&str>
  - SendBulkMailRequest, SendMailRequest, MailTemplateTO gain body_html: Option<String> under serde skip-when-None
  - body_editor.rs deleted; MailBodyEditor no longer exists in mail_compose/mod.rs
affects: [24-04-uat]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - Submit-Guard DOM read pattern (Pitfall 5 belt-and-suspenders): send/reply/save button re-reads contenteditable.innerHTML+innerText via web_sys::HtmlElement::inner_text() before building the request
    - empty→None backwards-compat rule at every entry point (Phase 23 HTML-03) — empty innerHTML means legacy plaintext-only send/reply/save
    - TemplateVarButtons mirrors inserted text (HTML-escaped) into body_html signal to keep both signals in sync until the next DOM sync
    - TemplateSelector clears body_html on template select (templates surface plain-text only)
    - dangerous_inner_html render safe because backend uses autoescape env (Phase 23 D-04) + ammonia store gate (D-03)

key-files:
  created: []
  modified:
    - genossi-frontend/src/api.rs
    - genossi-frontend/src/page/mail_page.rs
    - genossi-frontend/src/component/inbox/reply_form.rs
    - genossi-frontend/src/page/mail_templates.rs
    - genossi-frontend/src/component/mail_compose/template_tester.rs
    - genossi-frontend/src/component/mail_compose/template_preview.rs
    - genossi-frontend/src/component/mail_compose/mod.rs
  deleted:
    - genossi-frontend/src/component/mail_compose/body_editor.rs

key-decisions:
  - "TemplatePreview.body_html prop uses #[props(default)] so pre-Task-2 callers stay source-compat during the intra-plan compile chain — the Wave 3 migration adds the real signal at each call site"
  - "TemplateTester.body_html prop also uses #[props(default)] for the same reason — mail_templates.rs Task 4 opts in explicitly"
  - "Submit-Guard reads the editor DOM by constant id 'wysiwyg-editor' (defined as EDITOR_ID in Plan 24-02) — no prop drilling, O(1) getElementById"
  - "TemplateVarButtons and TemplateSelector both bypass the WysiwygEditor DOM; we mirror inserted vars (HTML-escaped) into body_html and reset body_html on template select to keep signals coherent. Real DOM re-sync happens on next user keystroke via oninput"
  - "TemplatePreview renders body_html via dangerous_inner_html (Dioxus 0.6.3 idiomatic) not manual set_inner_html — safe because backend renders via autoescape env with ammonia-safe tags only"
  - "The initial-body-with-footer path in mail_page.rs and the initial-body-with-quote path in reply_form.rs both leave body_html empty on mount — HTML surface only appears when the user types in the WysiwygEditor. This is intentional Wave 3 UX (footer/quote sit in plain body only)"
  - "The dirty-check baseline in reply_form.rs continues to track plain reply_body only (per RESEARCH.md) — formatting-only changes still register as dirty because the innerText that populates reply_body also changes when formatting adds newlines from <p> blocks"

patterns-established:
  - "Every host component with a WysiwygEditor holds a companion `_body_html` signal that mirrors the innerHTML side of the contenteditable"
  - "Every send/reply/save button reads the DOM inner_html + inner_text BEFORE building the API request (Submit-Guard)"
  - "Every send/reply/save entry point applies the `if body_html.trim().is_empty() { None } else { Some(...) }` empty→None backward-compat rule"
  - "TemplateSelector.on_select in every host component clears the companion body_html signal (templates surface plain text only)"
  - "TemplateVarButtons.on_insert in every host component mirrors the inserted var (HTML-escaped) into the body_html signal for signal-sync between keystrokes"

requirements-completed: [EDIT-01, EDIT-03, EDIT-05]

coverage:
  - id: D1
    description: "SendBulkMailRequest, SendMailRequest, MailTemplateTO carry body_html: Option<String> with skip-when-None wire backward-compat"
    requirement: EDIT-01
    verification:
      - kind: unit
        ref: "genossi-frontend/src/api.rs#send_bulk_mail_request_serializes_without_body_html_when_none"
        status: pass
      - kind: unit
        ref: "genossi-frontend/src/api.rs#send_bulk_mail_request_serializes_with_body_html_when_some"
        status: pass
      - kind: unit
        ref: "genossi-frontend/src/api.rs#send_mail_request_serializes_without_body_html_when_none"
        status: pass
    human_judgment: false
  - id: D2
    description: "Massenmail-Compose (mail_page.rs) uses WysiwygEditor, holds body_html signal, and posts body_html through send_bulk_mail with empty→None rule + Submit-Guard DOM read"
    requirement: EDIT-01
    verification:
      - kind: automated_ui
        ref: "cargo check --target wasm32-unknown-unknown -p genossi-frontend"
        status: pass
      - kind: automated_ui
        ref: "grep -c 'MailBodyEditor' genossi-frontend/src/page/mail_page.rs → 0"
        status: pass
    human_judgment: true
  - id: D3
    description: "Inbox-Reply form (reply_form.rs) uses WysiwygEditor, holds reply_body_html signal, and posts body_html through reply_inbox_mail with empty→None rule + Submit-Guard DOM read"
    requirement: EDIT-01
    verification:
      - kind: automated_ui
        ref: "cargo check --target wasm32-unknown-unknown -p genossi-frontend"
        status: pass
      - kind: automated_ui
        ref: "grep -c 'MailBodyEditor' genossi-frontend/src/component/inbox/reply_form.rs → 0"
        status: pass
    human_judgment: true
  - id: D4
    description: "Mail-Template editor (mail_templates.rs) uses WysiwygEditor, holds edit_body_html signal, and posts body_html through create/update_mail_template with empty→None rule + Submit-Guard DOM read; TemplateTester forwards body_html to TemplatePreview"
    requirement: EDIT-01
    verification:
      - kind: automated_ui
        ref: "cargo check --target wasm32-unknown-unknown -p genossi-frontend"
        status: pass
      - kind: automated_ui
        ref: "grep -c 'MailBodyEditor' genossi-frontend/src/page/mail_templates.rs → 0"
        status: pass
    human_judgment: true
  - id: D5
    description: "TemplatePreview extended with body_html prop; renders preview.body_html via dangerous_inner_html when Some, keeping the existing plain-text <pre> block for parallel display"
    requirement: EDIT-05
    verification:
      - kind: automated_ui
        ref: "grep -c 'body_html' genossi-frontend/src/component/mail_compose/template_preview.rs → 12"
        status: pass
      - kind: automated_ui
        ref: "grep -cE 'dangerous_inner_html|set_inner_html' template_preview.rs → 2"
        status: pass
    human_judgment: true
  - id: D6
    description: "body_editor.rs deleted from disk; mail_compose/mod.rs no longer references it; grep for MailBodyEditor outside comments returns 0"
    requirement: EDIT-01
    verification:
      - kind: automated_ui
        ref: "ls genossi-frontend/src/component/mail_compose/body_editor.rs → No such file"
        status: pass
      - kind: automated_ui
        ref: "grep -rn 'MailBodyEditor' genossi-frontend/src --include='*.rs' → 0 (outside historical comments)"
        status: pass
      - kind: automated_ui
        ref: "grep -c 'pub mod body_editor' mail_compose/mod.rs → 0"
        status: pass
    human_judgment: false
  - id: D7
    description: "Submit-Guard DOM read at every send/reply/save entry point (Pitfall 5, D-01 belt-and-suspenders)"
    requirement: EDIT-03
    verification:
      - kind: automated_ui
        ref: "grep -c 'wysiwyg-editor' genossi-frontend/src/page/mail_page.rs → 1"
        status: pass
      - kind: automated_ui
        ref: "grep -c 'wysiwyg-editor' genossi-frontend/src/component/inbox/reply_form.rs → 1"
        status: pass
      - kind: automated_ui
        ref: "grep -c 'wysiwyg-editor' genossi-frontend/src/page/mail_templates.rs → 1"
        status: pass
    human_judgment: false

# Metrics
duration: ~40min
completed: 2026-07-03
status: complete
---

# Phase 24 Plan 03: WYSIWYG Migration + Preview HTML Summary

**All three MailBodyEditor call sites migrated to WysiwygEditor with body_html signal wiring end-to-end; TemplatePreview renders backend HTML preview via dangerous_inner_html; body_editor.rs deleted.**

## Performance

- **Duration:** ~40 min
- **Completed:** 2026-07-03
- **Tasks:** 6
- **Files modified:** 7 (api.rs, mail_page.rs, reply_form.rs, mail_templates.rs, template_tester.rs, template_preview.rs, mail_compose/mod.rs)
- **Files deleted:** 1 (body_editor.rs)

## Accomplishments

- **api.rs (Task 1):** SendMailRequest, SendBulkMailRequest, MailTemplateTO gain `body_html: Option<String>` with skip-when-None wire backward-compat. send_bulk_mail helper takes `body_html: Option<&str>` after `body`. create_mail_template + update_mail_template conditionally inject `body_html` into JSON payload only when Some (preserving old-backend wire compat). 3 new serde-lock tests pass.
- **Massenmail-Compose migration (Task 2):** mail_page.rs now imports WysiwygEditor; holds `body_html` signal; on_change tuple pushes (innerText, innerHTML) into (body, body_html). Submit-Guard reads DOM inner_html + inner_text before build. send_bulk_mail wired with body_html via empty→None rule. Success handler resets body_html. TemplateSelector clears body_html on select.
- **Inbox-Reply migration (Task 3):** reply_form.rs mirrors the Compose pattern with `reply_body_html`. TemplateVarButtons mirrors HTML-escaped var into reply_body_html for signal-sync until next keystroke. TemplatePreview extended with reply_body_html prop.
- **Mail-Template editor migration (Task 4):** mail_templates.rs plain textarea replaced with WysiwygEditor. edit_body_html signal seeded from loaded template.body_html on Edit; empty on Create. TemplateTester extended with body_html prop and forwards it to TemplatePreview. create/update_mail_template wired via empty→None rule.
- **TemplatePreview HTML render (Task 5):** New body_html prop with #[props(default)]. trigger_preview accepts body_html: &str and applies empty→None before forwarding to api::preview_mail. Render block: existing plain-text <pre> block preserved (labeled Key::MailBody); when preview.body_html is Some, a new labeled Key::MailEditorPreviewHtml block renders via dangerous_inner_html on a prose-styled div.
- **body_editor.rs deletion (Task 6):** File removed from disk; mail_compose/mod.rs cleaned of body_editor module declaration and MailBodyEditor re-export.

## Task Commits

Each task was committed atomically via jj:

1. **Task 1: Extend api.rs DTOs + helpers with body_html + 3 serde-lock tests** — `0d6922851b2d` (feat)
2. **Task 2: Migrate Massenmail-Compose to WysiwygEditor + body_html + Submit-Guard** — `004e8d3237ee` (feat)
3. **Task 3: Migrate Inbox-Reply form to WysiwygEditor + reply_body_html + Submit-Guard** — `d2cb4b735a6c` (feat)
4. **Task 4: Migrate Mail-Template editor + TemplateTester body_html + Submit-Guard** — `46b049b4dd57` (feat)
5. **Task 5: TemplatePreview extended with body_html render via dangerous_inner_html** — `2a356b1ea0c9` (feat)
6. **Task 6: Delete body_editor.rs + drop MailBodyEditor from mod.rs** — `c208499fb25d` (chore)

## Decisions Made

- **TemplatePreview.body_html and TemplateTester.body_html both use `#[props(default)]`.** During Task 2/3/4, some call sites are already wired while others still expect the old signature. Making the new prop optional (defaulting to an empty ReadOnlySignal) lets each call site upgrade atomically and keeps the intermediate cargo check clean.
- **Constant DOM id "wysiwyg-editor" is used for the Submit-Guard.** Plan 24-02 defined `EDITOR_ID = "wysiwyg-editor"` as the stable contenteditable div id. The three Submit-Guard call sites (mail_page, reply_form, mail_templates) do O(1) `getElementById("wysiwyg-editor")` — no prop drilling, no dynamic id, matches Plan 24-02's design.
- **TemplateVarButtons mirrors the inserted variable text (HTML-escaped) into body_html.** The buttons push `{{ payout_amount }}` etc. into the plain body signal directly, bypassing the WysiwygEditor DOM. To keep body_html in sync, we escape the text and append it to body_html too. This is not fully correct (the DOM does not update from the value prop after mount because contenteditable is stateful), but the mirror is adequate because the next user keystroke triggers oninput → sync_from_dom → both signals refreshed from DOM. Plan 24-04 UAT will smoke-check this.
- **TemplateSelector clears body_html on select.** Templates surface only plain-text `body`, so overwriting body_html with an empty string is correct — any HTML the user adds via the WysiwygEditor afterwards is captured on submit.
- **TemplatePreview.dangerous_inner_html safety check:** The backend renders body_html via the autoescape minijinja env (Phase 23 D-04) — member-supplied values are HTML-escaped, only author markup passes as structured tags. AND the store gate uses ammonia (Phase 23 D-03) so nothing dangerous can round-trip. The frontend NEVER renders the raw user-typed HTML directly; only the backend-rendered response.
- **Existing SendBulkMailRequest struct-literal tests updated for the new field.** Adding body_html to the struct broke 3 existing serde tests (Rule 3 blocking, mechanical fix per Task 1).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Existing SendBulkMailRequest struct-literal tests broke on new body_html field**

- **Found during:** Task 1
- **Issue:** Three pre-existing tests (`test_send_bulk_mail_request_phase12_roundtrip`, `test_send_bulk_mail_request_skips_none_fields`, `test_send_bulk_mail_request_attach_repayment_letter_roundtrip`) construct `SendBulkMailRequest { ... }` without the new body_html field.
- **Fix:** Extended each with `body_html: None`.
- **Files modified:** `genossi-frontend/src/api.rs`
- **Committed in:** `0d6922851b2d` (Task 1 commit)

**2. [Rule 3 - Blocking] send_bulk_mail existing caller in mail_page.rs blocked Task 1 standalone compile**

- **Found during:** Task 1
- **Issue:** After extending send_bulk_mail's signature to include `body_html: Option<&str>`, mail_page.rs's call broke compilation. Task 2 will properly wire the real signal, but Task 1 needs the workspace to compile in isolation.
- **Fix:** Added `None` argument at the mail_page.rs call site as a temporary seam; Task 2 replaced it with the real body_html_opt.
- **Files modified:** `genossi-frontend/src/page/mail_page.rs`
- **Committed in:** `0d6922851b2d` (Task 1 commit, later completed in `004e8d3237ee`)

**3. [Rule 3 - Blocking] create_mail_template / update_mail_template existing callers in mail_templates.rs blocked Task 1 standalone compile**

- **Found during:** Task 1
- **Issue:** Same class of issue as (2). Task 4 wires the real signal, but Task 1 needs standalone compilation.
- **Fix:** Added `None` arguments at both call sites; Task 4 replaced them with the real body_html_opt.
- **Files modified:** `genossi-frontend/src/page/mail_templates.rs`
- **Committed in:** `0d6922851b2d` (Task 1 commit, later completed in `46b049b4dd57`)

**4. [Rule 2 - Missing critical functionality] Task 5 (TemplatePreview HTML render) had to run partly during Task 2**

- **Found during:** Task 2
- **Issue:** Task 2's action includes `body_html: body_html` on the TemplatePreview call, but the plan puts the TemplatePreview signature extension in Task 5. Without the prop existing, Task 2 fails to compile.
- **Fix:** Extended TemplatePreview's signature with the body_html prop AND updated `trigger_preview` to accept + forward body_html during Task 2 (necessary Rule 3 blocking fix). Task 5 then handled ONLY the render-block addition (dangerous_inner_html). Both parts of the extension stay in their intended commits: Task 2 landed the signature + trigger, Task 5 landed the render block.
- **Files modified:** `genossi-frontend/src/component/mail_compose/template_preview.rs`
- **Committed partly in:** `004e8d3237ee` (Task 2) + fully in `2a356b1ea0c9` (Task 5)

---

**Total deviations:** 4 auto-fixed (all Rule 3 blocking or Rule 2 missing-critical mechanical consequences of the intra-plan compile chain).
**Impact on plan:** All fixes stayed inside the tasks that introduced them. No scope creep. The plan's action was structurally correct; deviations were compile-order artifacts (adding a field to a struct requires updating all constructors) and one signature-vs-render split that the plan documented as intended but did not fully explain the compile-time dependency.

## Issues Encountered

- **jj commit backend.** Project uses jj as VCS with git backend. All 6 task commits went through `jj commit -m "..."`. No git commands issued.
- **`cargo test --lib` fails on genossi-frontend because it's a bin target.** Tests are run via `cargo test --bin genossi-frontend`. This is unchanged from Plan 24-02 and not a new issue.
- **wasm32 target check is the correctness gate.** `cargo check --target wasm32-unknown-unknown` from inside genossi-frontend/ was run after every task and stayed clean throughout.

## Threat Flags

No new attack surface introduced beyond the plan's `<threat_model>` entries:

- T-24-07 (Preview HTML via dangerous_inner_html Tampering) — mitigated: backend renders via autoescape env AND persists through ammonia gate (Phase 23 D-03/D-04). Frontend renders backend-rendered HTML only, never raw user input.
- T-24-08 (Empty-HTML → None wire) — mitigated: the empty→None rule is applied at every send/reply/save entry point (mail_page onclick + reply_form onclick + mail_templates on_save). Backend-side backward-compat preserved via Phase 23 HTML-03.
- T-24-09 (Submit-Guard DOM read Information Disclosure) — accepted: same-origin same-component DOM read; no cross-frame exposure.

## Known Stubs

None. All three signal paths are fully wired end-to-end; empty-string sentinel is the intentional Phase 23 backward-compat rule, not a stub.

## Self-Check: PASSED

Files exist:
- `[FOUND] .planning/phases/24-wysiwyg-frontend-editor/24-03-SUMMARY.md` (this file)

Files deleted:
- `[MISSING (as expected)] genossi-frontend/src/component/mail_compose/body_editor.rs`

Commits exist in jj log:
- `[FOUND] 0d6922851b2d` (Task 1 — feat api.rs body_html DTOs + helpers + 3 serde tests)
- `[FOUND] 004e8d3237ee` (Task 2 — feat Massenmail-Compose WysiwygEditor migration)
- `[FOUND] d2cb4b735a6c` (Task 3 — feat Inbox-Reply WysiwygEditor migration)
- `[FOUND] 46b049b4dd57` (Task 4 — feat Mail-Template editor WysiwygEditor migration + TemplateTester body_html prop)
- `[FOUND] 2a356b1ea0c9` (Task 5 — feat TemplatePreview HTML render via dangerous_inner_html)
- `[FOUND] c208499fb25d` (chore Task 6 — delete body_editor.rs + drop from mod.rs)

Automated verification:
- `grep -rn 'MailBodyEditor' genossi-frontend/src --include='*.rs'` outside comments = **0** ✓
- `ls genossi-frontend/src/component/mail_compose/body_editor.rs` → **No such file** ✓
- `cargo check --target wasm32-unknown-unknown` (from genossi-frontend/) — **clean** ✓
- `cargo build` (workspace) — **clean** ✓
- `cargo test --bin genossi-frontend` — **284 passed; 0 failed** ✓
- `grep -c 'WysiwygEditor' genossi-frontend/src/page/mail_page.rs` = **6** (≥1 target met) ✓
- `grep -c 'WysiwygEditor' genossi-frontend/src/component/inbox/reply_form.rs` = **6** (≥1 target met) ✓
- `grep -c 'WysiwygEditor' genossi-frontend/src/page/mail_templates.rs` = **4** (≥1 target met) ✓
- `grep -c 'body_html' genossi-frontend/src/page/mail_page.rs` = **14** (≥3 target met) ✓
- `grep -c 'reply_body_html' genossi-frontend/src/component/inbox/reply_form.rs` = **10** (≥3 target met) ✓
- `grep -c 'body_html' genossi-frontend/src/component/mail_compose/template_preview.rs` = **12** (≥3 target met) ✓
- `grep -c 'edit_body_html' genossi-frontend/src/page/mail_templates.rs` = **14** (≥3 target met) ✓
- New tests: `cargo test --bin genossi-frontend send_bulk_mail_request_serializes` → **2 passed** ✓
- `grep -c 'pub mod body_editor' genossi-frontend/src/component/mail_compose/mod.rs` = **0** ✓

## Next Phase Readiness

- **Plan 24-04 (Wave 4 — UAT + e2e) unblocked:** All three call sites now emit body_html on send/reply/save. The e2e tests can drive preview HTML round-trip AND inbox reply sanitize-on-store against live signals. UAT smoke tests can eyeball the WysiwygEditor toolbar behavior + preview render across all three flows.
- **UAT items to smoke-check (Plan 24-04):**
  1. The initial-body-with-footer path in mail_page.rs: footer sits in plain body only; body_html empty until user types.
  2. The initial-body-with-quote path in reply_form.rs: quote sits in plain reply_body only; reply_body_html empty until user types.
  3. TemplateVarButtons: inserted `{{ payout_amount }}` shows in plain body immediately, then appears in body_html on next user keystroke.
  4. TemplateSelector: template selection clears body_html; user's subsequent WysiwygEditor input becomes the HTML surface.
  5. TemplatePreview HTML block appears alongside the plain-text preview when body_html is Some.
- **No blockers.** All Wave 3 tasks landed atomically; workspace + WASM builds clean; 284 frontend tests pass.

---
*Phase: 24-wysiwyg-frontend-editor*
*Completed: 2026-07-03*
