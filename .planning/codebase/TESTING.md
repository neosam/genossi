# Testing Patterns

**Analysis Date:** 2026-05-02

## Test Framework

**Runner:**
- `tokio` (async runtime) with `#[tokio::test]` macro for async tests
- Standard `#[test]` for synchronous unit tests
- No custom test runner configuration detected

**Build Command:**
```bash
cargo test                           # Run all tests in workspace
cargo test -p genossi_service_impl   # Run tests for specific package
cargo test test_name                 # Run specific test by name
cargo test -- --nocapture           # Print output from tests
```

**Config:**
- Test configuration in Cargo.toml feature flags: `[features] default = ["mock_auth"]` (enables mock authentication for testing, `genossi_bin/Cargo.toml:7`)
- Dev dependencies: `reqwest`, `tempfile`, `csv`, `zip`, `rust_xlsxwriter` for test utilities (`genossi_bin/Cargo.toml:48`)

## Test File Organization

**Location:**
- **Unit tests:** Inline in source files using `#[cfg(test)] mod tests { }` pattern
- **Integration tests:** `genossi_bin/tests/e2e_tests.rs` (single E2E suite in `tests/` directory)
- **Test infrastructure:** `genossi_rest/src/test_server.rs` (shared utilities)

**Naming:**
- Test functions: `test_*` prefix (e.g., `test_get_all_members_empty`, `test_create_and_get_member`, `test_build_timestamp_request_deterministic`)
- Mock helpers: `mock_*` or `setup_mock_*` prefix (e.g., `mock_config_enabled()`, `setup_mock_tx()`)
- Sample data: `sample_*` prefix (e.g., `sample_member()`)

**Example Directory Structure:**
```
genossi_service_impl/src/
├── member.rs              # Implementation + inline tests
├── timestamp.rs           # Implementation + test macros
├── audit_macros.rs        # Macro definitions
└── lib.rs

genossi_bin/tests/
└── e2e_tests.rs           # End-to-end tests

genossi_rest/src/
└── test_server.rs         # Test infrastructure
```

## Test Structure

**Unit Test Pattern** (from `genossi_service_impl/src/timestamp.rs`):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_timestamp_disabled() {
        let tx_dao = setup_mock_tx();
        let mut audit_log_dao = MockAuditLogDao::new();
        audit_log_dao.expect_get_latest_hash().returning(|_| Ok(None));

        let config = mock_config_disabled();

        let service = TimestampServiceImpl::new(tx_dao, audit_ts_dao, audit_log_dao, Arc::new(config));

        let result = service.create_timestamp().await;
        assert!(matches!(result, Err(ServiceError::InternalError(_))));
    }
}
```

**E2E Test Pattern** (from `genossi_bin/tests/e2e_tests.rs`):
```rust
async fn setup() -> TestServer {
    let pool = Arc::new(
        SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create in-memory database"),
    );
    
    sqlx::migrate!("../migrations/sqlite")
        .run(&*pool)
        .await
        .expect("Failed to run migrations");
    
    let rest_state = RestStateImpl::new(pool);
    start_test_server(rest_state).await
}

#[tokio::test]
async fn test_create_and_get_member() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let member = sample_member();
    
    let response = client
        .post(server.url("/api/members"))
        .json(&member)
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    let created: MemberTO = response.json().await.unwrap();
    assert!(created.id.is_some());
}
```

**Patterns:**
- **Setup:** Call helper function (e.g., `setup()`, `setup_mock_tx()`) to initialize test state
- **Arrange:** Create test data (e.g., `sample_member()`)
- **Act:** Invoke the function/service being tested (with mocks returning expected values)
- **Assert:** Use `assert_eq!()`, `assert!()`, `assert_ne!()`, or `assert!(matches!(...))` for expectations
- **Teardown:** Implicit via scope exit (drop test server, in-memory DB freed, mocks cleaned)

## Mocking

**Framework:** `mockall` (0.13, from workspace dependencies in `Cargo.toml`)

**Automation:**
- Trait definition: Use `#[automock(type Transaction = MockTransaction;)]` on traits to auto-generate mocks
- DAO trait example: `genossi_dao/src/lib.rs:58` shows `#[automock]` on `TransactionDao` trait
- Generated mocks: `MockMemberDao`, `MockTransactionDao`, `MockConfigService`, etc.

