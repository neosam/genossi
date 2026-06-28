# Phase 21: Reply-Komfort — Antwort im vollflächigen Modal - Pattern Map

**Mapped:** 2026-06-27
**Files analyzed:** 5 (all MODIFIED — frontend-only relocation phase)
**Analogs found:** 5 / 5

> Relocation phase. No new files created. Every modified file has a strong in-repo
> analog. The locked decisions (D-01..D-10) point directly at the established
> `membership_adjust_modal.rs` pattern, the `Modal` wrapper, and the existing
> pure-fn-plus-tests structure already inside `reply_form.rs`.

## File Classification

| Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---------------|------|-----------|----------------|---------------|
| `genossi-frontend/src/component/inbox/reply_form.rs` | component (form) | request-response (existing send) + event-driven (close/dirty) | `genossi-frontend/src/component/membership_adjust_modal.rs` (header + `on_close` + pure-fn tests) | exact (same project modal pattern) |
| `genossi-frontend/src/page/inbox_page.rs` | page (composition) | event-driven (toggle/open/close) | `genossi-frontend/src/page/assembly_details.rs:304-318` (`Modal { Form { on_close } }`) | exact |
| `genossi-frontend/src/i18n/mod.rs` | config (Key enum) | n/a | Phase 18 block at `mod.rs:727-731` (`MembershipAdjust*` keys) | exact |
| `genossi-frontend/src/i18n/de.rs` | config (translations) | n/a | `de.rs:650` (`MembershipAdjustModalTitle`) | exact |
| `genossi-frontend/src/i18n/en.rs` | config (translations) | n/a | `en.rs:643` (`MembershipAdjustModalTitle`) | exact |

## Pattern Assignments

### `genossi-frontend/src/component/inbox/reply_form.rs` (component, gains header + on_close + dirty-check)

**Analog:** `genossi-frontend/src/component/membership_adjust_modal.rs`

This file is MODIFIED in three ways: (1) add `on_close: EventHandler<()>` prop,
(2) render its own modal header as first child, (3) add a pure dirty-check helper
+ unit tests. Form CONTENT (subject, template selector, var buttons, attachment
picker, preview, body editor, send button) stays UNCHANGED (D-09).

**Prop signature pattern** — add `on_close` to the existing `#[component]` signature
(`reply_form.rs:13-23`). Mirror the `EventHandler<()>` style from
`membership_adjust_modal.rs:123`:
```rust
on_close: EventHandler<()>,
```

