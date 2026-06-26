---
status: partial
phase: 20-inbox-digest-t-glicher-posteingangs-benachrichtigungs-worker
source: [20-VERIFICATION.md]
started: 2026-06-27
updated: 2026-06-27
---

## Current Test

[awaiting human testing]

## Tests

### 1. Visueller Config-Abschnitt „Posteingangs-Benachrichtigung"
expected: Auf der Config-Seite erscheint ein eigener CollapsibleSection-Block (zwischen IMAP und WebDAV) mit Empfänger-Textfeld (komma-getrennt), Uhrzeit-Feld (HH:MM) und Speichern-Button. Inline-Fehler bei ungültiger E-Mail / ungültiger Uhrzeit sichtbar; leeres Empfänger-Feld ist gültig (deaktiviert das Feature). Werte bleiben nach Reload erhalten.
result: [pending]

### 2. Echter Digest-Mail-Versand zur konfigurierten Uhrzeit
expected: Bei konfigurierten Empfängern + Uhrzeit verschickt der Worker zur Server-Lokalzeit genau eine Plain-Text-Digest-Mail pro Empfänger mit allen offenen (nicht-archivierten) Posteingangs-Mails. Genau-ein-Versand-pro-Kalendertag-Garantie + Catch-up (verpasste Uhrzeit wird nachgeholt). Erfordert SMTP-Konfiguration, laufenden Server und Wartezeit bis zur Uhrzeit.
result: [pending]

## Summary

total: 2
passed: 0
issues: 0
pending: 2
skipped: 0
blocked: 0

## Gaps
