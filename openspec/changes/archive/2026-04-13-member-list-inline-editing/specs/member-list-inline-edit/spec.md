## ADDED Requirements

### Requirement: Edit mode toggle
The member list SHALL provide a global edit mode toggle via a button in the toolbar. When edit mode is active, the button label SHALL change to "Fertig" (done).

#### Scenario: Enter edit mode
- **WHEN** the user clicks the "Bearbeiten" button in the member list toolbar
- **THEN** the table switches to edit mode: editable cells become input fields, the button label changes to "Fertig"

#### Scenario: Exit edit mode
- **WHEN** the user clicks the "Fertig" button
- **THEN** the table switches back to view mode: all cells display as plain text, any unsaved changes in the currently focused row are saved first

### Requirement: Editable field rendering in edit mode
In edit mode, cells for editable fields SHALL render as input fields. Cells for read-only fields (action-driven or system-managed) SHALL remain as plain text.

#### Scenario: Editable fields as inputs
- **WHEN** the table is in edit mode
- **THEN** the following fields render as input fields: `member_number`, `first_name`, `last_name`, `email`, `company`, `street`, `house_number`, `postal_code`, `city`, `bank_account`, `comment`, `shares_at_joining`, `current_balance`

#### Scenario: Read-only fields as text
- **WHEN** the table is in edit mode
- **THEN** the following fields render as plain text: `join_date`, `exit_date`, `current_shares`, `migrated`, `active_status`

#### Scenario: Numeric fields use number input
- **WHEN** the table is in edit mode and a numeric field (`member_number`, `shares_at_joining`, `current_balance`) is displayed
- **THEN** the cell renders as `<input type="number">`

#### Scenario: String fields use text input
- **WHEN** the table is in edit mode and a text field (`first_name`, `last_name`, `email`, etc.) is displayed
- **THEN** the cell renders as `<input type="text">`

### Requirement: Checkbox column hidden in edit mode
The selection checkbox column SHALL NOT be displayed when the table is in edit mode.

#### Scenario: No checkboxes in edit mode
- **WHEN** the table is in edit mode
- **THEN** the checkbox column in the header and all row checkboxes are hidden

#### Scenario: Checkboxes visible in normal mode
- **WHEN** the table is in normal (view) mode
- **THEN** the checkbox column is visible as before

### Requirement: Row navigation disabled in edit mode
Row click navigation to the detail page SHALL be disabled when the table is in edit mode.

#### Scenario: No navigation on row click in edit mode
- **WHEN** the table is in edit mode and the user clicks on a row
- **THEN** the click does not navigate to the member detail page

### Requirement: Autosave on row focus loss
The system SHALL automatically save a row when focus moves away from it, if any field in that row has been modified.

#### Scenario: Save triggered on row blur with changes
- **WHEN** the user modifies a field in a row and then moves focus to a different row or outside the table
- **THEN** the system sends a `PUT /api/members/{id}` request with the updated member data

#### Scenario: No save on row blur without changes
- **WHEN** the user focuses a row but does not modify any field, then moves focus away
- **THEN** no save request is sent

#### Scenario: Save triggered when exiting edit mode
- **WHEN** the user clicks "Fertig" while a modified row is focused
- **THEN** the system saves the focused row before switching to view mode

### Requirement: Successful save handling
After a successful save, the system SHALL update the local member data with the response from the backend (including the new version UUID).

#### Scenario: Row updated after successful save
- **WHEN** a row save succeeds
- **THEN** the local member data for that row is replaced with the backend response, clearing the dirty state and updating the version for future saves

### Requirement: Error handling with row highlighting
When a save fails, the system SHALL highlight the affected row with a red background and display a toast notification with the error message.

#### Scenario: Row highlighted on save error
- **WHEN** a `PUT /api/members/{id}` request fails (e.g., validation error, version conflict, network error)
- **THEN** the affected row receives a red background highlight

#### Scenario: Toast shown on save error
- **WHEN** a row save fails
- **THEN** a toast notification appears with the error message (e.g., "Mitglied 42: Nachname darf nicht leer sein")

#### Scenario: Error cleared after successful retry
- **WHEN** a previously failed row is modified and saved successfully
- **THEN** the red highlight is removed and the row returns to normal styling

### Requirement: Required field frontend validation
The system SHALL validate required fields before sending the save request. Required fields: `first_name`, `last_name`.

#### Scenario: Empty required field prevented
- **WHEN** the user clears `first_name` or `last_name` and focus leaves the row
- **THEN** the field shows a red border, the row is highlighted red, and a toast shows the validation error. No PUT request is sent.

#### Scenario: Required field filled after error
- **WHEN** the user fills in a previously empty required field and focus leaves the row
- **THEN** the field red border is removed and the save proceeds normally
