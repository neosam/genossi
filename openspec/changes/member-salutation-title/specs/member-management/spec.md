## MODIFIED Requirements

### Requirement: Member data model
The system SHALL store members with the following fields:
- `id` (UUID, system-generated, primary key)
- `member_number` (i64, unique, user-visible identifier)
- `first_name` (String, required)
- `last_name` (String, required)
- `salutation` (Optional Enum: Herr, Frau, Firma)
- `title` (Optional String)
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
- **WHEN** a member is created with all fields provided including salutation and title
- **THEN** the system stores the member with a generated UUID, created timestamp, version UUID, `migrated` set to `false`, and the provided salutation and title values

#### Scenario: Member number uniqueness
- **WHEN** a member is created with a member_number that already exists
- **THEN** the system SHALL reject the creation with a validation error

#### Scenario: Member created without salutation and title
- **WHEN** a member is created without salutation and title
- **THEN** the system stores the member with salutation as NULL and title as NULL

#### Scenario: Salutation enum values
- **WHEN** a member is created or updated with a salutation value
- **THEN** the system SHALL only accept the values `Herr`, `Frau`, or `Firma`

#### Scenario: Invalid salutation value rejected
- **WHEN** a member is created or updated with a salutation value not in the allowed enum
- **THEN** the system SHALL reject the request with a validation error

### Requirement: Member list page
The frontend SHALL provide a page listing all members with their member number, name, key details, migration status, and active membership status on a user-selected reference date.

#### Scenario: View member list with active status
- **WHEN** an authenticated user navigates to the members page
- **THEN** the system displays a table with member_number, last_name, first_name, city, current_shares, join_date, migration status, and an active/inactive badge based on the reference date

#### Scenario: Salutation and title columns available
- **WHEN** a user opens the column selection
- **THEN** `salutation` and `title` SHALL be available as selectable columns

#### Scenario: Salutation column displays enum value
- **WHEN** the salutation column is enabled and a member has a salutation set
- **THEN** the column displays the salutation value (Herr, Frau, or Firma)

#### Scenario: Salutation inline editing
- **WHEN** the salutation column is enabled and the user edits a member's salutation
- **THEN** the system SHALL present a dropdown with options: empty, Herr, Frau, Firma

#### Scenario: Title inline editing
- **WHEN** the title column is enabled and the user edits a member's title
- **THEN** the system SHALL present a text input field

#### Scenario: Default columns unchanged
- **WHEN** the members page loads
- **THEN** salutation and title SHALL NOT be in the default column set

#### Scenario: Default reference date is today
- **WHEN** the members page loads
- **THEN** the date picker defaults to today's date and active status is computed against today

#### Scenario: Change reference date
- **WHEN** the user selects a different date in the date picker
- **THEN** the active/inactive badges update immediately for all members based on the new date

#### Scenario: Active member badge
- **WHEN** a member's join_date is on or before the reference date AND the member has no exit_date or exit_date is after the reference date
- **THEN** the member shows a green "Active" badge

#### Scenario: Inactive member badge
- **WHEN** a member's join_date is after the reference date OR the member's exit_date is on or before the reference date
- **THEN** the member shows a red "Inactive" badge

#### Scenario: Filter only active members
- **WHEN** the user enables the "Only active members" toggle
- **THEN** only members who are active on the selected reference date are shown in the list

#### Scenario: Filter toggle off shows all
- **WHEN** the "Only active members" toggle is disabled
- **THEN** all non-deleted members are shown regardless of active status

#### Scenario: Navigate to member detail
- **WHEN** a user clicks on a member row in the list
- **THEN** the system navigates to the member detail page

### Requirement: Member detail page
The frontend SHALL provide a page to view and edit a single member's data.

#### Scenario: View existing member with salutation and title
- **WHEN** a user navigates to the detail page of an existing member
- **THEN** all member fields including salutation (as dropdown) and title (as text input) are displayed in a form

#### Scenario: Edit salutation on detail page
- **WHEN** a user changes the salutation dropdown and clicks save
- **THEN** the system sends a PUT request with the updated salutation and navigates back to the member list on success

#### Scenario: Edit title on detail page
- **WHEN** a user changes the title text input and clicks save
- **THEN** the system sends a PUT request with the updated title and navigates back to the member list on success

#### Scenario: Create new member with salutation and title
- **WHEN** a user creates a new member and provides salutation and title
- **THEN** the system stores both fields with the new member

#### Scenario: join_date editable on new member
- **WHEN** a user is creating a new member
- **THEN** the `join_date` field SHALL be editable and determines the date of the automatically created Eintritt action

#### Scenario: join_date read-only on existing member
- **WHEN** a user is viewing an existing member
- **THEN** the `join_date` field SHALL be displayed as read-only, since it is derived from the Eintritt action

#### Scenario: exit_date read-only on existing member
- **WHEN** a user is viewing an existing member that has an Austritt or Todesfall action
- **THEN** the `exit_date` field SHALL be displayed as a read-only date value

#### Scenario: exit_date not shown when no exit action
- **WHEN** a user is viewing an existing member that has no Austritt or Todesfall action
- **THEN** the `exit_date` field SHALL be displayed as read-only with no value (empty)
