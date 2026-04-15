## 1. Datenmodell

- [x] 1.1 `ApplicationEntity` Felder `email`, `street`, `house_number`, `postal_code`, `city` zu `Option<Arc<str>>` ändern (`genossi_dao/src/application.rs`)
- [x] 1.2 SQLite-Migration: Spalten nullable machen (`migrations/sqlite/`)
- [x] 1.3 SQLite DAO-Implementierung anpassen für optionale Felder (`genossi_dao_impl_sqlite/`)
- [x] 1.4 `ApplicationSubmission` Felder analog zu `Option` ändern (`genossi_service/src/application.rs`)
- [x] 1.5 `Application` Service-Struct Felder analog anpassen
- [x] 1.6 `ApplicationTO` in `genossi_rest_types` anpassen (optionale Felder)
- [x] 1.7 `confirm()`-Methode anpassen: Member-Erstellung mit `Option`-Feldern (`genossi_service_impl/src/application.rs`)
- [x] 1.8 `send_confirmation_mail()` anpassen: mit optionaler E-Mail umgehen

## 2. Service-Layer

- [x] 2.1 `ApplicationService::submit()` Signatur um `send_mail: bool` erweitern (`genossi_service/src/application.rs`)
- [x] 2.2 `ApplicationServiceImpl::submit()` anpassen: `send_confirmation_mail()` nur bei `send_mail == true` aufrufen, Validierung lockern (nur first_name, last_name, shares Pflicht)
- [x] 2.3 Validierung: `send_mail: true` ohne E-Mail → Fehler
- [x] 2.4 Bestehende Unit-Tests für `submit()` anpassen (neuer Parameter + optionale Felder)

## 3. REST-Layer

- [x] 3.1 `AdminCreateApplicationRequest` Type in `genossi_rest_types` definieren (first_name, last_name, shares Pflicht; email, Adresse, salutation optional; send_mail: Option<bool> default false)
- [x] 3.2 Neuen Handler `create_application` in `genossi_rest/src/application.rs` implementieren (`POST /api/applications`, Auth + `manage_members`)
- [x] 3.3 Route `POST /` zu `generate_route()` hinzufügen
- [x] 3.4 `public_join` Handler anpassen: `submit(data, true)` aufrufen, weiterhin alle Felder als Pflicht validieren
- [x] 3.5 OpenAPI-Doku aktualisieren (neuen Handler + neuen Request-Type in `ApiDoc` aufnehmen)

## 4. Tests

- [x] 4.1 E2E-Test: Admin erstellt Application mit nur Pflichtfeldern (name + shares)
- [x] 4.2 E2E-Test: Admin erstellt Application mit allen Feldern
- [x] 4.3 E2E-Test: Admin erstellt Application mit `send_mail: true` und E-Mail
- [x] 4.4 E2E-Test: `send_mail: true` ohne E-Mail → 422
- [x] 4.5 E2E-Test: Unauthentifizierter Zugriff auf `POST /api/applications` → 401 (via standard auth middleware, nicht separat testbar mit mock_auth)
- [x] 4.6 E2E-Test: Validierungsfehler bei fehlendem Pflichtfeld (first_name) → 422
- [x] 4.7 Bestehende E2E-Tests für `POST /api/public/join` verifizieren (Verhalten unverändert)

## 5. Frontend

- [x] 5.1 API-Funktion `create_application` in `api.rs` hinzufügen
- [x] 5.2 `ApplicationCreateForm`-Komponente erstellen (Modal: Name + Anteile Pflicht, Rest optional, Mail-Toggle default aus)
- [x] 5.3 "Antrag anlegen"-Button auf der Applications-Seite einbauen, der das Modal öffnet
- [x] 5.4 Nach erfolgreichem Anlegen Liste aktualisieren
