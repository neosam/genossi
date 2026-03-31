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