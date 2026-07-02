---
phase: 24-wysiwyg-frontend-editor
plan: 01
subsystem: api
tags: [dioxus, wasm, contenteditable, web-sys, ammonia, i18n, minijinja, autoescape, body_html, wire-seam]

# Dependency graph
requires:
  - phase: 23-html-mail-backend
    provides: render_html_template autoescape env + sanitize_body_html_opt helper + MailJob.body_html column
provides:
  - PreviewRequest.body_html + PreviewResponse.body_html (backend + frontend mirror)
  - preview_mail handler renders body_html via render_html_template (autoescape) when caller supplies it
  - ReplyRequest.body_html + InboxService::reply body_html signature + sanitize-on-store gate at reply EP
  - api::preview_mail helper + api::reply_inbox_mail helper thread body_html end-to-end
  - genossi-frontend Cargo.toml web-sys features ClipboardEvent + DataTransfer (unblocks Plan 24-02 onpaste)
  - 19 Key::MailEditor* i18n variants with de + en translations
affects: [24-02-wysiwyg-component, 24-03-migration-preview]

# Tech tracking
tech-stack:
  added: [web-sys ClipboardEvent feature, web-sys DataTransfer feature]
  patterns: [wire-seam-first Wave 1, backend body_html render via autoescape env, sanitize-on-store at inbox reply EP, i18n keys land upfront in both locales]

key-files:
  created: []
  modified:
    - genossi_mail/src/rest.rs
    - genossi_mail/src/inbox_rest.rs
    - genossi_mail/src/inbox.rs
    - genossi-frontend/src/api.rs
    - genossi-frontend/src/component/mail_compose/template_preview.rs
    - genossi-frontend/src/component/inbox/reply_form.rs
    - genossi-frontend/Cargo.toml
    - genossi-frontend/src/i18n/mod.rs
    - genossi-frontend/src/i18n/de.rs
    - genossi-frontend/src/i18n/en.rs

key-decisions:
  - Wire-seam-first Wave 1: land backend body_html echo + inbox reply body_html + frontend api mirror + web-sys features + i18n keys atomically so Plan 24-02 (component) can compile without cross-cutting edits
  - preview_mail treats HTML render errors as recoverable — push into errors vec with 'HTML:' prefix, keep the plaintext preview flowing (parallels how existing Body render errors are handled)
  - InboxService::reply body_html is sanitized at the store boundary via sanitize_body_html_opt (D-03 EP wire pattern from Phase 23 Plan 04), Arc<str>-wrapped for MailJob storage
  - Wave 1 callers pass None for body_html in preview_mail/reply_inbox_mail — Plan 24-03 will wire real Signal<String> body_html signals
  - Both locales (de.rs + en.rs) receive all 19 arms in the same commit — no locale drift

patterns-established:
  - "Preview HTML render errors surface via PreviewResponse.errors with 'HTML:' prefix (analog to existing 'Subject:'/'Body:' error tags)"
  - "InboxService::reply body_html follows the Phase 23 D-03 sanitize-on-store gate at the entry point — worker never re-sanitizes"
  - "reply_inbox_mail JSON payload conditionally injects body_html only when Some, preserving wire backward-compat with pre-Phase-24 backends"
  - "PreviewRequest/PreviewResponse.body_html use #[serde(default, skip_serializing_if = \"Option::is_none\")] pattern established across Phase 23 Plan 04 DTOs"
  - "web-sys feature additions are grouped by trigger phase in Cargo.toml with a comment line per feature explaining the consumer plan"

requirements-completed: [EDIT-01, EDIT-04, EDIT-05]

