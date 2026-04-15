## ADDED Requirements

### Requirement: Verify timestamp token signature
The system SHALL verify that a stored TSR token is validly signed by the configured TSA's certificate. The verification SHALL check that the token's embedded hash matches the stored audit_hash.

#### Scenario: Valid token
- **WHEN** verification is requested for a timestamp with a valid TSR token
- **THEN** the system confirms the token signature is valid and the embedded hash matches the stored audit_hash

#### Scenario: Tampered token
- **WHEN** verification is requested for a timestamp whose TSR token has been modified in the database
- **THEN** the system reports the token signature as invalid

#### Scenario: Hash mismatch
- **WHEN** verification is requested and the hash embedded in the TSR token does not match the stored audit_hash
- **THEN** the system reports a hash mismatch

### Requirement: Verify audit log matches timestamp
The system SHALL verify that the audit_log hash chain, when replayed up to the entry count recorded in the timestamp, produces the same hash that was timestamped. This proves the audit log has not been manipulated since the timestamp was created.

#### Scenario: Audit log consistent with timestamp
- **WHEN** the audit_log entries up to audit_entry_count are hashed and the result matches the timestamp's audit_hash
- **THEN** the verification reports the audit log is consistent with the external timestamp

#### Scenario: Audit log manipulated after timestamp
- **WHEN** audit_log entries have been modified and the replayed hash does not match the timestamp's audit_hash
- **THEN** the verification reports that the audit log has been tampered with since the timestamp was created

### Requirement: Timestamp list REST endpoint
The system SHALL provide a REST endpoint `GET /api/audit/timestamps` that returns a list of all audit timestamp records (id, timestamp, audit_hash, audit_entry_count, status, webdav_path). The endpoint SHALL require `admin` privilege.

#### Scenario: List timestamps
- **WHEN** an admin sends GET /api/audit/timestamps
- **THEN** the system returns HTTP 200 with all timestamp records ordered by timestamp descending

#### Scenario: No timestamps
- **WHEN** an admin sends GET /api/audit/timestamps and no timestamps exist
- **THEN** the system returns HTTP 200 with an empty array

### Requirement: Single timestamp verification REST endpoint
The system SHALL provide a REST endpoint `GET /api/audit/timestamps/{id}/verify` that verifies a specific timestamp token and checks consistency with the current audit log. The endpoint SHALL require `admin` privilege.

The response SHALL contain:
- `token_valid` (boolean): whether the TSR token signature is valid
- `hash_matches` (boolean): whether the embedded hash matches the stored audit_hash
- `audit_log_consistent` (boolean): whether the audit log, replayed to audit_entry_count, produces the same hash
- `timestamp` (DateTime): when the timestamp was created
- `audit_hash` (String): the hash that was timestamped

#### Scenario: Full verification success
- **WHEN** an admin sends GET /api/audit/timestamps/{id}/verify for a valid timestamp
- **THEN** the system returns token_valid=true, hash_matches=true, audit_log_consistent=true

#### Scenario: Audit log tampered
- **WHEN** an admin verifies a timestamp but the audit log has been modified
- **THEN** the system returns token_valid=true, hash_matches=true, audit_log_consistent=false

### Requirement: Timestamp status in frontend
The audit log frontend page SHALL display a section showing the latest timestamp status: when it was created, the hash, and the verification result. An admin SHALL be able to trigger verification and manual timestamp creation from the UI.

#### Scenario: Latest timestamp displayed
- **WHEN** an admin views the audit log page and timestamps exist
- **THEN** the page shows the most recent timestamp's date, status, and hash

#### Scenario: No timestamps yet
- **WHEN** an admin views the audit log page and no timestamps exist
- **THEN** the page shows a message that no external timestamps have been created yet

#### Scenario: Trigger verification from UI
- **WHEN** an admin clicks "Verify" on a timestamp entry
- **THEN** the UI calls the verification endpoint and displays the result (token valid, hash matches, audit log consistent)

#### Scenario: Manual timestamp from UI
- **WHEN** an admin clicks "Zeitstempel jetzt erstellen" button
- **THEN** the UI calls POST /api/audit/timestamps and displays the result (success, no changes, or error)
