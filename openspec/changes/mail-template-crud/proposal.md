## Why

Email templates (subject + body with MiniJinja variables) are currently not persisted. Users must re-type or paste templates every time they send bulk mail. The predefined formal/informal salutation snippets are hardcoded in Rust source code and cannot be edited by users. Storing reusable email templates in the database lets users create, edit, and select templates when composing mail.

## What Changes

- New `mail_templates` SQLite table with fields: id (UUID), name (unique), subject, body, created, deleted, version
- New `MailTemplateDao` trait + SQLite implementation for CRUD operations
- New `MailTemplateService` trait + implementation with business logic (name uniqueness validation, soft delete)
- New REST endpoints under `/api/mail/templates` for full CRUD
- Database migration to seed the existing hardcoded formal/informal salutation snippets as initial template rows
- Remove hardcoded `TEMPLATE_FORMAL` and `TEMPLATE_INFORMAL` constants from source code after migration

## Capabilities

### New Capabilities
- `mail-templates`: Persistent storage and management of reusable email templates (name, subject, body) with full CRUD via REST API

### Modified Capabilities
- `mail-sending`: The mail sending UI/API can reference a saved template by ID to pre-fill subject and body when composing a mail

## Impact

- **Database**: New migration for `mail_templates` table + seed data
- **genossi_mail crate**: New DAO, service, and REST modules for mail templates
- **genossi_bin**: Wire up new DAO + service + REST routes
- **Existing code**: Hardcoded template constants in `genossi_mail/src/template.rs` become DB rows
- **API surface**: New `/api/mail/templates` endpoints added to OpenAPI/Swagger