**Usage Pattern** (from `genossi_service_impl/src/timestamp.rs`):
```rust
let mut mock_tx_dao = MockTransactionDao::new();

// Set up expected calls
mock_tx_dao.expect_transaction()
    .returning(|| {
        let mut tx = MockTransaction::new();
        tx.expect_clone().returning(MockTransaction::new);
        Ok(tx)
    });

mock_tx_dao.expect_commit().returning(|_| Ok(()));

// Use in service
let service = TimestampServiceImpl::new(mock_tx_dao, ...);
```

**Mocking Patterns:**
- `.expect_method_name()`: Declare expected method call
- `.returning(|| value)`: Specify return value (can use closures)
- `.returning(|arg1, arg2| ...)`: For methods with arguments
- Chain setup calls; mocks verify all expected calls occurred
- Nested mocks: Create `MockTransaction` inside `expect_transaction().returning(...)` for complex scenarios

**What to Mock:**
- DAO layer (all database operations): `MockMemberDao`, `MockTransactionDao`, etc.
- Service dependencies: `MockConfigService`, `MockPermissionService`
- External APIs: HTTP clients, mail services
- Transaction management for isolation tests

**What NOT to Mock:**
- Pure utility functions (no external dependencies)
- Serialization/deserialization (test with real instances)
- In-memory data structures that don't require setup
- Async runtime (use `#[tokio::test]`)

## Test Server Infrastructure

**Location:** `genossi_rest/src/test_server.rs`

**Structure:**
```rust
pub struct TestServer {
    pub addr: SocketAddr,
    pub handle: JoinHandle<()>,
}

impl TestServer {
    pub fn url(&self, path: &str) -> String {
        format!("http://{}{}", self.addr, path)
    }
}

pub async fn start_test_server<RestState: RestStateDef>(
    rest_state: RestState,
) -> TestServer {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Could not bind to random port");
    
    let addr = listener.local_addr().expect("Could not get local address");
    let app = create_app(rest_state).await;
    
    let handle = tokio::spawn(async move {
        axum::serve(listener, app.into_make_service_with_connect_info()).await.expect(...)
    });
    
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    
    TestServer { addr, handle }
}
```

**Key Features:**
- **Random Port Binding:** Binds to `127.0.0.1:0` to get any available port (avoids port conflicts between parallel tests)
- **Async Server:** Spawns Axum server in background tokio task
- **URL Helper:** `TestServer::url(path)` constructs full HTTP URL for requests
- **Automatic Cleanup:** `Drop` impl aborts server task when test ends (scope exit)
- **Start Delay:** 10ms sleep after spawn to ensure server is ready (in `test_server.rs:47`)

## Fixtures and Factories

**Test Data Factories:**
```rust
fn sample_member() -> MemberTO {
    MemberTO {
        id: None,
        member_number: 1,
        first_name: "Max".to_string(),
        last_name: "Mustermann".to_string(),
        email: Some("max@example.com".to_string()),
        // ... other fields with sensible defaults
    }
}
```

**Pattern:**
- Factory function creates complete, valid test object with defaults
- Functions are `fn` (not async) since they just construct objects
- Return transfer objects (`*TO`) for REST tests, domain objects for service tests
- Located near top of test module or in test file (e.g., `genossi_bin/tests/e2e_tests.rs:39`)

