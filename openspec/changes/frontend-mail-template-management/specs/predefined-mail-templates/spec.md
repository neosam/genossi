## MODIFIED Requirements

### Requirement: Template selection dropdown
The system SHALL display a dropdown above the mail body field that allows selecting a mail template. The dropdown SHALL load templates dynamically from the API (`GET /api/mail/templates`) instead of using hardcoded templates.

#### Scenario: Default state
- **WHEN** the mail compose form is opened
- **THEN** the dropdown SHALL show "Vorlage wählen..." with no template selected
- **AND** the available templates SHALL be loaded from the API

#### Scenario: Selecting a template
- **WHEN** the user selects a template from the dropdown
- **THEN** the body field SHALL be pre-filled with the selected template's body content
- **AND** if the template has a subject, the subject field SHALL remain unchanged (subject is informational only in the template)

#### Scenario: API unavailable
- **WHEN** the API call to load templates fails
- **THEN** the dropdown SHALL show "Vorlage wählen..." with no options available

#### Scenario: Editing after selection
- **WHEN** the user selects a template and then modifies the body text
- **THEN** the modifications SHALL be preserved and sent as the mail body
