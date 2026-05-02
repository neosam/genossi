# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Common Development Commands

### Build Commands
```bash
# Build entire workspace
cargo build

# Build specific package
cargo build -p genossi_bin

# Build with all features
cargo build --all-features

# Release build
cargo build --release
```

### Test Commands
```bash
# Run all tests in workspace
cargo test

# Run tests for specific package
cargo test -p genossi_service

# Run specific test by name
cargo test test_name

# Run tests with output
cargo test -- --nocapture

# Run end-to-end tests with real HTTP server
cargo test --test e2e_tests

# Run simple integration tests
cargo test --test simple_integration_tests
```

### Code Quality Commands
```bash
# Format code
cargo fmt

# Check formatting without changes
cargo fmt -- --check

# Run clippy for linting
cargo clippy

# Run clippy with all targets
cargo clippy --all-targets --all-features
```

### Database Commands
```bash
# Run migrations (executed automatically on startup)
sqlx migrate run --database-url sqlite:genossi.db --source migrations/sqlite

# Create new migration
sqlx migrate add <migration_name> --source migrations/sqlite

# Prepare offline query data for compilation
DATABASE_URL=sqlite:genossi.db cargo sqlx prepare
```

### Running the Application
```bash
# Run the server (default port 3000)
cargo run --bin genossi

# With environment variables
DATABASE_URL=sqlite:genossi.db SERVER_ADDRESS=0.0.0.0:8080 cargo run --bin genossi

# Access Swagger UI
# http://localhost:3000/swagger-ui/
```

## Architecture Overview

Genossi is a REST API server built with a clean, layered architecture using Rust. The project follows Domain-Driven Design principles with clear separation of concerns.

### Layer Structure

1. **DAO Layer** (`genossi_dao`, `genossi_dao_impl_sqlite`)
   - Defines data access interfaces with minimal implementation requirements
   - Only 3 required methods: `dump_all()`, `create()`, `update()`
   - Default implementations provided for `all()` and `find_by_id()`
   - No delete method - deletion handled at service layer via updates
   - SQLite implementation with SQLx for async database operations
   - Supports soft deletes with `deleted` timestamp field
   - Designed for easy multi-database support

2. **Service Layer** (`genossi_service`, `genossi_service_impl`)
   - Business logic and validation rules
   - Permission and authentication context handling
   - UUID generation service for entity IDs
   - User service for authentication/authorization
   - Transforms DAO errors to service-level errors
   - Handles entity deletion via update operations with `deleted` timestamps

3. **REST Layer** (`genossi_rest`, `genossi_rest_types`)
   - Axum-based HTTP server with async handlers
   - OpenAPI documentation via Utoipa
   - CORS support and middleware for context injection
   - Error handling and response transformation
   - Swagger UI at `/swagger-ui/`
   - ISO8601 datetime format in API responses
   - Flexible JSON deserialization for optional datetime fields

4. **Binary Layer** (`genossi_bin`)
   - Application entry point and dependency injection
   - Database connection pool management
   - Migration execution on startup
   - Service initialization and REST server startup

### Key Design Patterns

- **Dependency Injection**: All layers use trait-based dependencies, enabling easy testing with mockall
- **Repository Pattern**: DAO traits abstract database operations
- **Transaction Management**: Explicit transaction handling with begin/commit/rollback
- **Soft Deletes**: Entities use `deleted` timestamp instead of hard deletion
- **Version Control**: Each entity has a `version` field for optimistic locking
- **Component-First Frontend**: UI must be built from reusable components (`genossi-frontend/src/component/`), not inline HTML/RSX. When identical UI appears on multiple pages, extract it into a shared component. See `genossi-frontend/CLAUDE.md` for details.
- **Audit Logging**: All write operations on Member, MemberAction, MemberDocument, and Application are logged via `audited_create!`, `audited_update!`, `audited_delete!` macros. New entities that require audit logging must implement the `Auditable` trait (`genossi_dao/src/auditable.rs`) and use the audit macros instead of direct DAO calls.

### Audit Log System

