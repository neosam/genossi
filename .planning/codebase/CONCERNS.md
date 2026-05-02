# Codebase Concerns

**Analysis Date:** 2026-05-02

## Tech Debt

### Unsafe Send/Sync Markers in Dependency Injection

**Issue:** 9 dependency injection structs in `genossi_bin/src/lib.rs:69-241` use `unsafe impl Send` and `unsafe impl Sync` to work around trait bounds in the service layer composition. This is a code smell that can hide genuine synchronization issues.

**Files:** `genossi_bin/src/lib.rs` (lines 69-70, 92-95, 106-107, 126-127, 148-149, 180-181, 199-200, 221-222, 240-241)

**Impact:** 
- Makes code harder to reason about safety properties
- If trait bounds are too restrictive, a better solution (Arc-based wrapper, PhantomData pattern) might be cleaner
- Reduces rustc's ability to catch genuine thread-safety bugs

**Fix approach:** 
- Evaluate whether Arc<Mutex<_>> or Arc<RwLock<_>> wrappers around non-Send components would satisfy trait bounds
- Consider using a generic trait with conditional Send/Sync bounds instead of unsafe impls
- Document safety invariants if unsafe impls are necessary

---

### SQLite Connection Pool Configuration Uses Defaults

**Issue:** `genossi_bin/src/main.rs:24-28` creates the SQLite pool without explicitly configuring `max_connections` or `acquire_timeout`. This means the pool behavior is not documented or intentionally tuned for the workload.

**Files:** `genossi_bin/src/main.rs:24-28`

**Impact:**
- Pool exhaustion during load spikes could cause uncontrolled queue growth
- Acquire timeouts default to 30 seconds, which may be too long or too short depending on workload
- No visibility into pool configuration decisions

**Fix approach:**
- Add explicit `max_connections` (suggest 10-20 for SQLite, which is single-threaded) and `acquire_timeout` in `src/main.rs`
- Document rationale for chosen values
- Consider making these environment-configurable for production tuning

---

### Numeric Field Validation Missing at Database Level

**Issue:** `current_shares`, `shares_at_joining`, and `balance` fields on Member entities have no CHECK constraints in the SQLite schema. Negative values or overflow are possible at the database level, even though service-layer validation exists.

**Files:** Migrations `migrations/sqlite/20260331000000_create_member_table.sql` and related columns

**Impact:**
- Database layer doesn't enforce invariants (shares should be non-negative)
- Direct SQL inserts bypass service-layer validation
- Audit log could record invalid state transitions

**Fix approach:**
- Create new migration with `CHECK (current_shares >= 0)`, `CHECK (shares_at_joining >= 0)`, `CHECK (balance IS NULL OR balance >= 0)` constraints
- Backfill any violating rows in production before applying constraint
- Document numeric field semantics

---

### Missing Continuous Security Scanning

**Issue:** No `cargo audit` step in CI/CD pipeline. Dependencies may have known CVEs that are not detected until manually run.

**Files:** CI configuration (not found in codebase - assumed handled separately)

**Impact:**
- Dependency vulnerabilities can go undetected until next manual audit
- No automated alert for new CVEs in existing dependencies

**Fix approach:**
- Add `cargo install cargo-audit && cargo audit` as a CI step
- Consider integrating with Dependabot or equivalent service for automated vulnerability tracking

---

## Known Bugs

### Document Upload Body Limit Constraint Mismatch (HIGH)

**Issue:** Axum has a default body limit of 2 MB for all endpoints. Service-layer limits for document uploads are higher: 50 MB for member documents (`genossi_service_impl/src/member_document.rs:20`) and 10 MB for static documents (`genossi_mail/src/static_document_service.rs:25`). Uploads over 2 MB fail with HTTP 413 (Payload Too Large) before reaching the service layer.

**Files:**
- `genossi_rest/src/lib.rs` — where Axum router is built
- `genossi_service_impl/src/member_document.rs:20` — service-layer limit (50 MB)
- `genossi_mail/src/static_document_service.rs:25` — service-layer limit (10 MB)

