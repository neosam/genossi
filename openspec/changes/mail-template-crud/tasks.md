## 1. Database Migration

- [ ] 1.1 Create SQLite migration for `mail_templates` table (id BLOB, name TEXT UNIQUE, subject TEXT, body TEXT, created TEXT, deleted TEXT, version BLOB)
- [ ] 1.2 Add seed data migration inserting the two predefined templates (formal/informal greeting) with fixed UUIDs using INSERT OR IGNORE

## 2. DAO Layer

- [ ] 2.1 Add `MailTemplate` entity struct and `MailTemplateDao` trait to `genossi_mail/src/dao.rs` with methods: `create`, `update`, `dump_all`, `find_by_id`, `all` (non-deleted), `find_by_name` (non-deleted)
- [ ] 2.2 Implement `MailTemplateDao` for SQLite in `genossi_mail/src/dao_sqlite.rs`
- [ ] 2.3 Add unit tests for DAO trait mock setup

## 3. Service Layer

- [ ] 3.1 Add `MailTemplateService` trait to `genossi_mail/src/service.rs` (or new file) with methods: `create`, `update`, `delete`, `get`, `list`
- [ ] 3.2 Implement `MailTemplateService` with name uniqueness validation, soft delete logic, and version conflict detection
- [ ] 3.3 Add unit tests for service layer using mocked DAO

## 4. REST Layer

- [ ] 4.1 Add REST types (`MailTemplateTO`, `CreateMailTemplateRequest`, `UpdateMailTemplateRequest`) to `genossi_mail/src/rest.rs` or a new `rest_templates.rs`
- [ ] 4.2 Implement REST endpoints: POST (create), GET list, GET by id, PUT (update), DELETE (soft delete)
- [ ] 4.3 Add OpenAPI annotations (utoipa) for all endpoints
- [ ] 4.4 Register template routes under `/api/mail/templates` in the mail router

## 5. Wiring

- [ ] 5.1 Instantiate `MailTemplateDao` SQLite impl in `genossi_bin/src/main.rs` (or `lib.rs`)
- [ ] 5.2 Instantiate `MailTemplateService` and pass to REST state
- [ ] 5.3 Add `MailTemplateService` accessor to `MailRestState` trait and implement for app state

## 6. Integration Tests

- [ ] 6.1 Add e2e tests for full CRUD lifecycle (create, list, get, update, delete)
- [ ] 6.2 Add e2e test for duplicate name rejection (409)
- [ ] 6.3 Add e2e test for version conflict on update (409)
- [ ] 6.4 Add e2e test that predefined templates are present after migration
