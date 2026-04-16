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
The system SHALL provide a REST endpoint `GET /api/audit` that returns a paginated, filtered slice of audit log entries.

The endpoint SHALL accept the following optional query parameters for filtering:
- `entity_type`: filter by entity type
- `entity_id`: filter by entity ID
- `user_id`: filter by user who made the change
- `from`: filter entries from this timestamp (ISO8601, inclusive)
- `to`: filter entries until this timestamp (ISO8601, inclusive)
- `action`: filter by action type (create, update, delete, snapshot)

The endpoint SHALL accept the following optional pagination parameters:
- `page`: 0-based page index (defaults to 0; values less than 0 are clamped to 0)
- `size`: page size, restricted to one of {25, 50, 100, 200, 500} (defaults to 50; values outside the allowed set fall back to the default)

All filtering SHALL be performed at the database layer, not in memory. Results SHALL be ordered by `timestamp` descending with `id` descending as a stable tiebreaker.

The endpoint SHALL require `admin` privilege.

The response body SHALL be a JSON envelope with the following fields:
- `entries`: array of audit log entries on the requested page (each entry contains: id, timestamp, user_id, process, transaction_id, entity_type, entity_id, action, field_name, old_value, new_value)
- `total`: integer count of all entries matching the filter (independent of pagination)
- `page`: integer echoing the effective page index used
- `size`: integer echoing the effective page size used

#### Scenario: Filter by entity type
- **WHEN** an admin sends `GET /api/audit?entity_type=member`
- **THEN** the system returns HTTP 200 with `entries` containing only audit entries whose `entity_type` is "member" and `total` reflecting the count of all matching entries

#### Scenario: Filter by time range
- **WHEN** an admin sends `GET /api/audit?from=2026-01-01T00:00:00&to=2026-04-15T23:59:59`
- **THEN** the system returns only audit entries whose `timestamp` is within the inclusive range and `total` reflects the matching count

#### Scenario: Filter by user
- **WHEN** an admin sends `GET /api/audit?user_id=admin`
- **THEN** the system returns only audit entries whose `user_id` is "admin" and `total` reflects the matching count

#### Scenario: Combined filters
- **WHEN** an admin sends `GET /api/audit?entity_type=member&action=update`
- **THEN** the system returns only "update" audit entries for "member" entities and `total` reflects the count of that combined filter

#### Scenario: Default pagination
- **WHEN** an admin sends `GET /api/audit` with no pagination parameters
- **THEN** the system returns up to 50 entries from page 0 with `page=0` and `size=50` echoed in the response

#### Scenario: Explicit pagination
- **WHEN** an admin sends `GET /api/audit?page=2&size=100`
- **THEN** the system returns up to 100 entries starting from offset 200 with `page=2` and `size=100` echoed in the response

#### Scenario: Page size clamping
- **WHEN** an admin sends `GET /api/audit?size=10000`
- **THEN** the system falls back to the default size of 50 and echoes `size=50` in the response

#### Scenario: Page beyond total
- **WHEN** an admin sends `GET /api/audit?page=999` and only 30 entries match
- **THEN** the system returns HTTP 200 with `entries` empty, `total=30`, and the requested `page` echoed

#### Scenario: Stable ordering
- **WHEN** an admin paginates through results and entries share the same `timestamp`
- **THEN** entries are ordered by `id` descending so paging produces no duplicates and no skips across page boundaries

#### Scenario: Total reflects filter
- **WHEN** an admin sends `GET /api/audit?entity_type=application&page=0&size=50` and the unfiltered table contains thousands of rows but only 12 application entries exist
- **THEN** the response contains those 12 entries and `total=12`

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
