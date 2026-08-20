---
status: testing
phase: 32-frontend-compose-dialog
source: [32-VERIFICATION.md]
started: 2026-08-21T00:00:00Z
updated: 2026-08-21T00:00:00Z
---

## Current Test

number: 1
name: Compose-Seite öffnet mit „Zahlungserinnerung" vorbefüllt, TemplateSelector nur Antragsteller-Vorlagen
expected: |
  Seite öffnet mit Zahlungserinnerung-Vorlage vorausgewählt und befüllt;
  TemplateSelector-Dropdown enthält keine Mitglieder-Vorlagen.
awaiting: user response

## Tests

### 1. Vorbefüllung + Template-Filter (Compose-Seite)
expected: application_detail → „✉ E-Mail senden" (bei vorhandener Adresse) öffnet die Compose-Seite mit der Vorlage „Zahlungserinnerung" vorausgewählt und Betreff/Body befüllt; das TemplateSelector-Dropdown zeigt nur Antragsteller-Vorlagen (keine Mitglieder-Vorlagen).
result: [pending]

### 2. Debounced Live-Vorschau (~400ms, kein Flackern)
expected: Beim schnellen Tippen im Betreff/Editor aktualisiert sich die Vorschau mit ~400ms Verzögerung und zeigt aufgelöste Platzhalter; während des Wartens bleibt die letzte Vorschau sichtbar (kein leerer/flackernder Zwischenzustand). Nur die zuletzt eingegebene Version erscheint (Generation-Zähler verwirft veraltete Läufe).
result: [pending]

### 3. Senden-Button disabled während laufendem Request (kein Doppelversand)
expected: Beim Klick auf „E-Mail senden" (ggf. mit gedrosseltem Netzwerk) bleibt der Button disabled, solange der Request läuft, und das Label wechselt zu „Wird gesendet…"; ein Doppelklick löst keinen zweiten Sende-Request aus.
result: [pending]

### 4. Post-Send: Erfolgs-Toast + Rücksprung zur Antragsliste
expected: Nach erfolgreichem Versand erscheint ein Erfolgs-Toast („E-Mail-Auftrag erstellt"), danach Navigation zurück zu Route::ApplicationsPage.
result: [pending]

### 5. No-Email-Guard + Navigation (application_detail Button)
expected: Bei einem Antrag OHNE E-Mail-Adresse ist der „E-Mail senden"-Button disabled mit Hinweistext („Keine E-Mail-Adresse hinterlegt"); bei einem Antrag MIT Adresse navigiert der Klick zu /applications/{id}/compose. Nie ein stiller Fehlversuch.
result: [pending]

### 6. Timeline-Klick → Body-Panel (echter Body) + Long-Text-Backstop
expected: Klick auf einen Timeline-Eintrag öffnet das Inline-Body-Panel mit dem echten gespeicherten Body (rendered_body/rendered_html_body, nicht neu gerendert); bei sehr langem/HTML-lastigem Body bleibt der Text innerhalb des `max-h-96 overflow-auto`-Containers, ohne die Seite zu sprengen.
result: [pending]

## Summary

total: 6
passed: 0
issues: 0
pending: 6
skipped: 0
blocked: 0

## Gaps
