## 1. Service-Layer

- [ ] 1.1 `ApplicationService::submit()` Signatur um `send_mail: bool` erweitern (`genossi_service/src/application.rs`)
- [ ] 1.2 `ApplicationServiceImpl::submit()` anpassen: `send_confirmation_mail()` nur bei `send_mail == true` aufrufen (`genossi_service_impl/src/application.rs`)
- [ ] 1.3 Bestehende Unit-Tests für `submit()` anpassen (neuer Parameter)

## 2. REST-Layer

- [ ] 2.1 `AdminCreateApplicationRequest` Type in `genossi_rest_types` definieren (gleiche Felder wie `PublicJoinRequest` + `send_mail: Option<bool>`)
- [ ] 2.2 Neuen Handler `create_application` in `genossi_rest/src/application.rs` implementieren (`POST /api/applications`, Auth + `manage_members`)
- [ ] 2.3 Route `POST /` zu `generate_route()` hinzufügen
- [ ] 2.4 `public_join` Handler anpassen: `submit(data, true)` aufrufen
- [ ] 2.5 OpenAPI-Doku aktualisieren (neuen Handler in `ApiDoc` aufnehmen)

## 3. Tests

- [ ] 3.1 E2E-Test: Admin erstellt Application ohne Mail
- [ ] 3.2 E2E-Test: Admin erstellt Application mit `send_mail: true`
- [ ] 3.3 E2E-Test: Unauthentifizierter Zugriff auf `POST /api/applications` gibt 401
- [ ] 3.4 E2E-Test: Validierungsfehler bei fehlendem Pflichtfeld gibt 422
- [ ] 3.5 Bestehende E2E-Tests für `POST /api/public/join` verifizieren (Verhalten unverändert)

## 4. Frontend

- [ ] 4.1 API-Funktion `create_application` in `api.rs` hinzufügen
- [ ] 4.2 `ApplicationCreateForm`-Komponente erstellen (Modal mit Eingabefeldern + Mail-Toggle)
- [ ] 4.3 "Antrag anlegen"-Button auf der Applications-Seite einbauen, der das Modal öffnet
- [ ] 4.4 Nach erfolgreichem Anlegen Liste aktualisieren
