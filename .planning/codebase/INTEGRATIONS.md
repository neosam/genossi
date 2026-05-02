# External Integrations

**Analysis Date:** 2026-05-02

## APIs & External Services

**OIDC Authentication (Production):**
- Nextcloud OIDC Provider - User identity and authentication
  - SDK/Client: `axum-oidc` 0.6 (feature-gated: `oidc`)
  - Auth: Environment variables: `ISSUER`, `CLIENT_ID`, `CLIENT_SECRET`
  - Configuration: `genossi_rest/src/lib.rs` OIDC setup
  - Flow: Initial login via OIDC, session-based continuation (account deactivation has 30-day inactivity window)
  - Note: Older docs may incorrectly reference WordPress — **OIDC provider is Nextcloud only**

**RFC 3161 Timestamp Authority (Optional):**
- External TSA for document timestamping and qualified signatures
  - SDK/Client: `x509-tsp` 0.1.0, `cmpv2` 0.2.0, `der` 0.7, `cms` 0.2.3
  - Implementation: `genossi_service_impl/src/rfc3161.rs`
  - Purpose: Qualified timestamps for audit log entries (optional — not required for basic operation)
  - Protocol: RFC 3161 timestamp requests/responses over HTTPS

## Data Storage

**Databases:**
- SQLite file-based (`sqlite:genossi.db`)
  - Connection: Environment variable `DATABASE_URL` (default: `sqlite:genossi.db`)
  - Client: SQLx 0.8 with async-tokio runtime
  - Schema: Migrations in `/migrations/sqlite/` auto-run on startup
  - Tables: Member, MemberAction, MemberDocument, Application, AuditLog, ConfigEntry, MailJob, MailRecipient, UserPreferences, Session, Auth (users, roles, privileges)
  - Features: WAL mode for concurrent read access, soft deletes with `deleted` timestamp, optimistic locking via `version` field

**File Storage:**
- Local filesystem
  - Typst package cache: `./typst-packages` (downloaded from `https://packages.typst.org`)
  - Database file: `genossi.db` (SQLite)
  - Document templates: Stored in database (ConfigEntry with key patterns)

**Caching:**
- In-memory via Rust Arc/shared state
- Session cache via tower-sessions middleware
- No external cache service (Redis, Memcached) required

## Authentication & Identity

**Auth Provider:**
- Nextcloud OIDC (Production)
  - Implementation: `genossi_rest/src/lib.rs` with `axum-oidc` 0.6
  - Environment: `APP_URL`, `ISSUER`, `CLIENT_ID`, `CLIENT_SECRET`
  - Redirect URI: `https://<domain>/authenticate`
  - Post-logout redirect: `https://<domain>/`
  - Session table: stores user_id (Nextcloud username), expires timestamp, created timestamp

- Mock Authentication (Development)
  - Implementation: Feature-gated `mock_auth` (default)
  - Allows testing without Nextcloud/OIDC provider

**Session Management:**
- Tower-sessions middleware (`tower-sessions` 0.14)
- Tower-cookies for HTTP-only session cookies (`tower-cookies` 0.10)
- Expiry: Configurable, default 365 days (hardened in recent changes to 30-day inactivity timeout)
- Revocation: `POST /api/session/revoke-all` allows users to kill all active sessions

## Monitoring & Observability

**Error Tracking:**
- None detected (no Sentry, DataDog, etc.)
- Errors logged via tracing framework

**Logs:**
- Structured logging via `tracing` 0.1 + `tracing-subscriber` 0.3
- Environment-based filtering via `env-filter`
- Audit logging: `genossi_service_impl/src/audit_log.rs` with hash chain (SHA256)
  - REST endpoints: `GET /api/audit`, `GET /api/audit/{entity_type}/{entity_id}`, `GET /api/audit/verify`
  - One row per changed field, grouped by `transaction_id`
  - Hash chain: Each audit entry links to previous via SHA256

## CI/CD & Deployment

**Hosting:**
- Not specified in codebase (self-hosted deployment expected)
- Supports Docker/NixOS module (`module.nix` provides NixOS service definition)
- OpenSpec integration for change management

**CI Pipeline:**
- None detected in codebase (no GitHub Actions, GitLab CI config visible)
- Manual testing via `cargo test`, `cargo build`, `cargo run`

**Build Artifacts:**
- Rust binary: `genossi` (from `genossi_bin/src/main.rs`)
- Frontend WASM: Compiled via Dioxus CLI (`dx build`)

## Environment Configuration

**Required env vars:**
- `DATABASE_URL` - SQLite connection string (default: `sqlite:genossi.db`)
- `SERVER_ADDRESS` - Bind address (default: `0.0.0.0:3000`)
- `BASE_PATH` - Base URL for Swagger UI (default: `http://localhost:3000/`)

