# Architecture

**Analysis Date:** 2026-05-02

## System Overview

Genossi is a member management REST API with a Dioxus WASM frontend. The backend follows Domain-Driven Design with trait-based dependency injection, enabling independent layer testing and multi-database support. The system manages members, member actions, member documents, applications, and audit logging with strict permission controls.

```text
┌────────────────────────────────────────────────────────────────────────┐
│                          Frontend Layer (WASM)                          │
│  Dioxus Components (`genossi-frontend/src/component/*`)                │
│  Pages/Routes (`genossi-frontend/src/page/*`)                          │
│  Services: API client, state management, i18n                          │
└──────────────────────────────┬───────────────────────────────────────┘
                               │ HTTP REST API (Axum)
┌──────────────────────────────▼───────────────────────────────────────┐
│                      REST Layer (Handlers)                             │
│  `genossi_rest/src/{member,application,audit_log,validation,...}.rs` │
│  Request/response conversion, error handling, auth middleware         │
└──────────────────────────────┬───────────────────────────────────────┘
                               │
┌──────────────────────────────▼───────────────────────────────────────┐
│                      Service Layer (Logic)                             │
│  `genossi_service_impl/src/{member,application,permission,...}.rs`    │
│  Business logic, validation, permission checks, audit macros          │
│  Trait bounds: Transaction, Context, DAO interfaces                   │
└──────────────────────────────┬───────────────────────────────────────┘
                               │
┌──────────────────────────────▼───────────────────────────────────────┐
│                      DAO Layer (Persistence)                           │
│  Traits: `genossi_dao/src/{member,application,audit_log,...}.rs`      │
│  SQLite impl: `genossi_dao_impl_sqlite/src/{member,application,...}` │
│  Transaction management, soft deletes, optimistic locking             │
└──────────────────────────────┬───────────────────────────────────────┘
                               │
┌──────────────────────────────▼───────────────────────────────────────┐
│                      SQLite Database                                   │
│  Managed via SQLx, migrations in `migrations/sqlite/*.sql`            │
│  UUIDs stored as BLOB, timestamps as ISO8601 strings                  │
└─────────────────────────────────────────────────────────────────────┘
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

**Overall:** Multi-layer trait-based architecture with dependency injection.

**Key Characteristics:**
- **Trait boundaries separate layers** — DAOs are traits, services are generic over dependencies
- **Soft deletes** — Entities use `deleted: Option<PrimitiveDateTime>` instead of hard deletion
- **Optimistic locking** — `version: Uuid` field prevents concurrent update conflicts
- **Minimal DAO interface** — Only 3 required methods: `create`, `update`, `dump_all`
- **Audit-first** — Member, MemberAction, MemberDocument, Application use `audited_*!` macros
- **Context-driven permissions** — Services receive authentication context for authorization
- **Component-first frontend** — Reusable components in `genossi-frontend/src/component/`, never inline RSX

## Layers

**DAO Layer (Data Access Objects):**
- Purpose: Abstract database operations via traits
- Location: `genossi_dao/src/`, `genossi_dao_impl_sqlite/src/`
- Contains: Entity structs, DAO trait definitions, audit trail structures
- Depends on: sqlx, uuid, time crates
- Used by: Service layer via trait bounds
- Key abstractions: `TransactionDao`, `MemberDao`, `ApplicationDao`, `AuditLogDao`

**Service Layer:**
- Purpose: Business logic, validation, permission enforcement, audit logging
- Location: `genossi_service_impl/src/`
- Contains: Service implementations (`*ServiceImpl` structs), audit macros, validation logic
- Depends on: DAO traits, Context type, uuid service
- Used by: REST handlers
- Key services: `MemberServiceImpl`, `ApplicationServiceImpl`, `PermissionServiceImpl`, `SessionServiceImpl`

**REST Layer:**
- Purpose: HTTP request/response handling, authentication middleware, OpenAPI documentation
- Location: `genossi_rest/src/`
- Contains: Axum handlers, middleware, error conversion, Utoipa schema definitions
- Depends on: Service types (via generics), tower, axum, utoipa
- Used by: HTTP clients, Swagger UI
- Key files: `member.rs`, `application.rs`, `audit_log.rs`, `auth_middleware.rs`, `session_management.rs`

**Binary Layer:**
- Purpose: Application bootstrap, dependency injection, worker initialization
- Location: `genossi_bin/src/`
- Contains: `RestStateImpl` struct with all service wiring, worker spawn functions
- Depends on: All service/DAO layers, sqlx connection pool
- Used by: `main.rs` entry point
- Responsibilities: Create DAOs → Create Services → Wrap in RestStateImpl → Start workers

**Frontend Layer:**
- Purpose: User interface via Dioxus WASM
- Location: `genossi-frontend/src/`
- Contains: Reusable components, pages, routing, API client, state management
- Depends on: Dioxus, reqwest (for API calls), Tailwind CSS
- Used by: Browser via WASM
- Critical principle: **Component-first** — extract any UI that appears twice into `src/component/`

## Data Flow

### Primary Request Path (Create Member)

1. **Frontend** (`genossi-frontend/src/page/member_create.rs`) — User submits form via component
2. **API Client** (`genossi-frontend/src/api.rs` or `rest-types/src/lib.rs`) — Serializes form to JSON with ISO8601 dates
3. **REST Handler** (`genossi_rest/src/member.rs:create_member()`) — Receives POST request, extracts auth context
4. **Service** (`genossi_service_impl/src/member.rs:create()`) — Validates input via `ValidationService`, checks permissions via `PermissionService`
5. **Audit Macro** (`audited_create!` in `genossi_service_impl/src/audit_macros.rs`) — Performs DAO create + audit logging atomically
6. **DAO** (`genossi_dao_impl_sqlite/src/member.rs:create()`) — Executes INSERT via SQLx, returns error or success
7. **Audit Log** (`genossi_service_impl/src/audit_log.rs:build_create_entries()`) — Computes SHA256 hash chain, inserts one row per field changed
8. **Response** — REST handler converts `ServiceError` to HTTP status (200, 400, 401, 422, 500)

### Update Request Path (Member Soft Delete)

1. Frontend deletes member by calling PUT with `deleted: now()`
2. REST handler extracts context
3. Service calls `audited_update!` macro
4. Macro loads old entity, performs DAO update, builds diff
5. Only changed fields (e.g., `deleted` timestamp) are audit-logged
6. Hash chain continues from previous latest entry

### Audit Log Verification Path

1. **REST endpoint** `GET /api/audit/verify` — Calls `AuditLogDao::verify_hash_chain()`
2. **DAO** — Loads all entries grouped by transaction_id, ordered by created
3. **Verification** — For each entry: recompute hash from raw fields + previous entry's hash, compare to stored hash
4. **Response** — Returns array of mismatches (empty = valid)

**State Management:**
- **At rest** — Persisted in SQLite with BLOB UUIDs and ISO8601 timestamps
- **In transit** — JSON with ISO8601 datetime strings, flexible deserialization (optional datetime fields)
- **In memory** — Service layer holds transaction context; each request is independent (no stateful session except cookies)
- **Optimistic locking** — `version` UUID prevents lost updates; service checks version matches before update

## Key Abstractions

**Transaction:**
- Purpose: Encapsulate database transaction with begin/commit/rollback
- Examples: `genossi_dao_impl_sqlite::transaction::TransactionImpl`
- Pattern: Trait-based, async/await, passed through entire call stack
- Usage: `TransactionDao::transaction()` → acquire transaction → pass to all DAO calls → `TransactionDao::commit()`

**Entity:**
- Purpose: Represent domain objects with consistent structure
- Examples: `MemberEntity`, `ApplicationEntity`, `MemberActionEntity`
- Pattern: `id: Uuid` (BLOB), `created: PrimitiveDateTime`, `deleted: Option<PrimitiveDateTime>`, `version: Uuid`, entity-specific fields
- Audit: Entities marked for audit implement `Auditable` trait with `entity_type()`, `entity_id()`, `audit_fields()`

**Service Error:**
- Purpose: Represent domain-level errors
- Variants: `DataAccess`, `EntityNotFound`, `ValidationError`, `PermissionDenied`, `Unauthorized`, `Conflict`, `InternalError`, `SessionExpired`, `AuthenticationFailed`
- Conversion: `ServiceError` → `RestError` → HTTP status code

**REST Error:**
- Purpose: Represent HTTP response errors
- Variants: `NotFound` (404), `BadRequest` (400), `Conflict` (409), `Unauthorized` (401), `UnsupportedMediaType` (415), `InternalError` (500)
- Handler: `error_handler()` wraps async blocks, converts all errors to HTTP responses with JSON bodies

## Entry Points

**HTTP Server:**
- Location: `genossi_bin/src/main.rs`
- Triggers: `cargo run --bin genossi` or systemd service
- Responsibilities:
  1. Load `.env` file
  2. Initialize rustls CryptoProvider
  3. Connect to SQLite database
  4. Run migrations
  5. Create `RestStateImpl` with full DI
  6. Start background workers (mail, inbox, backup, timestamp)
  7. Start Axum server on `SERVER_ADDRESS` (default `0.0.0.0:3000`)

**Frontend:**
- Location: `genossi-frontend/src/main.rs`
- Triggers: `dx serve` or `dx build`
- Responsibilities:
  1. Initialize Dioxus router
  2. Load i18n translations
  3. Mount App component to DOM
  4. Proxy API calls to backend via `Dioxus.toml`

**REST Endpoints:**
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

**What happens:** A page component contains `rsx! { ... }` HTML directly instead of delegating to `src/component/` components.

**Why it's wrong:** When two pages need similar UI (e.g., member search, status bar, date input), they duplicate the code. Styling diverges, behavior differs, maintenance becomes impossible.

**Do this instead:** Check if reusable component exists in `genossi-frontend/src/component/`. If yes, use it. If no, create it:
```rust
// genossi-frontend/src/component/member_search.rs
#[component]
pub fn MemberSearch(on_search: EventHandler<String>) -> Element {
    rsx! { /* shared search UI here */ }
}

