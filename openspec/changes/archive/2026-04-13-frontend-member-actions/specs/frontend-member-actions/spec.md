## ADDED Requirements

### Requirement: Frontend MemberAction types
The frontend rest-types SHALL define `MemberActionTO`, `ActionTypeTO`, and `MigrationStatusTO` types matching the backend API response format, with serde Serialize/Deserialize and appropriate date handling.

#### Scenario: Deserialize action from API
- **WHEN** the frontend receives a JSON response from `GET /api/members/{id}/actions`
- **THEN** it SHALL deserialize into `Vec<MemberActionTO>` with all fields correctly mapped

### Requirement: Frontend API functions for actions
The frontend SHALL provide async API functions for Member-Action CRUD and migration status:
- `get_member_actions(member_id)` — GET `/api/members/{member_id}/actions`
- `create_member_action(member_id, action)` — POST `/api/members/{member_id}/actions`
- `update_member_action(member_id, action_id, action)` — PUT `/api/members/{member_id}/actions/{action_id}`
- `delete_member_action(member_id, action_id)` — DELETE `/api/members/{member_id}/actions/{action_id}`
- `get_migration_status(member_id)` — GET `/api/members/{member_id}/actions/migration-status`

#### Scenario: Fetch actions for member
- **WHEN** `get_member_actions(member_id)` is called
- **THEN** it SHALL return a list of actions sorted by date

#### Scenario: Create action
- **WHEN** `create_member_action(member_id, action)` is called with valid data
- **THEN** it SHALL return the created action with assigned ID and version

### Requirement: Actions list on member detail page
The member detail page SHALL display a list of all actions for the member below the member form, showing action_type, date, shares_change, and comment.

#### Scenario: View actions for existing member
- **WHEN** a user navigates to the detail page of an existing member
- **THEN** the actions list SHALL be loaded and displayed below the member data

#### Scenario: No actions yet
- **WHEN** a member has no actions
- **THEN** the section SHALL show a message indicating no actions exist

#### Scenario: Actions only shown for saved members
- **WHEN** the user is creating a new member (id = "new")
- **THEN** the actions section SHALL NOT be displayed

### Requirement: Action create/edit form
The member detail page SHALL provide an inline form to create and edit actions with fields for action_type, date, shares_change, transfer_member_id, effective_date, and comment.

#### Scenario: Create new action
- **WHEN** the user fills in the action form and clicks save
- **THEN** the system SHALL create the action via API and refresh the actions list

#### Scenario: Conditional fields based on action type
- **WHEN** the user selects a status action type (Eintritt, Austritt, Todesfall)
- **THEN** the shares_change field SHALL be hidden or disabled (fixed to 0)

#### Scenario: Transfer fields
- **WHEN** the user selects UebertragungEmpfang or UebertragungAbgabe
- **THEN** the transfer_member_id field SHALL be shown and required

#### Scenario: Effective date for Austritt
- **WHEN** the user selects Austritt as action type
- **THEN** the effective_date field SHALL be shown

#### Scenario: Edit existing action
- **WHEN** the user clicks an action in the list
- **THEN** the form SHALL be populated with the action data for editing

### Requirement: Delete action
The member detail page SHALL allow deleting an action from the list.

#### Scenario: Delete action with confirmation
- **WHEN** the user clicks delete on an action
- **THEN** the system SHALL delete the action via API and refresh the actions list

### Requirement: Migration status display
The member detail page SHALL show the migration status of the member.

#### Scenario: Member migrated
- **WHEN** the migration status is "migrated"
- **THEN** a green badge SHALL be displayed indicating the member is fully migrated

#### Scenario: Member pending migration
- **WHEN** the migration status is "pending"
- **THEN** an orange badge SHALL be displayed with details: expected shares, actual shares, expected action count, actual action count

#### Scenario: Migration status only for saved members
- **WHEN** the user is creating a new member
- **THEN** the migration status SHALL NOT be displayed

### Requirement: i18n for member actions
The frontend SHALL provide translations for all action-related UI elements in German and English.

#### Scenario: German translations
- **WHEN** the locale is German
- **THEN** action types SHALL be displayed as: Eintritt, Austritt, Todesfall, Aufstockung, Verkauf, Uebertragung Empfang, Uebertragung Abgabe

#### Scenario: English translations
- **WHEN** the locale is English
- **THEN** action types SHALL be displayed as: Entry, Exit, Death, Increase, Sale, Transfer In, Transfer Out