- **Auditable Trait**: Entities implement `Auditable` in `genossi_dao` to define `entity_type()`, `entity_id()`, and `audit_fields()` (data fields only, excluding id/version/created/deleted)
- **Audit Macros**: `audited_create!`, `audited_update!`, `audited_delete!` in `genossi_service_impl/src/audit_macros.rs` atomically perform DAO operations and log changes
- **Hash Chain**: Each audit entry contains SHA256 hash linking to the previous entry (`genossi_service_impl/src/audit_log.rs`)
- **One Row Per Field**: Each changed field gets its own audit_log row, grouped by `transaction_id`
- **REST Endpoints**: `GET /api/audit`, `GET /api/audit/{entity_type}/{entity_id}`, `GET /api/audit/verify`
- **Adding Audit to New Entities**: 1) Implement `Auditable` trait on the DAO entity, 2) Add `AuditLogDao` dependency via `gen_service_impl!`, 3) Replace direct DAO calls with audit macros, 4) Wire `audit_log_dao` in `genossi_bin/src/lib.rs`

### Entity Structure

Entities follow a consistent pattern:
- `id`: UUID (stored as BLOB in SQLite)
- `created`: Timestamp of creation
- `deleted`: Optional timestamp for soft delete
- `version`: UUID for optimistic locking
- Entity-specific fields (e.g., `name`, `age` for Person)

### Testing Approach

- **Unit Tests**: Use mockall for mocking dependencies
- **Integration Tests**: In `genossi_rest/tests/` test full API endpoints
- **E2E Tests**: Full end-to-end tests using real HTTP server instances with in-memory SQLite databases
- **Test Server Infrastructure**: `genossi_rest/src/test_server.rs` provides utilities for starting test servers with random ports
- **Test Isolation**: Each test gets its own in-memory database for complete isolation
- **Real HTTP Calls**: E2E tests use `reqwest` client to make actual HTTP requests
- Each layer can be tested independently due to trait boundaries
- Use `cargo test -p <package>` to test specific layers

### Datetime Handling

- **ISO8601 Format**: API responses use ISO8601 datetime format (`2025-09-21T13:25:15.454309545Z`)
- **Flexible Parsing**: Database layer supports multiple datetime formats (ISO8601 and SQLite default)
- **Optional Fields**: API requests can omit datetime fields - they default to `None` during deserialization
- **Custom Serialization**: `genossi_rest_types/src/lib.rs` contains custom ISO8601 serde handlers
- **Backward Compatibility**: Existing SQLite data with default format continues to work

### Environment Variables

- `DATABASE_URL`: SQLite database path (default: `sqlite:genossi.db`)
- `SERVER_ADDRESS`: Server bind address (default: `0.0.0.0:3000`)
- `BASE_PATH`: Base URL for Swagger UI (default: `http://localhost:3000/`)

### Important Files

- `/migrations/sqlite/`: Database migration files
- `/genossi_bin/src/main.rs`: Application entry point
- `/genossi_bin/tests/e2e_tests.rs`: End-to-end testing with real HTTP server
- `/genossi_rest/src/lib.rs`: REST server configuration and startup
- `/genossi_rest/src/test_server.rs`: Test server utilities
- `/genossi_rest_types/src/lib.rs`: ISO8601 datetime serialization
- `/genossi_service_impl/src/macros.rs`: Common implementation macros

### Known Issues & Troubleshooting

- **Database File Access**: Main binary requires valid SQLite file path; use e2e tests for testing without file system dependencies
- **Datetime Parsing Errors**: If datetime parsing fails, check format compatibility between ISO8601 and SQLite storage formats
- **422 Errors on Create**: If 422 errors occur on person creation, verify datetime field deserialization is working correctly with optional fields
- **Test Server Failures**: E2E tests bind to random ports; if port conflicts occur, tests will retry automatically
- **Migration Issues**: Ensure migrations are run on startup; SQLx will handle schema creation automatically

<!-- GSD:project-start source:PROJECT.md -->
## Project

**Genossi**

Mitgliederverwaltungs-Software für Genossenschaften. Ersetzt manuelle Excel-Listen durch eine REST-API mit Dioxus-WASM-Frontend, sodass Vorstände Mitgliederdaten verbandskonform pflegen, Anträge bearbeiten, Dokumente erzeugen und Audit-Spuren hinterlegen können. Ist heute produktiv im Einsatz; der nächste Meilenstein bringt papierlose Anwesenheits-Erfassung auf der Generalversammlung.

**Core Value:** Genossenschaften verwalten ihre Mitglieder ohne Excel — verbandskonform, nachvollziehbar (Audit-Hashchain), und mit weniger manueller Arbeit bei wiederkehrenden Vorgängen wie Anträgen, Dokumenten und Generalversammlungen.

### Constraints

