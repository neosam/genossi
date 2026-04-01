## Why

Der bestehende Validierungs-Endpoint prüft bisher nur Mitgliedsnummern-Lücken und Übertragungs-Gegenparts. Es gibt weitere Inkonsistenzen, die nach einem Import oder bei manueller Datenpflege auftreten können und bisher unentdeckt bleiben: Shares-Divergenz, fehlende Eintritts-Aktionen, Exit-Date/Austritt-Widersprüche und mehr.

## What Changes

- Neue Validierungsregel: Shares-Konsistenz (`current_shares` vs. Summe der `shares_change`)
- Neue Validierungsregel: Eintritt-Aktion fehlt oder ist doppelt
- Neue Validierungsregel: Exit-Date ohne Austritt-Aktion und umgekehrt
- Neue Validierungsregel: Aktive Mitglieder mit `current_shares <= 0`
- Neue Validierungsregel: Doppelte Mitgliedsnummern bei aktiven Mitgliedern
- Neue Validierungsregel: Ausgetretene Mitglieder mit verbleibenden Anteilen (Info-Level)
- Neue Validierungsregel: `migrated`-Flag stimmt nicht mit berechnetem Status überein
- Erweiterung des `ValidationResult` Structs und der Frontend-Anzeige um die neuen Regeln

## Capabilities

### New Capabilities

### Modified Capabilities
- `data-validation`: Erweiterung um 7 neue Validierungsregeln mit unterschiedlichen Schweregraden (Fehler, Warnung, Info)

## Impact

- Geänderte Dateien: `genossi_service/src/validation.rs`, `genossi_service_impl/src/validation.rs`, `genossi_rest_types/src/lib.rs`, `genossi-frontend/rest-types/src/lib.rs`
- Geänderte Frontend-Seite: `genossi-frontend/src/page/validation.rs`
- Neue i18n-Keys für die zusätzlichen Regeln
- Kein Impact auf bestehende Endpoints oder Datenstrukturen außer dem Validierungs-Response
