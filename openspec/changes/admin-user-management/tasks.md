## 1. Service Layer: Admin-Preference-Methoden

- [x] 1.1 `UserPreferenceService` Trait um `get_by_key_for_user(username, key, context)` und `upsert_for_user(username, key, value, context)` erweitern
- [x] 1.2 `UserPreferenceServiceImpl` implementieren: Admin-Privilege prüfen, dann DAO mit übergebenem `username` aufrufen
- [x] 1.3 Unit-Tests für die neuen Service-Methoden (Admin erlaubt, Nicht-Admin abgelehnt, Preference gefunden/nicht gefunden)

## 2. REST Layer: Admin-Preference-Endpoints

- [x] 2.1 Neue Handler `get_user_preference` und `upsert_user_preference` im Permission-REST-Modul (`genossi_rest/src/permission.rs`)
- [x] 2.2 Routen `/user/{username}/preferences/{key}` (GET, PUT) im Permission-Router registrieren
- [x] 2.3 OpenAPI-Dokumentation für die neuen Endpoints ergänzen
- [x] 2.4 E2E-Tests: Admin liest/schreibt Preference, Nicht-Admin wird abgelehnt, 404 bei fehlender Preference

## 3. Frontend: Berechtigungen-Seite

- [x] 3.1 API-Client-Funktionen: `get_user_preference_admin(username, key)`, `set_user_preference_admin(username, key, value)` hinzufügen
- [x] 3.2 Neue Seite `permissions_page.rs` mit User-Tabelle (Username, Anzeigename, Admin-Checkbox)
- [x] 3.3 Daten laden: alle User + pro User Rollen und sender_name
- [x] 3.4 Admin-Checkbox: Toggle ruft assign/remove user-role Endpoint auf
- [x] 3.5 Anzeigename: Editierbares Feld mit Speichern über PUT-Endpoint
- [x] 3.6 Navigation-Eintrag für die Berechtigungen-Seite (nur für Admins sichtbar)
