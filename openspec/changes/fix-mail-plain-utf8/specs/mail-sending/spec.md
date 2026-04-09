## MODIFIED Requirements

### Requirement: Send plain text mail via SMTP
The system SHALL send plain text emails using SMTP configuration from the config store. The SMTP configuration SHALL be read at send time from the following config keys: `smtp_host`, `smtp_port`, `smtp_user`, `smtp_pass`, `smtp_from`, `smtp_tls`.

The outgoing message SHALL declare its body with `Content-Type: text/plain; charset=utf-8` regardless of whether attachments are present, so that receiving clients correctly decode non-ASCII characters (e.g. German umlauts).

#### Scenario: Successful mail delivery
- **WHEN** `POST /api/mail/send` is called with valid `to_address`, `subject`, and `body`, and SMTP config is complete and correct
- **THEN** the system sends the mail via SMTP, stores the result with status `sent`, and returns the SentMail entity

#### Scenario: SMTP connection failure
- **WHEN** `POST /api/mail/send` is called but the SMTP server is unreachable
- **THEN** the system stores the result with status `failed` and the connection error message, and returns the SentMail entity with the error

#### Scenario: SMTP authentication failure
- **WHEN** `POST /api/mail/send` is called but SMTP credentials are incorrect
- **THEN** the system stores the result with status `failed` and the auth error message, and returns the SentMail entity with the error

#### Scenario: Plain text body with non-ASCII characters (no attachments)
- **WHEN** a mail without attachments is built whose body contains non-ASCII characters such as `ä`, `ö`, `ü`, `ß`
- **THEN** the serialized message SHALL contain the header value `text/plain; charset=utf-8` and the body SHALL be encoded such that any conformant MIME client (including GMX Android) decodes the characters correctly

#### Scenario: Plain text body with non-ASCII characters (with attachments)
- **WHEN** a mail with attachments is built whose body contains non-ASCII characters
- **THEN** the text part of the multipart message SHALL declare `Content-Type: text/plain; charset=utf-8` and the body SHALL be decoded correctly by receiving clients
