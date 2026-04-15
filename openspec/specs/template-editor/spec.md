## MODIFIED Requirements

### Requirement: PDF preview
The template management page SHALL provide a preview section that allows selecting either a member or an application for rendering. A toggle with options "Mitglied" and "Antrag" SHALL switch between member selection and application selection. The system SHALL render the current template with the selected entity's data and display the resulting PDF.

#### Scenario: Preview template with member
- **WHEN** a board member selects "Mitglied" in the toggle, selects a member, and clicks "PDF rendern"
- **THEN** the system SHALL call `POST /api/templates/render/{path}/{member_id}`
- **AND** the resulting PDF SHALL be opened in a new tab

#### Scenario: Preview template with application
- **WHEN** a board member selects "Antrag" in the toggle, selects an open application, and clicks "PDF rendern"
- **THEN** the system SHALL call `POST /api/templates/render-application/{path}/{application_id}`
- **AND** the resulting PDF SHALL be opened in a new tab

#### Scenario: Toggle resets selection
- **WHEN** a board member switches from "Mitglied" to "Antrag" or vice versa
- **THEN** the previously selected entity SHALL be cleared

#### Scenario: Application search shows only open applications
- **WHEN** a board member selects "Antrag" in the toggle and searches for an application
- **THEN** only applications with status "Offen" SHALL be shown in the search results

#### Scenario: Application search by name
- **WHEN** a board member types a name in the application search field
- **THEN** applications matching by first_name or last_name SHALL be shown
- **AND** results SHALL display as "Vorname Nachname (N Anteile)"
