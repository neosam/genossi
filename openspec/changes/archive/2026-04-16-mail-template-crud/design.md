## Context

Email templates for bulk mail are not persisted. Users must manually enter subject and body each time. Two hardcoded salutation snippets (formal/informal) exist in `genossi_mail/src/template.rs` but cannot be edited or extended by users. The project follows a layered architecture (DAO → Service → REST) with trait-based dependency injection and SQLite as the database.

## Goals / Non-Goals

**Goals:**
- Persistent CRUD for email templates (name, subject, body) in SQLite
- Own service layer (`MailTemplateService`) separate from `MailService`
- Unique template names with UUID primary keys
- Seed existing hardcoded formal/informal snippets via DB migration
- REST API under `/api/mail/templates` with OpenAPI docs

**Non-Goals:**
- Template categories, tags, or folder structure
- Template versioning or change history
- Frontend implementation (separate change)
- Modifying the mail sending flow to auto-load templates (frontend concern)

## Decisions

### Decision: Separate MailTemplateService instead of extending MailService
Mail templates are an independent concern from mail job execution. A dedicated `MailTemplateDao` + `MailTemplateService` keeps responsibilities clean and avoids bloating the existing `MailService` trait. This follows the same pattern as `ConfigService` being separate from other services.

### Decision: Entity follows standard project conventions
The `MailTemplate` entity uses the same structure as all other entities: `id` (UUID), `created`, `deleted` (soft delete), `version` (optimistic locking), plus domain fields `name`, `subject`, `body`. This keeps the codebase consistent.

### Decision: DAO lives in genossi_mail crate
Since mail templates are conceptually part of the mail domain, the DAO trait and SQLite implementation belong in `genossi_mail` alongside the existing `MailJobDao` and `MailRecipientDao`. The service trait also lives in `genossi_mail`.

### Decision: Seed migration for existing snippets
A SQL migration inserts the two existing templates (formal greeting, informal greeting) with fixed UUIDs. This ensures every deployment gets them. The hardcoded constants in `template.rs` are kept as test fixtures but no longer used at runtime.

### Decision: Name uniqueness enforced at DB level
A UNIQUE constraint on the `name` column in the migration. The service layer also validates before insert to provide a clear error message.

## Risks / Trade-offs

- [Risk: Name collisions on seed] → Fixed UUIDs in migration prevent duplicate inserts on re-run; INSERT OR IGNORE used.
- [Risk: Breaking existing hardcoded references] → The `TEMPLATE_FORMAL`/`TEMPLATE_INFORMAL` constants are only used in tests within `template.rs`. They can remain as test-only constants.
- [Trade-off: No service-level caching] → Templates are read from DB on every request. Acceptable for low-volume CRUD; can add caching later if needed.
