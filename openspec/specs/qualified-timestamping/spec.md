## ADDED Requirements

### Requirement: Audit timestamp data model
The system SHALL store audit timestamp records with the following fields:
- `id` (UUID, system-generated, primary key)
- `timestamp` (DateTime, when the timestamp was created)
- `audit_hash` (String, the audit_log entry_hash that was timestamped)
- `audit_entry_count` (Integer, number of audit_log entries at time of stamping)
- `tsr_token` (BLOB, the raw RFC 3161 TimeStampResp token)
- `webdav_path` (Optional String, path where the .tsr file was uploaded)
- `status` (String, one of "success", "tsa_failed", "upload_failed")

#### Scenario: Timestamp record stored
- **WHEN** a qualified timestamp is successfully obtained from the TSA
- **THEN** the system stores the record with all fields including the raw TSR token

#### Scenario: TSA request fails
- **WHEN** the TSA server is unreachable or returns an error
- **THEN** the system stores a record with status "tsa_failed" and null tsr_token

### Requirement: RFC 3161 timestamp request
The system SHALL create an RFC 3161 TimeStampReq containing the SHA256 hash of the current latest audit_log entry_hash and send it via HTTP POST to the configured TSA endpoint. The TSA endpoint URL SHALL be read from config key `tsa_url`. Authentication credentials SHALL be read from config keys `tsa_user` and `tsa_pass`.

#### Scenario: Successful timestamp request
- **WHEN** the TSA URL is configured and the TSA server is reachable
- **THEN** the system sends an HTTP POST with Content-Type `application/timestamp-query`, receives a TimeStampResp with Content-Type `application/timestamp-reply`, and stores the response token

#### Scenario: TSA not configured
- **WHEN** the config key `tsa_url` is not set or empty
- **THEN** the system skips the timestamp step and logs an INFO message

#### Scenario: TSA authentication
- **WHEN** `tsa_user` and `tsa_pass` are configured
- **THEN** the system includes HTTP Basic Authentication in the TSA request

#### Scenario: TSA authentication not needed
- **WHEN** `tsa_user` and `tsa_pass` are not configured
- **THEN** the system sends the TSA request without authentication

#### Scenario: TSA server unreachable
- **WHEN** the HTTP POST to the TSA server fails due to network error
- **THEN** the system logs an ERROR, stores a record with status "tsa_failed", and continues

#### Scenario: TSA returns error response
- **WHEN** the TSA server returns a TimeStampResp with a non-success status
- **THEN** the system logs an ERROR with the TSA status, stores a record with status "tsa_failed", and continues

### Requirement: Timestamp worker WebDAV responsibility
The timestamp worker SHALL NOT perform WebDAV uploads. The .tsr token upload to WebDAV is handled by the backup worker. The timestamp worker only creates timestamps and stores them locally in the database.

#### Scenario: Timestamp created
- **WHEN** the timestamp worker creates a timestamp successfully
- **THEN** the token is stored in the database with `webdav_path = NULL`; WebDAV upload is deferred to the backup worker

### Requirement: AuditTimestampDao additional methods
The AuditTimestampDao SHALL provide methods to support the backup worker's .tsr upload:
- `get_pending_upload()`: Returns all entries where `webdav_path IS NULL AND status = 'success'`
- `update_webdav_path(id, path)`: Sets the `webdav_path` for a given entry

#### Scenario: Query pending uploads
- **WHEN** the backup worker queries for pending uploads
- **THEN** only entries with status "success" and no webdav_path are returned

#### Scenario: Mark as uploaded
- **WHEN** a .tsr file is uploaded successfully
- **THEN** the backup worker calls `update_webdav_path` with the WebDAV path

### Requirement: Timestamp configuration
The system SHALL use the following config store keys:
- `tsa_url` (string): The RFC 3161 TSA endpoint URL
- `tsa_user` (string, optional): HTTP Basic Auth username
- `tsa_pass` (secret, optional): HTTP Basic Auth password
- `tsa_enabled` (bool, default false): Whether qualified timestamping is enabled
- `tsa_interval_hours` (integer, default 168): Interval in hours between automatic timestamps

