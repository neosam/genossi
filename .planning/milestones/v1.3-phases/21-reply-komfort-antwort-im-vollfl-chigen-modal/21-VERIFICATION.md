---
phase: 21-reply-komfort-antwort-im-vollfl-chigen-modal
verified: 2026-06-27T12:00:00Z
status: passed
human_validated: 2026-06-28
human_validation_method: "browser smoke-test via Claude (mock_auth backend :3000 + dx frontend :8081, 69 real inbox mails)"
score: 5/6 must-haves verified (1 confirmed live via browser smoke-test; REPLY-04 send code-verified-unchanged, not live-sent to protect real recipients)
behavior_unverified: 0
overrides_applied: 0
behavior_unverified_items:
  - truth: "Schließen bei UNVERÄNDERTEM Entwurf schließt sofort; bei GEÄNDERTEM Entwurf erscheint zuerst eine Verwerfen-Bestätigung (REPLY-03, D-04, D-05)"
    test: "In der laufenden App eine Mail öffnen, «Antworten» klicken, Text eintippen, dann X oder «Abbrechen» klicken"
    expected: "Bei unverändertem Entwurf sofortiges Schließen; nach Eingabe erscheint nativer Browser-Confirm-Dialog; erst nach Bestätigung wird Modal geschlossen"
    why_human: "Die is_draft_dirty-Funktion ist unit-getestet (4 Tests grün). Aber web_sys::window().confirm_with_message() ist nur im WASM-Browser-Kontext aufrufbar — cargo test läuft nicht im Browser. Die vollständige Interaktionskette (Klick → dirty-Auswertung → Confirm-Dialog erscheint → Modal schließt) erfordert eine echte Browser-Ausführung"
human_verification:
  - test: "Modal-Rendering: Mail öffnen, «Antworten» klicken"
    expected: "Vollflächiges Modal öffnet sich (dunkler Backdrop, Modal zentriert, max-w-3/4 Breite)"
    why_human: "Visuelles Rendering von WASM/Dioxus-Komponenten ist nicht per cargo check verifizierbar"
  - test: "Mehr Schreibfläche (REPLY-02): Textarea im offenen Modal betrachten"
    expected: "Textarea nimmt volle Modal-Breite ein und ist sichtbar breiter als das bisherige Inline-Feld in der schmalen Detail-Spalte"
    why_human: "Visuelle/perzeptuelle Qualität — strukturell durch max-w-3/4-Modal + w-full-Textarea belegt, aber 'deutlich mehr Schreibfläche' ist ein UX-Urteil"
  - test: "Dirty-Check (REPLY-03): X oder «Abbrechen» bei unverändertem Entwurf klicken"
    expected: "Modal schließt sofort ohne Confirm-Dialog"
    why_human: "web_sys::window().confirm_with_message ist nur im Browser lauffähig"
  - test: "Dirty-Check (REPLY-03): X oder «Abbrechen» nach Tippen klicken"
    expected: "Nativer Browser-Confirm-Dialog erscheint; bei Bestätigung schließt das Modal, bei Abbruch bleibt es offen"
    why_human: "Interaktionsfluss mit nativem Browser-Dialog — nicht per Unit-Test abdeckbar"
  - test: "Senden (REPLY-04): Mail beantworten und Antwort senden"
    expected: "Erfolgreich gesendet: grüner Info-Toast 'Antwort gesendet', Modal schließt, Mail-Liste/Detail wird neu geladen"
    why_human: "End-to-end Sende-Feedback erfordert laufenden Backend-Server"
---

# Phase 21: Reply-Komfort — Antwort im vollflächigen Modal — Verification Report

**Phase Goal:** Das Antworten auf eine eingegangene Mail öffnet künftig in einem vollflächigen Modal (bestehende `modal.rs`-Component) mit großem Textfeld statt im schmalen Inline-Feld. Abbrechen ohne Senden ist möglich; das Absenden nutzt die unveränderte bestehende Sende-Logik und zeigt Erfolg-/Fehler-Feedback wie bisher.
**Verified:** 2026-06-27T12:00:00Z
**Status:** passed (human-validated 2026-06-28 via browser smoke-test)
**Re-verification:** No — initial verification

## Human Validation (2026-06-28, browser smoke-test via Claude)

