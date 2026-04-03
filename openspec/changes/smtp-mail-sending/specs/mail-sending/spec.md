## ADDED Requirements

### Requirement: SentMail data model
The system SHALL store sent mails with the following fields:
- `id` (UUID, system-generated, primary key)
- `created` (DateTime, system-generated)
- `deleted` (Optional DateTime, for soft delete)
- `version` (UUID, for optimistic locking)
- `to_address` (TEXT, required)
- `subject` (TEXT, required)
- `body` (TEXT, required)
- `status` (TEXT, required): one of `sent`, `failed`
- `error` (Optional TEXT): error message from SMTP transport on failure
- `sent_at` (Optional DateTime): timestamp when the mail was actually sent

#### Scenario: Successful mail stored
- **WHEN** a mail is sent successfully
- **THEN** the system stores a SentMail entity with status `sent`, `sent_at` set to the current timestamp, and `error` as NULL

#### Scenario: Failed mail stored
- **WHEN** a mail fails to send
- **THEN** the system stores a SentMail entity with status `failed`, `error` set to the SMTP error message, and `sent_at` as NULL

### Requirement: Send plain text mail via SMTP
The system SHALL send plain text emails using SMTP configuration from the config store. The SMTP configuration SHALL be read at send time from the following config keys: `smtp_host`, `smtp_port`, `smtp_user`, `smtp_pass`, `smtp_from`, `smtp_tls`.

#### Scenario: Successful mail delivery
- **WHEN** `POST /api/mail/send` is called with valid `to_address`, `subject`, and `body`, and SMTP config is complete and correct
- **THEN** the system sends the mail via SMTP, stores the result with status `sent`, and returns the SentMail entity

#### Scenario: SMTP connection failure
- **WHEN** `POST /api/mail/send` is called but the SMTP server is unreachable
- **THEN** the system stores the result with status `failed` and the connection error message, and returns the SentMail entity with the error

#### Scenario: SMTP authentication failure
- **WHEN** `POST /api/mail/send` is called but SMTP credentials are incorrect
- **THEN** the system stores the result with status `failed` and the auth error message, and returns the SentMail entity with the error

### Requirement: SMTP config validation before send
The system SHALL validate that all required SMTP config keys are present before attempting to send a mail. If any key is missing, the send SHALL fail with a descriptive error without attempting SMTP connection.

#### Scenario: Missing SMTP config
- **WHEN** `POST /api/mail/send` is called but `smtp_host` is not configured
- **THEN** the system returns an error indicating that SMTP configuration is incomplete, without storing a SentMail entity

#### Scenario: All SMTP config present
- **WHEN** `POST /api/mail/send` is called and all SMTP config keys are set
- **THEN** the system proceeds with sending the mail

### Requirement: TLS mode selection
The system SHALL support three TLS modes for SMTP, configured via the `smtp_tls` config key:
- `none`: No encryption
- `starttls`: STARTTLS upgrade
- `tls`: Implicit TLS

#### Scenario: STARTTLS connection
- **WHEN** `smtp_tls` is set to `starttls`
- **THEN** the system connects to the SMTP server and upgrades to TLS via STARTTLS

#### Scenario: Implicit TLS connection
- **WHEN** `smtp_tls` is set to `tls`
- **THEN** the system connects to the SMTP server using implicit TLS

#### Scenario: No encryption
- **WHEN** `smtp_tls` is set to `none`
- **THEN** the system connects to the SMTP server without encryption

### Requirement: Mail sending REST endpoint
The system SHALL expose `POST /api/mail/send` accepting a JSON body with `to_address`, `subject`, and `body` fields.

#### Scenario: Valid mail request
- **WHEN** `POST /api/mail/send` is called with `{"to_address": "user@example.com", "subject": "Test", "body": "Hello"}`
- **THEN** the system attempts to send the mail and returns the SentMail entity

#### Scenario: Missing required field
- **WHEN** `POST /api/mail/send` is called without `to_address`
- **THEN** the system returns a 422 validation error

### Requirement: Sent mail history endpoint
The system SHALL expose `GET /api/mail/sent` returning all stored SentMail entities ordered by creation time descending.

#### Scenario: List sent mails
- **WHEN** `GET /api/mail/sent` is called
- **THEN** the system returns all SentMail entities with their status, error, and timestamps

#### Scenario: No mails sent yet
- **WHEN** `GET /api/mail/sent` is called and no mails have been sent
- **THEN** the system returns an empty list