#### Scenario: Timestamping enabled
- **WHEN** `tsa_enabled` is set to `true` and `tsa_url` is configured
- **THEN** the timestamp worker performs the timestamp step at the configured interval

#### Scenario: Timestamping disabled
- **WHEN** `tsa_enabled` is set to `false` or not set
- **THEN** the timestamp worker does not run

### Requirement: Eigenständiger Timestamp-Worker
The system SHALL run an independent background worker that periodically creates qualified timestamps. The worker SHALL be completely independent from the backup worker. Its interval SHALL be configured via `tsa_interval_hours` (default: 168 hours = 7 days).

#### Scenario: Worker runs at configured interval
- **WHEN** `tsa_enabled` is `true` and `tsa_url` is configured
- **THEN** the timestamp worker runs independently, creating a timestamp every `tsa_interval_hours` hours

#### Scenario: Worker skips when disabled
- **WHEN** `tsa_enabled` is `false` or not set
- **THEN** the timestamp worker does not run

#### Scenario: Worker handles TSA failure
- **WHEN** the TSA request fails during a worker cycle
- **THEN** the worker logs the error, stores a "tsa_failed" record, and continues sleeping until the next cycle

### Requirement: No timestamp when audit log is empty
The system SHALL skip the timestamp step when the audit_log table contains no entries.

#### Scenario: Empty audit log
- **WHEN** the timestamp step runs but the audit_log has no entries
- **THEN** the system logs an INFO message and skips the TSA request

### Requirement: No duplicate timestamps
The system SHALL skip the timestamp step when the current latest audit_log hash is identical to the hash in the most recent audit_timestamp record. This prevents creating redundant timestamps when no changes have occurred.

#### Scenario: No changes since last timestamp
- **WHEN** the latest audit_log entry_hash equals the audit_hash of the most recent audit_timestamp
- **THEN** the system skips the TSA request and logs an INFO message

#### Scenario: Changes since last timestamp
- **WHEN** the latest audit_log entry_hash differs from the most recent audit_timestamp's audit_hash
- **THEN** the system proceeds with the TSA request

### Requirement: Manual timestamp trigger
The system SHALL provide a REST endpoint `POST /api/audit/timestamps` that allows admins to manually trigger a qualified timestamp. The endpoint SHALL use the same create_timestamp logic as the worker, including duplicate detection (skip when hash unchanged). The endpoint SHALL require `admin` privilege.

#### Scenario: Manual trigger with changes
- **WHEN** an admin sends POST /api/audit/timestamps and the audit log has changed since the last timestamp
- **THEN** the system creates a new timestamp and returns HTTP 201 with the timestamp record

#### Scenario: Manual trigger without changes
- **WHEN** an admin sends POST /api/audit/timestamps but the audit log hash is unchanged since the last timestamp
- **THEN** the system returns HTTP 200 with a message indicating no changes to timestamp

#### Scenario: Manual trigger with empty audit log
- **WHEN** an admin sends POST /api/audit/timestamps but the audit log is empty
- **THEN** the system returns HTTP 200 with a message indicating no audit entries exist

#### Scenario: Manual trigger with TSA failure
- **WHEN** an admin sends POST /api/audit/timestamps but the TSA request fails
- **THEN** the system returns HTTP 502 with an error message indicating the TSA is unreachable

### Requirement: TSA configuration UI
The frontend SHALL provide a configuration page for admins to manage the TSA settings (tsa_enabled, tsa_url, tsa_user, tsa_pass, tsa_interval_hours). The page SHALL use the existing config store API.

#### Scenario: Configure TSA settings
- **WHEN** an admin navigates to the TSA configuration page
- **THEN** the page displays the current TSA settings and allows editing

#### Scenario: Save TSA configuration
- **WHEN** an admin saves the TSA configuration
- **THEN** the config store is updated and the timestamp worker picks up the new settings on its next cycle

#### Scenario: Password field masked
- **WHEN** the TSA configuration page is displayed
- **THEN** the `tsa_pass` field is masked (password input type)
