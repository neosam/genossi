## MODIFIED Requirements

### Requirement: Create member action
The system SHALL allow authenticated users to create member actions. The create operation SHALL be performed via `audited_create!` macro, logging all non-None action fields to the audit log. This includes actions created automatically during member creation (Eintritt, Aufstockung) and manually created actions.

#### Scenario: Manual action creation audit
- **WHEN** an authenticated user creates a new member action (e.g., Aufstockung)
- **THEN** the system creates the action and creates audit log entries for all non-None action fields with action "create"

#### Scenario: Automatic action creation audit
- **WHEN** the system automatically creates Eintritt and Aufstockung actions during member creation
- **THEN** audit log entries are created for each action's fields, with the user_id of the user who created the member

### Requirement: Update member action
The system SHALL perform member action updates via `audited_update!` macro, which loads the old action, performs the update, and logs only changed fields.

#### Scenario: Action update audit
- **WHEN** an authenticated user updates a member action changing shares_change and comment
- **THEN** the system updates the action and creates exactly 2 audit log entries for shares_change and comment

### Requirement: Delete member action
The system SHALL perform member action soft-deletes via `audited_delete!` macro, logging all audited field values.

#### Scenario: Action deletion audit
- **WHEN** an authenticated user deletes a member action
- **THEN** the system soft-deletes the action and creates audit log entries with action "delete" for all audited fields