coverage:
  - id: D1
    description: "PreviewRequest and PreviewResponse both carry body_html: Option<String> with wire backward-compat (skip_serializing_if = Option::is_none)"
    requirement: EDIT-05
    verification:
      - kind: unit
        ref: "genossi_mail/src/rest.rs#preview_response_serializes_without_body_html_when_none"
        status: pass
      - kind: unit
        ref: "genossi_mail/src/rest.rs#preview_response_serializes_with_body_html_when_some"
        status: pass
    human_judgment: false
  - id: D2
    description: "preview_mail handler renders body_html via render_html_template (autoescape env) when the request supplies body_html, and preserves errors-vec pattern"
    requirement: EDIT-05
    verification:
      - kind: unit
        ref: "cargo check -p genossi_mail"
        status: pass
    human_judgment: false
  - id: D3
    description: "InboxService::reply body_html sanitize gate — ammonia strips <script>, safe <p> markup survives"
    requirement: EDIT-01
    verification:
      - kind: unit
        ref: "genossi_mail/src/inbox.rs#reply_sanitizes_body_html_on_store"
        status: pass
    human_judgment: false
  - id: D4
    description: "Existing reply_* tests continue to pass with the mechanical None positional argument"
    requirement: EDIT-01
    verification:
      - kind: unit
        ref: "cargo test -p genossi_mail --lib reply_"
        status: pass
    human_judgment: false
  - id: D5
    description: "Frontend api::PreviewRequest, PreviewResponse, preview_mail helper, reply_inbox_mail helper thread body_html end-to-end"
    requirement: EDIT-05
    verification:
      - kind: unit
        ref: "genossi-frontend/src/api.rs#preview_request_serializes_without_body_html_when_none"
        status: pass
      - kind: unit
        ref: "genossi-frontend/src/api.rs#preview_request_serializes_with_body_html_when_some"
        status: pass
    human_judgment: false
  - id: D6
    description: "web-sys features ClipboardEvent + DataTransfer available in genossi-frontend/Cargo.toml (unblocks Plan 24-02 onpaste handler)"
    requirement: EDIT-04
    verification:
      - kind: automated_ui
        ref: "cargo check --target wasm32-unknown-unknown -p genossi-frontend"
        status: pass
    human_judgment: false
  - id: D7
    description: "19 Key::MailEditor* variants defined in mod.rs and translated in both de.rs and en.rs"
    requirement: EDIT-01
    verification:
      - kind: unit
        ref: "grep -c 'MailEditor' genossi-frontend/src/i18n/{mod,de,en}.rs = 19,19,19"
        status: pass
    human_judgment: false

# Metrics
duration: 32min
completed: 2026-07-03
status: complete
---

# Phase 24 Plan 01: WYSIWYG Foundation Seams Summary

**Backend body_html echo on preview_mail + inbox reply sanitize gate + frontend api mirror + web-sys ClipboardEvent/DataTransfer + 19 MailEditor* i18n keys — Wave 1 seam landed atomically so Plan 24-02's component can compile in isolation.**

## Performance

- **Duration:** ~32 min
- **Started:** 2026-07-02 (context load)
- **Completed:** 2026-07-03
- **Tasks:** 5
- **Files modified:** 10

## Accomplishments
- PreviewRequest/PreviewResponse both carry `body_html: Option<String>` with wire backward-compat via `#[serde(default, skip_serializing_if = "Option::is_none")]`.
- preview_mail handler renders `body_html` via `render_html_template` (autoescape env — member-supplied values HTML-escaped) when the request supplies it; HTML render errors are captured in the existing errors vec with `HTML:` prefix.
- ReplyRequest gains `body_html: Option<String>`; `InboxService::reply` trait accepts it as a new positional argument; the impl calls `sanitize_body_html_opt` at the store boundary (Phase 23 D-03 EP wire) and Arc-wraps for MailJob storage.
- Frontend `api::PreviewRequest`, `api::PreviewResponse`, `preview_mail` helper, and `reply_inbox_mail` helper all thread `body_html` end-to-end; the JSON payload for reply conditionally injects the key only when Some.
- `genossi-frontend/Cargo.toml` gains web-sys features `ClipboardEvent` and `DataTransfer` — direct precondition for the WYSIWYG editor's onpaste handler in Plan 24-02.
- 19 new `Key::MailEditor*` variants land in `i18n/mod.rs` with matching arms in `i18n/de.rs` and `i18n/en.rs` — Plan 24-02 can reference them without touching i18n.

## Task Commits

Each task was committed atomically via jj:

1. **Task 1: Extend PreviewRequest/Response with body_html + render HTML in preview_mail (backend)** — `1a2fb16f40a2` (feat)
2. **Task 2: Add body_html to ReplyRequest + InboxService::reply signature + sanitize-on-store (backend)** — `7436087763e4` (feat)
3. **Task 3: Mirror body_html on frontend api::PreviewRequest/PreviewResponse + reply_inbox_mail (frontend api layer)** — `97721210f8b1` (feat)
4. **Task 4: Add web-sys ClipboardEvent + DataTransfer features** — `7baa8c7167e9` (build)
5. **Task 5: Add 19 Key::MailEditor* variants + translations in de.rs and en.rs** — `f1dbc3bac534` (feat)

## Files Created/Modified

