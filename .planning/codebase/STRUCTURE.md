# Codebase Structure

**Analysis Date:** 2026-05-02

## Directory Layout

```
genossi3/
├── genossi_bin/                          # Binary layer (application entry point)
│   ├── src/
│   │   ├── main.rs                       # Server entry point, env/migrations/workers
│   │   └── lib.rs                        # RestStateImpl, DI wiring, service creation
│   └── Cargo.toml
├── genossi_dao/                          # DAO trait definitions
│   ├── src/
│   │   ├── lib.rs                        # Module exports, DaoError, TransactionDao trait
│   │   ├── member.rs                     # MemberDao trait, MemberEntity, Salutation, MemberStatus enums
│   │   ├── application.rs                # ApplicationDao trait, ApplicationEntity, ApplicationStatus
│   │   ├── member_action.rs              # MemberActionDao trait, ActionType enum
│   │   ├── member_document.rs            # MemberDocumentDao trait
│   │   ├── permission.rs                 # PermissionDao trait
│   │   ├── audit_log.rs                  # AuditLogDao trait, AuditLogEntry struct
│   │   ├── audit_timestamp.rs            # AuditTimestampDao trait
│   │   ├── auditable.rs                  # Auditable trait: entity_type(), entity_id(), audit_fields()
│   │   ├── user_preference.rs            # UserPreferenceDao trait
│   │   └── backup.rs                     # BackupDao trait
│   └── Cargo.toml
├── genossi_dao_impl_sqlite/              # SQLite DAO implementations
│   ├── src/
│   │   ├── lib.rs                        # Module exports, TransactionDaoImpl, TransactionImpl
│   │   ├── transaction.rs                # TransactionImpl: begin/commit/rollback via sqlx
│   │   ├── member.rs                     # MemberDaoImpl: SQL queries, Row → MemberEntity mapping
│   │   ├── application.rs                # ApplicationDaoImpl
│   │   ├── member_action.rs              # MemberActionDaoImpl
│   │   ├── member_document.rs            # MemberDocumentDaoImpl
│   │   ├── permission.rs                 # PermissionDaoImpl
│   │   ├── audit_log.rs                  # AuditLogDaoImpl
│   │   ├── audit_timestamp.rs            # AuditTimestampDaoImpl
│   │   ├── user_preference.rs            # UserPreferenceDaoImpl
│   │   └── backup.rs                     # BackupDaoImpl
│   └── Cargo.toml
├── genossi_service/                      # Service trait definitions
│   ├── src/
│   │   ├── lib.rs                        # Module exports, ServiceError, ValidationFailureItem
│   │   ├── member.rs                     # MemberService trait
│   │   ├── application.rs                # ApplicationService trait
│   │   ├── member_action.rs              # MemberActionService trait
│   │   ├── member_document.rs            # MemberDocumentService trait
│   │   ├── permission.rs                 # PermissionService trait, Authentication enum
│   │   ├── session.rs                    # SessionService trait
│   │   ├── member_import.rs              # MemberImportService trait
│   │   ├── user_preference.rs            # UserPreferenceService trait
│   │   ├── validation.rs                 # ValidationService trait
│   │   ├── document_storage.rs           # DocumentStorage trait
│   │   ├── auth_types.rs                 # AuthenticatedContext
│   │   ├── claim_context.rs              # Claim parsing from OIDC
│   │   ├── uuid_service.rs               # UuidService trait
│   │   ├── user_service.rs               # UserService trait (for OIDC/mock auth)
│   │   ├── timestamp.rs                  # TimestampService trait
│   │   └── template.rs                   # TemplateService trait
│   └── Cargo.toml
├── genossi_service_impl/                 # Service implementations + macros
│   ├── src/
│   │   ├── lib.rs                        # Module exports
│   │   ├── member.rs                     # MemberServiceImpl, create/update/delete, permission checks
│   │   ├── application.rs                # ApplicationServiceImpl
│   │   ├── member_action.rs              # MemberActionServiceImpl
│   │   ├── member_document.rs            # MemberDocumentServiceImpl
│   │   ├── member_import.rs              # MemberImportServiceImpl (Excel parsing via calamine)
│   │   ├── permission.rs                 # PermissionServiceImpl, role checks
│   │   ├── session.rs                    # SessionServiceImpl (OIDC JWT validation)
│   │   ├── user_preference.rs            # UserPreferenceServiceImpl
│   │   ├── validation.rs                 # ValidationServiceImpl, field validators
│   │   ├── document_storage.rs           # DocumentStorageImpl (filesystem)
│   │   ├── uuid_service.rs               # UuidServiceImpl (UUID v4 generation)
│   │   ├── user_service.rs               # UserService impls for auth modes
│   │   ├── timestamp.rs                  # TimestampServiceImpl (RFC 3161 via HTTP)
│   │   ├── timestamp_worker.rs           # Background worker for bulk timestamps
│   │   ├── template_storage.rs           # TemplateStorageImpl (mail templates, Typst PDF)
│   │   ├── pdf_generation.rs             # PDF generation from Typst
│   │   ├── rfc3161.rs                    # RFC 3161 timestamp protocol
│   │   ├── audit_log.rs                  # build_create/update/delete_entries(), hash chain computation
│   │   ├── audit_macros.rs               # audited_create!, audited_update!, audited_delete! macros
│   │   ├── macros.rs                     # Helper macros (gen_service_impl!, etc.)
│   │   └── Cargo.toml
├── genossi_rest/                         # REST API layer (Axum handlers)
│   ├── src/
│   │   ├── lib.rs                        # Router setup, RestStateDef trait, RestError, error_handler()
│   │   ├── member.rs                     # GET/POST/PUT/DELETE /api/member, import endpoint
│   │   ├── application.rs                # GET/POST/PUT /api/application
│   │   ├── member_action.rs              # GET/POST/PUT/DELETE /api/member-action
│   │   ├── member_document.rs            # GET/POST/DELETE /api/member-document
│   │   ├── permission.rs                 # GET /api/permission (role queries)
│   │   ├── audit_log.rs                  # GET /api/audit, /api/audit/{type}/{id}, /api/audit/verify
│   │   ├── audit_timestamp.rs            # GET /api/audit-timestamp (timestamp history)
│   │   ├── user_preference.rs            # GET/PUT /api/user-preference
│   │   ├── session.rs                    # GET /api/session (session info)
│   │   ├── session_management.rs         # GET /api/session/info, POST /api/session/revoke
│   │   ├── static_document.rs            # GET /api/static-document
│   │   ├── template.rs                   # GET/POST /api/template (mail templates)
│   │   ├── validation.rs                 # GET /api/validation/* (validation rules for UI)
│   │   ├── public_stats.rs               # GET /api/stats (cached public statistics)
│   │   ├── backup.rs                     # GET /api/backup (backup status)
│   │   ├── auth.rs                       # OAuth2 login/logout, JWT validation
│   │   ├── auth_middleware.rs            # Extracts Context from request, injects via Extension
│   │   ├── http_util.rs                  # Helper functions for HTTP handling
│   │   ├── mail_footer.rs                # Email footer generation
│   │   ├── test_server.rs                # Test utility: starts server on random port
│   │   └── dev.rs                        # Debug endpoints (only in debug_assertions)
│   ├── tests/
│   │   ├── e2e_tests.rs                  # End-to-end tests with real server
│   │   └── simple_integration_tests.rs   # Integration tests
│   └── Cargo.toml
├── genossi_rest_types/                   # Shared REST types (frontend ↔ backend)
│   ├── src/
│   │   ├── lib.rs                        # MemberTO, ApplicationTO, etc. (transfer objects)
│   │   │                                 # iso8601_datetime, iso8601_date serde modules
│   │   └── Cargo.toml
├── genossi-frontend/                     # Dioxus WASM frontend
│   ├── src/
│   │   ├── main.rs                       # Entry point: App router, i18n setup
│   │   ├── lib.rs                        # Re-exports
│   │   ├── component/                    # Reusable UI components (MUST extract duplicates)
│   │   │   ├── mod.rs                    # Exports all components
│   │   │   ├── base_components.rs        # Basic UI (buttons, inputs)
│   │   │   ├── member_search.rs          # Member search/filter component
│   │   │   ├── application_form.rs       # ApplicationForm component (create/edit modes)
│   │   │   ├── application_detail.rs     # ApplicationDetail component
│   │   │   ├── error_alert.rs            # Error display component
│   │   │   ├── status_bar.rs             # Status indicator component
│   │   │   ├── pagination_controls.rs    # Pagination buttons
│   │   │   ├── page_size_select.rs       # Items-per-page selector
│   │   │   ├── collapsible_section.rs    # Accordion-like section
│   │   │   ├── communication_timeline.rs # Timeline of communications
│   │   │   ├── timestamp_section.rs      # RFC 3161 timestamp display
│   │   │   ├── tsa_config.rs             # Timestamp authority configuration
│   │   │   ├── modal.rs                  # Modal dialog wrapper
│   │   │   ├── top_bar.rs                # Header navigation bar
│   │   │   ├── footer.rs                 # Footer component
│   │   │   ├── dropdown_base.rs          # Reusable dropdown component
│   │   │   ├── nav_group.rs              # Navigation grouping
│   │   │   ├── overlay.rs                # Modal overlay
│   │   │   ├── revoke_sessions_button.rs # Revoke all sessions
│   │   │   ├── wordpress_integration.rs  # WordPress plugin config
│   │   │   ├── mail_compose/            # Email composition (sub-components)
│   │   │   └── inbox/                   # Inbox views (sub-components)
│   │   ├── page/                        # Full page components (use components, no inline RSX)
│   │   │   ├── member_list.rs           # List all members
│   │   │   ├── member_detail.rs         # Single member view/edit
│   │   │   ├── member_create.rs         # Create new member
│   │   │   ├── application_list.rs      # Applications overview
│   │   │   ├── application_detail.rs    # Single application
│   │   │   ├── dashboard.rs             # Main landing page
│   │   │   └── ...                      # Other pages
│   │   ├── service/                     # Business logic services
│   │   │   ├── mod.rs                   # Service module exports
│   │   │   ├── api.rs                   # REST API client (reqwest-based)
│   │   │   ├── loader.rs                # Data loading utilities
│   │   │   └── ...                      # Other services
│   │   ├── state/                       # Data models and state management
│   │   │   ├── mod.rs                   # State module exports
│   │   │   ├── member.rs                # Member state/model
│   │   │   ├── application.rs           # Application state/model
│   │   │   └── ...                      # Other domain models
│   │   ├── i18n/                        # Internationalization (En, De, Cs)
│   │   │   ├── mod.rs                   # Key enum, Locale enum, i18n! macro
│   │   │   ├── en.rs                    # English translations
│   │   │   ├── de.rs                    # German translations
│   │   │   ├── cs.rs                    # Czech translations
│   │   │   └── service.rs               # i18n service (locale switching)
│   │   ├── router.rs                    # Dioxus Router setup
│   │   ├── api.rs                       # Shared API client functions
│   │   └── loader.rs                    # Data loading utilities
│   ├── rest-types/                      # Shared types (symlink or copy of genossi_rest_types)
│   │   ├── src/
│   │   │   └── lib.rs                   # Re-exports from genossi_rest_types
│   │   └── Cargo.toml
│   ├── Dioxus.toml                      # Dioxus config, backend proxy URLs
│   ├── tailwind.config.js               # Tailwind CSS custom colors, zoom classes
│   ├── input.css                        # Tailwind input CSS
│   └── Cargo.toml
├── genossi_mail/                         # Email service (SMTP, IMAP, templates)
│   ├── src/
│   │   ├── lib.rs                       # Module exports
│   │   ├── service.rs                   # MailServiceImpl (SMTP client)
│   │   ├── template.rs                  # Mail template rendering (Typst → HTML/plain text)
│   │   ├── inbox_service.rs             # Inbox polling via IMAP
│   │   ├── static_document_service.rs   # Static document attachment handling
│   │   ├── rest.rs                      # REST trait definitions for MailRestState
│   │   ├── rest_templates.rs            # Template REST endpoints
│   │   ├── inbox_rest.rs                # Inbox REST endpoints
│   │   ├── communication_rest.rs        # Communication history endpoints
│   │   └── Cargo.toml
├── genossi_backup/                       # Document sync and backup
│   ├── src/
│   │   ├── lib.rs                       # Module exports
│   │   ├── service.rs                   # Backup sync logic
│   │   └── Cargo.toml
├── genossi_config/                       # Configuration management
│   ├── src/
│   │   ├── lib.rs                       # Module exports
│   │   ├── service.rs                   # ConfigServiceImpl (CRUD for config entries)
│   │   ├── rest.rs                      # ConfigRestState trait
│   │   └── Cargo.toml
├── migrations/sqlite/                    # Database schema migrations
│   ├── 20250129000000_create_auth_tables.sql
│   ├── 20260331000000_create_member_table.sql
│   ├── 20260401000000_create_application_table.sql
│   ├── 20260415000000_create_audit_log_table.sql
│   └── ...
├── documents/                            # User-facing documentation
├── templates/                            # Typst email templates
│   ├── mail/
│   ├── pdf/
│   └── ...
├── openspec/                             # OpenSpec schema definitions
├── Cargo.toml                            # Workspace root
├── Cargo.lock                            # Dependency lock file
└── .planning/codebase/                   # GSD planning documents
    ├── ARCHITECTURE.md
    └── STRUCTURE.md
```

