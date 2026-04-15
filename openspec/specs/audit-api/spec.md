## ADDED Requirements

### Requirement: Get audit history for entity
The system SHALL provide a REST endpoint `GET /api/audit/{entity_type}/{entity_id}` that returns the audit log history for a specific entity. The endpoint SHALL require `admin` privilege.

The response SHALL be a JSON array of audit log entries ordered by timestamp ascending, each containing: id, timestamp, user_id, process, transaction_id, entity_type, entity_id, action, field_name, old_value, new_value.

#### Scenario: Retrieve member audit history
- **WHEN** an authenticated admin sends GET /api/audit/member/{member_id}
- **THEN** the system returns HTTP 200 with all audit log entries for that member, ordered by timestamp

#### Scenario: No audit history
- **WHEN** an authenticated admin sends GET /api/audit/member/{member_id} for a member with no changes
- **THEN** the system returns HTTP 200 with an empty array

#### Scenario: Unauthorized access
- **WHEN** a non-admin user sends GET /api/audit/member/{member_id}
- **THEN** the system returns HTTP 403

### Requirement: Get all audit log entries with filtering
The system SHALL provide a REST endpoint `GET /api/audit` that returns audit log entries with optional query parameters for filtering:
- `entity_type` (optional): filter by entity type
- `entity_id` (optional): filter by entity ID
- `user_id` (optional): filter by user who made the change
- `from` (optional): filter entries from this timestamp
- `to` (optional): filter entries until this timestamp
- `action` (optional): filter by action type (create, update, delete)

The endpoint SHALL require `admin` privilege. Results SHALL be ordered by timestamp descending.

#### Scenario: Filter by entity type
- **WHEN** an admin sends GET /api/audit?entity_type=member
- **THEN** the system returns only audit entries for entity_type "member"

#### Scenario: Filter by time range
- **WHEN** an admin sends GET /api/audit?from=2026-01-01T00:00:00&to=2026-04-15T23:59:59
- **THEN** the system returns only audit entries within the specified time range

#### Scenario: Filter by user
- **WHEN** an admin sends GET /api/audit?user_id=admin
- **THEN** the system returns only audit entries made by user "admin"

#### Scenario: Combined filters
- **WHEN** an admin sends GET /api/audit?entity_type=member&action=update
- **THEN** the system returns only "update" audit entries for "member" entities

### Requirement: Verify hash chain integrity
The system SHALL provide a REST endpoint `GET /api/audit/verify` that verifies the integrity of the audit log hash chain. The endpoint SHALL require `admin` privilege.

The response SHALL contain:
- `valid` (boolean): whether the chain is intact
- `total_entries` (integer): total number of audit entries checked
- `broken_links` (array): list of entry IDs where the chain breaks (empty if valid)

#### Scenario: Intact chain
- **WHEN** an admin sends GET /api/audit/verify and no entries have been tampered with
- **THEN** the system returns HTTP 200 with valid=true and empty broken_links

#### Scenario: Broken chain
- **WHEN** an admin sends GET /api/audit/verify and entries have been modified in the database
- **THEN** the system returns HTTP 200 with valid=false and broken_links containing the IDs of tampered entries

### Requirement: Audit endpoints in OpenAPI
The system SHALL include all audit log endpoints in the Swagger UI documentation at `/swagger-ui/`.

#### Scenario: Audit endpoints visible in Swagger
- **WHEN** a user navigates to /swagger-ui/
- **THEN** the audit log endpoints are listed and documented with request/response schemas
