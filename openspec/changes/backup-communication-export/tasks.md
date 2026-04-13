## 1. Database Layer

- [x] 1.1 Create migration `backup_communication_sync` table (columns: mail_type TEXT, mail_id BLOB, synced_at TEXT)
- [x] 1.2 Add `CommunicationBackupRow` struct to `genossi_dao/src/backup.rs` (member_number, first_name, last_name, direction, date, subject, body, from_address, to_address, mail_id)
- [x] 1.3 Add `all_communications()` method to `BackupDao` trait
- [x] 1.4 Add `BackupCommunicationSyncDao` trait with `is_synced(mail_type, mail_id)` and `mark_synced(mail_type, mail_id)` methods
- [x] 1.5 Implement `all_communications()` in `genossi_dao_impl_sqlite/src/backup.rs` — JOIN outbound (mail_recipients + mail_jobs + members) and inbound (inbound_mails + members) with UNION
- [x] 1.6 Implement `BackupCommunicationSyncDao` for SQLite

## 2. Generator

- [x] 2.1 Add filename sanitization helper to `genossi_backup/src/generator.rs` (transliterate umlauts, strip special chars, truncate to 50 chars)
- [x] 2.2 Add `generate_communication_filename()` function (takes date, direction, subject, optional UUID suffix for collision)
- [x] 2.3 Add `generate_communication_txt()` function (takes CommunicationBackupRow, returns String with header + separator + body)
- [x] 2.4 Add unit tests for filename sanitization (umlauts, special chars, truncation, spaces)
- [x] 2.5 Add unit tests for .txt content generation (inbound/outbound variants)

## 3. REST Backup Endpoint

- [x] 3.1 Extend `export_documents()` in `genossi_rest/src/backup.rs` to also fetch `all_communications()` from BackupDao
- [x] 3.2 Group communications by member directory name and write .txt files into `{member_dir}/kommunikation/` within the ZIP
- [x] 3.3 Handle filename collisions (detect duplicates, append UUID prefix)
- [x] 3.4 Add integration test: ZIP contains communication files for members with mails
- [x] 3.5 Add integration test: ZIP excludes mails without member assignment

## 4. WebDAV Worker

- [x] 4.1 Add communication sync step to `run_backup_cycle()` in `genossi_backup/src/worker.rs`
- [x] 4.2 Implement `sync_communications()` in `genossi_backup/src/sync.rs` — iterate new (un-synced) communications, upload .txt, mark as synced
- [x] 4.3 Create member directories and `kommunikation/` subfolders on WebDAV as needed
- [x] 4.4 Update backup status message to include communication sync stats
- [x] 4.5 Add unit test: new mails are uploaded and marked as synced
- [x] 4.6 Add unit test: already-synced mails are skipped

## 5. Wiring

- [x] 5.1 Add `BackupCommunicationSyncDao` to application state / dependency injection in `genossi_bin`
- [x] 5.2 Pass sync DAO to backup worker
- [x] 5.3 Add `all_communications()` access to `RestStateDef` trait if needed (not needed, BackupDao trait already accessible)
- [x] 5.4 Run end-to-end test confirming ZIP download includes communication files
