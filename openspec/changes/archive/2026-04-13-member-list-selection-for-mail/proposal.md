## Why

Aktuell muss man Empfänger für Bulk-Mails auf der Mail-Seite einzeln per Autocomplete-Suche zusammenstellen oder "Alle" wählen. Wenn man eine bestimmte Teilmenge braucht (z.B. alle aktiven Mitglieder in einer bestimmten Stadt, oder alle mit ausstehender Migration), muss man die Mitgliederliste separat filtern und dann die gleichen Personen auf der Mail-Seite nochmal suchen. Dieser Workflow ist umständlich und fehleranfällig.

## What Changes

- **Checkbox-Selektion in der Mitgliederliste**: Jede Zeile bekommt eine Checkbox mit großem Touch-Target für mobile Nutzung. Klick auf die Checkbox selektiert, Klick auf den Rest der Zeile navigiert weiterhin zur Detailseite.
- **"Alle gefilterten auswählen"**: Checkbox im Tabellenkopf selektiert/deselektiert alle aktuell gefilterten Mitglieder.
- **Selektions-Aktionsleiste**: Erscheint wenn Mitglieder ausgewählt sind, zeigt Anzahl und bietet "Mail senden"-Button. Erweiterbar für zukünftige Bulk-Aktionen.
- **Navigation zur Mail-Seite mit Vorauswahl**: Klick auf "Mail senden" übergibt die ausgewählten Member-IDs via GlobalSignal an die Mail-Seite.
- **Skalierbare Empfänger-Darstellung auf der Mail-Seite**: Bestehende Chip-Darstellung wird durch eine eingeklappte Zusammenfassung mit aufklappbarer scrollbarer Liste ersetzt, die auch bei 500+ Empfängern funktioniert.

## Capabilities

### New Capabilities
- `member-list-selection`: Checkbox-basierte Mehrfachauswahl in der Mitgliederliste mit "Alle filtern"-Toggle und Aktionsleiste

### Modified Capabilities
- `mail-sending`: Empfänger-Darstellung skalierbar machen (eingeklappt/aufklappbar statt Chip-Wand), vorausgewählte Empfänger aus GlobalSignal übernehmen

## Impact

- **Frontend**: `genossi-frontend/src/page/members.rs` (Checkbox-Spalte, Aktionsleiste), `genossi-frontend/src/page/mail_page.rs` (Empfänger-UI), neuer GlobalSignal für Selektion
- **Backend**: Keine Änderungen nötig — Selektion und Übergabe sind rein frontend-seitig
- **APIs**: Keine Änderungen
- **i18n**: Neue Übersetzungsschlüssel für Selektions-UI
