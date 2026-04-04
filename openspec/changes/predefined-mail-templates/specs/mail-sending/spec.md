## MODIFIED Requirements

### Requirement: Mail compose form
The mail compose form SHALL include a template selector dropdown between the subject field and the body field. The dropdown SHALL offer predefined templates that pre-fill the body with Jinja template syntax for personalized salutations.

#### Scenario: Compose form layout
- **WHEN** the mail compose form is displayed
- **THEN** a template dropdown appears above the body textarea, after the subject field

#### Scenario: Template pre-fills body
- **WHEN** a user selects a template from the dropdown
- **THEN** the body textarea is filled with the template content including Jinja variable placeholders
