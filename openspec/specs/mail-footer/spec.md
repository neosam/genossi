## Purpose

Configurable mail footer system that renders a per-user footer using a global template and user-specific preferences (e.g., sender name). The footer is pre-populated in the mail compose form and appended when templates are inserted.

## Requirements

### Requirement: Mail footer config
The system SHALL store a global mail footer template as config key `mail_footer`. The template SHALL support minijinja syntax with `sender_name` as available variable.

#### Scenario: Footer template configured
- **WHEN** the config key `mail_footer` is set to `"Mit freundlichen Grüßen\n{{ sender_name }}"`
- **THEN** the system uses this template for footer rendering

#### Scenario: No footer configured
- **WHEN** the config key `mail_footer` is not set
- **THEN** the system returns an empty string as rendered footer

### Requirement: Sender name user preference
The system SHALL store a sender display name per application user as user preference key `sender_name`. This is a free-text field independent of the application username.

#### Scenario: Sender name set
- **WHEN** user "admin" sets user preference `sender_name` to `"Anna Schmidt"`
- **THEN** the value `"Anna Schmidt"` is available for footer rendering

#### Scenario: Sender name not set
- **WHEN** user "admin" has no `sender_name` preference
- **THEN** the footer renders with an empty string for `sender_name`

### Requirement: Rendered footer endpoint
The system SHALL expose `GET /api/mail/footer` returning the rendered footer text for the current user. The endpoint SHALL load the `mail_footer` config template, load the current user's `sender_name` preference, render the template with minijinja, and return the result as plain text.

#### Scenario: Footer with sender name
- **WHEN** `GET /api/mail/footer` is called by user "admin" who has `sender_name` = `"Anna Schmidt"` and `mail_footer` = `"Mit freundlichen Grüßen\n{{ sender_name }}"`
- **THEN** the system returns `"Mit freundlichen Grüßen\nAnna Schmidt"`

#### Scenario: Footer without sender name
- **WHEN** `GET /api/mail/footer` is called by a user without `sender_name` preference
- **THEN** the system returns the footer template rendered with empty `sender_name`

#### Scenario: No footer template configured
- **WHEN** `GET /api/mail/footer` is called and `mail_footer` config is not set
- **THEN** the system returns an empty string

#### Scenario: Invalid footer template
- **WHEN** `GET /api/mail/footer` is called and `mail_footer` contains invalid minijinja syntax
- **THEN** the system returns an error response

### Requirement: Footer pre-population in mail compose
The frontend SHALL fetch the rendered footer via `GET /api/mail/footer` when the mail compose form opens. The body text field SHALL be initialized with `"\n\n" + rendered_footer`.

#### Scenario: Compose form opened with footer
- **WHEN** user opens the mail compose form and a footer is configured
- **THEN** the body text field contains the rendered footer preceded by two newlines

#### Scenario: Compose form opened without footer
- **WHEN** user opens the mail compose form and no footer is configured
- **THEN** the body text field is empty

### Requirement: Footer appended on template insertion
The frontend SHALL append the rendered footer when a predefined template (Formell/Informell) is inserted. The template content SHALL be followed by `"\n" + rendered_footer`.

#### Scenario: Formal template inserted with footer
- **WHEN** user selects the "Formell" template and a footer is configured
- **THEN** the body text field contains the formal template text followed by a newline and the rendered footer

#### Scenario: Template inserted without footer
- **WHEN** user selects a template and no footer is configured
- **THEN** the body text field contains only the template text

### Requirement: Footer is editable text
After insertion, the footer SHALL be normal editable text in the body field. The system SHALL NOT treat the footer differently from user-typed text during sending.

#### Scenario: User edits footer
- **WHEN** user modifies the footer text in the body field
- **THEN** the modified text is sent as-is, no footer re-rendering occurs
