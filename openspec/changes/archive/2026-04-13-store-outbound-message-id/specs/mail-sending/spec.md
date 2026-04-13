## ADDED Requirements

### Requirement: Capture outbound Message-ID

The mail worker SHALL capture the RFC 5322 `Message-ID` header of every outbound mail and persist it on the corresponding `MailRecipient` row. The `message_id` column SHALL be nullable to preserve backward compatibility with recipients created before this change. The captured value MUST be exactly the header value used during SMTP delivery (including angle brackets stripped or preserved consistently), so it can later be matched against inbound `In-Reply-To` headers.

#### Scenario: Recipient successfully sent

- **WHEN** the mail worker successfully sends a mail to a recipient via SMTP
- **THEN** the `MailRecipient` row is updated with status `sent` and `message_id` set to the `Message-ID` header of the delivered `lettre::Message`

#### Scenario: Recipient send fails

- **WHEN** the mail worker attempts to send a mail and SMTP delivery fails
- **THEN** the `MailRecipient` row is updated with the failure state and `message_id` remains `NULL`

#### Scenario: Legacy recipient row

- **WHEN** a `MailRecipient` row exists that was created before this change
- **THEN** the system reads its `message_id` as `NULL` without error