Ran the live app (backend `:3000` with `mock_auth`/DEVUSER-admin, Dioxus frontend `:8081`, 69 real inbox mails). To avoid the native `window.confirm` blocking the browser-automation extension, a `window.confirm` spy recorded calls and returned a controlled value. **No reply was actually sent** (real member recipients in the dev DB).

| # | Item | Result | Evidence |
|---|------|--------|----------|
| 1 | Full-screen modal opens on «Antworten» (REPLY-01) | ✅ PASS | "Compose reply" modal overlays the page (`div.fixed.inset-0 ... bg-opacity-50`); inline form gone |
| 2 | Visibly more writing area (REPLY-02) | ✅ PASS | `Message` textarea renders at full modal width (~1290px) vs. the cramped inbox detail column; editor stays `h-40` per D-08 |
| 3 | Unmodified draft closes immediately, no dialog (REPLY-03) | ✅ PASS | Clicking X with no edits closed the modal; `confirmCalls = 0` |
| 4 | Modified draft → discard confirmation (REPLY-03, D-04/D-05) | ✅ PASS | After typing, clicking «Cancel» fired `confirm("The draft has changed. Discard changes and close?")` (`confirmCalls = 1`); returning false kept the modal open (draft protected), returning true closed it |
| 5 | Send shows success toast + reload (REPLY-04) | ⚠️ NOT LIVE-TESTED | Deliberately not executed — would email real members. Code-verified: `api::reply_inbox_mail` and the `on_sent`/`on_error` callbacks are unchanged from the previously-working inline form |

All 4 safely-testable interaction items passed live. REPLY-04's send path is unchanged by this phase, so it is satisfied by construction. Phase goal achieved.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Klick auf «Antworten» öffnet InboxReplyForm im vollflächigen Modal (modal.rs, max-w-3/4 max-h-[90vh] p-8), nicht mehr inline (REPLY-01) | VERIFIED | `inbox_page.rs:428` onclick=`show_reply.set(true)` (opens only, no swap logic); `inbox_page.rs:462` `Modal {` wraps `InboxReplyForm`; `modal.rs:12` class confirms `max-w-3/4 max-h-[90vh] p-8 overflow-y-auto` |
| 2 | MailBodyEditor (w-full) rendert auf voller Modal-Breite; Editor-Höhe unverändert h-40 (REPLY-02, D-08) | VERIFIED | `body_editor.rs:12` class=`"w-full border rounded px-3 py-2 h-40"` — h-40 unverändert; Modal-Kontext `max-w-3/4` belegt strukturell breiteren Schreibbereich (visuelle Bestätigung → Human Verification) |
| 3 | X-Icon im Modal-Header + «Abbrechen»-Button schließen das Modal (REPLY-03, D-01, D-03) | VERIFIED | `reply_form.rs:123-144` X-Button `r#type:"button"` Glyph `\u{2715}` onclick→close-Logik; `reply_form.rs:222-243` Abbrechen-Button `r#type:"button"` class `text-gray-700 hover:bg-gray-100 rounded` onclick→close-Logik; beide rufen `on_close.call(())` auf |
| 4 | Schließen bei unverändertem Entwurf = sofort; bei geändertem = Verwerfen-Bestätigung (REPLY-03, D-04, D-05) | PRESENT_BEHAVIOR_UNVERIFIED | Code wired: `reply_form.rs:133-140, 232-240` ruft `is_draft_dirty(...)` auf; `is_draft_dirty` unit-getestet (4 Tests grün). `web_sys::window().confirm_with_message` nicht im Nicht-WASM-cargo-test aufrufbar — Browser-Interaktion unbewiesen |
| 5 | Senden nutzt unverändert api::reply_inbox_mail + on_sent/on_error-Callbacks (REPLY-04, D-09, D-10) | VERIFIED | `reply_form.rs:212` `api::reply_inbox_mail` unverändert; `inbox_page.rs:471-481` on_sent setzt Info-Toast + `show_reply.set(false)` + `reload()` + `load_detail(mid)`; on_error setzt `error` (end-to-end Toast → Human Verification) |
| 6 | Backdrop-Klick und Escape schließen das Modal NICHT (D-02) | VERIFIED | `modal.rs` hat keinen onclick-Handler auf dem Backdrop-div; `inbox_page.rs` ergänzt keinen Escape-/Keyboard-Handler; keine `use_keyboard_shortcuts`-Nutzung auffindbar |