## Directory Purposes

**genossi_bin/:**
- Purpose: Binary crate with application entry point and DI wiring
- Contains: `main.rs` (server bootstrap), `lib.rs` (RestStateImpl)
- Key files: `genossi_bin/src/main.rs` (dotenv → database → migrations → services → workers), `genossi_bin/src/lib.rs` (type aliases, RestStateImpl::new() with all DAO/service creation)

**genossi_dao/:**
- Purpose: DAO trait definitions (repository pattern interface)
- Contains: Entity structs, trait definitions, error types
- Key files: `genossi_dao/src/lib.rs` (exports, TransactionDao), `genossi_dao/src/member.rs` (MemberDao, MemberEntity), `genossi_dao/src/auditable.rs` (Auditable trait for audit logging)

**genossi_dao_impl_sqlite/:**
- Purpose: SQLite implementations of DAO traits via SQLx
- Contains: SQL queries, Row mapping to entities, transaction management
- Key files: `genossi_dao_impl_sqlite/src/transaction.rs` (TransactionImpl::begin/commit/rollback), `genossi_dao_impl_sqlite/src/member.rs` (SQL INSERT/UPDATE/SELECT for members)

**genossi_service/:**
- Purpose: Service trait definitions (business logic interface)
- Contains: Trait definitions, error types, authentication types
- Key files: `genossi_service/src/member.rs` (MemberService trait), `genossi_service/src/permission.rs` (PermissionService, Authentication enum), `genossi_service/src/auth_types.rs` (AuthenticatedContext from OIDC)

