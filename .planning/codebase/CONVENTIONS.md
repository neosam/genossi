# Coding Conventions

**Analysis Date:** 2026-05-02

## Naming Patterns

**Files:**
- Rust modules use `snake_case`: `member_service.rs`, `member_action.rs`, `member_document.rs`
- DAO modules: `*_dao.rs` and `*_dao_impl_sqlite.rs` for implementation
- Service implementation files match service file names: `genossi_service/src/member.rs` vs `genossi_service_impl/src/member.rs`
- REST layer handlers: `member.rs`, `member_action.rs`, `application.rs` in `genossi_rest/src/`
- Test infrastructure: `test_server.rs`, `e2e_tests.rs`

**Functions:**
- Handler functions: `get_all_members`, `create_member`, `update_member`, `delete_member` (REST layer, `genossi_rest/src/member.rs`)
- Service methods: `get_all()`, `get()`, `create()`, `update()`, etc. (trait definitions and implementations)
- Internal helpers: `recalc_dates()`, `recalc_migrated()`, `setup_mock_tx()`, `mock_config_enabled()` (snake_case, descriptive)
- Async functions use `async fn` consistently throughout

**Variables:**
- Request/response transfer objects suffix with `TO`: `MemberTO`, `ApplicationTO`, `UserPreferenceTO`, `MemberImportResultTO`
- DAO entities use plain names: `Member`, `Application`, `MemberAction`
- Service-level errors follow enum convention: `ServiceError` (DAO equivalent: `DaoError`)
- Mock objects prefix with `Mock`: `MockMemberDao`, `MockTransactionDao`, `MockConfigService`
- Loop variables: single letters (`i`, `item`, `entry`) where context is clear

**Types:**
- Public trait names: `MemberService`, `ApplicationService`, `TransactionDao`, `Transaction` (PascalCase)
- Trait implementations: `MemberServiceImpl<Deps>` pattern (postfix `Impl`, generic over `Deps`)
- Error enums: `ServiceError`, `DaoError`, `RestError` (broad coverage, non-specific error variants)
- State types: `MemberStatus`, `ApplicationStatus`, `ActionType`, `Salutation` (domain-specific enums)
- Dependency injection trait: `MemberServiceDeps`, `ApplicationServiceDeps` (postfix `Deps`)

## Code Style

**Formatting:**
- Tool: `cargo fmt` (standard Rust formatter, no custom rustfmt.toml)
- Line length: Rust's default (100 chars, but flexible)
- Indentation: 4 spaces (Rust default)

**Linting:**
- Tool: `cargo clippy --all-targets --all-features`
- No custom clippy.toml configuration detected
- All workspace members included in lint checks

**Imports and Module Organization:**
```rust
// Order observed in genossi_rest/src/member.rs
use axum::{ /* framework imports */ };
use genossi_mail::service::MailService;        // Domain service crates
use genossi_rest_types::{ /* DTOs */ };
use genossi_service::member::MemberService;    // Service trait imports
use std::sync::Arc;                             // Standard library
use tracing::instrument;                        // Logging/instrumentation
use utoipa::{ /* OpenAPI */ };                  // OpenAPI documentation
use uuid::Uuid;                                 // UUID support
use crate::{ /* local modules */ };             // Local crate imports
```

**Path Aliases:**
- No custom path aliases configured; all imports use absolute paths

## Error Handling

**Pattern:** Layered error transformation with conversion traits.

**DAO Layer** (`genossi_dao/src/lib.rs`):
- Error type: `DaoError` enum with variants: `DatabaseError(Arc<str>)`, `ParseError(Arc<str>)`, `NotFound`, `ConflictError(Arc<str>)`
- All async DAO methods return `Result<T, DaoError>`
- `From<uuid::Error>` and `From<time::error::Parse>` implementations for automatic conversion

**Service Layer** (`genossi_service/src/lib.rs`):
- Error type: `ServiceError` enum with variants: `DataAccess(Arc<str>)`, `EntityNotFound(uuid::Uuid)`, `ValidationError(Vec<ValidationFailureItem>)`, `PermissionDenied`, `InternalError(Arc<str>)`, `Conflict(Arc<str>)`, `Unauthorized`, `SessionExpired`, `AuthenticationFailed`
- `From<DaoError>` implementation maps DAO errors to service errors (NotFound → EntityNotFound)
- Validation errors collect field-level failures: `ValidationFailureItem { field: Arc<str>, message: Arc<str> }`
- Service methods: `async fn method(...) -> Result<T, ServiceError>`

**REST Layer** (`genossi_rest/src/lib.rs`):
- Error type: `RestError` enum with variants: `NotFound`, `BadRequest(String)`, `Conflict(String)`, `Unauthorized`, `UnsupportedMediaType(String)`, `InternalError(String)`
- `From<ServiceError>` implementation maps domain errors to HTTP status codes:
  - `EntityNotFound` → `404`
  - `ValidationError` → `400` (with field details)
  - `PermissionDenied` → `401`
  - `Conflict` → `409`
  - All others → `500`
- `error_handler()` function wraps async blocks and converts `Result<Response, RestError>` to HTTP responses
- Pattern: Wrap handler logic in async block: `error_handler((async { /* logic */ }).await)`

**No thiserror or anyhow:** Error types are custom enums with manual `impl Display` and `From` conversions.

## Logging

**Framework:** `tracing` crate for structured logging

