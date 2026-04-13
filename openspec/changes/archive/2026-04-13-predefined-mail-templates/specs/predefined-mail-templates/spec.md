## ADDED Requirements

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
The system SHALL display a dropdown above the mail body field that allows selecting a predefined template.

#### Scenario: Default state
- **WHEN** the mail compose form is opened
- **THEN** the dropdown shows "Vorlage wählen..." with no template selected

#### Scenario: Selecting a template
- **WHEN** the user selects "Formell" or "Informell" from the dropdown
- **THEN** the body field is pre-filled with the selected template content

#### Scenario: Editing after selection
- **WHEN** the user selects a template and then modifies the body text
- **THEN** the modifications are preserved and sent as the mail body
