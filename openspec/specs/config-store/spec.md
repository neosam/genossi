## ADDED Requirements

### Requirement: Config entry data model
The system SHALL store configuration entries with the following fields:
- `key` (TEXT, primary key)
- `value` (TEXT)
- `value_type` (TEXT): one of `string`, `int`, `bool`, `secret`

#### Scenario: Config entry stored
- **WHEN** a config entry is created with key `smtp_host`, value `mail.example.com`, and value_type `string`
- **THEN** the system stores the entry with the given key as primary key

### Requirement: Config upsert
The system SHALL support creating and updating config entries via a single upsert operation. If a key already exists, its value and value_type SHALL be replaced.

#### Scenario: Create new config entry
- **WHEN** a config entry with key `smtp_host` does not exist and a set operation is performed
- **THEN** the system creates the entry

#### Scenario: Update existing config entry
- **WHEN** a config entry with key `smtp_host` already exists and a set operation is performed with a new value
- **THEN** the system replaces the existing value

### Requirement: Config value type validation
The system SHALL validate values against their declared value_type when setting a config entry.

#### Scenario: Valid int value
- **WHEN** a config entry is set with value_type `int` and value `587`
- **THEN** the system accepts the entry

#### Scenario: Invalid int value
- **WHEN** a config entry is set with value_type `int` and value `not_a_number`
- **THEN** the system rejects the entry with a validation error

#### Scenario: Valid bool value
- **WHEN** a config entry is set with value_type `bool` and value `true`
- **THEN** the system accepts the entry

#### Scenario: Invalid bool value
- **WHEN** a config entry is set with value_type `bool` and value `yes`
- **THEN** the system rejects the entry with a validation error

### Requirement: Secret value masking on read
The system SHALL mask the value of config entries with value_type `secret` when returning them via the REST API. The masked value SHALL be `***`.

#### Scenario: Secret value masked in list
- **WHEN** the config list endpoint is called and an entry with value_type `secret` exists
- **THEN** the entry's value is returned as `***`

#### Scenario: Non-secret value returned as-is
- **WHEN** the config list endpoint is called and an entry with value_type `string` exists
- **THEN** the entry's actual value is returned

### Requirement: Config hard delete
The system SHALL support permanently deleting config entries by key.

#### Scenario: Delete existing entry
- **WHEN** a delete operation is performed for an existing key
- **THEN** the entry is permanently removed from the database

#### Scenario: Delete non-existing entry
- **WHEN** a delete operation is performed for a key that does not exist
- **THEN** the system returns a not-found error

### Requirement: Config REST endpoints
The system SHALL expose the following REST endpoints:
- `GET /api/config` — list all config entries (secrets masked)
- `PUT /api/config/{key}` — set a config entry (upsert)
- `DELETE /api/config/{key}` — delete a config entry

#### Scenario: List all config entries
- **WHEN** `GET /api/config` is called
- **THEN** the system returns all config entries with secret values masked

#### Scenario: Set config entry
- **WHEN** `PUT /api/config/smtp_host` is called with body `{"value": "mail.example.com", "value_type": "string"}`
- **THEN** the system upserts the entry and returns the stored entry

#### Scenario: Delete config entry
- **WHEN** `DELETE /api/config/smtp_host` is called
- **THEN** the system deletes the entry and returns 204 No Content