**genossi_service_impl/:**
- Purpose: Service implementations with business logic and audit macros
- Contains: Service impl structs, validation logic, audit macro definitions
- Key files: `genossi_service_impl/src/member.rs` (MemberServiceImpl with permission checks), `genossi_service_impl/src/audit_macros.rs` (audited_create/update/delete! macros), `genossi_service_impl/src/audit_log.rs` (SHA256 hash chain logic)

**genossi_rest/:**
- Purpose: HTTP request handlers (Axum-based REST layer)
- Contains: Axum route handlers, error conversion, OpenAPI definitions
- Key files: `genossi_rest/src/lib.rs` (router setup, RestStateDef trait), `genossi_rest/src/member.rs` (GET/POST/PUT/DELETE handlers for members), `genossi_rest/src/auth_middleware.rs` (context extraction from request)

**genossi_rest_types/:**
- Purpose: Shared data types for REST API (frontend ↔ backend)
- Contains: Transfer objects (MemberTO, ApplicationTO), ISO8601 serde modules
- Key files: `genossi_rest_types/src/lib.rs` (MemberTO, ApplicationTO, iso8601_datetime module with custom serialization)

**genossi-frontend/:**
- Purpose: Dioxus WASM frontend application
- Contains: Components, pages, services, routing, i18n
- Key directories:
  - `src/component/` — Reusable UI components (MUST extract duplicates here)
  - `src/page/` — Full page components (MUST use components, no inline RSX)
  - `src/service/` — API client, data loaders
  - `src/state/` — Domain models
  - `src/i18n/` — Multi-language support (En, De, Cs)

