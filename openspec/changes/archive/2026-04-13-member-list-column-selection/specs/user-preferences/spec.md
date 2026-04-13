## ADDED Requirements

### Requirement: User preference data model
The system SHALL store user preferences with the following fields:
- `id` (UUID, system-generated, primary key)
- `user_id` (UUID, references authenticated user)
- `key` (TEXT, preference identifier)
- `value` (TEXT, JSON-encoded preference value)
- `created` (DateTime, system-generated)
- `deleted` (Optional DateTime, for soft delete)
- `version` (UUID, for optimistic locking)

The combination of `user_id` and `key` SHALL be unique.

#### Scenario: Preference stored with all fields
- **WHEN** a user preference is created with user_id, key `member_list_columns`, and value `["member_number","last_name"]`
- **THEN** the system stores the preference with a generated UUID, created timestamp, and version UUID

#### Scenario: Duplicate key for same user rejected
- **WHEN** a preference with key `member_list_columns` already exists for user `abc-123` and another create is attempted with the same user_id and key
- **THEN** the system SHALL reject the creation with a conflict error

### Requirement: Get user preference by key
The system SHALL allow authenticated users to retrieve their own preference by key via `GET /api/user-preferences/{key}`.

#### Scenario: Preference found
- **WHEN** an authenticated user sends a GET request for key `member_list_columns` and a preference exists for that user and key
- **THEN** the system returns the preference with HTTP 200

#### Scenario: Preference not found
- **WHEN** an authenticated user sends a GET request for key `member_list_columns` and no preference exists for that user and key
- **THEN** the system returns HTTP 404

#### Scenario: User can only read own preferences
- **WHEN** user `abc-123` sends a GET request for key `member_list_columns`
- **THEN** the system SHALL only return preferences where `user_id` matches the authenticated user, never another user's preferences

### Requirement: Upsert user preference
The system SHALL allow authenticated users to create or update their own preference via `PUT /api/user-preferences/{key}`.

#### Scenario: Create new preference
- **WHEN** an authenticated user sends a PUT request for key `member_list_columns` with value `["member_number","last_name","first_name"]` and no preference exists for that key
- **THEN** the system creates the preference, assigns a UUID and version, and returns it with HTTP 200

#### Scenario: Update existing preference
- **WHEN** an authenticated user sends a PUT request for key `member_list_columns` with a new value and the preference already exists
- **THEN** the system updates the value, generates a new version UUID, and returns the updated preference with HTTP 200

#### Scenario: User can only upsert own preferences
- **WHEN** an authenticated user sends a PUT request
- **THEN** the system SHALL set `user_id` to the authenticated user's ID, regardless of any user_id in the request body
