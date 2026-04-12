## Context

Genossi currently supports manual backup exports via REST endpoints (`/api/backup/members`, `/api/backup/actions`, `/api/backup/documents`). These produce CSVs and a ZIP file on demand, protected by the `EXPORT_BACKUP` permission. There is no automated backup mechanism — data protection depends on users remembering to trigger exports.

The system already has two background workers (mail worker, inbox worker) that follow the same pattern: `tokio::spawn` with a loop that reads config from the DB and sleeps between iterations. Configuration is stored in the `config_entries` table and managed via the existing config UI.

NextCloud supports WebDAV natively at `remote.php/dav/files/<username>/`, making it the natural upload protocol. Only `PUT` (upload file) and `MKCOL` (create directory) are needed.

## Goals / Non-Goals

**Goals:**
- Automatic, scheduled backup of member lists, actions, and documents to NextCloud via WebDAV
- Delta sync for documents (only upload new/changed files) to minimize bandwidth
- Configurable via the existing frontend config page
- Resilient to transient NextCloud failures
- Visible backup status for administrators

**Non-Goals:**
- Backup restore functionality (out of scope — restore is a manual process)
- Encryption of backup data (NextCloud handles storage-level encryption)
- WebDAV operations beyond PUT/MKCOL (no PROPFIND, DELETE, MOVE)
- Multi-server backup (single WebDAV target)
- Backup of the SQLite database file itself

## Decisions

### 1. New `genossi_backup` crate

**Decision:** Create a dedicated crate rather than extending `genossi_rest` or `genossi_mail`.

**Rationale:** The backup worker has its own concerns (WebDAV protocol, document sync tracking, backup generation) that don't belong in the REST or mail layers. A dedicated crate keeps the layered architecture clean and allows independent testing.

**Alternative considered:** Adding to `genossi_mail` since it already has a worker pattern. Rejected because backup has no relation to mail functionality.

### 2. WebDAV via raw `reqwest` HTTP calls

**Decision:** Use `reqwest` directly for WebDAV operations (PUT, MKCOL) with Basic Auth, rather than adding a dedicated WebDAV client crate.

**Rationale:** We only need two HTTP methods. A full WebDAV client library would add dependency weight for unused features. `reqwest` is already in the dependency tree.

**Alternative considered:** `hyperdav` or `webdav-handler` crates. These add PROPFIND/LOCK/COPY support we don't need.

### 3. Delta document sync via local hash table

**Decision:** Track uploaded documents in a new `backup_document_sync` table with columns `(relative_path TEXT PK, content_hash TEXT, last_uploaded TEXT)`. Before uploading, compute SHA-256 of the document content and compare against the stored hash. Only upload if the hash differs or no entry exists.

**Rationale:** Avoids the need for WebDAV PROPFIND to check remote state. Keeps the WebDAV interaction minimal (PUT + MKCOL only). The local table is authoritative for sync state.

**Alternative considered:** Using WebDAV PROPFIND to compare remote file sizes/etags. Rejected because it adds protocol complexity and requires handling NextCloud-specific PROPFIND XML responses.

### 4. Year range from earliest join date

**Decision:** Query the earliest `join_date` from members to determine the start year for yearly member CSVs. Generate one CSV per year (stichtag 31.12.) from start year through the previous completed year, plus `mitgliederliste-aktuell.csv` with today's date.

**Rationale:** Automatically adapts to the organization's history without hardcoding. Years with no members at that point will produce empty-ish CSVs, which is acceptable.

### 5. Worker pattern consistent with mail worker

**Decision:** Use the same loop-with-sleep pattern as the mail worker: read config each iteration, sleep for the configured interval, retry on next iteration if errors occur.

**Rationale:** Proven pattern in the codebase. Config changes (interval, enabled/disabled) take effect on the next iteration without restart.

### 6. Config stored in existing config_entries table

**Decision:** Use the existing config store with keys prefixed `backup_webdav_*`. Password stored with `value_type: secret` (masked in API responses).

**Rationale:** Reuses the existing config infrastructure including REST API, frontend components, and secret masking.

### 7. Backup generation extracted to shared module

**Decision:** Extract CSV generation logic from `genossi_rest/src/backup.rs` into functions in `genossi_backup` that return `Vec<u8>`. Both the REST endpoints and the worker call these shared functions. The REST handlers become thin wrappers.

**Rationale:** Avoids duplicating the CSV column definitions and formatting logic.

### 8. Document upload as individual files (not ZIP)

**Decision:** Upload documents as individual files organized in member subdirectories, not as a single ZIP archive.

**Rationale:** NextCloud can version, search, and preview individual files. A ZIP would need to be downloaded and extracted to access any single document. The directory structure (`dokumente/001_Müller_Hans/Beitrittserklärung_beitritt.pdf`) matches the existing ZIP structure from the manual export.

### 9. Status tracking via config entries

**Decision:** Store `backup_last_run` (ISO8601 timestamp) and `backup_last_status` (success/error message) as config entries after each backup run.

**Rationale:** No new DB table needed. The frontend can read these alongside other config entries and display them in the backup config section.

## Risks / Trade-offs

**[Risk] First backup with many documents is slow** → Log progress during the first run. The worker runs in the background so it doesn't block the application. Subsequent runs only upload deltas.

**[Risk] WebDAV credentials stored in DB** → Password stored with `value_type: secret`, masked in API. Same security model as SMTP credentials. For stronger security, users can use NextCloud app-specific passwords with limited scope.

**[Risk] Hash table grows unbounded** → One row per document. Even with thousands of documents, this is negligible. Rows for deleted documents remain but cause no harm (orphaned hashes are never queried).

**[Risk] Concurrent backup runs** → The worker is a single `tokio::spawn` task with sequential execution. No concurrency issues. If the backup takes longer than the interval, the next run simply starts late.

**[Risk] Large file uploads may timeout** → Use `reqwest` with reasonable timeouts (30s per file). Log failures and continue with remaining files. Failed documents will be retried on the next run (hash not updated on failure).

**[Trade-off] No PROPFIND means we can't detect remote deletions** → If someone manually deletes a file from NextCloud, we won't re-upload it unless the local hash entry is also cleared. Acceptable because the backup is append/update-only by design.
