## 1. Database Migration

- [x] 1.1 Create SQLite migration for `mail_templates` table (id BLOB, name TEXT UNIQUE, subject TEXT, body TEXT, created TEXT, deleted TEXT, version BLOB)
- [x] 1.2 Add seed data migration inserting the two predefined templates (formal/informal greeting) with fixed UUIDs using INSERT OR IGNORE

## 2. DAO Layer

- [x] 2.1 Add `MailTemplate` entity struct and `MailTemplateDao` trait to `genossi_mail/src/dao.rs` with methods: `create`, `update`, `dump_all`, `find_by_id`, `all` (non-deleted), `find_by_name` (non-deleted)
- [x] 2.2 Implement `MailTemplateDao` for SQLite in `genossi_mail/src/dao_sqlite.rs`
- [x] 2.3 Add unit tests for DAO trait mock setup

## 3. Service Layer

- [x] 3.1 Add `MailTemplateService` trait to `genossi_mail/src/service.rs` (or new file) with methods: `create`, `update`, `delete`, `get`, `list`
- [x] 3.2 Implement `MailTemplateService` with name uniqueness validation, soft delete logic, and version conflict detection
- [x] 3.3 Add unit tests for service layer using mocked DAO

## 4. REST Layer

- [x] 4.1 Add REST types (`MailTemplateTO`, `CreateMailTemplateRequest`, `UpdateMailTemplateRequest`) to `genossi_mail/src/rest.rs` or a new `rest_templates.rs`
- [x] 4.2 Implement REST endpoints: POST (create), GET list, GET by id, PUT (update), DELETE (soft delete)
- [x] 4.3 Add OpenAPI annotations (utoipa) for all endpoints
- [x] 4.4 Register template routes under `/api/mail/templates` in the mail router

## 5. Wiring

- [x] 5.1 Instantiate `MailTemplateDao` SQLite impl in `genossi_bin/src/main.rs` (or `lib.rs`)
- [x] 5.2 Instantiate `MailTemplateService` and pass to REST state
- [x] 5.3 Add `MailTemplateService` accessor to `MailRestState` trait and implement for app state

## 6. Integration Tests

- [x] 6.1 Add e2e tests for full CRUD lifecycle (create, list, get, update, delete)
- [x] 6.2 Add e2e test for duplicate name rejection (409)
- [x] 6.3 Add e2e test for version conflict on update (409)
- [x] 6.4 Add e2e test that predefined templates are present after migration