- **Tech stack**: Rust + Axum + SQLx + SQLite Backend, Dioxus WASM Frontend — keine Sprachwechsel oder DB-Wechsel im Scope dieses Milestones
- **Architektur**: Layered DAO/Service/REST muss eingehalten werden; neue Entitäten implementieren bestehende Trait-Patterns — Why: Konsistenz mit gemappter Codebase, Testbarkeit
- **Frontend**: Component-First-Prinzip — keine inline-RSX-Duplikate; identische UI-Bausteine wandern in `genossi-frontend/src/component/` — Why: gelernte Lektion, in `CLAUDE.md` und Memory festgeschrieben
- **Security**: Helfer-QR-Codes sind One-Time-Use; nach Scan invalid — Why: kein unkontrollierter Zugriff auf Mitgliederdaten, auch wenn der QR-Code weitergegeben wird
- **Datenschutz**: Helfer sehen nur Mitgliedsnummer, Name, Titel, Anrede — Why: minimale Datenexposition, DSGVO-konforme Helfer-Funktion
- **Audit-Pflicht**: Bestehende auditierte Entitäten (Member, MemberAction, MemberDocument, Application) müssen weiterhin Audit-Macros verwenden; neue GV-Entitäten benötigen das **nicht**
- **Verbandskonformität**: Genossenschaftsverband akzeptiert Excel-Listen ungern — Software muss als Ersatz so funktionieren, dass das Protokoll der GV nachvollziehbar Anwesenheits-Zahlen ausweist
<!-- GSD:project-end -->

<!-- GSD:stack-start source:codebase/STACK.md -->
## Technology Stack

## Languages
- Rust 2021 edition - Backend API server and services (`genossi_bin`, `genossi_rest`, `genossi_service_impl`)
- Rust 2021 edition - Data access layer with SQLx (`genossi_dao_impl_sqlite`)
- Rust compiled to WASM - Frontend UI with Dioxus (`genossi-frontend`)
## Runtime
- Tokio async runtime 1.35+ - All async server operations
- WASM runtime (browser) - Frontend execution
- Cargo - Rust package manager for workspace
- npm/Node.js - Frontend tooling (Tailwind CSS, Dioxus CLI)
- `Cargo.lock` - Present, committed to repo
## Frameworks
- Axum 0.8.3 - REST API web framework with multipart form support
- Tokio 1.35+ - Async runtime with full feature set
- SQLx 0.8 - Async SQL query executor for SQLite
- SQLite - Embedded relational database with WAL mode support
- Dioxus 0.6.3 - React-like reactive UI framework (Rust-to-WASM)
- Tailwind CSS - Utility-first CSS framework (watch mode during dev)
- Utoipa 5.0 - OpenAPI code generation from Rust types
- Utoipa-Swagger-UI 9.0 - Swagger UI at `/swagger-ui/`
- Mockall 0.13 - Mock trait implementation for unit tests
- Cargo test - Built-in Rust test runner
- Dioxus CLI - Frontend build and dev server (`dx serve`, `dx build`)
- NixOS flake - Development environment with nix (`flake.nix`)
- OpenSpec - Change management system integrated via flake
## Key Dependencies
- `tokio` 1.35 - Async runtime (full features including networking, process, sync)
- `axum` 0.8.3 - HTTP server framework with CORS, cookies, sessions middleware
- `sqlx` 0.8 - Type-safe database access with compile-time query verification
- `serde` 1.0 + `serde_json` 1.0 - Serialization/deserialization for API responses
- `uuid` 1.6 - Entity ID generation (v4 random, serde support)
- `time` 0.3 - DateTime handling with serde + formatting + parsing
- `axum-oidc` 0.6 - Optional OIDC integration (feature-gated, production auth)
- `tower-sessions` 0.14 - Session management middleware
- `tower-cookies` 0.10 - Cookie handling middleware
- `lettre` 0.11 - SMTP email client with tokio1 async + TLS support
- `async-imap` 0.10 - IMAP client for inbox polling
- `tokio-rustls` 0.26 - TLS for IMAP connections
- `webpki-roots` 0.26 - Root CA certificates for TLS
- `mail-parser` 0.9 - RFC822 email parsing
- `typst` 0.14 - Document/PDF template language compiler
- `typst-pdf` 0.14 - PDF generation from Typst documents
- `calamine` 0.26 - Excel file parsing (for imports)
- `sha2` 0.10 - SHA256 hashing for audit hash chain
- `x509-tsp` 0.1.0 - RFC 3161 timestamp protocol (document timestamping)
- `cmpv2` 0.2.0 - CMS signature format parsing
- `der` 0.7 - DER encoding/decoding for X.509 structures
- `cms` 0.2.3 - Cryptographic Message Syntax support
- `const-oid` 0.9 - Object Identifier handling
- `spki` 0.7 - Subject Public Key Info structures
- `rustls` 0.23 - TLS client with ring crypto backend
- `zip` 2.0 - ZIP archive creation/extraction (deflate compression)
- `csv` 1.3 - CSV parsing and generation
- `tar` 0.4 - TAR archive handling (backups)
- `flate2` 1.0 - GZIP compression (backups)
- `path-clean` 1.0 - Path normalization
- `reqwest` 0.11/0.12 - HTTP client (various features builds)
- `tower` 0.5 - Middleware and service abstraction
- `tower-http` 0.6 - CORS, headers middleware
- `tower_governor` 0.6 - Rate limiting middleware
- `http` 1.1 - HTTP types (headers, methods, status codes)
- `minijinja` 2.0 - Email template rendering
- `tracing` 0.1 - Structured logging framework
- `tracing-subscriber` 0.3 - Logging backend with env-filter
- `dioxus-logger` 0.6.2 - Browser console logging
- `wasm-bindgen` 0.2.97 - Rust-to-JavaScript interop
- `wasm-bindgen-futures` 0.4.47 - Async support in WASM
- `serde-wasm-bindgen` 0.6 - Efficient serde with WASM
- `js-sys` 0.3.77 - Direct JavaScript API bindings
- `web-sys` 0.3 - Web APIs (Window, Document, FormData, File, Headers, etc.)
- `gloo-timers` 0.3 - setTimeout/setInterval for WASM
- `futures` 0.3 - Async utilities
- `async-recursion` 1.1 - Recursion support in async functions
- `thiserror` 2.0 - Error type derivation
- `manganis` 0.6.2 - Static asset embedding
## Configuration
- `.env` file format (dotenv 0.15)
- Key variables: `DATABASE_URL`, `SERVER_ADDRESS`, `BASE_PATH`, `APP_URL`, `ISSUER`, `CLIENT_ID`, `CLIENT_SECRET`
- See `.env.oidc.example` for OIDC setup
- `Cargo.toml` workspace with `edition = "2021"`, `resolver = "2"`
- Feature flags:
- Nix flake.nix for reproducible dev environment
- SQLite file-based (default: `sqlite:genossi.db`)
- Migrations in `/migrations/sqlite/` - auto-run on startup via sqlx-cli
## Platform Requirements
- Rust toolchain (1.70+ recommended by 2021 edition)
- Cargo
- SQLx CLI (for migrations)
- SQLite dev libraries
- Node.js (for frontend Tailwind CSS compilation)
- Nix (optional, recommended for reproducible environment)
- Linux/Unix-like OS (developed and deployed on Linux)
- SQLite compatible filesystem (WAL mode requires write access)
- Optional: SMTP server for outgoing mail
- Optional: IMAP server for inbox polling
- Optional: Nextcloud instance for OIDC provider
- Optional: WebDAV/Nextcloud for backup export (accessibility, NOT primary backup)
- Optional: RFC 3161 Timestamp Authority for document timestamping
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->
## Conventions

