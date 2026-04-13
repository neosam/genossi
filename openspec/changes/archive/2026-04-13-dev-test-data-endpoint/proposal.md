## Why

Beim lokalen Entwickeln und Testen muss man aktuell manuell Member-Daten anlegen, bevor man im Frontend etwas sehen kann. Das kostet jedes Mal Zeit, besonders nach einem Datenbank-Reset. Ein Endpunkt zum Erzeugen von Testdaten beschleunigt den Entwicklungszyklus erheblich.

## What Changes

- Neuer REST-Endpunkt `POST /api/dev/generate-test-data` zum Erzeugen von Member-Testdaten
- Endpunkt wird nur bei Debug-Builds kompiliert (`#[cfg(debug_assertions)]`)
- Erzeugt ca. 10 realistische deutsche Member-Datensätze mit unterschiedlichen Eigenschaften
- Idempotent: Wenn bereits Members existieren, werden keine neuen erzeugt
- Kein Auth-Check nötig (Dev-only Endpunkt)

## Capabilities

### New Capabilities
- `dev-test-data`: Debug-only REST-Endpunkt zur Erzeugung von Member-Testdaten fuer lokale Entwicklung

### Modified Capabilities

## Impact

- `genossi_rest`: Neues Modul `dev.rs` mit dem Endpunkt, nur bei `debug_assertions` kompiliert
- `genossi_rest/src/lib.rs`: Route registrieren, bedingt kompiliert
- Keine Auswirkung auf Release-Builds oder Produktion
- Keine neuen Dependencies noetig
