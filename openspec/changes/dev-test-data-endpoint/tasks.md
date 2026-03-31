## 1. REST-Endpunkt erstellen

- [x] 1.1 Neues Modul `genossi_rest/src/dev.rs` anlegen mit `#[cfg(debug_assertions)]` - enthaelt den Handler `generate_test_data` und die Route-Funktion
- [x] 1.2 Testdaten definieren: Mindestens 5 Member mit realistischen deutschen Daten (verschiedene Namen, Adressen, Share-Werte, ein ausgetretenes Mitglied)
- [x] 1.3 Handler implementieren: Get-all pruefen, wenn leer Members erzeugen (201), wenn nicht leer nichts tun (200)

## 2. Route registrieren

- [x] 2.1 `dev` Modul in `genossi_rest/src/lib.rs` bedingt einbinden (`#[cfg(debug_assertions)]`)
- [x] 2.2 Route `POST /api/dev/generate-test-data` ausserhalb der Auth-Middleware registrieren (bedingt kompiliert)

## 3. Tests

- [x] 3.1 E2E-Test: Endpunkt aufrufen und pruefen dass Members erzeugt werden (201)
- [x] 3.2 E2E-Test: Zweiter Aufruf gibt 200 zurueck und erzeugt keine Duplikate
