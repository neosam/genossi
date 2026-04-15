## ADDED Requirements

### Requirement: Mail template management page
The system SHALL provide a management page at `/mail/templates` with a two-column list-detail layout. The left column SHALL display a list of all mail templates. The right column SHALL display an editor for the selected template.

#### Scenario: Page loads and shows template list
- **WHEN** an admin navigates to `/mail/templates`
- **THEN** the left column SHALL display all non-deleted mail templates loaded from `GET /api/mail/templates`
- **AND** each list entry SHALL show the template name

#### Scenario: No templates exist
- **WHEN** the page loads and no templates exist
- **THEN** the list area SHALL show a message indicating no templates are available
- **AND** the "Neu erstellen" button SHALL still be visible

### Requirement: Create new mail template
The system SHALL allow creating a new mail template via a "Neu erstellen" button in the template list. Clicking the button SHALL open an empty editor in the right column with fields for name, subject, and body.

#### Scenario: Create template successfully
- **WHEN** the user clicks "Neu erstellen", fills in name, subject, and body, and clicks "Speichern"
- **THEN** the system SHALL call `POST /api/mail/templates` with the entered data
- **AND** the new template SHALL appear in the list
- **AND** the editor SHALL show the saved template

#### Scenario: Create template with duplicate name
- **WHEN** the user tries to create a template with a name that already exists
- **THEN** the system SHALL show an error message (409 from API)

### Requirement: Edit existing mail template
The system SHALL allow editing a mail template by clicking on it in the list. The editor SHALL be pre-filled with the template's current name, subject, and body.

#### Scenario: Edit and save template
- **WHEN** the user selects a template from the list, modifies the name/subject/body, and clicks "Speichern"
- **THEN** the system SHALL call `PUT /api/mail/templates/{id}` with the updated data and the current version
- **AND** the list SHALL reflect the updated name

#### Scenario: Version conflict on save
- **WHEN** the user saves a template that has been modified by another user since it was loaded
- **THEN** the system SHALL show an error message indicating a version conflict (409 from API)

### Requirement: Delete mail template
The system SHALL allow deleting a mail template via a "Löschen" button in the editor. Deletion SHALL require confirmation.

#### Scenario: Delete template with confirmation
- **WHEN** the user clicks "Löschen" and confirms the deletion
- **THEN** the system SHALL call `DELETE /api/mail/templates/{id}`
- **AND** the template SHALL be removed from the list
- **AND** the editor SHALL be cleared

#### Scenario: Cancel deletion
- **WHEN** the user clicks "Löschen" but cancels the confirmation
- **THEN** the template SHALL remain unchanged

### Requirement: Template variable insertion in editor
The editor SHALL include `TemplateVarButtons` so users can insert MiniJinja template variables (e.g., `{{ first_name }}`) into the body field.

#### Scenario: Insert variable into body
- **WHEN** the user clicks a variable button (e.g., "Vorname") in the template editor
- **THEN** `{{ first_name }}` SHALL be appended to the body textarea

### Requirement: Navigation to template management
The system SHALL provide navigation to the template management page from two locations: the TopBar menu (Kommunikation group) and a link on the mail compose page near the TemplateSelector.

#### Scenario: Navigate via TopBar
- **WHEN** an admin opens the Kommunikation dropdown in the TopBar
- **THEN** a "Mail-Vorlagen" menu item SHALL be visible
- **AND** clicking it SHALL navigate to `/mail/templates`

#### Scenario: Navigate via mail compose page
- **WHEN** an admin is on the mail compose page
- **THEN** a "Vorlagen verwalten" link SHALL be visible near the template selector dropdown
- **AND** clicking it SHALL navigate to `/mail/templates`

### Requirement: Admin-only access
The template management page SHALL only be accessible to users with the `admin` privilege. Unauthorized users SHALL see an access denied page.

#### Scenario: Non-admin access denied
- **WHEN** a user without `admin` privilege navigates to `/mail/templates`
- **THEN** the system SHALL display the access denied page