**Header pattern to ADD as first child of the form** — copy verbatim from
`membership_adjust_modal.rs:142-153`. This is THE source of truth (D-01, D-03,
UI-SPEC §Component & Interaction Contract):
```rust
// ── Modal header (always visible) ──
div { class: "flex items-center justify-between border-b border-gray-200 pb-3",
    h2 { class: "text-xl font-semibold text-gray-900",
        "{header_title}"
    }
    button {
        r#type: "button",
        class: "text-gray-500 hover:text-gray-700 px-2 py-1",
        onclick: move |_| on_close.call(()),   // → becomes attempt_close (see below)
        "\u{2715}"
    }
}
```
Note: the existing form root is `div { class: "border-t pt-3 mt-3 space-y-3", ... }`
(`reply_form.rs:100`). Inside a `Modal` the `border-t pt-3 mt-3` becomes redundant
(D-08 says roominess comes from `Modal`'s `p-8`); planner decides whether to keep
or wrap. The header should sit ABOVE the existing `"An: {from_address}"` line.

**i18n usage pattern** — the form does NOT currently call i18n. Add `use_i18n` like
`membership_adjust_modal.rs:126` + `:139`:
```rust
let i18n = use_i18n();
let header_title = i18n.t(Key::InboxReplyModalTitle).to_string();
let cancel_label = i18n.t(Key::InboxReplyCancel).to_string();
```
Imports needed: `use crate::i18n::{use_i18n, Key};` (matching `membership_adjust_modal.rs:24`).

**"Abbrechen" footer button pattern** — neutral (NOT accent) styling, next to the
existing send button. Mirror the neutral cancel button in
`membership_adjust_modal.rs:388-394`:
```rust
button {
    r#type: "button",
    class: "px-4 py-2 text-gray-700 hover:bg-gray-100 rounded",
    onclick: move |_| { /* attempt_close */ },
    "{cancel_label}"
}
```
The existing send button (`reply_form.rs:154-174`, `bg-blue-500`) is the ONLY accent
control — keep it unchanged (UI-SPEC §Color).

**Dirty-check pure helper + baseline pattern (D-04, D-05, D-06)** — the analog for
"pure fn alongside the component + `#[cfg(test)] mod tests`" is the EXISTING
`build_original_quote` / `compose_initial_body` (`reply_form.rs:181-209`) and their
8 tests (`reply_form.rs:211-265`). Add a new pure fn in the same module:
```rust
/// Pure dirty-check: the draft differs from the post-footer-load baseline.
fn is_draft_dirty(
    subject: &str, body: &str,
    baseline_subject: &str, baseline_body: &str,
) -> bool {
    subject != baseline_subject || body != baseline_body
}
```
**Baseline-trap wiring (D-05, CRITICAL):** `reply_body` starts as just the quote
(`reply_form.rs:28-37`), then the on-mount `use_effect` (`reply_form.rs:59-71`)
calls `compose_initial_body` and OVERWRITES the body. The baseline snapshot MUST be
captured INSIDE that same `use_effect`, AFTER `reply_body.set(initial)`. Add
`baseline_subject` / `baseline_body` signals (like the existing `cached_footer` /
`cached_quote` signals at `reply_form.rs:38-43`) and set them in the effect:
```rust
let mut baseline_subject = use_signal(String::new);
let mut baseline_body = use_signal(String::new);
// ...inside the footer use_effect, after reply_body.set(initial):
baseline_subject.set(reply_subject.read().clone());
baseline_body.set(initial);   // or reply_body.read().clone()
```
The `TemplateSelector` `on_select` (`reply_form.rs:108-123`) ALSO replaces the body —
planner must decide whether selecting a template re-baselines or counts as "dirty"
(treat as dirty is the safe default; selecting a template is an intentional edit).

**`attempt_close` closure (Claude's Discretion, D-07)** — bundle the X-button and
"Abbrechen" handlers. Native confirm is acceptable; the project already wires
`web_sys` (CONTEXT D-07). Pattern: read signals, call pure `is_draft_dirty`, branch:
```rust
let attempt_close = move |_| {
    let dirty = is_draft_dirty(
        &reply_subject.read(), &reply_body.read(),
        &baseline_subject.read(), &baseline_body.read(),
    );
    if !dirty {
        on_close.call(());
    } else if web_sys::window()
        .and_then(|w| w.confirm_with_message(&confirm_msg).ok())
        .unwrap_or(false)
    {
        on_close.call(());
    }
};
```
(`web_sys` is already a frontend dependency — see project tech stack; confirm exact
import path during planning.)

**Test pattern to ADD** — mirror the existing `mod tests` (`reply_form.rs:211-265`)
and the `MembershipAdjust` pure-fn test block style. Cover: unchanged draft → not
dirty; subject changed → dirty; body changed → dirty; both equal incl. the
quote-vs-baseline trap (D-05). CLAUDE.md (global) requires tests for the change.

**Button-reload guard:** ALL new buttons use `r#type: "button"` (memory
`feedback_dioxus_button_type`; UI-SPEC §Close affordances). The analog header/footer
buttons already do this.

---

### `genossi-frontend/src/page/inbox_page.rs` (page, wrap inline form in Modal)

**Analog:** `genossi-frontend/src/page/assembly_details.rs:304-318`

**Modal-wraps-form pattern** — the established way a page composes a modal child
that renders its own header + `on_close`:
```rust
// assembly_details.rs:304-318
if *show_create.read() {
    Modal {
        CreateTokenForm {
            assembly_id: assembly_id,
            on_close: move |_| show_create.set(false),
            on_created: move |resp| { show_create.set(false); /* ... */ },
            on_error: on_error,
        }
    }
}
```

**Apply to inbox_page.rs:** the existing inline block `if *show_reply.read() { ...
InboxReplyForm {...} }` (`inbox_page.rs:451-487`) is wrapped in `Modal { ... }`, and
the new `on_close` prop is added to the `InboxReplyForm` invocation:
```rust
on_close: move |_| show_reply.set(false),
```
Keep the EXISTING `on_sent` (`inbox_page.rs:473-480`) and `on_error`
(`inbox_page.rs:481-483`) callbacks UNCHANGED — `on_sent` already does
`show_reply.set(false)` + `reload()` + `load_detail(...)` (D-10, CONTEXT
Integration Points). `Modal` is already imported and used elsewhere in the codebase;
confirm/add the import in this page.

**Toggle-button simplification (D-10)** — the toggle at `inbox_page.rs:426-433`
currently flips `show_reply` and swaps label "Antworten" / "Antwort abbrechen".
Simplify to only open the modal:
```rust
// inbox_page.rs:426-433 → becomes
onclick: move |_| show_reply.set(true),
// label: always "Antworten" (closing now happens inside the modal)
```
`show_reply` signal declared at `inbox_page.rs:52`; reset on mail open at
`inbox_page.rs:119` — both stay. `i18n` is already in scope (`inbox_page.rs:43`).

---

### `genossi-frontend/src/i18n/mod.rs` + `de.rs` + `en.rs` (config, 3 new keys)

**Analog:** Phase 18 `MembershipAdjust*` block — `mod.rs:727-731`, `de.rs:650`,
`en.rs:643`, plus the distinct-translation test at `mod.rs:1002-1014`.

**Key enum pattern** (`mod.rs`) — add a Phase 21 block to the `Key` enum (place near
the end of the enum, comment-grouped like Phase 18 at `mod.rs:727`):
```rust
// ─── Phase 21 ─── InboxReplyForm Modal ────
/// Modal-Header-Titel der Reply-Maske.
InboxReplyModalTitle,
/// "Abbrechen"-Button im Reply-Modal.
InboxReplyCancel,
/// Confirm-Text bei geändertem Entwurf.
InboxReplyDiscardConfirm,
```

**Translation match-arm pattern** (`de.rs:650`, `en.rs:643`) — add arms in BOTH
files (only `En` + `De` exist — `genossi-frontend/CLAUDE.md`):
```rust
// de.rs
Key::InboxReplyModalTitle => "Antwort verfassen".into(),
Key::InboxReplyCancel => "Abbrechen".into(),
Key::InboxReplyDiscardConfirm =>
    "Der Entwurf wurde geändert. Änderungen verwerfen und schließen?".into(),
// en.rs
Key::InboxReplyModalTitle => "Compose reply".into(),
Key::InboxReplyCancel => "Cancel".into(),
Key::InboxReplyDiscardConfirm =>
    "The draft has changed. Discard changes and close?".into(),
```
Copy text is locked by UI-SPEC §Copywriting Contract.

**Distinct-translation test** — if the Phase-18 test at `mod.rs:1002-1014` enforces
distinct De/En per key, add the 3 new keys to an equivalent list (or note that
"Abbrechen"/"Cancel" etc. are distinct). Verify the test convention during planning.

## Shared Patterns

### Modal header + close (X-icon)
**Source:** `genossi-frontend/src/component/membership_adjust_modal.rs:142-153`
**Apply to:** `reply_form.rs` (the new self-rendered header)
- container `flex items-center justify-between border-b border-gray-200 pb-3`
- title `h2.text-xl.font-semibold.text-gray-900`
- X-button neutral gray, `r#type: "button"`, glyph `"\u{2715}"`, `onclick → close`

### Modal wrapper (verbatim, do not modify)
**Source:** `genossi-frontend/src/component/modal.rs` (`max-w-3/4 max-h-[90vh] p-8 overflow-y-auto`)
**Apply to:** `inbox_page.rs` (wrap `InboxReplyForm`)

### Page composes `Modal { Form { on_close } }`
**Source:** `genossi-frontend/src/page/assembly_details.rs:304-318`
**Apply to:** `inbox_page.rs:451-487`

### Pure helper + `#[cfg(test)] mod tests` in component file
**Source:** `genossi-frontend/src/component/inbox/reply_form.rs:181-265` (existing
`build_original_quote`/`compose_initial_body` + 8 tests)
**Apply to:** the new `is_draft_dirty` helper

### i18n key + dual-locale translation
**Source:** `mod.rs:727-731` / `de.rs:650` / `en.rs:643`
**Apply to:** all 3 new keys, BOTH `de.rs` and `en.rs`

### Dioxus button-reload guard
**Source:** project memory `feedback_dioxus_button_type`; every button in
`membership_adjust_modal.rs` uses `r#type: "button"`
**Apply to:** every new/modified button in `reply_form.rs`

## No Analog Found

None. Every modified file maps to an existing in-repo pattern.

## Metadata

**Analog search scope:** `genossi-frontend/src/component/`, `genossi-frontend/src/page/`, `genossi-frontend/src/i18n/`
**Files scanned:** reply_form.rs, membership_adjust_modal.rs, modal.rs, inbox_page.rs, assembly_details.rs, i18n/{mod,de,en}.rs
**Pattern extraction date:** 2026-06-27
