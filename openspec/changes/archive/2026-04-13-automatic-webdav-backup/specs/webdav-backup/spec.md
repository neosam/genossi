## ADDED Requirements

### Requirement: WebDAV backup worker
The system SHALL run a background worker that periodically uploads backup data to a WebDAV server. The worker SHALL read its configuration from the config store on each iteration and sleep for the configured interval between runs.

#### Scenario: Worker runs when enabled
- **WHEN** `backup_webdav_enabled` config is set to `true` and all required config keys are present
- **THEN** the worker performs a full backup cycle (members, actions, documents) and uploads to the configured WebDAV server

#### Scenario: Worker skips when disabled
- **WHEN** `backup_webdav_enabled` config is set to `false` or missing
- **THEN** the worker sleeps for the configured interval without performing any backup

#### Scenario: Worker sleeps for configured interval
- **WHEN** a backup cycle completes (success or failure)
- **THEN** the worker sleeps for `backup_interval_hours` hours (default: 24) before the next cycle

### Requirement: Member CSV upload per year
The system SHALL generate and upload one member CSV file per calendar year, from the earliest member join date year through the previous completed year. Each CSV SHALL use 31.12. of that year as the snapshot date. Files SHALL be named `mitgliederliste-{year}.csv`.

#### Scenario: Upload yearly member CSVs
- **WHEN** the earliest member join date is in 2020 and the current year is 2026
- **THEN** the worker uploads `mitgliederliste-2020.csv` through `mitgliederliste-2025.csv`, each with the respective 31.12. snapshot date

#### Scenario: CSV format matches manual export
- **WHEN** a yearly member CSV is generated
- **THEN** the CSV contains the same columns and format as the manual backup export (UTF-8 with BOM, semicolon-separated headers: Mitgliedsnummer, Anrede, Titel, Vorname, Nachname, etc.)

### Requirement: Current member CSV upload
The system SHALL generate and upload `mitgliederliste-aktuell.csv` with today's date as the snapshot date on every backup run.

#### Scenario: Upload current member list
- **WHEN** a backup cycle runs on 2026-04-12
- **THEN** the worker uploads `mitgliederliste-aktuell.csv` with snapshot date 2026-04-12

### Requirement: Actions CSV upload
The system SHALL generate and upload `aktionen.csv` containing all actions on every backup run, overwriting the previous version.

#### Scenario: Upload actions CSV
- **WHEN** a backup cycle runs
- **THEN** the worker uploads `aktionen.csv` to the configured WebDAV directory, replacing any existing file

### Requirement: Document upload with delta sync
The system SHALL upload documents as individual files organized in member subdirectories under a `dokumente/` directory. Only new or changed documents SHALL be uploaded based on content hash comparison.

#### Scenario: Upload new document
- **WHEN** a document exists in the system with no corresponding entry in the `backup_document_sync` table
- **THEN** the worker uploads the document via WebDAV PUT and stores its SHA-256 hash in the sync table

#### Scenario: Skip unchanged document
- **WHEN** a document exists and its SHA-256 hash matches the hash stored in the `backup_document_sync` table
- **THEN** the worker skips the upload for that document

#### Scenario: Re-upload changed document
- **WHEN** a document exists and its SHA-256 hash differs from the hash in the `backup_document_sync` table
- **THEN** the worker uploads the updated document and updates the stored hash

#### Scenario: Document directory structure
- **WHEN** a document belongs to member 001 (Hans Müller) with document type "Beitrittserklärung" and filename "beitritt.pdf"
- **THEN** the document is uploaded to `dokumente/001_Müller_Hans/Beitrittserklärung_beitritt.pdf`

### Requirement: Document sync tracking table
The system SHALL maintain a `backup_document_sync` table with columns:
- `relative_path` (TEXT, primary key): the document's path in local storage
- `content_hash` (TEXT): SHA-256 hex digest of the document content
- `last_uploaded` (TEXT): ISO8601 timestamp of the last successful upload

#### Scenario: Sync entry created on first upload
- **WHEN** a document is uploaded for the first time
- **THEN** a new row is created with the document's relative_path, content_hash, and current timestamp

#### Scenario: Sync entry updated on re-upload
- **WHEN** a changed document is re-uploaded
- **THEN** the existing row's content_hash and last_uploaded are updated

### Requirement: WebDAV directory creation
The system SHALL create required directories on the WebDAV server using MKCOL before uploading files. Directory creation SHALL be idempotent (ignore 405 Method Not Allowed if directory already exists).

#### Scenario: Create base directory
- **WHEN** the first backup runs
- **THEN** the worker creates the configured base directory (e.g., `genossi-export`) via MKCOL

#### Scenario: Create member document subdirectory
- **WHEN** uploading a document for a member whose subdirectory does not yet exist
- **THEN** the worker creates the subdirectory (e.g., `genossi-export/dokumente/001_Müller_Hans/`) via MKCOL

#### Scenario: Directory already exists
- **WHEN** MKCOL returns 405 (Method Not Allowed) because the directory already exists
- **THEN** the worker ignores the error and continues

### Requirement: WebDAV authentication
The system SHALL authenticate WebDAV requests using HTTP Basic Authentication with the configured username and password.

#### Scenario: Successful authentication
- **WHEN** valid WebDAV credentials are configured
- **THEN** all WebDAV requests include the Basic Auth header and succeed

#### Scenario: Authentication failure
- **WHEN** invalid credentials are configured and a WebDAV request returns 401/403
- **THEN** the worker logs the error and aborts the current backup cycle

### Requirement: Backup error resilience
The system SHALL handle WebDAV errors gracefully. If a backup cycle fails, the worker SHALL log the error, update the backup status, and retry on the next scheduled interval.

#### Scenario: WebDAV server unreachable
- **WHEN** the WebDAV server is unreachable during a backup cycle
- **THEN** the worker logs the error, sets `backup_last_status` to an error message, and retries on the next interval

#### Scenario: Individual document upload failure
- **WHEN** a single document fails to upload but others succeed
- **THEN** the worker logs the failure, continues with remaining documents, and does not update the sync hash for the failed document

### Requirement: Backup status tracking
The system SHALL update config entries `backup_last_run` (ISO8601 timestamp) and `backup_last_status` (success message or error description) after each backup cycle.

#### Scenario: Successful backup
- **WHEN** a backup cycle completes without errors
- **THEN** `backup_last_run` is set to the current timestamp and `backup_last_status` is set to a success message

#### Scenario: Failed backup
- **WHEN** a backup cycle fails
- **THEN** `backup_last_run` is set to the current timestamp and `backup_last_status` is set to the error description

### Requirement: Backup logging
The system SHALL log backup progress and errors using the `tracing` framework at appropriate levels: INFO for cycle start/completion and document counts, WARN for individual file failures, ERROR for cycle-level failures.

#### Scenario: Log backup start
- **WHEN** a backup cycle begins
- **THEN** the worker logs an INFO message with the WebDAV target URL

#### Scenario: Log document sync progress
- **WHEN** documents are being synced
- **THEN** the worker logs INFO with total document count, uploaded count, and skipped count

#### Scenario: Log individual upload failure
- **WHEN** a single document upload fails
- **THEN** the worker logs a WARN message with the document path and error details