## Naming Patterns
- Rust modules use `snake_case`: `member_service.rs`, `member_action.rs`, `member_document.rs`
- DAO modules: `*_dao.rs` and `*_dao_impl_sqlite.rs` for implementation
- Service implementation files match service file names: `genossi_service/src/member.rs` vs `genossi_service_impl/src/member.rs`
- REST layer handlers: `member.rs`, `member_action.rs`, `application.rs` in `genossi_rest/src/`
- Test infrastructure: `test_server.rs`, `e2e_tests.rs`
- Handler functions: `get_all_members`, `create_member`, `update_member`, `delete_member` (REST layer, `genossi_rest/src/member.rs`)
- Service methods: `get_all()`, `get()`, `create()`, `update()`, etc. (trait definitions and implementations)
- Internal helpers: `recalc_dates()`, `recalc_migrated()`, `setup_mock_tx()`, `mock_config_enabled()` (snake_case, descriptive)
- Async functions use `async fn` consistently throughout
- Request/response transfer objects suffix with `TO`: `MemberTO`, `ApplicationTO`, `UserPreferenceTO`, `MemberImportResultTO`
- DAO entities use plain names: `Member`, `Application`, `MemberAction`
- Service-level errors follow enum convention: `ServiceError` (DAO equivalent: `DaoError`)
- Mock objects prefix with `Mock`: `MockMemberDao`, `MockTransactionDao`, `MockConfigService`
- Loop variables: single letters (`i`, `item`, `entry`) where context is clear
- Public trait names: `MemberService`, `ApplicationService`, `TransactionDao`, `Transaction` (PascalCase)
- Trait implementations: `MemberServiceImpl<Deps>` pattern (postfix `Impl`, generic over `Deps`)
- Error enums: `ServiceError`, `DaoError`, `RestError` (broad coverage, non-specific error variants)
- State types: `MemberStatus`, `ApplicationStatus`, `ActionType`, `Salutation` (domain-specific enums)
- Dependency injection trait: `MemberServiceDeps`, `ApplicationServiceDeps` (postfix `Deps`)
## Code Style
- Tool: `cargo fmt` (standard Rust formatter, no custom rustfmt.toml)
- Line length: Rust's default (100 chars, but flexible)
- Indentation: 4 spaces (Rust default)
- Tool: `cargo clippy --all-targets --all-features`
- No custom clippy.toml configuration detected
- All workspace members included in lint checks
- No custom path aliases configured; all imports use absolute paths
## Error Handling
- Error type: `DaoError` enum with variants: `DatabaseError(Arc<str>)`, `ParseError(Arc<str>)`, `NotFound`, `ConflictError(Arc<str>)`
- All async DAO methods return `Result<T, DaoError>`
- `From<uuid::Error>` and `From<time::error::Parse>` implementations for automatic conversion
- Error type: `ServiceError` enum with variants: `DataAccess(Arc<str>)`, `EntityNotFound(uuid::Uuid)`, `ValidationError(Vec<ValidationFailureItem>)`, `PermissionDenied`, `InternalError(Arc<str>)`, `Conflict(Arc<str>)`, `Unauthorized`, `SessionExpired`, `AuthenticationFailed`
- `From<DaoError>` implementation maps DAO errors to service errors (NotFound → EntityNotFound)
- Validation errors collect field-level failures: `ValidationFailureItem { field: Arc<str>, message: Arc<str> }`
- Service methods: `async fn method(...) -> Result<T, ServiceError>`
- Error type: `RestError` enum with variants: `NotFound`, `BadRequest(String)`, `Conflict(String)`, `Unauthorized`, `UnsupportedMediaType(String)`, `InternalError(String)`
- `From<ServiceError>` implementation maps domain errors to HTTP status codes:
- `error_handler()` function wraps async blocks and converts `Result<Response, RestError>` to HTTP responses
- Pattern: Wrap handler logic in async block: `error_handler((async { /* logic */ }).await)`
## Logging
- Handlers use `#[instrument(skip(rest_state))]` macro from `tracing` to log function entry (in `genossi_rest/src/member.rs:42`)
- Skip large objects like `rest_state` to avoid verbose logs: `#[instrument(skip(rest_state, ...other_skips))]`
- Log level: INFO for major operations, DEBUG for detailed state
- No explicit log calls in handlers; instrumentation captures entry/exit
- All log output goes through `tracing-subscriber` with `env-filter` support
- Environment variable: `RUST_LOG` controls verbosity (standard tokio/tracing pattern)
## Comments
- Document public trait methods with `///` doc comments (e.g., `genossi_service/src/claim_context.rs`)
- Explain why (not what) for complex logic
- Mark constants with purpose: `const MEMBER_SERVICE_PROCESS: &str = "member-service";` (in service implementation, near top)
- Skip comments for self-documenting code (e.g., function names and types make intent clear)
- Public functions document behavior, edge cases, and return values
- Trait methods document contract (example: `genossi_service/src/claim_context.rs`)
- Inline comments are rare; self-documenting names preferred
- Avoid repeating type info in comments
## Function Design
- Typical handler: 20–50 lines (from setup through error_handler response)
- Service methods: 15–40 lines (permission check, DAO call, conversion)
- Private helpers break complex logic (e.g., `recalc_dates()`, `recalc_migrated()` in `genossi_service_impl/src/member.rs:33`)
- Rest handlers: `State<RestState>`, `Extension<Context>`, `Path(id)`, `Json(body)` (Axum extraction)
- Service methods: context (`Authentication<Self::Context>`), optional transaction (`Option<Self::Transaction>`)
- Avoid large parameter lists; use trait-bound generics for dependencies (`Deps: MemberServiceDeps`)
- `Result<T, Error>` for all fallible operations
- `Arc<[T]>` for collections returned from service (immutable, efficient sharing, example: `get_all()` in `genossi_service_impl/src/member.rs:94`)
- JSON serialization in REST layer: `serde_json::to_string(&object)?` then wrap in `Body::new(...)`
## Module Design
- Trait definitions live in `genossi_service/src/*/` (interface-centric)
- Implementations live in `genossi_service_impl/src/*/` (separate package)
- Re-export commonly used types in `lib.rs`: `pub use session::{SessionServiceImpl, MockSessionServiceImpl};` (example from `genossi_service_impl/src/lib.rs:22`)
- `lib.rs` files declare submodules and selectively re-export public types
- No glob re-exports; explicit `pub use` for clarity
- Example: `genossi_service/src/lib.rs` declares modules but does NOT re-export service traits (consumer imports directly: `use genossi_service::member::MemberService`)
- Service traits are generic over `Context` and `Transaction` types
- Implementations use `gen_service_impl!` macro to reduce boilerplate (see `genossi_service_impl/src/macros.rs`)
- Dependencies injected via trait-bound generic `Deps` with associated types for each DAO/service
## Audit Logging Macros
- `audited_create!`: Performs DAO create, logs all non-None fields
- `audited_update!`: Loads old entity, performs DAO update, logs only changed fields
- `audited_delete!`: Sets `deleted` timestamp, performs soft-delete update, logs all fields as deletion
- Expects `self` to have `audit_log_dao` and `uuid_service` fields
- `$process`: string identifier for the operation (e.g., `MEMBER_SERVICE_PROCESS = "member-service"`)
- `$user_id`: UUID of the actor performing the change
- Handles SHA256 hash chain linking; transaction IDs group field changes
- Member, MemberAction, MemberDocument, Application (must implement `Auditable` trait in `genossi_dao/src/auditable.rs`)
- Any new entity requiring audit trail must: (1) implement `Auditable`, (2) add `AuditLogDao` dependency, (3) use audit macros instead of direct DAO calls
## Transfer Objects (TO) and Conversions
- `MemberTO`: REST representation of Member with serialization support
- `impl From<&Member> for MemberTO`: Convert DAO entity to transfer object
- Serde with custom datetime serialization: `#[serde(with = "iso8601_datetime")]` for optional `PrimitiveDateTime` fields
- Custom serde module: `iso8601_datetime::serialize()` and `iso8601_datetime::deserialize()` (in `genossi_rest_types/src/lib.rs:10`)
- ISO8601 format in API responses; flexible parsing on input
- Optional fields default to `None` during deserialization if omitted
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->
## Architecture

