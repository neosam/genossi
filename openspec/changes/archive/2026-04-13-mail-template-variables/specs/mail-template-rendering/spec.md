## ADDED Requirements

### Requirement: Template rendering with MiniJinja
The system SHALL render email subject and body as MiniJinja templates, replacing template variables with member-specific data before sending. The template syntax SHALL support variable interpolation (`{{ variable_name }}`) and conditional logic (`{% if %}`, `{% elif %}`, `{% else %}`, `{% endif %}`).

#### Scenario: Simple variable substitution
- **WHEN** an email template body contains `Hallo {{ first_name }} {{ last_name }}`
- **THEN** the system renders `Hallo Max Mustermann` for a member with first_name "Max" and last_name "Mustermann"

#### Scenario: Conditional logic in template
- **WHEN** an email template body contains `{% if salutation == "Frau" %}Sehr geehrte Frau{% elif salutation == "Herr" %}Sehr geehrter Herr{% endif %} {{ last_name }}`
- **THEN** the system renders `Sehr geehrter Herr Mustermann` for a member with salutation "Herr" and last_name "Mustermann"

#### Scenario: Optional field is null
- **WHEN** an email template body contains `{% if company %}Firma: {{ company }}{% endif %}` and the member has no company set
- **THEN** the system renders an empty string for that block

#### Scenario: Subject as template
- **WHEN** an email template subject contains `Einladung fuer {{ first_name }}`
- **THEN** the system renders `Einladung fuer Max` for a member with first_name "Max"

### Requirement: Member template context
The system SHALL provide all MemberEntity fields as template variables. The following variables SHALL be available:

- `member_number` (integer)
- `first_name` (string)
- `last_name` (string)
- `email` (string or null)
- `company` (string or null)
- `comment` (string or null)
- `street` (string or null)
- `house_number` (string or null)
- `postal_code` (string or null)
- `city` (string or null)
- `join_date` (string, ISO date format)
- `shares_at_joining` (integer)
- `current_shares` (integer)
- `current_balance` (integer)
- `exit_date` (string or null, ISO date format)
- `bank_account` (string or null)
- `migrated` (boolean)
- `salutation` (string or null)
- `title` (string or null)

#### Scenario: All fields accessible
- **WHEN** an email template references `{{ member_number }}`, `{{ join_date }}`, and `{{ current_shares }}`
- **THEN** the system renders the member's actual values for each field

#### Scenario: Null fields in context
- **WHEN** a member has no `exit_date` set and the template references `{{ exit_date }}`
- **THEN** the system renders an empty string (MiniJinja default for undefined/null)

### Requirement: Template validation before job creation
The system SHALL validate email templates (both subject and body) before creating a mail job. Validation SHALL include syntax checking and probe-rendering against all recipient members. If validation fails, the system SHALL return an error and NOT create the mail job.

#### Scenario: Syntax error in template
- **WHEN** `POST /api/mail/send-bulk` is called with body `Hallo {{ first_name` (unclosed tag)
- **THEN** the system returns a 400 error with a descriptive message about the syntax error and does not create a mail job

#### Scenario: Unknown variable in template
- **WHEN** `POST /api/mail/send-bulk` is called with body `{{ nonexistent_field }}`
- **THEN** the system returns a 400 error indicating the unknown variable

#### Scenario: Valid template passes validation
- **WHEN** `POST /api/mail/send-bulk` is called with body `Hallo {{ first_name }}` and all recipients have a member_id
- **THEN** the system validates the template successfully and creates the mail job

### Requirement: Preview endpoint
The system SHALL expose `POST /api/mail/preview` accepting a JSON body with `subject` (string), `body` (string), and `member_id` (UUID string). The endpoint SHALL render both subject and body as templates against the specified member's data and return the result.

#### Scenario: Successful preview
- **WHEN** `POST /api/mail/preview` is called with `{"subject": "Hallo {{ first_name }}", "body": "Liebe/r {{ first_name }}...", "member_id": "<valid-uuid>"}`
- **THEN** the system returns `{"subject": "Hallo Max", "body": "Liebe/r Max...", "errors": []}`

#### Scenario: Preview with syntax error
- **WHEN** `POST /api/mail/preview` is called with `{"subject": "Test", "body": "{{ unclosed", "member_id": "<valid-uuid>"}`
- **THEN** the system returns `{"subject": "Test", "body": "", "errors": ["syntax error: ..."]}`

#### Scenario: Preview with unknown member
- **WHEN** `POST /api/mail/preview` is called with a `member_id` that does not exist
- **THEN** the system returns a 404 error

### Requirement: Template rendering in worker
The mail worker SHALL render subject and body templates for each recipient using the recipient's linked member data. The worker SHALL load member data via MemberDao using the recipient's `member_id`.

#### Scenario: Worker renders per-recipient template
- **WHEN** the worker processes a pending recipient with member_id pointing to member "Max Mustermann"
- **THEN** the worker renders the job's subject and body templates with Max's data before sending

#### Scenario: Worker handles missing member gracefully
- **WHEN** the worker processes a recipient whose member_id references a deleted or missing member
- **THEN** the worker marks the recipient as failed with an error message indicating the member could not be found
