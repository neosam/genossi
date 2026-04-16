## Purpose

Predefined mail templates (Formell/Informell) that generate gender-aware, personalized salutations with title support for use in the mail compose form.

## Requirements

### Requirement: Formal mail template
The system SHALL provide a predefined "Formell" template that generates a gender-aware formal salutation with title support and formal closing.

#### Scenario: Male recipient with title
- **WHEN** template "Formell" is applied for a member with salutation "Herr" and title "Dr."
- **THEN** the body starts with "Sehr geehrter Herr Dr. {last_name}," and ends with "Mit freundlichen Grüßen"

#### Scenario: Female recipient without title
- **WHEN** template "Formell" is applied for a member with salutation "Frau" and no title
- **THEN** the body starts with "Sehr geehrte Frau {last_name}," and ends with "Mit freundlichen Grüßen"

#### Scenario: Recipient without salutation
- **WHEN** template "Formell" is applied for a member with no salutation
- **THEN** the body starts with "Sehr geehrtes Mitglied {last_name}," and ends with "Mit freundlichen Grüßen"

### Requirement: Informal mail template
The system SHALL provide a predefined "Informell" template that generates a gender-aware informal greeting with title support and casual closing.

#### Scenario: Male recipient with title
- **WHEN** template "Informell" is applied for a member with salutation "Herr" and title "Dr."
- **THEN** the body starts with "Lieber Dr. {first_name}," and ends with "Viele Grüße"

#### Scenario: Female recipient without title
- **WHEN** template "Informell" is applied for a member with salutation "Frau" and no title
- **THEN** the body starts with "Liebe {first_name}," and ends with "Viele Grüße"

#### Scenario: Recipient without salutation
- **WHEN** template "Informell" is applied for a member with no salutation
- **THEN** the body starts with "Hallo {first_name}," and ends with "Viele Grüße"

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