**Trigger:** POST any file > 2 MB to `/api/member/{id}/document` or static document upload endpoints

**Workaround:** Users must upload files in chunks < 2 MB

**Fix approach:** 
- Configure Axum `DefaultBodyLimit::max()` per-route for upload endpoints
- Member document route: 50 MB limit
- Static document route: 10 MB limit (or from `STATIC_DOCUMENTS_MAX_BYTES` env var)
- Keep global 2 MB default for all other endpoints (defense-in-depth)

---

## Security Considerations

### Config Secret Values Stored in Plaintext (MEDIUM)

**Issue:** Secret configuration values (`backup_webdav_password`, `public_api_key`, etc. — marked with `value_type = "secret"`) are masked in the REST API response (`genossi_config/src/rest.rs:39-40`), but stored as plaintext in the SQLite database. If the database file is compromised (backup leak, server breach), all secrets are exposed.

**Files:**
- `genossi_config/src/rest.rs:39-40` — REST masking (correct)
- `genossi_config/src/dao_sqlite.rs` — no encryption on `set()`/`get()`
- `genossi_config/src/service.rs` — no key management

**Current mitigation:** Database file is on the server filesystem; access requires OS-level credentials

**Recommendations:** 
1. Implement AES-GCM or ChaCha20-Poly1305 encryption at rest for secret values
2. Derive encryption key from environment variable (`CONFIG_ENCRYPTION_KEY`) set during deployment
3. Provide migration to encrypt existing plaintext secrets on first startup
4. Keep encryption backward-compatible for development mode (if key not set, allow plaintext)
5. This is tracked as a proposal in `openspec/changes/config-secrets-encryption/`

---

### PII Data Retention and Compliance (MEDIUM)

**Issue:** Three compliance findings from the security audit (2026-04-18):
1. **M3 — Field-Level Access Control:** All authenticated users with `manage_members` permission can see all member fields including bank account information. No field-level access control exists.
2. **M5 — No Hard Delete:** Only soft deletes are implemented (via `deleted` timestamp). Member PII persists indefinitely in the database and audit logs, preventing full compliance with GDPR Art. 17 (right to erasure).
3. **I3 — PII in Logs:** Member data may appear in debug logs if not carefully skipped with `#[instrument(skip(...))]` annotations.

**Files:**
- `genossi_rest_types/src/lib.rs` — MemberTO exposes all fields
- `genossi_service_impl/src/audit_macros.rs:82-110` — soft delete implementation
- `genossi_rest/src/lib.rs` — various handlers log context

**Current mitigation:** Soft deletes prevent stale data from being returned; audit logging provides accountability

**Recommendations:**
1. Evaluate whether `manage_members` privilege should be split into separate levels (e.g., `view_members`, `view_sensitive_member_info`, `manage_members`)
2. Implement anonymization endpoint: set PII fields to hashes/nulls in DB and audit log for deleted members (preserves audit trail without identification)
3. Audit all `tracing::` calls to ensure PII-carrying fields are skipped or sanitized
4. Track as proposals in `openspec/changes/gdpr-data-protection/`

---

### OIDC Provider Assumption (Nextcloud, Not WordPress)

**Issue:** Security context assumes OIDC provider is Nextcloud (per user memory: `project_oidc_provider.md`). Older documentation or comments may reference WordPress incorrectly.

**Files:**
- `OIDC-CONFIG.md` — verify provider is Nextcloud
- `genossi_service/src/auth_types.rs` — check OIDC integration

**Current mitigation:** Code structure uses generic OIDC, provider-specific logic is isolated in config

**Recommendations:**
1. Search codebase for any "WordPress" references and update to "Nextcloud"
2. Verify OIDC configuration in OIDC-CONFIG.md matches Nextcloud endpoints
3. Test OIDC flow with actual Nextcloud instance in staging

---

## Performance Bottlenecks

### Large Mail DAO SQLite Implementation

