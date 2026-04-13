## MODIFIED Requirements

### Requirement: Bulk mail sending endpoint
The system SHALL expose `POST /api/mail/send-bulk` accepting a JSON body with `to_addresses` (array of `BulkRecipient`), `subject`, `body`, and optional `attachment_ids` (array of document ID strings) fields. The system sends one individually addressed email per recipient, stores one `MailRecipient` entry per recipient, and returns the `MailJob`. When `attachment_ids` is provided and non-empty, `to_addresses` SHALL contain exactly one entry; otherwise the system SHALL reject the request with a 400 error.

#### Scenario: Bulk send to multiple recipients
- **WHEN** `POST /api/mail/send-bulk` is called with `{"to_addresses": [{"address": "a@example.com"}, {"address": "b@example.com"}], "subject": "Test", "body": "Hello"}`
- **THEN** the system sends one email to each address individually, stores each result as a separate MailRecipient entity, and returns the MailJob

#### Scenario: Bulk send with empty list
- **WHEN** `POST /api/mail/send-bulk` is called with an empty `to_addresses` array
- **THEN** the system returns an empty result without attempting SMTP connection

#### Scenario: Bulk send partial failure
- **WHEN** `POST /api/mail/send-bulk` is called with multiple addresses and some fail
- **THEN** the system continues sending to remaining addresses, stores each result individually (sent or failed), and returns all results

#### Scenario: Single send with attachments
- **WHEN** `POST /api/mail/send-bulk` is called with one recipient and `attachment_ids: ["doc-uuid"]`
- **THEN** the system validates the attachments, creates the job and recipient with linked attachments, and returns the MailJob

#### Scenario: Multiple recipients with attachments rejected
- **WHEN** `POST /api/mail/send-bulk` is called with two recipients and `attachment_ids: ["doc-uuid"]`
- **THEN** the system returns a 400 error indicating attachments are only supported for single-recipient sends
