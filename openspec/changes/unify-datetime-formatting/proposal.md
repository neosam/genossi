## Why

Datums- und Zeitanzeigen im Frontend sind inkonsistent. Für reine Datumswerte gibt es zwar `i18n.format_date(&time::Date)`, aber für DateTimes (mit Uhrzeit) existiert kein zentraler Formatter. Das führt zu zwei Symptomen:

1. **Drei lokale Hilfsfunktionen** zerschneiden ISO-Strings auf eigene Faust:
   - `application_list.rs:23` `format_datetime(Option<String>) -> String`
   - `application_detail.rs:8` `format_datetime` (eigene Kopie)
   - `communication_timeline.rs:7` `format_datetime_short(&str)`
2. **Stellen ganz ohne Formatter** rendern den rohen ISO-String inkl. Nanosekunden — z. B. `timestamp_section.rs:206` mit `"{ts.timestamp}"`. Resultat: `2026-04-16T16:03:34.512345678Z` direkt in der UI.

Beide Symptome sind ein Aufräumfall mit gemeinsamem Pattern: zentrale, lokalisierbare Datumsformatierung wie sie für reine Daten schon existiert.

## What Changes

- Neue Methode `i18n.format_datetime(...)` für Zeitstempel mit Uhrzeit (Format: lokales Datum + Stunden:Minuten).
- Optional: zweite Methode `i18n.format_datetime_long(...)` mit Sekundenauflösung für Audit-/Verifikations-Kontexte, wo genaue Sekundengenauigkeit wichtig ist.
- Drei lokale Hilfsfunktionen entfernen und durch die i18n-Variante ersetzen.
- Stellen mit roher ISO-Anzeige (Audit-Log, Timestamp-Sektion u. a.) auf den i18n-Aufruf umstellen.

## Capabilities

### New Capabilities
- `frontend-datetime-formatting`: Einheitliche, lokalisierbare Formatierung von Datums- und Zeitstempelanzeigen im Frontend.

### Modified Capabilities
<!-- keine - keine bestehenden Specs zur Datetime-Formatierung im Frontend; das hier ist die erste explizite Capability dafür -->

## Impact

- **Frontend**:
  - `genossi-frontend/src/i18n/i18n.rs` und `genossi-frontend/src/i18n/mod.rs` — neue Methoden auf der `I18n`-Struktur und den Locale-Implementierungen.
  - `genossi-frontend/src/component/application_list.rs:23-35` — lokalen Helfer entfernen, i18n nutzen.
  - `genossi-frontend/src/component/application_detail.rs:8` — dito.
  - `genossi-frontend/src/component/communication_timeline.rs:7` — dito.
  - `genossi-frontend/src/component/timestamp_section.rs:206` (und ähnliche Stellen mit rohem ISO-String) — auf i18n umstellen.
  - `genossi-frontend/src/page/audit_log.rs` — vermutlich ebenfalls Stellen mit rohen Zeitstempeln (zu verifizieren).
- **Backend**: Keine Änderung — Backend liefert weiterhin ISO8601 mit Nanosekunden.
- **Berechtigungen**: Keine Änderung.