**Issue:** `genossi_mail/src/dao_sqlite.rs` is 1924 lines. This file handles mail jobs, recipients, attachments, templates, static documents, inbound mail, and communication entries. Multiple conversion functions and datetime parsing logic are repeated, and transaction handling is distributed across many functions.

**Files:** `genossi_mail/src/dao_sqlite.rs`

**Cause:** Single-file SQLite DAO for mail subsystem; no layering or helper module abstraction

**Improvement path:**
- Extract datetime parsing and UUID conversion to a utilities module (reuse across `genossi_dao_impl_sqlite/src/`)
- Consider splitting mail DAO into separate files: `mail_job_dao.rs`, `mail_recipient_dao.rs`, `mail_template_dao.rs`, `static_document_dao.rs`, etc.
- This is not blocking but makes maintenance harder as the file grows

---

### Frontend API Client is 1579 Lines

**Issue:** `genossi-frontend/src/api.rs` is 1579 lines and contains all HTTP client logic for the frontend. It handles member operations, applications, templates, documents, and more in a single file with multiple chained `.unwrap()` calls.

**Files:** `genossi-frontend/src/api.rs`

**Cause:** 
- No API client code generation or abstraction (e.g., openapi-generator, graphql-codegen)
- Repeated fetch/JSON parsing boilerplate with chained `.unwrap()` on `resp.json().unwrap()`

**Current issues:**
- Line 131-132: `window().unwrap().location().origin().unwrap()` — panics if window is unavailable
- Line 343: `resp.json().unwrap()` — panics on malformed JSON
- Line 512, 568, 1178: Similar patterns

**Improvement path:**
- Extract fetch + JSON parsing into a helper macro or trait to reduce boilerplate and ensure consistent error handling
- Consider using typed HTTP client (e.g., reqwest for frontend via wasm bindings, or codegen from OpenAPI spec)
- This will reduce lines and improve reliability

---

## Fragile Areas

### Audit Log Hash Chain Verification

