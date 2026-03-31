## MODIFIED Requirements

### Requirement: Member detail page
The frontend SHALL provide a page to view and edit a single member's data, including an actions section and migration status display for existing members.

#### Scenario: View existing member
- **WHEN** a user navigates to the detail page of an existing member
- **THEN** all member fields are displayed in a form, followed by the migration status badge and the actions section

#### Scenario: Edit and save member
- **WHEN** a user modifies fields and clicks save
- **THEN** the system sends a PUT request and navigates back to the member list on success

#### Scenario: Create new member
- **WHEN** a user navigates to the member detail page without an ID (new member)
- **THEN** an empty form is displayed without actions section or migration status, and saving sends a POST request
