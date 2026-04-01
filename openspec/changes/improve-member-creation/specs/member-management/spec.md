## MODIFIED Requirements

### Requirement: Create member
The system SHALL allow authenticated users with `manage_members` privilege to create new members via `POST /api/members`. When `member_number` is 0, the system SHALL auto-assign the next available number. The system SHALL automatically create `Eintritt` and `Aufstockung` actions and set `current_shares` from `shares_at_joining`, `current_balance` to 0, and `action_count` to 0.

#### Scenario: Successful member creation with auto member number
- **WHEN** an authenticated user with `manage_members` privilege sends a POST request with `member_number` set to 0, `join_date`, and `shares_at_joining`
- **THEN** the system creates the member with the next available member number, creates Eintritt and Aufstockung actions, sets computed fields, and returns the created member with HTTP 200

#### Scenario: Successful member creation with explicit member number
- **WHEN** an authenticated user with `manage_members` privilege sends a POST request with a positive `member_number`
- **THEN** the system creates the member with the provided member number, creates Eintritt and Aufstockung actions, sets computed fields, and returns the created member with HTTP 200

#### Scenario: Missing required fields
- **WHEN** a POST request is sent without `first_name` or `last_name`
- **THEN** the system returns HTTP 400 with field-level validation errors

#### Scenario: Duplicate member number
- **WHEN** a POST request is sent with a `member_number` that already exists
- **THEN** the system returns HTTP 400 with a validation error for `member_number`

#### Scenario: Insufficient privileges
- **WHEN** a user without `manage_members` privilege attempts to create a member
- **THEN** the system returns HTTP 401 Unauthorized
