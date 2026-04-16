## MODIFIED Requirements

### Requirement: Public join endpoint

The system SHALL provide a public endpoint `POST /api/public/join` that accepts membership applications without user authentication. The endpoint SHALL require a valid API key in the `X-Api-Key` header. The API-key comparison SHALL be performed in constant time to prevent timing side-channel attacks.

The endpoint SHALL validate all input fields according to the rules in the "Public join input validation" requirement below, before any database operation.

#### Scenario: Successful application submission

- **WHEN** a POST request is sent to `/api/public/join` with a valid API key and all required fields matching the validation rules
- **THEN** the system creates an application with status "Offen" and returns HTTP 201

#### Scenario: Missing API key

- **WHEN** a POST request is sent to `/api/public/join` without an `X-Api-Key` header
- **THEN** the system returns HTTP 401

#### Scenario: Invalid API key (constant-time compare)

- **WHEN** a POST request is sent to `/api/public/join` with an incorrect API key
- **THEN** the system returns HTTP 401, and the time to process the request SHALL NOT depend on which byte of the key differs

#### Scenario: Missing required field

- **WHEN** a POST request is sent with a valid API key but without the `email` field
- **THEN** the system returns HTTP 422 with a field-specific error listing `email` as missing

#### Scenario: Shares below minimum

- **WHEN** a POST request is sent with shares set to 0
- **THEN** the system returns HTTP 422 with a field-specific error indicating shares must be at least 1

## ADDED Requirements

### Requirement: Public join input validation

The system SHALL validate input to `POST /api/public/join` according to these rules, and SHALL return HTTP 422 with a structured error response listing all violated rules when any rule fails. The error response SHALL have the shape `{"errors": [{"field": "<name>", "message": "<reason>"}, ...]}`.

| Field | Rule |
|-------|------|
| `first_name` | required, 1..=128 characters |
| `last_name` | required, 1..=128 characters |
| `email` | required, must contain `@`, length 3..=320 |
| `street` | required, 1..=128 characters |
| `house_number` | required, 1..=32 characters |
| `postal_code` | required, 1..=16 characters |
| `city` | required, 1..=128 characters |
| `title` | optional, if present max 64 characters |
| `shares` | required, >= 1 |

All validation failures in a single request SHALL be collected and returned together (not short-circuit on first error).

#### Scenario: Email without @-sign

- **WHEN** a POST request has `email: "foo"` and all other fields valid
- **THEN** the system returns HTTP 422 with `errors` including `{"field": "email", "message": "invalid email format"}`

#### Scenario: First name too long

- **WHEN** a POST request has `first_name` with 200 characters
- **THEN** the system returns HTTP 422 with `errors` including `{"field": "first_name", "message": "too long (max 128)"}`

#### Scenario: Multiple validation failures in one request

- **WHEN** a POST request has `email: ""` AND `shares: 0`
- **THEN** the system returns HTTP 422 with `errors` containing entries for BOTH `email` and `shares`

#### Scenario: Valid submission with optional fields omitted

- **WHEN** a POST request has all required fields within the limits and omits `title`
- **THEN** the system accepts the submission and returns HTTP 201
