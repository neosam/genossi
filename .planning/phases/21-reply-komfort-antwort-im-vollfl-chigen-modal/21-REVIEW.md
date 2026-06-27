---
phase: 21-reply-komfort-antwort-im-vollfl-chigen-modal
reviewed: 2026-06-27T00:00:00Z
depth: standard
files_reviewed: 5
files_reviewed_list:
  - genossi-frontend/src/component/inbox/reply_form.rs
  - genossi-frontend/src/page/inbox_page.rs
  - genossi-frontend/src/i18n/mod.rs
  - genossi-frontend/src/i18n/de.rs
  - genossi-frontend/src/i18n/en.rs
findings:
  critical: 1
  warning: 2
  info: 2
  total: 5
status: issues_found
---

# Phase 21: Code Review Report

**Reviewed:** 2026-06-27
**Depth:** standard
**Files Reviewed:** 5
**Status:** issues_found

## Summary

This phase moves the inbox reply form into a full-screen `Modal` with a header (title +
X-close), an «Abbrechen» button, and an unsaved-draft dirty-check that prompts before
discarding. The five known project-specific Dioxus pitfalls were each checked:

- **No render-path signal mutation** — all `set`/`write` calls live inside `use_effect`,
  `spawn`, or onclick closures. No infinite re-render loop. (PASS)
- **Button reload-bug** — both *new* buttons (X-icon line 123, «Abbrechen» line 222) carry
  `r#type: "button"`. (PASS, with one consistency nit — IN-01)
- **Closure/Signal move semantics** — `on_close` is a `Copy` `EventHandler`, all captured
  signals are `Copy`, and `confirm_msg` is cloned per closure. No double-move. (PASS)
- **i18n completeness** — the three Phase-21 keys exist and are distinct in both `de.rs`
  (703-705) and `en.rs` (696-698); the match is exhaustive; the Phase-21 distinctness test
  covers them. No wrong-locale copy-paste. (PASS)
- **D-05 async dirty-baseline** — the baseline *snapshot* is correctly placed at the end of
  the footer `use_effect` (lines 85-86). However the *body composition* half of D-05 is
  defective: the same effect unconditionally overwrites `reply_body`, discarding any text the
  user typed during the async window. (FAIL — see CR-01)

The `Modal` component (`component/modal.rs`) renders children only — it has no
backdrop-click / Escape close — so the dirty-check cannot be bypassed through the modal
shell. That is correct for draft safety, though it also means there is intentionally no
Escape/overlay dismissal.

## Critical Issues

### CR-01: Footer-load effect overwrites user-typed body — data loss + missed dirty detection

**File:** `genossi-frontend/src/component/inbox/reply_form.rs:71-88`

**Issue:** The on-mount footer effect awaits `api::get_mail_footer` (a network call) and then
unconditionally replaces the body:

```rust
if let Ok(footer) = api::get_mail_footer(&config).await {
    ...
    let initial = compose_initial_body(&footer, &quote);
    if !initial.is_empty() {
        reply_body.set(initial);          // <-- clobbers whatever the user typed
    }
}
baseline_subject.set(reply_subject.read().clone());
baseline_body.set(reply_body.read().clone());
```

The body editor (`MailBodyEditor`) is rendered and editable immediately on mount, pre-filled
with the quote block. A user who starts replying (top-posting into the pre-filled body)
before `get_mail_footer` resolves will have their text silently destroyed when the footer
arrives — `reply_body.set(initial)` does not check whether the body was modified. Worse, the
baseline is then snapshotted from the *overwritten* body, so the dirty-check (CR-flow) never
registers that the user's content existed: closing the modal will report "not dirty" and the
user gets no confirmation that anything was lost. This is the unaddressed half of the D-05
concern — the snapshot timing was fixed, but the overwrite was not guarded.

The window is narrow (footer load is usually fast and is empty when no footer is configured),
but on a slow connection or a configured-footer install it is a realistic data-loss path for
exactly the primary use case (typing a reply).

**Fix:** Only compose the initial body if the user has not touched it yet — capture the
pre-footer body and bail out of the overwrite if it changed, e.g.:

