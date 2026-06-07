---
id: 260607-vij
type: quick
title: Reply-Form vorbefuellt Originalmail als Quote-Block
status: complete
created: 2026-06-07
completed: 2026-06-07
---

# Quick 260607-vij - Summary

## What changed
Beim Antworten auf eine Inbox-Mail wird die Originalmail jetzt als Quote-Block
unten im Reply-Body vorbefuellt. Format: deutscher
"Am {datum} schrieb {absender}:"-Header gefolgt von der Original-Body, in der
jede Zeile mit "> " praefixiert ist (leere Zeilen werden zu ">").

Layout im Body beim Oeffnen der Form:

    [zwei Leerzeilen Schreibflaeche]
    {Signatur-Footer}
    
    Am 07.06.2026 14:30 schrieb max@example.com:
    > Originaltext Zeile 1
    >
    > Originaltext Zeile 2

Der Footer steht weiterhin zwischen Eingabezone und Quote, nicht UNTER dem
Quote. Bei Template-Auswahl bleibt der Quote erhalten.

## Files touched
- `genossi-frontend/src/component/inbox/reply_form.rs`
  - Drei neue Props: `original_body`, `original_from`, `original_date`.
  - Pre-Fill des `reply_body`-Signals mit Quote-Block (sofort, synchron).
  - Footer-`use_effect` ergaenzt nach Footer-Load den vollstaendigen Body
    via neuer `compose_initial_body(footer, quote)`-Helper.
  - `TemplateSelector::on_select` haengt jetzt Footer UND Quote an das Template.
  - Neue private Helper: `build_original_quote`, `compose_initial_body`.
  - `#[cfg(test)] mod tests` mit 8 Unit-Tests.
- `genossi-frontend/src/page/inbox_page.rs`
  - Aufruf der `InboxReplyForm` mit drei neuen Werten aus `InboundMailDetailTO`
    (`body_text`, `from_address`, formatiertes `received_at`).

## Verification
- `cargo check` -> 0 Errors, 32 pre-existing dead-code Warnings (nichts Neues).
- `cargo test --bin genossi-frontend reply_form` -> 8 passed, 0 failed.

Getestete Faelle:
- Leerer Body -> leerer Quote (kein Header).
- Single-Line Body -> "Am ... schrieb ...:\n> ...".
- Multi-Line mit Leerzeilen -> "> ..." pro Zeile, ">" fuer Leerzeilen.
- Trailing Newline -> wird durch `lines()` korrekt verworfen.
- `compose_initial_body` in allen 4 Permutationen (footer/quote x leer/nicht-leer).

## Out of scope (bewusst nicht angepackt)
- Backend bleibt 1:1: der Worker sendet weiterhin nur den Body, kein zusaetzliches
  Original-Quote serverseitig.
- HTML-Body-Pfad ist nicht abgedeckt; wenn die Mail nur HTML hatte und `body_text`
  leer ist, wird der Quote-Block leer sein - bewusst akzeptiert.
- Bestehender Race-Condition-Bug (User tippt vor Footer-Load -> Footer-Load
  ueberschreibt Input) wurde nicht behoben.

## Commit
Wird im naechsten Schritt als jj-Commit erstellt.
