## MODIFIED Requirements

### Requirement: Member list page
The frontend SHALL provide a page listing all members with their member number, name, and key details. The page SHALL include filter controls for active status and migration status.

#### Scenario: View member list
- **WHEN** an authenticated user navigates to the members page
- **THEN** the system displays a table of all active members with member_number, last_name, first_name, city, current_shares, join_date, exit_date, migration status, and active status

#### Scenario: Navigate to member detail
- **WHEN** a user clicks on a member row in the list
- **THEN** the system navigates to the member detail page

#### Scenario: Filter by pending migration
- **WHEN** the user enables the "Only pending migrations" filter checkbox
- **THEN** the member list SHALL show only members where `migrated` is `false`

#### Scenario: Combine migration filter with active filter
- **WHEN** the user enables both "Only pending migrations" and "Only active members" filters
- **THEN** the member list SHALL show only members that are both active and have pending migrations

#### Scenario: Disable migration filter
- **WHEN** the user disables the "Only pending migrations" filter checkbox
- **THEN** the member list SHALL show all members regardless of migration status (subject to other active filters)
