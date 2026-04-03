## MODIFIED Requirements

### Requirement: Member detail page
The frontend SHALL provide a page to view and edit a single member's data.

#### Scenario: View existing member
- **WHEN** a user navigates to the detail page of an existing member
- **THEN** all member fields are displayed in a form, with `join_date` and `exit_date` shown as read-only fields

#### Scenario: Edit and save member
- **WHEN** a user modifies fields and clicks save
- **THEN** the system sends a PUT request and navigates back to the member list on success

#### Scenario: Create new member
- **WHEN** a user navigates to the member detail page without an ID (new member)
- **THEN** an empty form is displayed with `join_date` as an editable date field, and saving sends a POST request

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
