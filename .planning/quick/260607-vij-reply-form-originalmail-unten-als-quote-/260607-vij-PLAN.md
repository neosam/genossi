---
id: 260607-vij
type: quick
title: Reply-Form vorbefüllt Originalmail als Quote-Block
status: planned
created: 2026-06-07
---

# Quick 260607-vij — Reply-Form: Originalmail als Quote-Block

## Goal
Beim Antworten auf eine Inbox-Mail wird die Originalmail unten im Reply-Body als
Quote-Block vorbefüllt (mit `>`-Prefix pro Zeile und einem deutschen
"Am ... schrieb ...:"-Header), damit der Empfaenger den Konversationskontext sieht.
Der bestehende Signatur-Footer bleibt erhalten und steht zwischen Eingabezone
und Quote-Block.

## Files
- `genossi-frontend/src/component/inbox/reply_form.rs` — neue Props +
  Quote-Helper + erweitertes Body-Pre-Fill (Footer & Template-Auswahl).
- `genossi-frontend/src/page/inbox_page.rs` — Aufruf der `InboxReplyForm` mit
  drei neuen Props (`original_body`, `original_from`, `original_date`).

## Tasks

### T1 — Helper + Component-Erweiterung
- **Files:** `genossi-frontend/src/component/inbox/reply_form.rs`
- **Action:**
  - Private Helper `build_original_quote(body: &str, from: &str, date: &str) -> String`
    bauen, der den Outlook-Style-Quote-Block erzeugt:
    `"Am {date} schrieb {from}:\n> Zeile1\n> Zeile2\n…"`. Leere Zeilen
    werden zu `>` (kein trailing space). Bei leerem `body` → leerer String.
  - Drei neue Props an `InboxReplyForm` ergänzen:
    `original_body: String`, `original_from: String`, `original_date: String`.
  - Footer-`use_effect` so umbauen, dass das initiale `reply_body` jetzt
    `"\n\n{footer}\n\n{quote}"` ergibt (Footer/Quote werden bei leerem Inhalt
    weggelassen, ohne überflüssige Leerzeilen).
  - `TemplateSelector::on_select` ebenfalls anpassen, damit der Quote-Block
    erhalten bleibt, wenn der User ein Template wählt
    (`"{template}\n{footer}\n\n{quote}"`).
- **Verify:** `cargo check -p genossi-frontend --target wasm32-unknown-unknown`
  bleibt grün (oder `cargo check -p genossi-frontend` ohne Target, falls die
  WASM-Toolchain nicht da ist — Hauptsache der Rust-Compiler frisst die
  Component-Signatur).
- **Done:** Unit-Test in `reply_form.rs` (`#[cfg(test)] mod tests`) deckt
  `build_original_quote` ab — mindestens 3 Fälle: leerer Body, einfacher
  Body, Body mit Leerzeilen. `cargo test -p genossi-frontend` läuft grün.

### T2 — Aufrufstelle anpassen
- **Files:** `genossi-frontend/src/page/inbox_page.rs`
- **Action:** Im `InboxReplyForm`-Aufruf (≈ Z. 462) drei neue Props mit Werten
  aus `d: InboundMailDetailTO`:
  - `original_body: d.body_text.clone()`
  - `original_from: d.from_address.clone()` (oder vorhandene `from_addr`-Variable)
  - `original_date: i18n.format_datetime(&d.received_at)` (analog Z. 285)
- **Verify:** `cargo check -p genossi-frontend` grün.
- **Done:** Build-Check grün, Component-First-Prinzip bleibt gewahrt
  (keine inline-RSX-Duplikate eingefügt).

## must_haves
- Reply-Body enthält direkt nach dem Öffnen den Quote-Block der Originalmail.
- Footer steht weiterhin zwischen Eingabe und Quote (nicht UNTER dem Quote).
- Template-Wechsel zerstört den Quote-Block NICHT.
- `build_original_quote` ist Unit-getestet.
- `cargo check`/`cargo test -p genossi-frontend` bleibt grün.

## Out of scope
- Kein Backend-Change (Worker bleibt 1:1, sendet weiterhin nur den Body).
- Keine Anpassung am HTML-Body-Pfad (nur `body_text`); falls die Mail
  ausschließlich HTML hatte, wird der Quote-Block leer sein — bewusst akzeptiert.
- Kein "Original-Mail als .eml anhängen" — separater Task falls gewünscht.
