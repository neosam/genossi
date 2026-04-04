## ADDED Requirements

### Requirement: Column definition registry
The frontend SHALL define a static registry of all available columns for the member list. Each column definition SHALL include:
- `key` (string identifier matching MemberTO field name)
- `label_key` (i18n translation key)
- `editable` (boolean, whether the field can be inline-edited — for future use)
- A render function that extracts the display value from a MemberTO

#### Scenario: All MemberTO display fields available
- **WHEN** the column registry is initialized
- **THEN** it SHALL contain entries for: `member_number`, `last_name`, `first_name`, `email`, `company`, `street`, `house_number`, `postal_code`, `city`, `current_shares`, `current_balance`, `shares_at_joining`, `bank_account`, `comment`, `join_date`, `exit_date`, `migrated`, `active_status`

### Requirement: Default column set
The system SHALL display a default set of columns when the user has no saved column preference. The default columns SHALL be: `member_number`, `last_name`, `first_name`, `city`, `current_shares`, `join_date`, `exit_date`, `migrated`, `active_status`.

#### Scenario: New user sees default columns
- **WHEN** a user opens the member list for the first time and has no saved `member_list_columns` preference
- **THEN** the table displays the default column set

#### Scenario: Preference loading fails gracefully
- **WHEN** the preference API returns an error or the stored value contains unknown column keys
- **THEN** the system SHALL fall back to the default column set, ignoring unknown keys

### Requirement: Column picker UI
The frontend SHALL provide a column picker accessible via a button in the member list toolbar. The picker SHALL display all available columns as checkboxes.

#### Scenario: Open column picker
- **WHEN** the user clicks the "Spalten" button in the toolbar
- **THEN** a popover appears showing all available columns with checkboxes indicating which are currently visible

#### Scenario: Toggle column visibility
- **WHEN** the user checks or unchecks a column in the picker
- **THEN** the table immediately updates to show or hide that column

#### Scenario: Close column picker
- **WHEN** the user clicks outside the popover
- **THEN** the popover closes

### Requirement: Persist column selection
The frontend SHALL save the selected columns to the backend whenever the user changes the selection.

#### Scenario: Column selection saved
- **WHEN** the user changes the column selection via the picker
- **THEN** the frontend sends a `PUT /api/user-preferences/member_list_columns` request with the selected column keys as a JSON array

#### Scenario: Column selection restored on page load
- **WHEN** the user navigates to the member list
- **THEN** the frontend fetches `GET /api/user-preferences/member_list_columns` and applies the stored column selection

### Requirement: Dynamic table rendering
The member list table SHALL render columns dynamically based on the current column selection instead of hardcoded markup.

#### Scenario: Table headers match selection
- **WHEN** the user has selected columns `["member_number", "last_name", "city"]`
- **THEN** the table header row displays exactly those three column headers in registry order

#### Scenario: Table cells match selection
- **WHEN** the table renders a member row with selected columns `["member_number", "last_name", "city"]`
- **THEN** each row contains exactly three cells with the corresponding field values

#### Scenario: Checkbox column always present
- **WHEN** the table is in normal mode (not edit mode)
- **THEN** a checkbox column is always prepended regardless of column selection