// genossi-frontend/src/page/member_list.rs
#[component]
pub fn MemberListPage() -> Element {
    rsx! { MemberSearch(on_search: |query| { /* filter members */ }) }
}

// genossi-frontend/src/page/member_import.rs — reuses same component
#[component]
pub fn MemberImportPage() -> Element {
    rsx! { MemberSearch(on_search: |query| { /* filter by name */ }) }
}
```

### Hard Delete Without Audit Trail

**What happens:** A service calls `dao.delete(id)` which removes the row, losing audit history.

**Why it's wrong:** No way to recover data, verify who deleted what, or comply with GDPR "right to be forgotten with notice".

**Do this instead:** Use soft delete via update with `deleted = now()`:
```rust
// DAO layer: update, not delete
pub async fn soft_delete(&self, id: Uuid, tx: Tx) -> Result<(), DaoError> {
    let now = time::PrimitiveDateTime::now();
    // UPDATE member SET deleted = ?, version = ? WHERE id = ? AND version = ?
    self.update(&entity_with_deleted, "deletion", tx).await
}

// Service layer: wrap with audit
crate::audited_delete!($self, $dao, $entity_id, $process, $user_id, $tx);
```

### Manual Hash Chain Computation

**What happens:** Service or DAO code recomputes the SHA256 hash inline instead of using `audit_log::compute_entry_hash()`.

**Why it's wrong:** Risk of different implementations computing different hashes, breaking verification.

**Do this instead:** Always use `genossi_service_impl::audit_log::compute_entry_hash()` from the centralized module.

### Service Creating Its Own Transaction

**What happens:** Service calls `TransactionDao::transaction()` instead of receiving an optional transaction from the caller.

**Why it's wrong:** Caller cannot batch multiple operations in one transaction; each service call creates a separate transaction.

**Do this instead:** Accept `Option<Transaction>` parameter; let REST handler decide transaction scope:
```rust
// Service signature
async fn create(&self, entity: E, tx: Option<Tx>) -> Result<(), ServiceError> {
    let tx = self.transaction_dao.use_transaction(tx).await?;
    // ... operations ...
    self.transaction_dao.commit(tx).await?;
}
```

### Unvalidated User Input to Audit Log

**What happens:** Service logs user-supplied text (e.g., member name) directly to audit log without sanitization.

**Why it's wrong:** User can inject newlines, pipes, or null bytes that break hash chain parsing.

**Do this instead:** Validation service sanitizes before storing. Audit log stores Arc<str>, not raw user input. Hash computation treats special chars as literal bytes.

## Error Handling

**Strategy:** Convert layer-specific errors to domain errors, then to HTTP responses.

**Patterns:**

1. **DAO → Service:** `DaoError` maps to `ServiceError` via `From` impl:
   ```rust
   DaoError::NotFound → ServiceError::EntityNotFound(uuid)
   DaoError::ConflictError → ServiceError::Conflict(message)
   DaoError::DatabaseError → ServiceError::DataAccess(message)
   ```

2. **Service → REST:** `ServiceError` maps to `RestError` via `From` impl:
   ```rust
   ServiceError::PermissionDenied → RestError::Unauthorized → 401
   ServiceError::ValidationError → RestError::BadRequest → 400
   ServiceError::EntityNotFound → RestError::NotFound → 404
   ServiceError::Conflict → RestError::Conflict → 409
   ```

3. **REST Error Handler:** `error_handler()` in `genossi_rest/src/lib.rs` wraps async blocks:
   ```rust
   error_handler((async { /* handler code */ }).await)
   // Returns HTTP response with error JSON body
   ```

## Cross-Cutting Concerns

**Logging:** Tracing crate with env-filter. REST handlers instrument with `#[instrument]` macro. Workers log to stdout/stderr. Configuration: `RUST_LOG=genossi=info,tower_http=debug`.

**Validation:** `ValidationService` in `genossi_service_impl/src/validation.rs` provides reusable validators. Returns `Vec<ValidationFailureItem>` with field names and messages. REST handler converts to 400 BadRequest with JSON error details.

**Authentication:** 
- **OIDC mode** (`--features oidc`): `genossi_service_impl/src/session.rs` validates JWT from Nextcloud, stores session cookie
- **Mock mode** (`--features mock_auth`): `genossi_service/src/permission.rs:MockContext` allows any request for testing
- Middleware extracts context from cookie/header, passes via `Extension<Context>` to handlers

**Authorization:** `PermissionService` checks roles (admin, user) and entity-level permissions via `PermissionDao`. Returns `PermissionDenied` if context lacks required role.

**Audit Logging:** `audited_create!`, `audited_update!`, `audited_delete!` macros in `genossi_service_impl/src/audit_macros.rs` wrap DAO calls. Each changed field gets one audit_log row. Hash chain verified via `GET /api/audit/verify`.

---

*Architecture analysis: 2026-05-02*
