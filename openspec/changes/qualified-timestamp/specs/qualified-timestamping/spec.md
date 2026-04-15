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
- **THEN** the system logs an ERROR, stores a record with status "tsa_failed", and continues the backup cycle

#### Scenario: TSA returns error response
- **WHEN** the TSA server returns a TimeStampResp with a non-success status
- **THEN** the system logs an ERROR with the TSA status, stores a record with status "tsa_failed", and continues

### Requirement: Timestamp token WebDAV upload
The system SHALL upload the .tsr token file to the configured WebDAV server in an `audit-timestamps/` subdirectory. The file SHALL be named `audit-checkpoint-{ISO8601-timestamp}.tsr`.

#### Scenario: Successful upload
- **WHEN** a TSR token is obtained and WebDAV is configured
- **THEN** the system uploads the token to `audit-timestamps/audit-checkpoint-2026-04-15T14:30:00.tsr` and updates the record's webdav_path

#### Scenario: WebDAV upload fails
- **WHEN** a TSR token is obtained but WebDAV upload fails
- **THEN** the system logs a WARN, sets the record status to "upload_failed", and continues (the token is still stored locally in the database)

#### Scenario: WebDAV not configured
- **WHEN** a TSR token is obtained but WebDAV backup is not configured
- **THEN** the token is stored locally only, no upload is attempted

### Requirement: Timestamp configuration
The system SHALL use the following config store keys:
- `tsa_url` (string): The RFC 3161 TSA endpoint URL
- `tsa_user` (string, optional): HTTP Basic Auth username
- `tsa_pass` (secret, optional): HTTP Basic Auth password
- `tsa_enabled` (bool, default false): Whether qualified timestamping is enabled

#### Scenario: Timestamping enabled
- **WHEN** `tsa_enabled` is set to `true` and `tsa_url` is configured
- **THEN** the backup worker performs the timestamp step

#### Scenario: Timestamping disabled
- **WHEN** `tsa_enabled` is set to `false` or not set
- **THEN** the backup worker skips the timestamp step entirely

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
