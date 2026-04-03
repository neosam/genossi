## Why

Wenn man auf der Mail-Seite "Alle" auswählt, werden alle Mitglieder als Empfänger hinzugefügt — einschließlich Mitglieder, die bereits ausgetreten sind (`exit_date` gesetzt). Das ist unerwünscht, da ausgetretene Mitglieder keine Vereins-E-Mails mehr erhalten sollen.

Die Mitglieder-Seite filtert bereits korrekt nach aktiven/inaktiven Mitgliedern (`is_active()`-Funktion basierend auf `exit_date`), aber die Mail-Seite nutzt diesen Filter nicht.

## What Changes

- Die "Alle"-Auswahl auf der Mail-Seite filtert ausgetretene Mitglieder heraus
- Nur Mitglieder ohne `exit_date` (oder mit `exit_date` in der Zukunft) werden als "Alle" ausgewählt
- Die Anzeige der Mitglieder-Anzahl beim "Alle"-Button spiegelt die gefilterte Anzahl wider

## Capabilities

### Modified Capabilities
- `mail-recipient-selection`: Die "Alle"-Auswahl berücksichtigt den Aktiv-Status der Mitglieder. Ausgetretene Mitglieder werden nicht mehr automatisch ausgewählt.

## Impact

- **Frontend Mail-Seite**: `mail_page.rs` — Filterlogik beim "Alle"-Button und bei der Zählung der Mitglieder mit E-Mail
- **Keine Backend-Änderungen**: Die Filterung erfolgt im Frontend mit den vorhandenen `exit_date`-Daten
- **Keine Datenbank-Änderungen**
