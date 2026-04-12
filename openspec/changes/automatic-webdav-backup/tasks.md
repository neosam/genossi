## 1. Database & Migration

- [x] 1.1 Create SQLite migration for `backup_document_sync` table (relative_path TEXT PK, content_hash TEXT, last_uploaded TEXT)

## 2. Crate Setup & Shared Backup Logic

- [x] 2.1 Create `genossi_backup` crate with Cargo.toml (dependencies: reqwest, sha2, genossi_dao, genossi_service, genossi_config, tracing, tokio, async-trait, csv, time)
- [x] 2.2 Extract CSV generation logic from `genossi_rest/src/backup.rs` into `genossi_backup/src/generator.rs` (members CSV, actions CSV as `Vec<u8>`)
- [x] 2.3 Refactor `genossi_rest/src/backup.rs` REST handlers to use the shared generator functions
- [x] 2.4 Add generator function for earliest join year query (new DAO method or extend BackupDao)

## 3. WebDAV Client

- [x] 3.1 Implement `genossi_backup/src/webdav.rs` with WebDAV client struct (base_url, auth) and methods: `mkcol(path)`, `put(path, data)`
- [x] 3.2 Handle MKCOL idempotency (ignore 405 when directory already exists)
- [x] 3.3 Add unit tests for WebDAV client (mock HTTP responses)

## 4. Document Sync

- [x] 4.1 Implement DAO for `backup_document_sync` table (get_hash, upsert_hash) in `genossi_dao_impl_sqlite`
- [x] 4.2 Define DAO trait in `genossi_dao/src/backup.rs` (or new file) for document sync operations
- [x] 4.3 Implement delta sync logic in `genossi_backup/src/sync.rs`: compute SHA-256, compare with stored hash, upload only if different
- [x] 4.4 Add unit tests for delta sync logic (mock DAO and WebDAV)

## 5. Backup Worker

- [x] 5.1 Implement `genossi_backup/src/worker.rs` with main worker loop: read config, check enabled, run backup cycle, update status, sleep
- [x] 5.2 Implement backup cycle: upload yearly member CSVs, current member CSV, actions CSV, sync documents
- [x] 5.3 Implement config reading helpers (parse WebDAV URL, credentials, interval, enabled flag from config entries)
- [x] 5.4 Implement status tracking: write `backup_last_run` and `backup_last_status` config entries after each cycle
- [x] 5.5 Add tracing logging: INFO for cycle start/complete, document counts; WARN for individual failures; ERROR for cycle failures
- [x] 5.6 Add unit tests for worker logic (mock config service, DAO, WebDAV client)

## 6. Binary Integration

- [x] 6.1 Add `genossi_backup` dependency to `genossi_bin/Cargo.toml`
- [x] 6.2 Add `start_backup_worker` method to `RestStateImpl` in `genossi_bin/src/lib.rs` (following mail/inbox worker pattern)
- [x] 6.3 Call `start_backup_worker()` in `main.rs` alongside existing workers

## 7. Frontend: Config UI

- [x] 7.1 Add i18n keys for WebDAV backup section (German, English, Czech): labels, placeholders, status messages
- [x] 7.2 Add WebDAV backup form state signals to `ConfigPage` (url, username, password, directory, interval, enabled, password_set, last_run, last_status)
- [x] 7.3 Load existing backup config values in the `reload` function
- [x] 7.4 Implement WebDAV backup settings section UI (input fields, toggle, save button) consistent with SMTP/IMAP sections
- [x] 7.5 Implement save handler: persist all backup config entries via existing config API
- [x] 7.6 Implement backup status display (last run, status with color coding)
- [x] 7.7 Handle password field behavior (show "gesetzt" indicator, don't overwrite on empty save)

## 8. End-to-End Testing

- [x] 8.1 Add E2E tests for backup worker with mock WebDAV server (verify uploads, directory creation, delta sync)
- [x] 8.2 Add E2E test for config page backup section (verify settings persist and load correctly)
