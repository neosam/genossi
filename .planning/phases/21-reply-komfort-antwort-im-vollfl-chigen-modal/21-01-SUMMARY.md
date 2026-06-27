---
phase: 21-reply-komfort-antwort-im-vollfl-chigen-modal
plan: 01
subsystem: frontend
status: complete
tags: [dioxus, modal, reply, i18n, inbox]
requires:
  - "Modal component (genossi-frontend/src/component/modal.rs)"
  - "InboxReplyForm (genossi-frontend/src/component/inbox/reply_form.rs)"
  - "i18n Key enum + de.rs/en.rs"
provides:
  - "InboxReplyForm renders in full-screen Modal with own header + close affordances"
  - "Pure is_draft_dirty helper + dirty-guarded close (X / Abbrechen)"
  - "3 new i18n keys (InboxReplyModalTitle, InboxReplyCancel, InboxReplyDiscardConfirm) in both locales"
affects:
  - "genossi-frontend/src/page/inbox_page.rs (reply wiring)"
tech-stack:
  added: []
  patterns:
    - "membership_adjust_modal header pattern (justify-between + X-glyph)"
    - "pure-fn + #[cfg(test)] mod tests in component file"
    - "native web_sys confirm_with_message for discard guard"
key-files:
  created: []
  modified:
    - genossi-frontend/src/i18n/mod.rs
    - genossi-frontend/src/i18n/de.rs
    - genossi-frontend/src/i18n/en.rs
    - genossi-frontend/src/component/inbox/reply_form.rs
    - genossi-frontend/src/page/inbox_page.rs
decisions:
  - "D-05 baseline snapshot captured inside footer use_effect, after body compose"
  - "D-07 native window.confirm_with_message (no nested in-app confirm modal)"
  - "D-08 MailBodyEditor left at h-40; roominess from wide Modal context"
metrics:
  duration: "~25m"
  completed: 2026-06-27
  tasks: 3
  files-modified: 5
---

# Phase 21 Plan 01: Reply-Komfort — Antwort im vollflächigen Modal Summary

Das bestehende `InboxReplyForm` öffnet jetzt im vollflächigen `Modal` statt inline in der schmalen Detail-Spalte; es rendert seinen eigenen Header mit X-Icon und einen «Abbrechen»-Button, beide schützen vor versehentlichem Entwurfsverlust über einen puren, unit-getesteten Dirty-Check gegen eine nach dem Footer-Load gesnapshottete Baseline (D-05).

## What Was Built

- **Task 1 — i18n (Commit `bffa16a`):** Drei neue `Key`-Varianten (`InboxReplyModalTitle`, `InboxReplyCancel`, `InboxReplyDiscardConfirm`) in `mod.rs`, übersetzt in `de.rs` ("Antwort verfassen" / "Abbrechen" / "Der Entwurf wurde geändert. Änderungen verwerfen und schließen?") und `en.rs` ("Compose reply" / "Cancel" / "The draft has changed. Discard changes and close?"). Neuer Test `phase_21_keys_have_distinct_de_en_translations` (DE != EN, beide nicht leer) — grün.
- **Task 2 — reply_form.rs (Commit `efeadf6`):** Neuer Prop `on_close: EventHandler<()>`; selbst-gerenderter Modal-Header (Container `flex items-center justify-between border-b border-gray-200 pb-3`, Titel `text-xl font-semibold text-gray-900`, X-Glyph `\u{2715}`, `r#type: "button"`); neutraler «Abbrechen»-Button neben dem unveränderten Senden-Button in einer `flex gap-2`-Zeile. Zwei neue Baseline-Signals werden INNERHALB des Footer-`use_effect` NACH dem Body-Compose gesetzt (deckt alle drei async-Pfade ab). Pure Helfer `is_draft_dirty(subject, body, baseline_subject, baseline_body) -> bool` + 4 Unit-Tests (unverändert→false, subject→true, body→true, D-05-Falle: post-footer-Baseline ≠ Erst-Quote → false). Close-Logik: bei nicht-dirty sofort `on_close.call(())`, sonst nativer `web_sys::window().confirm_with_message`.
- **Task 3 — inbox_page.rs (Commit `3917cea`):** `Modal` importiert und um `InboxReplyForm` gewrappt; `on_close: move |_| show_reply.set(false)` ergänzt; `on_sent`/`on_error` unverändert (D-10). Toggle-Button vereinfacht auf `show_reply.set(true)` mit konstantem Label "Antworten". Kein Backdrop-/Escape-Handler (D-02).

## Key Decisions

- **D-05 (Baseline-Falle):** Die Dirty-Baseline wird am Ende des async-Footer-Bodys aus dem dann-aktuellen `reply_body`/`reply_subject` gelesen — nicht vom quote-only Erststring. Ein unberührter Entwurf gilt damit korrekt als nicht-dirty.
- **D-07:** Verwerfen-Bestätigung über nativen Browser-Confirm; statischer i18n-Text, kein Injection-Vektor (T-21-01 accept).
- **D-08:** `MailBodyEditor` bleibt `h-40` (von Compose geteilt); die größere Schreibfläche entsteht allein aus dem breiten `max-w-3/4`-Modal.

## Deviations from Plan

None — plan executed exactly as written.

## Verification

- `cargo test -p genossi-frontend phase_21_keys_have_distinct` → 1 passed.
- `cargo test -p genossi-frontend is_draft_dirty` → 4 passed.
- `cargo check -p genossi-frontend` → exit 0 (alle drei Tasks kompilieren zusammen).

## Known Stubs

None.

## Self-Check: PASSED
- FOUND: genossi-frontend/src/i18n/mod.rs (InboxReplyModalTitle, phase_21 test)
- FOUND: genossi-frontend/src/component/inbox/reply_form.rs (on_close, is_draft_dirty, baseline_body.set, confirm_with_message, \u{2715})
- FOUND: genossi-frontend/src/page/inbox_page.rs (Modal wrap, on_close, show_reply.set(true))
- FOUND commits: bffa16a, efeadf6, 3917cea
