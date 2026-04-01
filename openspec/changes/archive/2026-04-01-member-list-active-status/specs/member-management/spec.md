## MODIFIED Requirements

### Requirement: Member list page
The frontend SHALL provide a page listing all members with their member number, name, key details, migration status, and active membership status on a user-selected reference date.

#### Scenario: View member list with active status
- **WHEN** an authenticated user navigates to the members page
- **THEN** the system displays a table with member_number, last_name, first_name, city, current_shares, join_date, migration status, and an active/inactive badge based on the reference date

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