**genossi_mail/:**
- Purpose: Email service (SMTP sending, IMAP polling, template rendering)
- Contains: MailService, inbox polling, Typst template rendering
- Key files: `genossi_mail/src/service.rs` (SMTP client via lettre), `genossi_mail/src/inbox_service.rs` (IMAP inbox polling)

**genossi_backup/:**
- Purpose: Document and communication synchronization
- Contains: Backup sync logic, document tracking
- Key files: `genossi_backup/src/service.rs` (sync logic)

**genossi_config/:**
- Purpose: Runtime configuration management (config_entries table)
- Contains: ConfigService, CRUD for runtime config
- Key files: `genossi_config/src/service.rs` (ConfigServiceImpl)

**migrations/sqlite/:**
- Purpose: Database schema version control
- Contains: SQL migration files, numbered by timestamp
- Key patterns: Create table migrations, seed data migrations, alter table migrations
- Executed automatically on startup via `sqlx::migrate!()` macro

**templates/:**
- Purpose: Email template definitions in Typst
- Contains: Mail and PDF templates
- Key pattern: Templates are provisioned on startup via `template_storage().provision_defaults()`

## Key File Locations

**Entry Points:**
- `genossi_bin/src/main.rs`: Server bootstrap (dotenv, database connection, migrations, workers)
- `genossi-frontend/src/main.rs`: WASM app bootstrap (router, i18n, component mount)

