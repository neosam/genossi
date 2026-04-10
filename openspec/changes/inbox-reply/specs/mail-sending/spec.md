## MODIFIED Requirements

### Requirement: SentMail data model
The system SHALL store mail jobs with the following fields (existing fields unchanged, new field added):
- `id` (UUID, system-generated, primary key)
- `created` (DateTime, system-generated)
- `deleted` (Optional DateTime, for soft delete)
- `version` (UUID, for optimistic locking)
- `subject` (TEXT, required)
- `body` (TEXT, required)
- `status` (TEXT, required): one of `running`, `done`, `failed`
- `total_count` (INTEGER)
- `sent_count` (INTEGER)
- `failed_count` (INTEGER)
- **`reply_to_inbound_mail_id`** (Optional UUID): when set, this job is a reply to the referenced inbound mail

#### Scenario: Regular mail job created
- **WHEN** a bulk mail job is created via the mail sending flow
- **THEN** the system stores a MailJob entity with `reply_to_inbound_mail_id` as NULL

#### Scenario: Reply mail job created
- **WHEN** a reply to an inbound mail is created
- **THEN** the system stores a MailJob entity with `reply_to_inbound_mail_id` set to the inbound mail's UUID

### Requirement: Send plain text mail via SMTP
The system SHALL send plain text emails using SMTP configuration from the config store. The SMTP configuration SHALL be read at send time from the following config keys: `smtp_host`, `smtp_port`, `smtp_user`, `smtp_pass`, `smtp_from`, `smtp_tls`. When the mail job has a `reply_to_inbound_mail_id` and the referenced inbound mail has a `message_id`, the outbound mail SHALL include `In-Reply-To` and `References` headers with the inbound mail's message ID.

#### Scenario: Successful mail delivery
- **WHEN** a mail is sent for a job without `reply_to_inbound_mail_id`
- **THEN** the system sends the mail via SMTP without `In-Reply-To` or `References` headers

#### Scenario: Reply mail delivery with threading
- **WHEN** a mail is sent for a job with `reply_to_inbound_mail_id` set and the inbound mail has a `message_id`
- **THEN** the system sends the mail with `In-Reply-To: <message_id>` and `References: <message_id>` headers

#### Scenario: Reply mail delivery without message ID
- **WHEN** a mail is sent for a job with `reply_to_inbound_mail_id` set but the inbound mail has no `message_id`
- **THEN** the system sends the mail without threading headers