### Backend
- `genossi_mail/src/rest.rs` — PreviewRequest.body_html, PreviewResponse.body_html, preview_mail handler renders HTML through autoescape env, 2 new serde-lock tests, 3 existing PreviewResponse constructors extended with body_html: None
- `genossi_mail/src/inbox_rest.rs` — ReplyRequest.body_html field, reply_inbox handler threads it into svc.reply(...)
- `genossi_mail/src/inbox.rs` — InboxService::reply trait signature accepts body_html: Option<String>, impl sanitizes via sanitize_body_html_opt + Arc::from, 5 existing tests updated with `None` positional arg, new reply_sanitizes_body_html_on_store test

### Frontend
- `genossi-frontend/src/api.rs` — PreviewRequest.body_html, PreviewResponse.body_html, preview_mail helper accepts body_html: Option<&str>, reply_inbox_mail helper accepts body_html: Option<&str> with conditional JSON injection, 2 new serde-lock tests
- `genossi-frontend/src/component/mail_compose/template_preview.rs` — caller passes None; error-branch PreviewResponse literal gets body_html: None
- `genossi-frontend/src/component/inbox/reply_form.rs` — caller passes None
- `genossi-frontend/Cargo.toml` — +ClipboardEvent, +DataTransfer web-sys features
- `genossi-frontend/src/i18n/mod.rs` — +19 Key::MailEditor* variants under a "Phase 24: WYSIWYG editor labels" comment
- `genossi-frontend/src/i18n/de.rs` — +19 German translations (Fett, Kursiv, Unterstrichen, Durchgestrichen, Aufzählung, Nummerierte Liste, Überschrift 1/2/3, Absatz, Zitat, Link, Link entfernen, Link einfügen, URL, Anzeige-Text (optional), Einfügen, Abbrechen, HTML-Vorschau)
- `genossi-frontend/src/i18n/en.rs` — +19 English translations (Bold, Italic, Underline, Strikethrough, Bulleted list, Numbered list, Heading 1/2/3, Paragraph, Blockquote, Link, Remove link, Insert link, URL, Display text (optional), Insert, Cancel, HTML preview)

## Decisions Made

- **HTML render errors are soft failures on preview.** When `render_html_template` fails on the caller-supplied `body_html`, the handler pushes the message into `PreviewResponse.errors` with an `HTML:` prefix and leaves `rendered_body_html = None`. This mirrors how existing `Subject:` and `Body:` render errors are handled so the frontend continues to see the plaintext preview even when the author's HTML template is broken. (Not an architectural change — same errors-vec pattern.)
- **`Arc::from` on the sanitized reply body_html.** `sanitize_body_html_opt` returns `Option<String>`; `MailJob.body_html` is `Option<Arc<str>>`. The impl wraps `Arc::from` at the assignment point rather than changing the sanitize helper signature, because the helper is shared with paths that use `String` (send_test_mail_with_body). Minimal-diff, no ripple through Phase 23's helpers.
- **Wave 1 callers pass `None`.** `template_preview.rs` and `reply_form.rs` pass `None` for the new `body_html` positional param. Wiring real `Signal<String>` body_html is explicitly Plan 24-03's job — this plan is the seam, not the migration.
- **Both locales in one commit.** Rather than staging German and English separately, all 19 keys and translations land together to prevent locale drift (the `Locale::En` fallback bug memory from earlier phases).

## Deviations from Plan

**1. [Rule 3 - Blocking] `MailJob.body_html` expects `Arc<str>`, not `String`**
- **Found during:** Task 2 (reply impl sanitize wire)
- **Issue:** The plan action wrote `body_html: crate::service::sanitize_body_html_opt(body_html.as_deref())` — that yields `Option<String>`, but `MailJob.body_html: Option<Arc<str>>`. Type mismatch.
- **Fix:** Chained `.map(Arc::from)` on the helper's return so the sanitized string is Arc-wrapped at the assignment site. `sanitize_body_html_opt` signature stays unchanged (shared with paths that keep `String`).
- **Files modified:** `genossi_mail/src/inbox.rs`
- **Verification:** `cargo test -p genossi_mail --lib reply_` — all 6 reply tests pass, including the new `reply_sanitizes_body_html_on_store`.
- **Committed in:** `7436087763e4` (Task 2 commit)

