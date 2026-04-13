## MODIFIED Requirements

### Requirement: Send plain text mail via SMTP
The system SHALL send plain text emails using SMTP configuration from the config store. The SMTP configuration SHALL be read at send time from the following config keys: `smtp_host`, `smtp_port`, `smtp_user`, `smtp_pass`, `smtp_from`, `smtp_tls`. Sending is now performed asynchronously via the mail queue worker, not synchronously in the HTTP request.

#### Scenario: Successful mail delivery
- **WHEN** the background worker processes a pending recipient with valid SMTP config
- **THEN** the system sends the mail via SMTP, updates the recipient status to `sent`, and increments the job's sent_count

#### Scenario: SMTP connection failure
- **WHEN** the background worker processes a pending recipient but the SMTP server is unreachable
- **THEN** the system updates the recipient status to `failed` with the connection error message, increments the job's failed_count, and continues with the next recipient

#### Scenario: SMTP authentication failure
- **WHEN** the background worker processes a pending recipient but SMTP credentials are incorrect
- **THEN** the system updates the recipient status to `failed` with the auth error message, increments the job's failed_count, and continues with the next recipient

### Requirement: Mail sending REST endpoint
The system SHALL expose `POST /api/mail/send` accepting a JSON body with `to_address`, `subject`, and `body` fields. The endpoint SHALL create a mail job with one recipient and return HTTP 202 with the created job. The mail is sent asynchronously by the background worker.

#### Scenario: Valid mail request
- **WHEN** `POST /api/mail/send` is called with `{"to_address": "user@example.com", "subject": "Test", "body": "Hello"}`
- **THEN** the system creates a mail job with one recipient, returns HTTP 202 with the MailJob entity

#### Scenario: Missing required field
- **WHEN** `POST /api/mail/send` is called without `to_address`
- **THEN** the system returns a 422 validation error

### Requirement: Bulk mail sending endpoint
The system SHALL expose `POST /api/mail/send-bulk` accepting a JSON body with `to_addresses` (array of objects with `address` and optional `member_id`), `subject`, and `body` fields. The endpoint SHALL create a single mail job with one recipient per address and return HTTP 202 with the created job. Sending is handled asynchronously by the background worker.

#### Scenario: Bulk send to multiple recipients
- **WHEN** `POST /api/mail/send-bulk` is called with 600 recipients
- **THEN** the system creates one mail job with 600 recipients (all pending), returns HTTP 202 with the MailJob entity immediately

#### Scenario: Bulk send with empty list
- **WHEN** `POST /api/mail/send-bulk` is called with an empty `to_addresses` array
- **THEN** the system returns a 400 error

### Requirement: Sent mail history endpoint
The system SHALL expose `GET /api/mail/jobs` returning all stored MailJob entities ordered by creation time descending. This replaces the previous `GET /api/mail/sent` endpoint.

#### Scenario: List mail jobs
- **WHEN** `GET /api/mail/jobs` is called
- **THEN** the system returns all MailJob entities with their status, counts, and timestamps

#### Scenario: No jobs yet
- **WHEN** `GET /api/mail/jobs` is called and no jobs exist
- **THEN** the system returns an empty list

## MODIFIED Requirements

### Requirement: Bulk mail batching
The system SHALL process mail recipients sequentially with a configurable interval between sends (default: 36 seconds, configured via `mail_send_interval_seconds`). This replaces the previous batch-of-10-with-500ms-pause approach.

#### Scenario: Large recipient list
- **WHEN** a mail job is created with 600 recipients
- **THEN** the background worker processes them one at a time, waiting the configured interval between each send

## REMOVED Requirements

### Requirement: SentMail data model
**Reason**: Replaced by MailJob + MailRecipient data model which provides grouping and better tracking.
**Migration**: All mail data is now accessed via `GET /api/mail/jobs` and `GET /api/mail/jobs/{id}`. The `sent_mails` table is dropped.