**OIDC env vars (when `--features oidc`):**
- `APP_URL` - Application URL for callback (e.g., `https://example.com`)
- `ISSUER` - OIDC provider issuer URL (e.g., `https://nextcloud.example.com`)
- `CLIENT_ID` - OAuth client ID from provider
- `CLIENT_SECRET` - OAuth client secret from provider

**SMTP/Mail env vars (database-stored):**
- Loaded from ConfigEntry table, not env vars
- Keys: `smtp_host`, `smtp_port`, `smtp_user`, `smtp_pass`, `smtp_from`, `smtp_from_name` (optional), `smtp_tls` (optional)

**IMAP env vars (database-stored):**
- Loaded from ConfigEntry table
- Keys: `imap_host`, `imap_port`, `imap_user`, `imap_pass`, `imap_mailbox`, `imap_tls` (optional)

**Secrets location:**
- Environment variables (dev/deployment)
- NixOS service credentials via `clientSecretFile` (production via module.nix)
- Database ConfigEntry (SMTP/IMAP credentials)

## Webhooks & Callbacks

**Incoming:**
- `POST /authenticate` - OIDC callback endpoint (handled by `axum-oidc`)
- `GET /logout` - OIDC post-logout redirect (handled by `axum-oidc`)

**Outgoing:**
- None detected (no outbound webhooks to external services)
- Email sending via SMTP outbound connection only

## Email Integration

**Outbound (SMTP):**
- Lettre 0.11 with Tokio async + TLS support
- Configuration: Database ConfigEntry (`smtp_host`, `smtp_port`, `smtp_user`, `smtp_pass`, `smtp_from`)
- Implementation: `genossi_mail/src/service.rs` (SmtpConfig, build_transport)
- Worker: `genossi_mail/src/worker.rs` processes mail jobs async
- Features: Template rendering via Minijinja, attachments, multipart support

**Inbound (IMAP):**
- Async-imap 0.10 with TLS (required, no plaintext support)
- Configuration: Database ConfigEntry (`imap_host`, `imap_port`, `imap_user`, `imap_pass`, `imap_mailbox`)
- Implementation: `genossi_mail/src/inbox_imap.rs`
- Worker: `genossi_mail/src/inbox.rs` - periodic polling (fetches new messages, marks seen, moves to archive)
- Message parsing: `mail-parser` 0.9 for RFC822 format
- Storage: Messages stored in database via `MailRecipient` and parsed fields

## Backup & Export

**WebDAV/Nextcloud Export (Accessibility, NOT Backup):**
- Purpose: CSV/ZIP export for non-technical Nextcloud users (accessibility)
- **Important:** NextCloud-Export is NOT the primary backup strategy. Restic handles encrypted recovery backup separately.
- SDK: Custom WebDAV client via `reqwest` (in `genossi_backup/src/webdav.rs`)
- Implementation: `genossi_backup/src/sync.rs` - periodic export generator
- Methods: MKCOL (directory creation), PUT (file upload)
- Authentication: HTTP Basic Auth (username/password from ConfigEntry)
- Format: CSV files + ZIP archives containing database exports

**Document Generation:**
- Typst document templates: `typst` 0.14 + `typst-pdf` 0.14
- Template storage: `genossi_service_impl/src/document_storage.rs`
- Package resolution: Downloads from `https://packages.typst.org` on demand
- PDF generation: `genossi_service_impl/src/pdf_generation.rs`
- Qualified signatures: Optional RFC 3161 timestamps appended to audit entries

## Rate Limiting

**Implementation:**
- `tower_governor` 0.6 middleware
- Configuration: Not visible in main code (likely in REST layer)

## CORS & Security

**CORS Configuration:**
- `tower-http` 0.6 with flexible origin handling
- Origin list from environment config
- Configured in `genossi_rest/src/lib.rs`

**Security Headers:**
- `Strict-Transport-Security`: max-age=63072000 (2 years)
- `X-Content-Type-Options`: nosniff
- `X-Frame-Options`: DENY
- `Referrer-Policy`: strict-origin-when-cross-origin
- `Permissions-Policy`: camera=(), microphone=(), geolocation=(), payment=()

## Frontend Integration

**API Communication:**
- Dioxus frontend communicates via REST API to Axum backend
- Client: `reqwest` 0.12 in frontend
- Shared types: `genossi-frontend/rest-types/` (separate crate with serde types)
- Proxy configuration: `Dioxus.toml` for dev backend routing

---

*Integration audit: 2026-05-02*
