## 1. Backend: REST-Types & Service-Interface

- [x] 1.1 `UpdateApplicationRequest`-Struct in `genossi_rest_types` anlegen (Felder: salutation, title, first_name, last_name, email, street, house_number, postal_code, city, shares, version)
- [x] 1.2 `update_application(id, request)`-Methode im `ApplicationService`-Trait in `genossi_service` definieren

## 2. Backend: Service-Implementierung

- [x] 2.1 `update_application` in `genossi_service_impl` implementieren (Antrag laden, Version prüfen, Felder aktualisieren, DAO update aufrufen)
- [x] 2.2 Unit-Tests für die Service-Methode (Erfolg, Versionskonflikt, nicht gefunden, Validierung)

## 3. Backend: REST-Endpoint

- [x] 3.1 PUT-Handler `/api/applications/{id}` in `genossi_rest` implementieren
- [x] 3.2 Endpoint in OpenAPI-Dokumentation (Utoipa) registrieren
- [x] 3.3 E2E-Tests für den PUT-Endpoint (Erfolg, 409, 404, 422)

## 4. Frontend: API-Funktion

- [x] 4.1 `update_application(config, id, request)` in `api.rs` hinzufügen

## 5. Frontend: Wiederverwendbares Formular

- [x] 5.1 `ApplicationCreateForm` zu `ApplicationForm` refactoren mit `ApplicationFormMode`-Enum (Create/Edit)
- [x] 5.2 Edit-Modus: Felder mit bestehenden Daten vorbefüllen, Mail-Checkbox ausblenden, Submit ruft `update_application` auf
- [x] 5.3 Referenzen auf `ApplicationCreateForm` in `applications_page.rs` auf `ApplicationForm` aktualisieren

## 6. Frontend: Edit-Button in Detailansicht

- [x] 6.1 "Bearbeiten"-Button in `ApplicationDetail`-Komponente hinzufügen
- [x] 6.2 State-Management: Bei Klick auf Bearbeiten Detail schließen und ApplicationForm im Edit-Modus öffnen
- [x] 6.3 Nach erfolgreichem Update Antragsliste neu laden