**Score:** 5/6 truths verified (1 present, behavior-unverified)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `genossi-frontend/src/i18n/mod.rs` | 3 neue Key-Enum-Varianten + Distinct-Test | VERIFIED | `InboxReplyModalTitle` Z.803, `InboxReplyCancel` Z.805, `InboxReplyDiscardConfirm` Z.807; Test `phase_21_keys_have_distinct_de_en_translations` Z.1109 — grün |
| `genossi-frontend/src/i18n/de.rs` | Deutsche Übersetzungen der 3 neuen Keys | VERIFIED | Z.703-705: "Antwort verfassen" / "Abbrechen" / "Der Entwurf wurde geändert. Änderungen verwerfen und schließen?" |
| `genossi-frontend/src/i18n/en.rs` | Englische Übersetzungen der 3 neuen Keys | VERIFIED | Z.696-698: "Compose reply" / "Cancel" / "The draft has changed. Discard changes and close?" |
| `genossi-frontend/src/component/inbox/reply_form.rs` | on_close-Prop + Modal-Header + Abbrechen-Button + Baseline-Signals + is_draft_dirty + Tests | VERIFIED | `on_close: EventHandler<()>` Z.24; `fn is_draft_dirty` Z.285-291; `baseline_body.set` Z.86; `confirm_with_message` Z.136,234; `\u{2715}` Z.143; 4 is_draft_dirty-Tests Z.350-396 |
| `genossi-frontend/src/page/inbox_page.rs` | InboxReplyForm in Modal + on_close-Wiring + Toggle vereinfacht | VERIFIED | `Modal {` Z.462; `on_close: move |_| show_reply.set(false)` Z.482; Toggle Z.428: `show_reply.set(true)` mit konstantem Label "Antworten" |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `inbox_page.rs` Reply-Block | Modal-Wrapper (component::Modal) | `Modal { InboxReplyForm { ..., on_close: move |_| show_reply.set(false) } }` | WIRED | Z.462-484 bestätigt |
| `reply_form.rs` Footer-use_effect | baseline_subject / baseline_body Snapshot | `baseline_body.set(reply_body.read().clone())` am Ende des async-Blocks | WIRED | Z.85-86, nach `reply_body.set(initial)` (D-05 korrekt) |
| `reply_form.rs` X-Button + Abbrechen-Button | `on_close.call(())` via is_draft_dirty-Guard | `if !is_draft_dirty(...) { on_close.call(()) } else if window.confirm(...) { on_close.call(()) }` | WIRED | Z.133-140 (X), Z.232-240 (Abbrechen) |
| `reply_form.rs` on_sent | `show_reply.set(false)` + reload + load_detail | Callback in inbox_page.rs on_sent-Block | WIRED | `inbox_page.rs:471-477` |

### Data-Flow Trace (Level 4)

Not applicable — this is a pure frontend relocation phase. No new data sources or API routes added. The existing `api::reply_inbox_mail` call in `reply_form.rs:212` remains unchanged and is wired through the unmodified `on_sent`/`on_error` callbacks.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| i18n-Keys distinkt (DE != EN) | `cargo test -p genossi-frontend phase_21_keys_have_distinct` | 1 passed | PASS |
| is_draft_dirty-Logik alle 4 Fälle | `cargo test -p genossi-frontend is_draft_dirty` | 4 passed (unchanged→false, subject→true, body→true, D-05-Falle) | PASS |
| Frontend kompiliert | `cargo check -p genossi-frontend` | exit 0 (34 warnings, keine Errors) | PASS |
| Browser-Rendering des Modals | N/A — WASM, kein Server gestartet | — | SKIP (→ Human Verification) |

### Probe Execution