```rust
let pre_footer = reply_body.read().clone();
if let Ok(footer) = api::get_mail_footer(&config).await {
    cached_footer.set(footer.clone());
    let quote = cached_quote.read().clone();
    let initial = compose_initial_body(&footer, &quote);
    // Only seed the body if the user hasn't started typing in the load window.
    if !initial.is_empty() && reply_body.read().clone() == pre_footer {
        reply_body.set(initial);
    }
}
baseline_subject.set(reply_subject.read().clone());
baseline_body.set(reply_body.read().clone());
```

(Equivalently: load the footer *before* the body editor accepts focus, or compose the body
synchronously from a cached footer.)

## Warnings

### WR-01: Mixed localization inside the modal — send button and "An:" label stay hardcoded German

**File:** `genossi-frontend/src/component/inbox/reply_form.rs:147, 219`

**Issue:** The phase localized the modal title and cancel button (`InboxReplyModalTitle`,
`InboxReplyCancel`) but left the *primary* action button and the recipient label hardcoded:

- line 147: `"An: {from_address}"`
- line 219: `if *sending.read() { "Sende..." } else { "Antwort senden" }`

An EN-locale user now sees a mixed-language modal: an English header "Compose reply" and
"Cancel" next to a German "Antwort senden" / "Sende..." / "An:". Because the surrounding
header was just localized, this inconsistency is newly visible and is squarely within this
phase's scope (the send button was re-indented in this same diff).

**Fix:** Add `Key::InboxReplyMailTo`, `Key::InboxReplySend`, and `Key::InboxReplySending`
(or reuse existing `Key::MailTo` / `Key::MailSend` / `Key::MailSending`, which already exist
in both locales) and render them via `i18n.t(...)` instead of literals.

### WR-02: Dirty-check false-positive during the footer-load window (baseline is empty until load completes)

**File:** `genossi-frontend/src/component/inbox/reply_form.rs:54-55, 85-86, 129-140`

**Issue:** `baseline_subject` / `baseline_body` are initialized to empty strings and are only
populated at the end of the footer effect. If the user opens the modal and clicks X or
«Abbrechen» *before* `get_mail_footer` resolves, `is_draft_dirty` compares the (non-empty,
quote-pre-filled) body against an empty baseline and returns `true`, so an untouched draft
triggers a spurious "Änderungen verwerfen?" confirmation. This is the inverse of the bug the
D-05 baseline fix was meant to prevent. The same root cause (late baseline) also means a
subject edit made during the load window is folded into the baseline and is *not* flagged as
dirty afterward.

**Fix:** Seed `baseline_subject` / `baseline_body` synchronously with the initial composed
values (or a `baseline_ready: Signal<bool>` guard so the dirty-check treats "not yet loaded"
as not-dirty). Combine with the CR-01 fix so the baseline always reflects the body the user
actually sees.

## Info

### IN-01: Send button lacks `r#type: "button"` (consistency with the documented reload-bug guard)

**File:** `genossi-frontend/src/component/inbox/reply_form.rs:200-220`

**Issue:** The two new close buttons correctly set `r#type: "button"`, but the «Antwort
senden» button does not. There is no `<form>` wrapping this RSX, so the documented WASM
page-reload bug does not actually trigger here, but project memory
(`feedback_dioxus_button_type`) mandates `r#type: "button"` on all onclick buttons as a
guard. Adding it keeps the modal consistent and future-proof if a `<form>` is ever
introduced.

**Fix:** Add `r#type: "button",` to the send button.

### IN-02: `cached_quote` signal duplicates the local `quote_block` String

**File:** `genossi-frontend/src/component/inbox/reply_form.rs:34-48`

**Issue:** `quote_block` is computed once as a plain `String`, then mirrored into a
`cached_quote` signal solely so async closures can read it. This is harmless but slightly
redundant — the quote is static for the form's lifetime and could be captured by clone into
the closures (as the body initializer already does at lines 35-44) without a signal. Minor
readability/footprint note, not a defect.

**Fix:** Optional — capture `quote_block.clone()` into the footer effect and `TemplateSelector`
closure instead of routing through a signal, or keep as-is if the signal aids clarity.

---

_Reviewed: 2026-06-27_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
