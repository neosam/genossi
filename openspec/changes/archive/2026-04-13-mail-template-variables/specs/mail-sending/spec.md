## MODIFIED Requirements

### Requirement: Bulk mail sending endpoint
The system SHALL expose `POST /api/mail/send-bulk` accepting a JSON body with `to_addresses` (array of BulkRecipient with `address` and `member_id`), `subject`, and `body` fields. The subject and body SHALL be treated as MiniJinja templates. Before creating the mail job, the system SHALL validate both templates by probe-rendering against all recipient members. If validation fails, the system SHALL return a 400 error with descriptive error messages and NOT create the mail job. All recipients MUST have a `member_id`. The system sends one individually addressed email per recipient, with subject and body rendered per-recipient using their member data.

#### Scenario: Bulk send with template variables
- **WHEN** `POST /api/mail/send-bulk` is called with `{"to_addresses": [{"address": "max@example.com", "member_id": "uuid-1"}, {"address": "erika@example.com", "member_id": "uuid-2"}], "subject": "Hallo {{ first_name }}", "body": "Liebe/r {{ first_name }}..."}`
- **THEN** the system validates the templates against both members, creates the mail job with the template as body, and the worker later renders personalized emails for each recipient

#### Scenario: Bulk send with invalid template
- **WHEN** `POST /api/mail/send-bulk` is called with body containing `{{ unclosed`
- **THEN** the system returns a 400 error with the template syntax error and does not create a mail job

#### Scenario: Bulk send with missing member_id
- **WHEN** `POST /api/mail/send-bulk` is called with a recipient that has no `member_id`
- **THEN** the system returns a 400 error indicating that all recipients must have a member_id

#### Scenario: Bulk send with empty list
- **WHEN** `POST /api/mail/send-bulk` is called with an empty `to_addresses` array
- **THEN** the system returns an error without attempting validation or job creation

#### Scenario: Bulk send partial validation failure
- **WHEN** `POST /api/mail/send-bulk` is called with a template referencing `{{ nonexistent }}` which fails for all members
- **THEN** the system returns a 400 error with the validation error and does not create a mail job

### Requirement: Send plain text mail via SMTP
The system SHALL send plain text emails using SMTP configuration from the config store. The SMTP configuration SHALL be read at send time from the following config keys: `smtp_host`, `smtp_port`, `smtp_user`, `smtp_pass`, `smtp_from`, `smtp_tls`. For mails with a linked member, the worker SHALL render subject and body templates against the member's data before sending.

#### Scenario: Successful mail delivery with template
- **WHEN** the worker processes a pending recipient with member_id and the job has template subject/body
- **THEN** the worker loads the member data, renders the templates, sends the personalized email via SMTP, and stores the result with status `sent`

#### Scenario: Successful mail delivery without template syntax
- **WHEN** the worker processes a pending recipient and the job body contains no template tags
- **THEN** the worker sends the body as-is (plain text passthrough)

#### Scenario: Template rendering failure at send time
- **WHEN** the worker processes a recipient but template rendering fails (e.g., member deleted between validation and send)
- **THEN** the worker marks the recipient as `failed` with an error message describing the rendering failure
