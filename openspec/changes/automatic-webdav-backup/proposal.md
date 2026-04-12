## Why

Users can currently export backups manually (member CSV, actions CSV, documents ZIP), but there is no automated backup. If someone forgets to run the export, data could be lost. An automatic, scheduled backup to NextCloud via WebDAV ensures continuous off-site data protection without manual intervention.

## What Changes

- New background worker that periodically uploads backups to a configurable WebDAV server (NextCloud)
- Uploads member CSVs per year (snapshot at 31.12.) plus a current-date CSV, actions CSV (overwritten each run), and individual document files organized by member subdirectories
- Only changed/new documents are uploaded (delta sync tracked via local DB table with content hashes)
- Year range derived automatically from earliest member join date to current year
- WebDAV configuration (URL, credentials, target directory, interval, enabled flag) stored in the existing config system
- Frontend configuration UI section on the existing config page (analogous to SMTP/IMAP settings)
- Backup status tracking (last run timestamp, last status) visible in the frontend
- Resilient to NextCloud downtime: errors are logged and retried on next interval

## Capabilities

### New Capabilities
- `webdav-backup`: Automatic scheduled backup of members, actions, and documents to a WebDAV server with delta document sync and configurable interval
- `webdav-backup-config-ui`: Frontend configuration section for WebDAV backup settings with status display

### Modified Capabilities
- `config-store`: New config keys for WebDAV backup settings (url, username, password, directory, interval, enabled, last_run, last_status)

## Impact

- **New crate**: `genossi_backup` with WebDAV client, worker loop, and backup generation logic
- **Refactoring**: CSV/document generation logic extracted from `genossi_rest/src/backup.rs` into shared code usable by both REST endpoints and the backup worker
- **Database**: New `backup_document_sync` table for tracking uploaded document hashes; new config entries for WebDAV settings
- **Dependencies**: `reqwest` (already available) for WebDAV HTTP calls
- **Binary**: New background worker spawned in `genossi_bin` alongside mail/inbox workers
- **Frontend**: New section on `ConfigPage` for WebDAV backup configuration and status display
