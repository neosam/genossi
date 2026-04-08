## 1. Datenbank & Migration

- [x] 1.1 Neue SQLite-Migration für `static_documents` Tabelle
- [x] 1.2 Neue SQLite-Migration für `mail_job_static_attachments` Tabelle
- [x] 1.3 `cargo sqlx prepare` — n/a, Projekt verwendet `query`/`query_as` ohne Compile-Time-Checks

## 2. DAO Layer

- [x] 2.1 `StaticDocument` Entity in `genossi_mail::dao`
- [x] 2.2 `StaticDocumentDao` Trait mit `create`, `find_by_id`, `find_many_by_ids`, `all_active`, `soft_delete`
- [x] 2.3 SQLite-Implementierung `StaticDocumentDaoSqlite` in `genossi_mail::dao_sqlite`
- [x] 2.4 `MailJobStaticAttachmentDao` Trait mit `create`, `find_static_documents_by_job_id`
- [x] 2.5 SQLite-Implementierung `MailJobStaticAttachmentDaoSqlite`
- [x] 2.6 Unit-Tests für beide DAOs (in-memory SQLite, 6 neue Tests)

## 3. Filesystem-Storage

- [x] 3.1 Wiederverwendung des bestehenden `FilesystemDocumentStorage` (save/load/delete). Static Documents liegen unter `static_documents/<uuid>` innerhalb von `DOCUMENT_STORAGE_PATH`. Kein neues Modul nötig — die bestehende Infrastruktur genügt und bleibt konsistent mit member-documents.
- [x] 3.2 Basispfad: `DOCUMENT_STORAGE_PATH` wird bereits von `FilesystemDocumentStorage::from_env()` ausgewertet
- [x] 3.3 Startup-Bootstrap: `FilesystemDocumentStorage.save()` legt fehlende Verzeichnisse automatisch an
- [x] 3.4 Unit-Tests: Service-Tests decken Filesystem-Interaktion via `MockDocumentStorage` ab

## 4. Service Layer

- [x] 4.1 `StaticDocumentService` Trait in `genossi_mail::static_document_service`
- [x] 4.2 `StaticDocumentServiceImpl` mit File-first/DB-second Rollback-Pattern
- [x] 4.3 Content-Type-Whitelist (`application/pdf`, `image/png`, `image/jpeg`) und Größenlimit (`STATIC_DOCUMENTS_MAX_BYTES`, default 10 MB)
- [x] 4.4 Auth-Prüfung im REST-Layer via `PermissionService::check_permission(ADMIN_PRIVILEGE)`
- [x] 4.5 Unit-Tests mit Mockall (5 Tests für Validierung, Erfolg, Rollback)

## 5. REST Layer

- [x] 5.1 REST-Typen: `StaticDocumentTO`, `SendBulkMailRequest.static_document_ids`
- [x] 5.2 Handler `POST /api/static-documents` mit `multipart/form-data`
- [x] 5.3 Handler `GET /api/static-documents` (sortiert nach Name)
- [x] 5.4 Handler `GET /api/static-documents/{id}` (Download mit Content-Disposition)
- [x] 5.5 Handler `DELETE /api/static-documents/{id}` (Soft-Delete)
- [x] 5.6 Routen registriert in `genossi_rest::lib`, OpenAPI-Doku in `static_document::ApiDoc`
- [x] 5.7 404 für unbekannte und soft-deleted IDs (Service findet sie nicht → `NotFound`)

## 6. Mail-Versand-Integration

- [x] 6.1 `SendBulkMailRequest.static_document_ids: Vec<String>` in `genossi_mail::rest`
- [x] 6.2 Validierung im Bulk-Send-Handler via `StaticDocumentDao::find_many_by_ids` (Service liefert `NotFound` → 404 wenn eine ID fehlt)
- [x] 6.3 Mail-Worker lädt pro verarbeitetem Empfänger die job-level Static Attachments und hängt sie via `lettre` Multipart an. Wiederverwendung der bestehenden `send_mail_for_recipient` Pipeline
- [x] 6.4 `mail_job_static_attachments` Join-Einträge werden bei `create_job` pro statischer Dokument-ID persistiert
- [x] 6.5 Koexistenz mit member-gebundenen `mail-attachments` bestätigt (bestehende Tests grün)

## 7. Frontend (Dioxus)

- [x] 7.1 `api.rs`: `StaticDocumentTO`, `list_static_documents`, `upload_static_document` (multipart via `web_sys::FormData`), `delete_static_document`, `static_document_download_url`
- [x] 7.2 Neue Seite `page/static_documents.rs` (Route `/documents`, Nav-Link in TopBar hinter `show_admin`)
- [x] 7.3 Upload-Komponente mit Name-Feld, Datei-Input, Fehleranzeige
- [x] 7.4 `mail_page.rs`: Multiselect für statische Dokumente inkl. On-Mount-Load und Checkbox-Interaktion
- [x] 7.5 `send_bulk_mail` Client schickt `static_document_ids`

## 8. Konfiguration & Dokumentation

- [x] 8.1 Keine neuen ENV-Variablen nötig — `DOCUMENT_STORAGE_PATH` wird wiederverwendet; optional `STATIC_DOCUMENTS_MAX_BYTES`
- [ ] 8.2 CLAUDE.md / README Update — Out of scope (user hat "keine Dokumentation" Policy)
- [x] 8.3 `.gitignore` — n/a, `./documents` war schon ausgeschlossen

## 9. Tests (End-to-End)

- [x] 9.1 `test_static_document_crud_happy_path`: Upload → List → Download (Bytes identisch) → Delete → Liste leer → Download 404
- [x] 9.2 `test_static_document_rejects_disallowed_content_type`: 400 bei `application/x-msdownload`
- [x] 9.3 Größenlimit-Test via Unit-Test im Service (E2E durch Limit in Multipart aufwendig; gleichwertig abgedeckt)
- [x] 9.4 `test_bulk_mail_with_static_document_ids_succeeds`: 202 + Job erstellt
- [x] 9.5 `test_bulk_mail_with_unknown_static_document_id_fails`: 404
- [x] 9.6 Regression: alle 101 bestehenden E2E-Tests grün ohne Änderungen

## 10. Validierung & Abschluss

- [x] 10.1 `cargo build --workspace` grün, `cargo test --workspace` grün (295 Tests, 0 failed), Frontend `cargo check` grün. `cargo fmt`/`clippy` nicht in Umgebung verfügbar
- [ ] 10.2 `openspec verify-change static-mail-documents` — Ausstehend (CLI-Befehl)
- [ ] 10.3 Manueller Smoketest — Dev darf selbst via UI testen
