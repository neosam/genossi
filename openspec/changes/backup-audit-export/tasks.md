## 1. DAO Erweiterung

- [x] 1.1 Add `get_pending_upload` method to AuditTimestampDao trait (entries with webdav_path IS NULL AND status = 'success')
- [x] 1.2 Add `update_webdav_path` method to AuditTimestampDao trait (id, path)
- [x] 1.3 Implement both methods in AuditTimestampDaoImpl for SQLite
- [x] 1.4 Write unit tests for get_pending_upload and update_webdav_path

## 2. Audit-Log CSV Generator

- [x] 2.1 Add `generate_audit_log_csv` function in genossi_backup (takes AuditLogEntry slice, returns CSV bytes)
- [x] 2.2 Write unit tests for CSV generation (empty log, multiple entries, special characters in values)

## 3. Backup-Worker Integration

- [x] 3.1 Add AuditLogDao and AuditTimestampDao as dependencies to backup worker function signatures
- [x] 3.2 Add audit-log CSV export step: fetch all entries, generate CSV, upload to WebDAV as `audit-log.csv`
- [x] 3.3 Add .tsr upload step: fetch pending uploads, upload each to `audit-timestamps/`, update webdav_path
- [x] 3.4 Update status message to include audit export results
- [ ] 3.5 Write tests for backup worker with audit export (CSV upload, .tsr upload, failure handling)

## 4. Timestamp-Worker Cleanup

- [x] 4.1 Remove WebDAV upload code from timestamp_worker.rs
- [x] 4.2 Remove genossi_backup dependency from genossi_service_impl if no longer needed

## 5. Wiring

- [x] 5.1 Pass AuditLogDao and AuditTimestampDao to backup worker in genossi_bin
- [x] 5.2 Run full test suite to verify nothing is broken