**Configuration:**
- `genossi_bin/src/lib.rs`: RestStateImpl DI wiring (all services created here)
- `Dioxus.toml`: Frontend proxy configuration (backend URLs)
- `tailwind.config.js`: Tailwind CSS custom colors, zoom classes
- `.env`: Runtime config (DATABASE_URL, SERVER_ADDRESS, mail credentials)

**Core Logic:**
- `genossi_service_impl/src/member.rs`: Member CRUD + permission checks
- `genossi_service_impl/src/audit_log.rs`: SHA256 hash chain + audit entry generation
- `genossi_service_impl/src/audit_macros.rs`: audited_create!, audited_update!, audited_delete! macros
- `genossi_rest/src/lib.rs`: REST router, error handling, context extraction
- `genossi_rest/src/member.rs`: Member REST handlers (GET all, GET one, POST create, PUT update, DELETE soft-delete)
- `genossi-frontend/src/component/member_search.rs`: Member search component (reusable across pages)

**Testing:**
- `genossi_rest/tests/e2e_tests.rs`: End-to-end tests with real server
- `genossi_rest/src/test_server.rs`: TestServer utility for starting test servers on random ports
- Individual service test modules (e.g., `genossi_service_impl/src/member.rs` contains unit tests)

**Database:**
- `migrations/sqlite/`: All migration files
- Default database: `genossi.db` in project root (configurable via `DATABASE_URL`)

## Naming Conventions

**Files:**
- Service implementations: `{entity}.rs` in `genossi_service_impl/src/`
- REST handlers: `{entity}.rs` in `genossi_rest/src/`
- DAO traits: `{entity}.rs` in `genossi_dao/src/`
- DAO implementations: `{entity}.rs` in `genossi_dao_impl_sqlite/src/`
- Components: `{component_name}.rs` in `genossi-frontend/src/component/`
- Pages: `{page_name}.rs` in `genossi-frontend/src/page/`

**Types & Functions:**
- Service types: `{Entity}ServiceImpl` (e.g., `MemberServiceImpl`)
- DAO types: `{Entity}DaoImpl` (e.g., `MemberDaoImpl`)
- REST transfer objects: `{Entity}TO` (e.g., `MemberTO`)
- Handler functions: `{method}_{entity}` (e.g., `get_member`, `create_member`)
- Component functions: PascalCase (e.g., `MemberSearch`, `ApplicationForm`)

**Entity Conventions:**
- Entity struct: `{Entity}Entity` (e.g., `MemberEntity`) — stored in DAO layer
- Transfer object: `{Entity}TO` (e.g., `MemberTO`) — used in REST API
- Service input: `{Entity}Input` or struct with builder pattern
- DAO query results: Maps sqlx::Row directly to Entity via from_row/try_from

## Where to Add New Code

**New Feature (e.g., new entity like "MemberNotes"):**
1. **DAO Trait:** Create `genossi_dao/src/member_notes.rs` with:
   - `MemberNotesEntity` struct (id, created, deleted, version, + entity-specific fields)
   - `MemberNotesDao` trait with `create`, `update`, `find_by_id`, `dump_all`
   - Add `Auditable` impl if audit logging needed

