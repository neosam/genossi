## 1. DB-Migration

- [x] 1.1 SQLite-Migration erstellen: `ALTER TABLE application ADD COLUMN title TEXT`

## 2. Backend: DAO

- [x] 2.1 `title: Option<Arc<str>>` zu `ApplicationEntity` in `genossi_dao/src/application.rs` hinzufügen
- [x] 2.2 SQLite DAO-Implementierung anpassen: `title` in `dump_all`, `create`, `update` Queries

## 3. Backend: Service

- [x] 3.1 `title: Option<Arc<str>>` zu `Application` und `ApplicationSubmission` in `genossi_service/src/application.rs` hinzufügen
- [x] 3.2 From-Implementierungen zwischen `Application` und `ApplicationEntity` um `title` erweitern
- [x] 3.3 `confirm()` in `genossi_service_impl/src/application.rs` anpassen: `title` vom Application auf den neuen Member übertragen

## 4. Backend: REST

- [x] 4.1 `title: Option<String>` zu `ApplicationTO`, `AdminCreateApplicationRequest` und `PublicJoinRequest` in `genossi_rest_types/src/lib.rs` hinzufügen
- [x] 4.2 REST-Handler anpassen: `title` in `public_join` und `create_application` aus dem Request an `ApplicationSubmission` übergeben

## 5. Backend: Typst-Integration

- [x] 5.1 `build_inputs_application()` in `genossi_service_impl/src/pdf_generation.rs` um `title` erweitern
- [x] 5.2 Bestehende Tests für `build_inputs_application` um `title` erweitern

## 6. Frontend: Admin-Formular

- [x] 6.1 Anrede-Dropdown (optional: leer/Herr/Frau/Firma) im `ApplicationCreateForm` hinzufügen
- [x] 6.2 Titel-Textfeld (optional) im `ApplicationCreateForm` hinzufügen
- [x] 6.3 Beide Felder an `AdminCreateApplicationRequest` übergeben
- [x] 6.4 `title`-Feld zum Frontend `AdminCreateApplicationRequest` in `api.rs` hinzufügen

## 7. Frontend: Application-Detail

- [x] 7.1 Titel in der Application-Detail-Ansicht anzeigen (wenn vorhanden)

## 8. Tests

- [x] 8.1 E2E-Test: Application mit Titel erstellen und bestätigen, prüfen dass Member den Titel hat
- [x] 8.2 E2E-Test: Application ohne Titel erstellen, prüfen dass Felder NULL sind
