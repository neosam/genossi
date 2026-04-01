## Why

Nach einem Excel-Import wurde bemerkt, dass ein Mitglied fehlte. Aktuell gibt es keine Möglichkeit, die Datenintegrität systematisch zu prüfen. Ein Validierungs-Endpoint soll Inkonsistenzen wie Lücken in Mitgliedsnummern oder fehlende Gegenpart-Übertragungen aufdecken, damit sie schnell erkannt und behoben werden können.

## What Changes

- Neuer `ValidationService` Trait und Implementierung im Service Layer
- Neuer REST-Endpoint `GET /api/validation` der alle Validierungsregeln ausführt
- Validierungsregel: Mitgliedsnummern-Lücken im Bereich min..max erkennen
- Validierungsregel: Übertragungen ohne korrespondierenden Gegenpart finden (UebertragungAbgabe ↔ UebertragungEmpfang mit matchendem Datum, Shares und Member-IDs)
- Neue Frontend-Seite die den Validierungs-Endpoint aufruft und Ergebnisse anzeigt

## Capabilities

### New Capabilities
- `data-validation`: Globaler Validierungs-Service und REST-Endpoint zur Prüfung der Datenintegrität (Mitgliedsnummern-Lücken, Übertragungs-Konsistenz)

### Modified Capabilities

## Impact

- Neue Dateien: `genossi_service/src/validation.rs`, `genossi_service_impl/src/validation.rs`, `genossi_rest/src/validation.rs`, `genossi_rest_types/src/validation.rs`
- Neue Frontend-Seite für Validierungsergebnisse
- Nutzt bestehende `MemberDao` und `MemberActionDao` als Dependencies
- Kein Impact auf bestehende Endpoints oder Datenstrukturen
