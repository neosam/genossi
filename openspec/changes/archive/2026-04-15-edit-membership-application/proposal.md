## Why

Mitgliedsanträge können aktuell nur erstellt und angesehen werden. Tippfehler oder nachträgliche Korrekturen (z.B. falsche Adresse, geänderte Anteilszahl) erfordern momentan das Löschen und Neuerstellen des Antrags. Ein Edit-Workflow spart Zeit und bewahrt den Erstellungszeitpunkt.

## What Changes

- Neuer REST-Endpoint `PUT /api/applications/{id}` zum Aktualisieren von Antragsfeldern
- Service-Layer-Methode zum Validieren und Durchreichen von Updates an die DAO-Schicht (die bereits `update()` unterstützt)
- Neuer Request-Typ `UpdateApplicationRequest` in den REST-Types
- Frontend: Refactoring des `ApplicationCreateForm` zu einem wiederverwendbaren `ApplicationForm`-Komponenten, der sowohl Create- als auch Edit-Modus unterstützt
- Edit-Button in der `ApplicationDetail`-Ansicht, der das Formular im Edit-Modus öffnet
- Frontend-API-Funktion `update_application()`

## Capabilities

### New Capabilities
- `application-edit`: Ermöglicht das Bearbeiten bestehender Mitgliedsanträge durch Admins (Backend-Endpoint + Frontend-UI)

### Modified Capabilities

## Impact

- **Backend**: Neuer PUT-Endpoint in `genossi_rest`, neue Service-Methode in `genossi_service`/`genossi_service_impl`, neuer Request-Typ in `genossi_rest_types`
- **Frontend**: Refactoring von `application_create_form.rs` zu generischem `ApplicationForm`, Anpassung von `application_detail.rs` für Edit-Button, neue API-Funktion in `api.rs`
- **API**: Neuer Endpoint, keine Breaking Changes an bestehenden Endpoints
- **Tests**: E2E-Tests für den neuen Endpoint, Unit-Tests für Service-Layer
