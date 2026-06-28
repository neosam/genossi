---
created: 2026-06-28T20:04:15.018Z
title: HTML-Mail-Support statt nur Textmails
area: general
files:
  - genossi_mail/src/lib.rs
  - genossi-frontend/src/component/
---

## Problem

Der Mailversand unterstützt aktuell nur reine Textmails. Für formatierte
Benachrichtigungen (z.B. Digest, Anträge, Mitgliederkommunikation) brauchen
wir HTML-Mails mit Layout, Links und ggf. Branding. Reiner Text wirkt
unprofessionell und schränkt die Gestaltung von Mailings ein.

Zusätzlich braucht das Frontend einen Editor, mit dem sich HTML-Mails
verfassen lassen (WYSIWYG bzw. Rich-Text), sodass Vorstände formatierte
Mails ohne HTML-Kenntnisse erstellen können.

## Solution

TBD — grobe Richtung:
- `lettre` unterstützt `multipart/alternative` (Text + HTML). Bestehenden
  Versand in `genossi_mail` so erweitern, dass parallel zur Textvariante eine
  HTML-Variante mitgeschickt wird (Fallback bleibt Text).
- Template-Rendering läuft bereits über `minijinja` — prüfen, ob HTML-Templates
  dort sauber eingebunden werden können (Auto-Escaping beachten).
- Entscheiden: Text aus HTML ableiten oder beide Varianten getrennt pflegen.
- Tests für den HTML-/Text-Multipart-Aufbau ergänzen.

Frontend:
- Rich-Text-/WYSIWYG-Editor als wiederverwendbare Komponente in
  `genossi-frontend/src/component/` (Component-First, kein inline-RSX).
- WASM-taugliche Editor-Lösung evaluieren (z.B. contenteditable-Wrapper oder
  Anbindung einer JS-Editor-Lib via wasm-bindgen). Output: sauberes, sanitiztes
  HTML, das als HTML-Mail-Body weitergereicht wird.
- Plain-Text-Fallback aus dem Editor-Inhalt ableiten.
