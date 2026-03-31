## MODIFIED Requirements

### Requirement: Member data model
The system SHALL store members with the following fields:
- `id` (UUID, system-generated, primary key)
- `member_number` (i64, unique, user-visible identifier)
- `first_name` (String, required)
- `last_name` (String, required)
- `email` (Optional String)
- `company` (Optional String)
- `comment` (Optional String)
- `street` (Optional String)
- `house_number` (Optional String)
- `postal_code` (Optional String)
- `city` (Optional String)
- `join_date` (Date, required)
- `shares_at_joining` (i32, required)
- `current_shares` (i32, required)
- `current_balance` (i64 in cents, required)
- `exit_date` (Optional Date)
- `bank_account` (Optional String)
- `migrated` (bool, default false)
- `created` (DateTime, system-generated)
- `deleted` (Optional DateTime, for soft delete)
- `version` (UUID, for optimistic locking)

#### Scenario: Member stored with all fields
- **WHEN** a member is created with all fields provided
- **THEN** the system stores the member with a generated UUID, created timestamp, version UUID, and `migrated` set to `false`

#### Scenario: Member number uniqueness
- **WHEN** a member is created with a member_number that already exists
- **THEN** the system SHALL reject the creation with a validation error

### Requirement: Member list page
The frontend SHALL provide a page listing all members with their member number, name, key details, and migration status.

#### Scenario: View member list
- **WHEN** an authenticated user navigates to the members page
- **THEN** the system displays a table of all active members with member_number, last_name, first_name, city, current_shares, join_date, and a migration status indicator

#### Scenario: Navigate to member detail
- **WHEN** a user clicks on a member row in the list
- **THEN** the system navigates to the member detail page
