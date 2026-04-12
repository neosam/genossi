## MODIFIED Requirements

### Requirement: Create member
The system SHALL allow authenticated users with `manage_members` privilege to create new members via `POST /api/members`. When the member status is `FehlerhaftErfasst`, the system SHALL NOT create automatic Eintritt and Aufstockung actions, and SHALL set `current_shares` to 0 regardless of `shares_at_joining`.

#### Scenario: Successful member creation
- **WHEN** an authenticated user with `manage_members` privilege sends a POST request with valid member data and status `Normal`
- **THEN** the system creates the member, assigns a UUID and version, sets the created timestamp, creates an Eintritt action and an Aufstockung action, and returns the created member with HTTP 200

#### Scenario: Member creation with FehlerhaftErfasst status
- **WHEN** an authenticated user creates a member with status `FehlerhaftErfasst`
- **THEN** the system creates the member with `current_shares = 0`, does NOT create Eintritt or Aufstockung actions, and returns the created member with HTTP 200

#### Scenario: Missing required fields
- **WHEN** a POST request is sent without `first_name`, `last_name`, `join_date`, or `member_number`
- **THEN** the system returns HTTP 400 with field-level validation errors

#### Scenario: Insufficient privileges
- **WHEN** a user without `manage_members` privilege attempts to create a member
- **THEN** the system returns HTTP 401 Unauthorized