**Pattern:**
- Handlers use `#[instrument(skip(rest_state))]` macro from `tracing` to log function entry (in `genossi_rest/src/member.rs:42`)
- Skip large objects like `rest_state` to avoid verbose logs: `#[instrument(skip(rest_state, ...other_skips))]`
- Log level: INFO for major operations, DEBUG for detailed state
- No explicit log calls in handlers; instrumentation captures entry/exit

**Structured Logging:**
- All log output goes through `tracing-subscriber` with `env-filter` support
- Environment variable: `RUST_LOG` controls verbosity (standard tokio/tracing pattern)

## Comments

**When to Comment:**
- Document public trait methods with `///` doc comments (e.g., `genossi_service/src/claim_context.rs`)
- Explain why (not what) for complex logic
- Mark constants with purpose: `const MEMBER_SERVICE_PROCESS: &str = "member-service";` (in service implementation, near top)
- Skip comments for self-documenting code (e.g., function names and types make intent clear)

**Doc Comments:**
```rust
/// Look up the MIME type for a given extension (case-insensitive).
/// Returns `None` if the extension is not in the whitelist.
pub fn lookup_allowed_mime(extension: &str) -> Option<&'static str>
```
- Public functions document behavior, edge cases, and return values
- Trait methods document contract (example: `genossi_service/src/claim_context.rs`)

**No Excessive Comments:**
- Inline comments are rare; self-documenting names preferred
- Avoid repeating type info in comments

## Function Design

**Size:**
- Typical handler: 20–50 lines (from setup through error_handler response)
- Service methods: 15–40 lines (permission check, DAO call, conversion)
- Private helpers break complex logic (e.g., `recalc_dates()`, `recalc_migrated()` in `genossi_service_impl/src/member.rs:33`)

**Parameters:**
- Rest handlers: `State<RestState>`, `Extension<Context>`, `Path(id)`, `Json(body)` (Axum extraction)
- Service methods: context (`Authentication<Self::Context>`), optional transaction (`Option<Self::Transaction>`)
- Avoid large parameter lists; use trait-bound generics for dependencies (`Deps: MemberServiceDeps`)

**Return Values:**
- `Result<T, Error>` for all fallible operations
- `Arc<[T]>` for collections returned from service (immutable, efficient sharing, example: `get_all()` in `genossi_service_impl/src/member.rs:94`)
- JSON serialization in REST layer: `serde_json::to_string(&object)?` then wrap in `Body::new(...)`

## Module Design

**Exports:**
- Trait definitions live in `genossi_service/src/*/` (interface-centric)
- Implementations live in `genossi_service_impl/src/*/` (separate package)
- Re-export commonly used types in `lib.rs`: `pub use session::{SessionServiceImpl, MockSessionServiceImpl};` (example from `genossi_service_impl/src/lib.rs:22`)

**Barrel Files:**
- `lib.rs` files declare submodules and selectively re-export public types
- No glob re-exports; explicit `pub use` for clarity
- Example: `genossi_service/src/lib.rs` declares modules but does NOT re-export service traits (consumer imports directly: `use genossi_service::member::MemberService`)

**Traits and Generics:**
- Service traits are generic over `Context` and `Transaction` types
- Implementations use `gen_service_impl!` macro to reduce boilerplate (see `genossi_service_impl/src/macros.rs`)
- Dependencies injected via trait-bound generic `Deps` with associated types for each DAO/service

## Audit Logging Macros

**Pattern:** Three macros automate audit trail creation for audited entities (Member, MemberAction, MemberDocument, Application).

**Macros:**
- `audited_create!`: Performs DAO create, logs all non-None fields
- `audited_update!`: Loads old entity, performs DAO update, logs only changed fields
- `audited_delete!`: Sets `deleted` timestamp, performs soft-delete update, logs all fields as deletion

**Usage Pattern** (from `genossi_service_impl/src/audit_macros.rs:6`):
```rust
audited_create!($self, $dao, $entity, $process, $user_id, $tx)
```
- Expects `self` to have `audit_log_dao` and `uuid_service` fields
- `$process`: string identifier for the operation (e.g., `MEMBER_SERVICE_PROCESS = "member-service"`)
- `$user_id`: UUID of the actor performing the change
- Handles SHA256 hash chain linking; transaction IDs group field changes

**Mandatory for:**
- Member, MemberAction, MemberDocument, Application (must implement `Auditable` trait in `genossi_dao/src/auditable.rs`)
- Any new entity requiring audit trail must: (1) implement `Auditable`, (2) add `AuditLogDao` dependency, (3) use audit macros instead of direct DAO calls

## Transfer Objects (TO) and Conversions

**Pattern:** `*TO` suffix for REST/API DTOs; `From<&DAO>` for conversion.

**Example** (from `genossi_rest_types/src/lib.rs`):
- `MemberTO`: REST representation of Member with serialization support
- `impl From<&Member> for MemberTO`: Convert DAO entity to transfer object
- Serde with custom datetime serialization: `#[serde(with = "iso8601_datetime")]` for optional `PrimitiveDateTime` fields

**Datetime Handling:**
- Custom serde module: `iso8601_datetime::serialize()` and `iso8601_datetime::deserialize()` (in `genossi_rest_types/src/lib.rs:10`)
- ISO8601 format in API responses; flexible parsing on input
- Optional fields default to `None` during deserialization if omitted

---

*Convention analysis: 2026-05-02*
