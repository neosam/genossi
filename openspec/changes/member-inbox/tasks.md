## 1. Dependencies & Setup

- [x] 1.1 Add `async-imap` and `mail-parser` to `genossi_mail/Cargo.toml`
- [x] 1.2 Verify the crates compile on the project toolchain

## 2. Database

- [x] 2.1 Create migration `migrations/sqlite/<ts>_create_inbound_mails_table.sql` with columns per the spec and `UNIQUE(uid_validity, imap_uid)`
- [x] 2.2 Run migration locally and verify schema
- [x] 2.3 `cargo sqlx prepare`

## 3. Config Store

- [x] 3.1 Document and support new IMAP config keys (`imap_host`, `imap_port`, `imap_user`, `imap_pass`, `imap_tls`, `imap_mailbox`, `imap_archive_mailbox`, `imap_poll_interval_seconds`)
- [x] 3.2 Extend any config-key enum or constants list where SMTP keys are declared

## 4. DAO Layer

- [x] 4.1 Add `InboundMail` struct in `genossi_mail/src/dao.rs` with trait `InboundMailDao` (create, get_by_id, list_active, list_by_status, assign_member, unassign, set_status, exists_by_uid)
- [x] 4.2 Implement the trait for SQLite in `genossi_mail/src/dao_sqlite.rs`
- [x] 4.3 Unit-test DAO against an in-memory SQLite database

## 5. Inbox Service

- [x] 5.1 Define `InboxService` trait in `genossi_mail/src/service.rs` (list, get, assign, unassign, mark_read, archive, ignore)
- [x] 5.2 Implement the trait with access to the DAO, config store, and an IMAP client factory
- [x] 5.3 Implement `mark_read` and `archive` to connect to IMAP and apply the server-side effect synchronously; on failure return an error and leave local state unchanged
- [x] 5.4 Mock-based unit tests for the service

## 6. Inbox Worker

- [x] 6.1 Create `genossi_mail/src/inbox_worker.rs` with a tokio task running a poll loop driven by `imap_poll_interval_seconds`
- [x] 6.2 On each tick: read config, connect, select mailbox, detect `UIDVALIDITY`, fetch UIDs greater than the highest stored UID for that validity
- [x] 6.3 For each fetched message: parse via `mail-parser`, extract text body (prefer `text/plain`), detect HTML, count attachments, read `In-Reply-To`, insert into DAO
- [x] 6.4 Do NOT set `\Seen` or move messages during polling
- [x] 6.5 Handle missing config, connection errors, and auth errors by logging and skipping the cycle (no crashes)
- [x] 6.6 Wire the worker into `genossi_bin/src/lib.rs` alongside the send worker

## 7. REST Layer

- [x] 7.1 Add REST module for `/api/inbox` in `genossi_mail/src/rest.rs` (or a new sibling module)
- [x] 7.2 Endpoints: `GET /api/inbox`, `GET /api/inbox/{id}`, `POST /api/inbox/{id}/assign`, `POST /api/inbox/{id}/unassign`, `POST /api/inbox/{id}/mark-read`, `POST /api/inbox/{id}/archive`, `POST /api/inbox/{id}/ignore`
- [x] 7.3 Annotate with `utoipa` for OpenAPI
- [x] 7.4 Register the routes in `genossi_rest/src/lib.rs`
- [x] 7.5 Define request/response types in `genossi_rest_types` and re-export where needed
- [x] 7.6 Integration tests for each endpoint against an in-memory backend (IMAP side-effects stubbed)

## 8. Frontend

- [x] 8.1 Add REST client bindings for `/api/inbox` endpoints in `genossi-frontend/src/api.rs`
- [x] 8.2 Add route `/inbox` and a nav entry in `top_bar.rs`
- [x] 8.3 Implement `genossi-frontend/src/page/inbox_page.rs` with list view (sender, subject, received date, assignment label)
- [x] 8.4 Detail view: text body, "hat Anhänge"-indicator, member-assignment dropdown with sender-email-based prefill
- [x] 8.5 Action buttons: mark read, archive, ignore, unassign
- [x] 8.6 Add i18n strings (de + en)

## 9. E2E Tests

- [x] 9.1 E2E test: seed an `inbound_mails` row directly via DAO, call `GET /api/inbox`, assert payload shape
- [x] 9.2 E2E test: assign a mail to a member, assert `status = assigned` and assignment label in list response
- [x] 9.3 E2E test: ignore a mail, assert it disappears from the default list
- [x] 9.4 E2E test: archive endpoint with IMAP side-effect stubbed, assert `status = archived`

## 10. Verification

- [x] 10.1 `cargo fmt` and `cargo clippy --all-targets`
- [x] 10.2 `cargo test` across the workspace
- [x] 10.3 Manual smoke test against a real IMAP mailbox with a test account
- [x] 10.4 `openspec validate member-inbox --strict`