**2. [Rule 3 - Blocking] `PreviewResponse` constructor extension broke 3 existing tests**
- **Found during:** Task 1 (added body_html field to PreviewResponse struct)
- **Issue:** Three existing tests (`test_preview_response_serializes_used_dummy_repayment_when_true`, `test_preview_response_skips_used_dummy_repayment_when_false`, `test_preview_response_roundtrip_with_dummy_flag`) constructed `PreviewResponse { ... }` without the new field.
- **Fix:** Extended each with `body_html: None`.
- **Files modified:** `genossi_mail/src/rest.rs`
- **Verification:** `cargo test -p genossi_mail --lib preview_response` — all 3 pre-existing + 2 new tests pass.
- **Committed in:** `1a2fb16f40a2` (Task 1 commit)

**3. [Rule 3 - Blocking] Frontend `PreviewResponse` constructor in template_preview.rs error branch**
- **Found during:** Task 3 (added body_html field to api::PreviewResponse struct)
- **Issue:** `template_preview.rs`'s error-branch literal `PreviewResponse { ... }` (line 33) was missing the new body_html field, breaking WASM compilation.
- **Fix:** Added `body_html: None` in the error-branch literal.
- **Files modified:** `genossi-frontend/src/component/mail_compose/template_preview.rs`
- **Verification:** `cargo check --target wasm32-unknown-unknown` clean.
- **Committed in:** `97721210f8b1` (Task 3 commit)

---

**Total deviations:** 3 auto-fixed (3 blocking type/constructor mismatches — all were mechanical consequences of adding a field to an existing DTO).
**Impact on plan:** All fixes stayed inside the tasks that introduced them. No scope creep. The plan's action was structurally correct; the deviations were low-level Rust-type propagations that the plan didn't spell out because the DTOs' concrete types (Arc<str>, existing constructor locations) live outside the plan's read_first line ranges.

## Issues Encountered

- **jj commit backend.** Project uses jj as VCS with git backend. Commits went through `jj commit -m "..."`. No git commands issued.
- **web-sys features must be validated via wasm32-unknown-unknown target.** genossi-frontend is excluded from the workspace root Cargo.toml (`resolver = "2"`), so verification required a per-crate `cargo check --target wasm32-unknown-unknown` from inside the `genossi-frontend/` directory.

## Threat Flags

No new attack surface introduced beyond the plan's `<threat_model>` entries:

- T-24-01 (preview_mail Tampering) — mitigated by rendering via autoescape env, no persistence.
- T-24-02 (InboxService::reply body_html Tampering) — mitigated by `sanitize_body_html_opt` at the store boundary (verified by new `reply_sanitizes_body_html_on_store` test — ammonia strips `<script>`).
- T-24-03 (preview_mail Information Disclosure) — accepted: preview is admin-authenticated, echoing rendered HTML back to the caller exposes no new data.

## Self-Check: PASSED

- `.planning/phases/24-wysiwyg-frontend-editor/24-01-SUMMARY.md` — present (this file)
- Task 1 commit `1a2fb16f40a2` — present in jj log
- Task 2 commit `7436087763e4` — present in jj log
- Task 3 commit `97721210f8b1` — present in jj log
- Task 4 commit `7baa8c7167e9` — present in jj log
- Task 5 commit `f1dbc3bac534` — present in jj log
- `grep -c 'pub body_html' genossi_mail/src/rest.rs` = 6 (≥6 target met)
- `grep -c 'pub body_html' genossi_mail/src/inbox_rest.rs` = 1 (target met)
- `grep -c 'MailEditor' genossi-frontend/src/i18n/{mod,de,en}.rs` = 19,19,19 (target met)
- `grep -c 'ClipboardEvent' Cargo.toml` = 1; `DataTransfer` = 1 (target met)
- `cargo test -p genossi_mail --lib` — 252 tests passed
- `cargo check --target wasm32-unknown-unknown` on genossi-frontend — clean
- `cargo test preview_request_serializes` on genossi-frontend — 2 tests passed

## Next Phase Readiness

- **Plan 24-02 (WYSIWYG component) unblocked:** web-sys features + all 19 i18n keys present in mod.rs, de.rs, en.rs. Component can compile against these without touching i18n or Cargo.toml.
- **Plan 24-03 (migration + preview) unblocked:** frontend `api::preview_mail` and `api::reply_inbox_mail` accept `body_html: Option<&str>` — wiring real `Signal<String>` body_html signals in `mail_page.rs`, `reply_form.rs`, `template_tester.rs` is a straight-line edit; both endpoints render/persist body_html end-to-end.
- **No blockers.** All backend seams verified with unit tests; ammonia sanitize gate proven via `reply_sanitizes_body_html_on_store`.

---
*Phase: 24-wysiwyg-frontend-editor*
*Completed: 2026-07-03*