2. **DAO Implementation:** Create `genossi_dao_impl_sqlite/src/member_notes.rs` with:
   - SQL `CREATE TABLE member_notes (...)` migration in `migrations/sqlite/`
   - `MemberNotesDaoImpl` with SQLx query implementations
   - `impl From<sqlx::Row> for MemberNotesEntity`

3. **Service Trait:** Create `genossi_service/src/member_notes.rs` with:
   - `MemberNotesService` trait (create, update, delete, get)
   - Associated type `type Context: ...`

4. **Service Implementation:** Create `genossi_service_impl/src/member_notes.rs` with:
   - `MemberNotesServiceImpl` struct
   - Implement permission checks via `PermissionService`
   - Use `audited_create!`, `audited_update!`, `audited_delete!` macros

5. **REST Handler:** Create `genossi_rest/src/member_notes.rs` with:
   - Axum handlers: `get_all_member_notes`, `get_member_note`, `create_member_note`, etc.
   - `#[utoipa::path(...)]` attributes for OpenAPI docs
   - Call service methods, convert errors via `From` impl

6. **REST Types:** Add to `genossi_rest_types/src/lib.rs`:
   - `MemberNotesTO` struct with serde attrs

7. **DI Wiring:** In `genossi_bin/src/lib.rs`:
   - Create `MemberNotesDaoImpl` with pool
   - Create `MemberNotesServiceImpl` with dependencies
   - Add to `RestStateImpl` struct and `new()` method
   - Implement `RestStateDef` trait associate types

8. **Router:** In `genossi_rest/src/lib.rs`:
   - Add `member_notes::generate_route()` to router

**New Component (e.g., "MemberNotesForm" used by multiple pages):**
1. Create `genossi-frontend/src/component/member_notes_form.rs`:
   - Define `#[component] fn MemberNotesForm(...) -> Element { rsx! { ... } }`
   - Export in `genossi-frontend/src/component/mod.rs`

2. Use in pages:
   ```rust
   // genossi-frontend/src/page/member_detail.rs
   rsx! { MemberNotesForm(on_save: |notes| { /* save */ }) }
   ```

**Utilities:**
- Shared helpers (functions, macros): `genossi_service_impl/src/macros.rs` (see `gen_service_impl!` macro)
- Validation rules: Add to `genossi_service_impl/src/validation.rs`
- Response conversion: Add `From<...> for RestError` impl in `genossi_rest/src/lib.rs`

## Special Directories

**migrations/sqlite/:**
- Purpose: Schema version control via SQLx
- Generated: No (hand-written SQL)
- Committed: Yes
- Pattern: Each file is numbered by timestamp (e.g., `20260415000000_create_audit_log_table.sql`)
- Auto-run: On startup via `sqlx::migrate!()` in `genossi_bin/src/main.rs`

**documents/:**
- Purpose: User-facing documentation
- Generated: No (hand-written Markdown/Typst)
- Committed: Yes

**templates/:**
- Purpose: Email and PDF template definitions
- Generated: No (Typst source files)
- Committed: Yes
- Runtime: Templates provisioned on startup

**openspec/:**
- Purpose: OpenAPI/spec definitions
- Generated: Partially (Utoipa generates route docs)
- Committed: Yes

**.planning/codebase/:**
- Purpose: GSD planning documents (architecture, structure, conventions, concerns, testing)
- Generated: Yes (by `gsd-map-codebase` command)
- Committed: Yes (tracked in git)

**target/:**
- Purpose: Cargo build artifacts
- Generated: Yes (by `cargo build`)
- Committed: No (.gitignored)

**.sqlx/:**
- Purpose: SQLx offline query metadata (for compile-time checking without database)
- Generated: Yes (by `cargo sqlx prepare`)
- Committed: Yes (needed for CI)

**genossi-frontend/src/component/**
- Purpose: **Reusable UI components** (CRITICAL: component-first principle)
- Rule: **NEVER inline RSX in pages.** If a component is used twice, extract it here.
- Example: `MemberSearch`, `ApplicationForm`, `ErrorAlert` are shared across pages.

**genossi-frontend/src/page/**
- Purpose: Full page components
- Rule: **MUST compose components from `src/component/`.** Pages read like high-level UI descriptions, not raw HTML.
- Example: `MemberListPage` uses `MemberSearch` and `PaginationControls` instead of defining search logic inline.

---

*Structure analysis: 2026-05-02*
