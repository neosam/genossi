# Technology Stack

**Analysis Date:** 2026-05-02

## Languages

**Primary:**
- Rust 2021 edition - Backend API server and services (`genossi_bin`, `genossi_rest`, `genossi_service_impl`)
- Rust 2021 edition - Data access layer with SQLx (`genossi_dao_impl_sqlite`)

**Secondary:**
- Rust compiled to WASM - Frontend UI with Dioxus (`genossi-frontend`)

## Runtime

**Environment:**
- Tokio async runtime 1.35+ - All async server operations
- WASM runtime (browser) - Frontend execution

**Package Manager:**
- Cargo - Rust package manager for workspace
- npm/Node.js - Frontend tooling (Tailwind CSS, Dioxus CLI)

**Lockfile:**
- `Cargo.lock` - Present, committed to repo

## Frameworks

**Core Backend:**
- Axum 0.8.3 - REST API web framework with multipart form support
- Tokio 1.35+ - Async runtime with full feature set

**Database:**
- SQLx 0.8 - Async SQL query executor for SQLite
- SQLite - Embedded relational database with WAL mode support

**Frontend:**
- Dioxus 0.6.3 - React-like reactive UI framework (Rust-to-WASM)
- Tailwind CSS - Utility-first CSS framework (watch mode during dev)

**API Documentation:**
- Utoipa 5.0 - OpenAPI code generation from Rust types
- Utoipa-Swagger-UI 9.0 - Swagger UI at `/swagger-ui/`

**Testing:**
- Mockall 0.13 - Mock trait implementation for unit tests
- Cargo test - Built-in Rust test runner

**Build/Dev:**
- Dioxus CLI - Frontend build and dev server (`dx serve`, `dx build`)
- NixOS flake - Development environment with nix (`flake.nix`)
- OpenSpec - Change management system integrated via flake

## Key Dependencies

**Critical:**
- `tokio` 1.35 - Async runtime (full features including networking, process, sync)
- `axum` 0.8.3 - HTTP server framework with CORS, cookies, sessions middleware
- `sqlx` 0.8 - Type-safe database access with compile-time query verification
- `serde` 1.0 + `serde_json` 1.0 - Serialization/deserialization for API responses
- `uuid` 1.6 - Entity ID generation (v4 random, serde support)
- `time` 0.3 - DateTime handling with serde + formatting + parsing

**Authentication & Authorization:**
- `axum-oidc` 0.6 - Optional OIDC integration (feature-gated, production auth)
- `tower-sessions` 0.14 - Session management middleware
- `tower-cookies` 0.10 - Cookie handling middleware

**Email:**
- `lettre` 0.11 - SMTP email client with tokio1 async + TLS support
- `async-imap` 0.10 - IMAP client for inbox polling
- `tokio-rustls` 0.26 - TLS for IMAP connections
- `webpki-roots` 0.26 - Root CA certificates for TLS
- `mail-parser` 0.9 - RFC822 email parsing

**Document Generation:**
- `typst` 0.14 - Document/PDF template language compiler
- `typst-pdf` 0.14 - PDF generation from Typst documents
- `calamine` 0.26 - Excel file parsing (for imports)

**Cryptography & Signing:**
- `sha2` 0.10 - SHA256 hashing for audit hash chain
- `x509-tsp` 0.1.0 - RFC 3161 timestamp protocol (document timestamping)
- `cmpv2` 0.2.0 - CMS signature format parsing
- `der` 0.7 - DER encoding/decoding for X.509 structures
- `cms` 0.2.3 - Cryptographic Message Syntax support
- `const-oid` 0.9 - Object Identifier handling
- `spki` 0.7 - Subject Public Key Info structures
- `rustls` 0.23 - TLS client with ring crypto backend

**Data & File Handling:**
- `zip` 2.0 - ZIP archive creation/extraction (deflate compression)
- `csv` 1.3 - CSV parsing and generation
- `tar` 0.4 - TAR archive handling (backups)
- `flate2` 1.0 - GZIP compression (backups)
- `path-clean` 1.0 - Path normalization

**HTTP & Web:**
- `reqwest` 0.11/0.12 - HTTP client (various features builds)
- `tower` 0.5 - Middleware and service abstraction
- `tower-http` 0.6 - CORS, headers middleware
- `tower_governor` 0.6 - Rate limiting middleware
- `http` 1.1 - HTTP types (headers, methods, status codes)
- `minijinja` 2.0 - Email template rendering

**Logging & Observability:**
- `tracing` 0.1 - Structured logging framework
- `tracing-subscriber` 0.3 - Logging backend with env-filter

**Frontend (Dioxus/WASM):**
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

**Environment:**
- `.env` file format (dotenv 0.15)
- Key variables: `DATABASE_URL`, `SERVER_ADDRESS`, `BASE_PATH`, `APP_URL`, `ISSUER`, `CLIENT_ID`, `CLIENT_SECRET`
- See `.env.oidc.example` for OIDC setup

**Build:**
- `Cargo.toml` workspace with `edition = "2021"`, `resolver = "2"`
- Feature flags:
  - `mock_auth` (default) - Development authentication
  - `oidc` - Production OIDC via Nextcloud
- Nix flake.nix for reproducible dev environment

**Database:**
- SQLite file-based (default: `sqlite:genossi.db`)
- Migrations in `/migrations/sqlite/` - auto-run on startup via sqlx-cli

## Platform Requirements

**Development:**
- Rust toolchain (1.70+ recommended by 2021 edition)
- Cargo
- SQLx CLI (for migrations)
- SQLite dev libraries
- Node.js (for frontend Tailwind CSS compilation)
- Nix (optional, recommended for reproducible environment)

**Production:**
- Linux/Unix-like OS (developed and deployed on Linux)
- SQLite compatible filesystem (WAL mode requires write access)
- Optional: SMTP server for outgoing mail
- Optional: IMAP server for inbox polling
- Optional: Nextcloud instance for OIDC provider
- Optional: WebDAV/Nextcloud for backup export (accessibility, NOT primary backup)
- Optional: RFC 3161 Timestamp Authority for document timestamping

---

*Stack analysis: 2026-05-02*