## System Overview
```text
```
## Component Responsibilities
| Component | Responsibility | File |
|-----------|----------------|------|
| Binary | Application entry point, DI wiring, server startup | `genossi_bin/src/{main,lib}.rs` |
| REST Layer | HTTP handlers, request/response, OpenAPI docs | `genossi_rest/src/lib.rs` |
| Service Layer | Business logic, validation, auth context | `genossi_service_impl/src/lib.rs` |
| DAO Layer (trait) | Minimal repository interface (`create`, `update`, `dump_all`, `find_by_id`) | `genossi_dao/src/lib.rs` |
| DAO Layer (impl) | SQLite via SQLx, transaction management | `genossi_dao_impl_sqlite/src/lib.rs` |
| Types | ISO8601 datetime serde, shared REST types | `genossi_rest_types/src/lib.rs` |
| Frontend | Dioxus WASM components, pages, services | `genossi-frontend/src/{component,page,service}` |
| Mail | Email generation, SMTP, IMAP, templates | `genossi_mail/src/lib.rs` |
| Backup | Document and communication sync | `genossi_backup/src/lib.rs` |
| Audit Log | Hash chain verification, transaction tracking | `genossi_service_impl/src/audit_log.rs` |
## Pattern Overview
- **Trait boundaries separate layers** — DAOs are traits, services are generic over dependencies
- **Soft deletes** — Entities use `deleted: Option<PrimitiveDateTime>` instead of hard deletion
- **Optimistic locking** — `version: Uuid` field prevents concurrent update conflicts
- **Minimal DAO interface** — Only 3 required methods: `create`, `update`, `dump_all`
- **Audit-first** — Member, MemberAction, MemberDocument, Application use `audited_*!` macros
- **Context-driven permissions** — Services receive authentication context for authorization
- **Component-first frontend** — Reusable components in `genossi-frontend/src/component/`, never inline RSX
## Layers
- Purpose: Abstract database operations via traits
- Location: `genossi_dao/src/`, `genossi_dao_impl_sqlite/src/`
- Contains: Entity structs, DAO trait definitions, audit trail structures
- Depends on: sqlx, uuid, time crates
- Used by: Service layer via trait bounds
- Key abstractions: `TransactionDao`, `MemberDao`, `ApplicationDao`, `AuditLogDao`
- Purpose: Business logic, validation, permission enforcement, audit logging
- Location: `genossi_service_impl/src/`
- Contains: Service implementations (`*ServiceImpl` structs), audit macros, validation logic
- Depends on: DAO traits, Context type, uuid service
- Used by: REST handlers
- Key services: `MemberServiceImpl`, `ApplicationServiceImpl`, `PermissionServiceImpl`, `SessionServiceImpl`
- Purpose: HTTP request/response handling, authentication middleware, OpenAPI documentation
- Location: `genossi_rest/src/`
- Contains: Axum handlers, middleware, error conversion, Utoipa schema definitions
- Depends on: Service types (via generics), tower, axum, utoipa
- Used by: HTTP clients, Swagger UI
- Key files: `member.rs`, `application.rs`, `audit_log.rs`, `auth_middleware.rs`, `session_management.rs`
- Purpose: Application bootstrap, dependency injection, worker initialization
- Location: `genossi_bin/src/`
- Contains: `RestStateImpl` struct with all service wiring, worker spawn functions
- Depends on: All service/DAO layers, sqlx connection pool
- Used by: `main.rs` entry point
- Responsibilities: Create DAOs → Create Services → Wrap in RestStateImpl → Start workers
- Purpose: User interface via Dioxus WASM
- Location: `genossi-frontend/src/`
- Contains: Reusable components, pages, routing, API client, state management
- Depends on: Dioxus, reqwest (for API calls), Tailwind CSS
- Used by: Browser via WASM
- Critical principle: **Component-first** — extract any UI that appears twice into `src/component/`
## Data Flow
### Primary Request Path (Create Member)
### Update Request Path (Member Soft Delete)
### Audit Log Verification Path
- **At rest** — Persisted in SQLite with BLOB UUIDs and ISO8601 timestamps
- **In transit** — JSON with ISO8601 datetime strings, flexible deserialization (optional datetime fields)
- **In memory** — Service layer holds transaction context; each request is independent (no stateful session except cookies)
- **Optimistic locking** — `version` UUID prevents lost updates; service checks version matches before update
## Key Abstractions
- Purpose: Encapsulate database transaction with begin/commit/rollback
- Examples: `genossi_dao_impl_sqlite::transaction::TransactionImpl`
- Pattern: Trait-based, async/await, passed through entire call stack
- Usage: `TransactionDao::transaction()` → acquire transaction → pass to all DAO calls → `TransactionDao::commit()`
- Purpose: Represent domain objects with consistent structure
- Examples: `MemberEntity`, `ApplicationEntity`, `MemberActionEntity`
- Pattern: `id: Uuid` (BLOB), `created: PrimitiveDateTime`, `deleted: Option<PrimitiveDateTime>`, `version: Uuid`, entity-specific fields
- Audit: Entities marked for audit implement `Auditable` trait with `entity_type()`, `entity_id()`, `audit_fields()`
- Purpose: Represent domain-level errors
- Variants: `DataAccess`, `EntityNotFound`, `ValidationError`, `PermissionDenied`, `Unauthorized`, `Conflict`, `InternalError`, `SessionExpired`, `AuthenticationFailed`
- Conversion: `ServiceError` → `RestError` → HTTP status code
- Purpose: Represent HTTP response errors
- Variants: `NotFound` (404), `BadRequest` (400), `Conflict` (409), `Unauthorized` (401), `UnsupportedMediaType` (415), `InternalError` (500)
- Handler: `error_handler()` wraps async blocks, converts all errors to HTTP responses with JSON bodies
## Entry Points
- Location: `genossi_bin/src/main.rs`
- Triggers: `cargo run --bin genossi` or systemd service
- Responsibilities:
- Location: `genossi-frontend/src/main.rs`
- Triggers: `dx serve` or `dx build`
- Responsibilities:
- Pattern: `/api/{entity_type}/{id}` for REST (GET, POST, PUT, DELETE)
- OpenAPI: `/swagger-ui/` (Utoipa-generated)
- Audit: `GET /api/audit`, `GET /api/audit/{entity_type}/{entity_id}`, `GET /api/audit/verify`
## Architectural Constraints
- **Threading:** Tokio async/await; all handlers and service methods are `async`. Workers run in background tasks.
- **Global state:** `RestStateImpl` contains Arc-wrapped services shared across all request handlers. Mail/inbox/backup workers hold config service references.
- **Circular imports:** Avoided via trait-based design; DAO traits don't depend on service layer.
- **Transaction scope:** Passed from REST handler → Service → DAO. All queries in single transaction must complete before commit.
- **Version conflicts:** Service layer checks `version` UUID before update; mismatch returns `ConflictError`.
- **Soft deletes:** No DELETE queries; updates set `deleted` timestamp. Queries must filter `WHERE deleted IS NULL`.
- **Component extraction:** Frontend components in `genossi-frontend/src/component/`. Pages in `src/page/` MUST use components, never inline RSX.
- **Audit logging:** Only `Member`, `MemberAction`, `MemberDocument`, `Application` are audited. New auditable entities require `Auditable` trait impl + wiring in `RestStateImpl::new()`.
## Anti-Patterns
### Inline RSX in Pages
```rust
#[component]
#[component]
#[component]
```
### Hard Delete Without Audit Trail
```rust
```
### Manual Hash Chain Computation
### Service Creating Its Own Transaction
```rust
```
### Unvalidated User Input to Audit Log
## Error Handling
## Cross-Cutting Concerns
- **OIDC mode** (`--features oidc`): `genossi_service_impl/src/session.rs` validates JWT from Nextcloud, stores session cookie
- **Mock mode** (`--features mock_auth`): `genossi_service/src/permission.rs:MockContext` allows any request for testing
- Middleware extracts context from cookie/header, passes via `Extension<Context>` to handlers
<!-- GSD:architecture-end -->

