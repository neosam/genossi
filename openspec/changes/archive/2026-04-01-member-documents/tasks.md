## 1. Database & Migration

- [x] 1.1 Create SQLite migration `add_member_document` with `member_document` table (id, member_id, document_type, description, file_name, mime_type, relative_path, created, deleted, version)

## 2. DAO Layer

- [x] 2.1 Add `MemberDocumentEntity` struct and `MemberDocumentDao` trait in `genossi_dao/src/member_document.rs` with `dump_all`, `create`, `update`, `find_by_member_id` methods
- [x] 2.2 Implement `MemberDocumentDaoImpl` in `genossi_dao_impl_sqlite/src/member_document.rs` with SQLite queries

## 3. Storage Layer

- [x] 3.1 Add `DocumentStorage` trait and `StorageError` type in `genossi_service/src/document_storage.rs`
- [x] 3.2 Implement `FilesystemDocumentStorage` in `genossi_service_impl/src/document_storage.rs` with `DOCUMENT_STORAGE_PATH` env-var support and directory auto-creation

## 4. Service Layer

- [x] 4.1 Add `MemberDocument` service struct and `DocumentType` enum in `genossi_service/src/member_document.rs`
- [x] 4.2 Add `MemberDocumentService` trait with `upload`, `list`, `download`, `delete` methods
- [x] 4.3 Implement `MemberDocumentServiceImpl` in `genossi_service_impl/src/member_document.rs` with singleton-replacement logic, validation (Other requires description, 50 MB limit), and board-only permission check

## 5. REST Layer

- [x] 5.1 Add `MemberDocumentTO` in `genossi_rest_types/src/lib.rs`
- [x] 5.2 Add REST handlers in `genossi_rest/src/member_document.rs`: POST upload (multipart), GET list, GET download, DELETE
- [x] 5.3 Register document routes under `/members/{member_id}/documents` in `genossi_rest/src/lib.rs` and add to OpenAPI docs

## 6. Binary Wiring

- [x] 6.1 Add `MemberDocumentServiceDependencies`, wire DAO + Storage + Service in `genossi_bin/src/lib.rs`
- [x] 6.2 Add `DOCUMENT_STORAGE_PATH` env-var reading and `FilesystemDocumentStorage` initialization

## 7. Backend Tests

- [x] 7.1 Add unit tests for `MemberDocumentServiceImpl` (singleton replacement, validation, permission checks) with mocked DAO and storage
- [x] 7.2 Add E2E tests in `genossi_bin/tests/e2e_tests.rs` for upload, list, download, delete endpoints

## 8. Frontend

- [x] 8.1 Add `MemberDocumentTO` and `DocumentTypeTO` to `genossi-frontend/rest-types/src/lib.rs`
- [x] 8.2 Add API functions in `genossi-frontend/src/api.rs`: upload_document (multipart), list_documents, download_document, delete_document
- [x] 8.3 Add i18n keys for document section (Dokumente, Hochladen, Dokumenttyp, Beitrittserklärung, Beitrittsbestätigung, Aufstockung, Sonstige, Beschreibung, etc.) in `mod.rs`, `de.rs`, `en.rs`
- [x] 8.4 Add Documents section to `member_details.rs`: document list table, upload form with type dropdown + file input + description field, download and delete buttons
