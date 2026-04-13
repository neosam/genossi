## Why

Mitglieder wurden in der Vergangenheit teilweise fehlerhaft erfasst. Diese Eintraege waren nie echte Mitgliedschaften, lassen sich aber mit dem aktuellen Datenmodell nicht korrekt abbilden. Soft Delete wuerde die Historie verlieren, eine Austritt-Action wuerde faelschlicherweise eine ehemalige Mitgliedschaft implizieren. Es wird ein explizites Statusfeld benoetigt, um solche Faelle sauber zu kennzeichnen.

## What Changes

- Neues Enum-Feld `status` auf der Member-Entity mit initialem Wert `Normal`
- Neuer Enum-Wert `FehlerhaftErfasst` fuer Mitglieder, die nie echte Mitglieder waren
- Mitglieder mit Status `FehlerhaftErfasst` werden bei `is_active()` und `count_active` ausgeschlossen
- Status ist beim Anlegen und nachtraeglich setzbar
- Mitgliedsnummer bleibt bei fehlerhaft erfassten Mitgliedern erhalten
- Mitglieder mit `FehlerhaftErfasst` sind in der Gesamtliste sichtbar, aber klar markiert
- Das Enum ist erweiterbar fuer zukuenftige Sonderfaelle

## Capabilities

### New Capabilities
- `member-status`: Erweiterbares Statusfeld auf der Member-Entity zur Klassifizierung von Mitgliedern (Normal, FehlerhaftErfasst). Beeinflusst Aktiv-Zaehlung und Sichtbarkeit.

### Modified Capabilities
- `member-management`: Neues Feld `status` im Member-Datenmodell, Aenderung der Aktiv-Logik, Anpassung von Create/Update API

## Impact

- **DAO Layer**: Neues Feld `status` in `MemberEntity`, Enum-Mapping TEXT<->Rust
- **Database**: Migration fuer neue Spalte `status` mit Default `Normal`
- **Service Layer**: `count_active` und Aktiv-Filterung muessen `status` beruecksichtigen
- **REST Layer**: `status`-Feld in API-Requests/Responses, Filtermoeglichkeit
- **Frontend**: Status-Anzeige in Mitgliederliste, Auswahl beim Anlegen/Bearbeiten
- **Bestehende Daten**: Alle existierenden Mitglieder erhalten automatisch `Normal` via Default