<!-- GSD:skills-start source:skills/ -->
## Project Skills

| Skill | Description | Path |
|-------|-------------|------|
| openspec-apply-change | Implement tasks from an OpenSpec change. Use when the user wants to start implementing, continue implementation, or work through tasks. | `.claude/skills/openspec-apply-change/SKILL.md` |
| openspec-archive-change | Archive a completed change in the experimental workflow. Use when the user wants to finalize and archive a change after implementation is complete. | `.claude/skills/openspec-archive-change/SKILL.md` |
| openspec-bulk-archive-change | Archive multiple completed changes at once. Use when archiving several parallel changes. | `.claude/skills/openspec-bulk-archive-change/SKILL.md` |
| openspec-continue-change | Continue working on an OpenSpec change by creating the next artifact. Use when the user wants to progress their change, create the next artifact, or continue their workflow. | `.claude/skills/openspec-continue-change/SKILL.md` |
| openspec-explore | Enter explore mode - a thinking partner for exploring ideas, investigating problems, and clarifying requirements. Use when the user wants to think through something before or during a change. | `.claude/skills/openspec-explore/SKILL.md` |
| openspec-ff-change | Fast-forward through OpenSpec artifact creation. Use when the user wants to quickly create all artifacts needed for implementation without stepping through each one individually. | `.claude/skills/openspec-ff-change/SKILL.md` |
| openspec-new-change | Start a new OpenSpec change using the experimental artifact workflow. Use when the user wants to create a new feature, fix, or modification with a structured step-by-step approach. | `.claude/skills/openspec-new-change/SKILL.md` |
| openspec-onboard | Guided onboarding for OpenSpec - walk through a complete workflow cycle with narration and real codebase work. | `.claude/skills/openspec-onboard/SKILL.md` |
| openspec-propose | Propose a new change with all artifacts generated in one step. Use when the user wants to quickly describe what they want to build and get a complete proposal with design, specs, and tasks ready for implementation. | `.claude/skills/openspec-propose/SKILL.md` |
| openspec-sync-specs | Sync delta specs from a change to main specs. Use when the user wants to update main specs with changes from a delta spec, without archiving the change. | `.claude/skills/openspec-sync-specs/SKILL.md` |
| openspec-verify-change | Verify implementation matches change artifacts. Use when the user wants to validate that implementation is complete, correct, and coherent before archiving. | `.claude/skills/openspec-verify-change/SKILL.md` |
| release-version | > Release a new version of genossi. Generates release notes from changes since the last tag, runs cli-update-version.sh with the release notes as tag message, and reports the new version number. Use when the user says "release", "neue Version", "Version releasen", or "/release-version". | `.claude/skills/release-version/SKILL.md` |
<!-- GSD:skills-end -->

<!-- GSD:workflow-start source:GSD defaults -->
## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:
- `/gsd-quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd-debug` for investigation and bug fixing
- `/gsd-execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->

<!-- GSD:profile-start -->
## Developer Profile

> Profile not yet configured. Run `/gsd-profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->
