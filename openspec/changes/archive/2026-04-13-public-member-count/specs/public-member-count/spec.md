## ADDED Requirements

### Requirement: Public member count endpoint
The system SHALL provide a public (unauthenticated) HTTP endpoint at `GET /api/public/member-count` that returns the count of active members as JSON `{ "count": <number> }`.

#### Scenario: Successful count retrieval when enabled
- **WHEN** config key `public_stats_enabled` is set to `true` and a GET request is made to `/api/public/member-count`
- **THEN** the system SHALL respond with HTTP 200 and a JSON body `{ "count": <n> }` where `<n>` is the number of active members

#### Scenario: Endpoint disabled by default
- **WHEN** config key `public_stats_enabled` is not set or set to `false` and a GET request is made to `/api/public/member-count`
- **THEN** the system SHALL respond with HTTP 403 Forbidden

#### Scenario: No authentication required
- **WHEN** a GET request is made to `/api/public/member-count` without any authentication headers or session
- **THEN** the system SHALL process the request without requiring authentication

### Requirement: Active member counting
The system SHALL count only active members. A member is active when the `deleted` field is NULL and the `exit_date` is either NULL or in the future.

#### Scenario: Member with no exit date is counted
- **WHEN** a member has `deleted` = NULL and `exit_date` = NULL
- **THEN** the member SHALL be included in the active count

#### Scenario: Member with future exit date is counted
- **WHEN** a member has `deleted` = NULL and `exit_date` is a date in the future
- **THEN** the member SHALL be included in the active count

#### Scenario: Member with past exit date is not counted
- **WHEN** a member has `deleted` = NULL and `exit_date` is a date in the past
- **THEN** the member SHALL NOT be included in the active count

#### Scenario: Soft-deleted member is not counted
- **WHEN** a member has `deleted` set to a timestamp (non-NULL)
- **THEN** the member SHALL NOT be included in the active count regardless of `exit_date`

### Requirement: Response caching
The system SHALL cache the member count and the config value with a TTL of 5 minutes to minimize database load.

#### Scenario: Cached response within TTL
- **WHEN** a request is made within 5 minutes of the last database query
- **THEN** the system SHALL return the cached count without querying the database

#### Scenario: Cache expired
- **WHEN** a request is made after 5 minutes since the last database query
- **THEN** the system SHALL query the database for fresh data and update the cache

### Requirement: Config-gated access
The system SHALL check the config key `public_stats_enabled` (type: bool) before serving the member count. The config value SHALL also be cached with 5 minutes TTL.

#### Scenario: Config changed from false to true
- **WHEN** an admin sets `public_stats_enabled` to `true` via `PUT /api/config/public_stats_enabled`
- **THEN** the endpoint SHALL start returning member counts within 5 minutes (after config cache expires)

#### Scenario: Config changed from true to false
- **WHEN** an admin sets `public_stats_enabled` to `false`
- **THEN** the endpoint SHALL start returning 403 within 5 minutes (after config cache expires)