**Issue:** The audit log uses a SHA256 hash chain where each entry's `entry_hash` is computed from the previous entry's hash (`genossi_service_impl/src/audit_log.rs:7-43`). A verification endpoint exists (`genossi_rest/src/audit_log.rs:218`), but:
1. No automatic background verification runs
2. Hash chain is only as strong as the initial seed (first entry's `prev_hash` is user-supplied)
3. If the audit table itself is corrupted (rows deleted or reordered), the chain verification may not detect it

**Files:**
- `genossi_service_impl/src/audit_log.rs:7-43` — hash computation
- `genossi_rest/src/audit_log.rs:218` — verification endpoint
- `genossi_dao_impl_sqlite/src/audit_timestamp.rs` — RFC 3161 timestamping (related)

**Why fragile:** 
- Manual verification required; no alerting if chain is broken
- Depends on entry order, which SQLite maintains via rowid but is not guaranteed by SQL semantics
- No tamper detection if the database file itself is modified offline

**Safe modification:**
- Run `/api/audit/verify` endpoint periodically and alert on failures
- Consider adding an RFC 3161 timestamp to the initial entry as a trust anchor
- Add NOT NULL constraint and UNIQUE index on (transaction_id, field_name) to prevent duplicate entries
- Document that the hash chain proves integrity of the audit_log table only if rows are not deleted/reordered

---

### Version Field for Optimistic Locking

**Issue:** Optimistic locking is implemented via a `version` UUID field. When an update is received, the service layer checks if `entity.version == update.version` (`genossi_service_impl/src/application.rs:498`), and rejects with a Conflict error if they don't match. On successful update, the version is regenerated with a new UUID.

**Files:** 
- `genossi_service_impl/src/application.rs:498` — version check
- `genossi_service_impl/src/member.rs` — similar pattern
- `genossi_rest_types/src/lib.rs` — API transfer objects include version

**Why fragile:**
- Version check is not atomic with the update in SQLite (transaction is used, but the check happens in the service layer, not a database constraint)
- Client could receive stale version and submit update with old version, causing Conflict
- No automatic retry/merging logic — client must handle Conflict and refetch

**Safe modification:**
- Document that clients must refetch entity after Conflict error
- Consider adding a database-level version column with DEFAULT and auto-increment for simpler implementation (numeric version is easier to reason about than UUID)
- Add integration test that reproduces concurrent update scenario and verifies Conflict is returned
- This is acceptable as-is if clients handle Conflict gracefully

---

### Soft Delete Semantics

**Issue:** Soft deletes are implemented by setting a `deleted` timestamp field. Queries use `WHERE deleted IS NULL` to exclude soft-deleted rows. This works but requires discipline across all query code.

**Files:** 
- `genossi_service_impl/src/audit_macros.rs:82-110` — soft delete sets deleted timestamp
- `genossi_dao_impl_sqlite/src/member.rs` — queries check `deleted IS NULL`
- Multiple DAO implementations

**Why fragile:**
- Easy to miss a WHERE clause and return soft-deleted data
- No database-level enforcement; a direct SQL query can see deleted rows
- Soft-deleted rows take up space and slow down queries (indexes include them)

**Safe modification:**
- Add a database-level trigger or view that prevents querying the raw table (force use of a view that filters `deleted IS NULL`)
- Add test cases for each DAO method to verify soft-deleted rows are not returned
- Consider a naming convention: queries that include soft-deleted rows get a suffix (`_all()` vs. `all()`)

---

### Datetime Parsing Flexibility

**Issue:** Multiple datetime formats are supported for compatibility:
1. ISO8601 (preferred) — `2025-09-21T13:25:15.454309545Z`
2. SQLite default — `2025-09-21 13:25:15`
3. SQLite with subsecond — `2025-09-21 13:25:15.454309545`

See `genossi_mail/src/dao_sqlite.rs:14-30` and similar code in other DAO implementations.

**Files:**
- `genossi_mail/src/dao_sqlite.rs:14-30` — parse_datetime fallback chain
- `genossi_dao_impl_sqlite/src/member.rs:20-30` — similar
- `genossi-frontend/rest-types/src/lib.rs:54-87` — frontend datetime handling

**Why fragile:**
- Multiple format strings create maintenance burden (changes to time crate format syntax require updates in multiple files)
- `.unwrap()` calls on format parsing (`parse("[year]-[month]-[day]").unwrap()` at lines 20, 25, 30)
- If a new format is introduced, all DAOs must be updated

**Safe modification:**
- Extract datetime parsing/formatting to a single utility module used by all DAOs
- Consider standardizing on ISO8601 only (no fallback) and migrate existing data in a migration
- Use `expect()` with a descriptive message for format parse (which should never fail since format is hardcoded)
- Add test cases for edge cases (leap year, subsecond precision, timezone handling)

---

## Test Coverage Gaps

### Frontend Browser Integration (Manual Testing)

**Issue:** Frontend requires manual smoke testing in a real browser to verify Dioxus component rendering and Tailwind CSS styles. This is noted in the security audit findings (commit 7600e3c) as an open task.

**Files:** `genossi-frontend/` (all)

**What's not tested:**
- Component layout and styling (Tailwind CSS application)
- JavaScript interop (`web_sys::window()`, `JsFuture`, etc.)
- DOM event handlers and state changes in the browser
- Responsive design and mobile layout

**Risk:** High — layout bugs, broken interactions, or styling regressions only surface when someone actually opens the app in a browser

**Priority:** Medium (functional API tests catch most bugs; styling is less critical)

---

### E2E Test Coverage for CORS

**Issue:** The security quick-fixes commit (7600e3c) added tests for CORS method/header whitelists, but coverage may still be incomplete.

**Files:** `genossi_bin/tests/e2e_tests.rs` — verify CORS test count and coverage

**What to verify:**
- Preflight requests for disallowed methods (e.g., PATCH, HEAD) are rejected
- Preflight requests for disallowed headers (e.g., X-Custom-Header) are rejected
- Non-preflight requests with allowed methods/headers succeed
- Origins not in the whitelist are rejected

---

### Audit Log Verification Endpoint Not Tested

**Issue:** The `/api/audit/verify` endpoint verifies the hash chain, but no E2E test exists that:
1. Corrupts an audit log entry (simulating tampering)
2. Calls verify endpoint
3. Checks that broken links are detected

**Files:** 
- `genossi_rest/src/audit_log.rs:218` — verify endpoint
- No E2E test in `genossi_bin/tests/e2e_tests.rs`

**Risk:** Hash chain verification code could have a bug and never be detected until a real tamper occurs

**Priority:** High (security-critical code should have test coverage)

---

### SQLx Offline Query Data Staleness

**Issue:** `.sqlx/` directory contains pre-compiled query metadata (26 `.json` files as of 2026-04-17). This data is used during compilation to validate SQL queries at build time. If migrations have been added since the last `cargo sqlx prepare` run, new queries may not have offline data.

**Files:** `.sqlx/` (last modified 2026-04-17)

**Risk:**
- If new SQL queries are added without running `cargo sqlx prepare`, the build will fail during compilation with sqlx offline mode enabled
- Developers must remember to run this command after adding/changing SQL
- CI must ensure offline data is up to date

**Recommendations:**
1. Add `cargo sqlx prepare` as a pre-commit hook or CI step
2. Document in CLAUDE.md: "Run `DATABASE_URL=sqlite:genossi.db cargo sqlx prepare` after adding new queries"
3. Check `.sqlx/` files into git to catch staleness in code review

---

## Missing Critical Features

### No Automatic Secrets Rotation

**Issue:** Configuration secrets (SMTP password, WebDAV credentials, API keys) are static once set in the database. There's no mechanism to rotate them without manual database intervention.

**Files:** `genossi_config/src/dao_sqlite.rs`, `genossi_config/src/rest.rs`

**Problem:** 
- Compromised credentials can't be invalidated without taking the app offline
- No audit trail of secret changes
- No support for key versioning (if encryption is added later)

**Recommendation:** 
- This is low priority for MVP but should be addressed before production deployment
- Consider a separate `config_secret_rotation` API (admin-only) that accepts new secret value, stores old value for rollback, and logs change

---

### GDPR Anonymization Not Implemented

**Issue:** No mechanism to anonymize deleted member PII in the database and audit logs (GDPR Art. 17). The soft-delete approach preserves data indefinitely.

**Files:** All member-related code

**Problem:**
- Cannot fully comply with user requests to erase their data
- Audit logs retain sensitive information (name, email, bank account) even after member deletion

**Recommendation:** 
Implement anonymization endpoint as proposed in `openspec/changes/gdpr-data-protection/`:
1. Admin-only endpoint: `POST /api/member/{id}/anonymize`
2. Replace PII in member table: name → "ANONYMIZED", email → hash, bank info → null
3. Replace PII in audit log: old_value/new_value → hashes or null
4. Log the anonymization action itself
5. This is tracked in proposals and should be implemented before production

---

## Scaling Limits

### SQLite as Single-Threaded Bottleneck

**Issue:** SQLite is single-threaded for write operations. While the connection pool allows multiple connections, writes are serialized at the database level. For high-concurrency workloads, this becomes a bottleneck.

**Current capacity:** Suitable for a single-server organization/association (~100-1000 active members)

**Limit:** Reaches bottleneck around 10+ concurrent write transactions (e.g., multiple users updating members, documents, or mail jobs simultaneously)

**Scaling path:**
- For small to medium deployments (< 500 members): SQLite is fine; use read replicas if needed
- For large deployments (> 1000 members, high concurrency): Migrate to PostgreSQL or MySQL
- The DAO layer is abstraction-friendly; `genossi_dao_impl_sqlite` would have a `genossi_dao_impl_postgres` equivalent

---

## Dependencies at Risk

### serde_json Panic Handling (FIXED in 7600e3c)

**Issue:** The security quick-fixes commit eliminated 52 instances of `.unwrap()` on `serde_json::to_string()` calls that could panic. This is now fixed via:
- Uniform `error_handler(async { ... }.await)` pattern with `?` propagation
- `From<serde_json::Error>` impls in RestError, MailServiceError, ConfigServiceError

**Files:** Covered by commit 7600e3c (fixed)

**Status:** Resolved. No further action needed.

---

### Error Handling Inconsistency (Proposal: backend-error-thiserror-migration)

**Issue:** Backend crates (`genossi_rest`, `genossi_config`, `genossi_mail`, `genossi_service`) define error enums manually with custom `From` impls and missing `Display` trait. The frontend already uses `thiserror` crate (see `genossi-frontend/src/error.rs`). This creates inconsistency and boilerplate.

**Files:**
- `genossi_rest/src/lib.rs:74-87` — RestError with manual From impls
- `genossi_config/src/service.rs` — ConfigServiceError (likely similar)
- `genossi_mail/src/service.rs` — MailServiceError (likely similar)

**Impact:** 
- Each new error source requires 5-10 lines of From/Into boilerplate
- No unified Display impls; error handling relies on string formatting in handlers
- Harder to add centralized error logging

**Recommendation:** 
- Migrate to `#[derive(thiserror::Error)]` and `#[from]` attributes
- Add `#[error("...")]` display messages to all variants
- Keep manual From impls where mapping logic is non-trivial
- Tracked as proposal in `openspec/changes/backend-error-thiserror-migration/`

---

## Architecture Concerns

### Circular Concerns Between Auth and Permissions

**Issue:** Authentication (extracting user from request) and authorization (checking permissions) are intertwined in `genossi_rest/src/auth_middleware.rs`. The middleware both extracts context and checks permissions in some handlers, making the separation of concerns unclear.

**Files:** `genossi_rest/src/auth_middleware.rs:38-99` — require_authentication and require_admin both handle context extraction

**Why fragile:** 
- Changes to auth logic may inadvertently affect permission checks
- Testing requires both auth and permission services to be mocked
- Hard to add new permission checks without touching middleware code

**Recommendation:** 
- Keep middleware focused on auth context extraction only
- Move permission checks into handlers or a dedicated permission-check middleware
- Current design works but is somewhat monolithic

---

### Bearer Token Treated as Session ID

**Issue:** `genossi_rest/src/auth_middleware.rs:126` treats a Bearer token the same as a session cookie — both are passed to `session_service.extract_auth_context()`. This conflates two different auth mechanisms.

**Files:** `genossi_rest/src/auth_middleware.rs:120-131`

**Why fragile:**
- Comment says "In a real implementation, this might validate JWT tokens differently"
- If Bearer tokens are JWTs, they should be validated with a public key, not by looking them up in the session table
- If they're opaque tokens, they should be stored in a separate token table

**Recommendation:** 
- Clarify the intended Bearer token validation strategy:
  - Option A: JWT tokens (validate signature, ignore session table)
  - Option B: Opaque tokens (validate against token table with expiry)
  - Option C: API keys (validate against API key table)
- Implement the chosen strategy explicitly, with separate code paths from session auth

---

## Outstanding Proposals (openspec/changes/)

The following proposals have been filed and should be prioritized:

| Proposal | Priority | Category | Status |
|----------|----------|----------|--------|
| `config-secrets-encryption` | medium | security | Proposal stage (design.md present) |
| `gdpr-data-protection` | medium | security | Proposal stage |
| `fix-upload-body-limit` | high | bugfix | Proposal stage (HIGH: users can't upload files > 2 MB) |
| `code-quality-hardening` | low | quality | Proposal stage (N1, N3, N4, I1 — DI, pool config, CHECK constraints, cargo-audit) |
| `backend-error-thiserror-migration` | low | quality | Proposal stage (code cleanup, boilerplate reduction) |
| `assembly-checkin` | unknown | feature | Proposal stage |
| `user-menu-dropdown` | unknown | feature | Proposal stage |

**Action:** Review proposals in priority order. `fix-upload-body-limit` is HIGH and should be completed soon.

---

*Concerns audit: 2026-05-02*