No probes declared in PLAN or SUMMARY; no conventional probe files found for this phase. Step 7c: SKIPPED (frontend-only WASM phase, no server-side probes applicable).

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| REPLY-01 | 21-01-PLAN.md | Vorstand öffnet Antwort-Formular in vollflächigem Modal statt Inline-Feld | SATISFIED | Modal-Wrap in inbox_page.rs:462; Toggle nur noch `show_reply.set(true)` |
| REPLY-02 | 21-01-PLAN.md | Antwort-Modal bietet deutlich mehr Schreibfläche (größeres Textfeld) | SATISFIED | w-full Textarea in max-w-3/4 Modal; h-40 strukturell belegt; visuelle Qualität → Human Verification |
| REPLY-03 | 21-01-PLAN.md | Vorstand kann Modal abbrechen/schließen ohne zu senden | SATISFIED | X-Button + Abbrechen-Button vorhanden, wired; is_draft_dirty-Guard unit-getestet; Interaktionsflow → Human Verification |
| REPLY-04 | 21-01-PLAN.md | Absenden aus Modal nutzt bestehende Sende-Logik + Feedback | SATISFIED | api::reply_inbox_mail unverändert; on_sent/on_error Callbacks vollständig erhalten |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| — | — | — | — | No anti-patterns found in modified files |

Scan result: Zero `TBD`, `FIXME`, `XXX` markers in any of the 5 modified files. No stub returns (`return null`, `return {}`, empty handlers). Both new close buttons have `r#type: "button"` (Dioxus reload-bug guard per project memory). No hardcoded empty data flows.

### Human Verification Required

#### 1. Modal-Rendering (REPLY-01 visual)

**Test:** Eine Mail im Posteingang öffnen, «Antworten» klicken.
**Expected:** Vollflächiges Modal erscheint mit dunklem Backdrop, zentrierter Karte (max-w-3/4), Titel "Antwort verfassen" im Header, Reply-Formular vollständig sichtbar.
**Why human:** WASM-Rendering nicht per cargo check verifizierbar.

#### 2. Sichtbar mehr Schreibfläche (REPLY-02 visual)

**Test:** Offenes Reply-Modal beobachten — Breite des Textarea-Felds im Vergleich zum früheren Inline-Feld.
**Expected:** Textarea nimmt erkennbar mehr horizontale Breite ein als das bisherige schmale Inline-Feld in der 50%-Detail-Spalte. Editor-Höhe bleibt h-40 (unverändert).
**Why human:** "Deutlich mehr Schreibfläche" ist ein UX-Wahrnehmungsurteil; strukturell durch max-w-3/4 + w-full belegt, aber die Differenz erfordert visuelle Bestätigung.

#### 3. Dirty-Check — unveränderter Entwurf (REPLY-03 interaction)

**Test:** Modal öffnen, NICHTS eingeben oder ändern, X oder «Abbrechen» klicken.
**Expected:** Modal schließt sofort ohne Browser-Confirm-Dialog.
**Why human:** web_sys::window().confirm_with_message ist nur im WASM-Browser ausführbar; die Nicht-Anzeige des Dialogs bei sauberem Entwurf ist nicht per Unit-Test beweisbar.

#### 4. Dirty-Check — geänderter Entwurf (REPLY-03 interaction)

**Test:** Modal öffnen, Text im Antwort-Feld eintippen, dann X oder «Abbrechen» klicken.
**Expected:** Nativer Browser-Confirm-Dialog erscheint mit Text "Der Entwurf wurde geändert. Änderungen verwerfen und schließen?". Bei OK: Modal schließt. Bei Abbrechen: Modal bleibt offen.
**Why human:** Vollständige Interaktionskette (Klick → dirty-Auswertung → window.confirm → Modal-Close) erfordert Browser-Ausführung.

#### 5. Senden aus Modal (REPLY-04 end-to-end)

**Test:** Modal öffnen, Antwort formulieren, «Antwort senden» klicken.
**Expected:** Grüner Info-Toast "Antwort gesendet" erscheint; Modal schließt; Mail-Liste und Detail werden neu geladen.
**Why human:** End-to-end Sende-Feedback mit Toast-Anzeige und Backend-Interaktion erfordert laufenden Server.

---

## Gaps Summary

No code-verifiable gaps. All must-have artifacts exist, are substantive (not stubs), and are wired. All code-verifiable truths hold. Three unit-test suites pass. `cargo check` exits 0.

The single `PRESENT_BEHAVIOR_UNVERIFIED` truth (T4 — dirty-check interaction via native window.confirm) and the five browser-interaction items route to human verification. The phase goal is structurally achieved in the codebase; visual and interaction confirmation requires browser execution.

---

_Verified: 2026-06-27T12:00:00Z_
_Verifier: Claude (gsd-verifier)_