**Mock Factories:**
```rust
fn setup_mock_tx() -> MockTransactionDao {
    let mut tx_dao = MockTransactionDao::new();
    tx_dao.expect_transaction().returning(|| { ... });
    tx_dao.expect_commit().returning(|_| Ok(()));
    tx_dao
}

fn mock_config_enabled() -> MockConfigService {
    let mut config = MockConfigService::new();
    config.expect_get_all().returning(|| Ok(vec![...]));
    config
}
```

**Location:**
- In `#[cfg(test)] mod tests { }` block of the implementation file
- Shared test utilities in dedicated modules (e.g., `genossi_rest/src/test_server.rs`)

## Coverage

**Requirements:** No enforced coverage target detected (no coverage configuration in Cargo.toml or files)

**View Coverage:**
```bash
cargo tarpaulin --out Html   # Generate HTML coverage report (requires tarpaulin)
# Coverage report in `tarpaulin-report.html`
```

**Current Patterns:**
- Unit tests cover service logic, error cases, and edge conditions
- Integration tests (E2E) verify REST endpoints with real database interactions
- Mock-based unit tests isolate components

## Test Types

**Unit Tests:**
- **Scope:** Individual service methods, pure functions
- **Approach:** Inline tests with mocked DAO/service dependencies
- **Examples:** `timestamp.rs` tests (timestamp creation with various config states)
- **Mocking:** All external dependencies via mockall-generated mocks
- **Run:** `cargo test -p genossi_service_impl`

**Integration Tests:**
- **Scope:** Service layer with real DAO implementations (in-memory SQLite)
- **Approach:** Not heavily used; E2E tests dominate
- **Note:** No dedicated integration test suite found; service tests rely on mocks

**E2E Tests:**
- **Scope:** Full REST API endpoints through HTTP
- **Approach:** Real async Axum server + in-memory SQLite database per test
- **Location:** `genossi_bin/tests/e2e_tests.rs`
- **Infrastructure:** `TestServer` with random port binding (from `genossi_rest/src/test_server.rs`)
- **Setup:** Each test creates fresh in-memory database, runs migrations, starts HTTP server
- **Database:** `sqlite::memory:` (ephemeral, isolated per test)
- **HTTP Client:** `reqwest::Client` for making actual HTTP requests
- **Port Strategy:** Random port binding via `TcpListener::bind("127.0.0.1:0")` ensures no conflicts
- **Examples:** Member CRUD operations, role management, email sending
- **Run:** `cargo test --test e2e_tests` or `cargo test --bin genossi`

## Common Patterns

**Async Testing:**
```rust
#[tokio::test]
async fn test_async_operation() {
    let result = async_function().await;
    assert_eq!(result, expected);
}
```
- Use `#[tokio::test]` (not `#[test]`) for async test functions
- Await all futures; no blocking calls (compatible with Tokio runtime)
- Mock async functions with `.returning()` closures that return futures

**Error Testing:**
```rust
#[tokio::test]
async fn test_timestamp_disabled() {
    let config = mock_config_disabled();
    let service = TimestampServiceImpl::new(..., Arc::new(config));
    
    let result = service.create_timestamp().await;
    assert!(matches!(result, Err(ServiceError::InternalError(_))));
}
```
- Use `assert!(matches!(...))` for error variant matching
- Test error types, not just success paths
- Verify error messages contain context (e.g., validation errors list fields)

**Feature-Gated Tests:**
```rust
#[tokio::test]
#[ignore]  // Requires network access to freetsa.org
async fn test_integration_freetsa() {
    // Real network test
}
```
- Mark tests requiring external resources with `#[ignore]`
- Use `cargo test -- --ignored` to run only ignored tests
- Skip ignored tests in CI pipelines by default

**Testing Soft Deletes:**
- Entities use `deleted: Option<PrimitiveDateTime>` field
- Test soft delete: Create → update with deleted timestamp set → verify still queryable with `deleted.is_none()` filter
- No hard delete tests (not implemented at DAO layer)

---

*Testing analysis: 2026-05-02*
