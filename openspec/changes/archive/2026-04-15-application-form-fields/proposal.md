## Why

Das Admin-Formular zum manuellen Anlegen von Eintrittserklärungen fehlt das Anrede-Feld (obwohl das Datenmodell es unterstützt) und ein Titel-Feld existiert im Application-Modell gar nicht. Der Titel ("Dr.", "Prof.") wird beim Member bereits unterstützt, fehlt aber bei Applications — dadurch geht die Information bei Anträgen verloren und muss nach Bestätigung manuell nachgetragen werden.

## What Changes

- Anrede-Dropdown (Herr/Frau/Firma, optional) im Admin-Formular `ApplicationCreateForm` anzeigen und an den Request übergeben
- Neues optionales `title`-Feld im gesamten Application-Stack: DB-Migration, DAO, Service, REST-Types, REST-Endpoints (Admin + öffentlich)
- Titel-Feld im Admin-Formular `ApplicationCreateForm` anzeigen
- Titel-Feld im öffentlichen `PublicJoinRequest` verfügbar machen
- Bei Application-Bestätigung (`confirm`) den Titel in den neuen Member übernehmen
- `build_inputs_application()` im PdfGenerator um `title` erweitern

## Capabilities

### New Capabilities

### Modified Capabilities
- `auto-member-creation`: Application-Datenmodell bekommt `title`-Feld, Bestätigung überträgt `title` zum Member
- `member-management`: `PublicJoinRequest` bekommt optionales `title`-Feld

## Impact

- **DB**: Migration zum Hinzufügen der `title`-Spalte in `application`-Tabelle
- **Backend**: `genossi_dao/src/application.rs`, `genossi_service/src/application.rs`, `genossi_service_impl/src/application.rs`, `genossi_rest_types/src/lib.rs`, `genossi_rest/src/application.rs`, `genossi_service_impl/src/pdf_generation.rs`
- **Frontend**: `genossi-frontend/src/component/application_create_form.rs`, `genossi-frontend/src/api.rs`
- **API**: `AdminCreateApplicationRequest` und `PublicJoinRequest` bekommen `title`-Feld, `ApplicationTO` bekommt `title`-Feld. Keine Breaking Changes (neue Felder sind optional).
