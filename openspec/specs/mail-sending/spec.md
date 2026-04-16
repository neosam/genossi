## Requirements

### Requirement: Send plain text mail via SMTP
The system SHALL send plain text emails using SMTP configuration from the config store. The SMTP configuration SHALL be read at send time from the following config keys: `smtp_host`, `smtp_port`, `smtp_user`, `smtp_pass`, `smtp_from`, `smtp_tls`. Sending is now performed asynchronously via the mail queue worker, not synchronously in the HTTP request. For mails with a linked member, the worker SHALL render subject and body templates against the member's data before sending. When the mail job has a `reply_to_inbound_mail_id` and the referenced inbound mail has a `message_id`, the outbound mail SHALL include `In-Reply-To` and `References` headers with the inbound mail's message ID.

The outgoing message SHALL declare its body with `Content-Type: text/plain; charset=utf-8` regardless of whether attachments are present, so that receiving clients correctly decode non-ASCII characters (e.g. German umlauts).

#### Scenario: Successful mail delivery
- **WHEN** the background worker processes a pending recipient with valid SMTP config
- **THEN** the system sends the mail via SMTP, updates the recipient status to `sent`, and increments the job's sent_count

#### Scenario: Successful mail delivery with template
- **WHEN** the worker processes a pending recipient with member_id and the job has template subject/body
- **THEN** the worker loads the member data, renders the templates, sends the personalized email via SMTP, and stores the result with status `sent`

#### Scenario: Successful mail delivery without template syntax
- **WHEN** the worker processes a pending recipient and the job body contains no template tags
- **THEN** the worker sends the body as-is (plain text passthrough)

#### Scenario: Template rendering failure at send time
- **WHEN** the worker processes a recipient but template rendering fails (e.g., member deleted between validation and send)
- **THEN** the worker marks the recipient as `failed` with an error message describing the rendering failure

#### Scenario: SMTP connection failure
- **WHEN** the background worker processes a pending recipient but the SMTP server is unreachable
- **THEN** the system updates the recipient status to `failed` with the connection error message, increments the job's failed_count, and continues with the next recipient

#### Scenario: SMTP authentication failure
- **WHEN** the background worker processes a pending recipient but SMTP credentials are incorrect
- **THEN** the system updates the recipient status to `failed` with the auth error message, increments the job's failed_count, and continues with the next recipient

#### Scenario: Reply mail delivery with threading
- **WHEN** a mail is sent for a job with `reply_to_inbound_mail_id` set and the inbound mail has a `message_id`
- **THEN** the system sends the mail with `In-Reply-To: <message_id>` and `References: <message_id>` headers

#### Scenario: Reply mail delivery without message ID
- **WHEN** a mail is sent for a job with `reply_to_inbound_mail_id` set but the inbound mail has no `message_id`
- **THEN** the system sends the mail without threading headers

#### Scenario: Plain text body with non-ASCII characters (no attachments)
- **WHEN** a mail without attachments is built whose body contains non-ASCII characters such as `ä`, `ö`, `ü`, `ß`
- **THEN** the serialized message SHALL contain the header value `text/plain; charset=utf-8` and the body SHALL be encoded such that any conformant MIME client (including GMX Android) decodes the characters correctly

#### Scenario: Plain text body with non-ASCII characters (with attachments)
- **WHEN** a mail with attachments is built whose body contains non-ASCII characters
- **THEN** the text part of the multipart message SHALL declare `Content-Type: text/plain; charset=utf-8` and the body SHALL be decoded correctly by receiving clients

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
The system SHALL expose `POST /api/mail/send` accepting a JSON body with `to_address`, `subject`, and `body` fields. The endpoint SHALL create a mail job with one recipient and return HTTP 202 with the created job. The mail is sent asynchronously by the background worker.

#### Scenario: Valid mail request
- **WHEN** `POST /api/mail/send` is called with `{"to_address": "user@example.com", "subject": "Test", "body": "Hello"}`
- **THEN** the system creates a mail job with one recipient, returns HTTP 202 with the MailJob entity

#### Scenario: Missing required field
- **WHEN** `POST /api/mail/send` is called without `to_address`
- **THEN** the system returns a 422 validation error

### Requirement: Bulk mail sending endpoint
The system SHALL expose `POST /api/mail/send-bulk` accepting a JSON body with `to_addresses` (array of BulkRecipient with `address` and `member_id`), `subject`, `body`, and optional `attachment_ids` (array of document ID strings) fields. The subject and body SHALL be treated as MiniJinja templates. Before creating the mail job, the system SHALL validate both templates by probe-rendering against all recipient members. If validation fails, the system SHALL return a 400 error with descriptive error messages and NOT create the mail job. All recipients MUST have a `member_id`. The system sends one individually addressed email per recipient, with subject and body rendered per-recipient using their member data. When `attachment_ids` is provided and non-empty, `to_addresses` SHALL contain exactly one entry; otherwise the system SHALL reject the request with a 400 error.

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

#### Scenario: Bulk send partial failure
- **WHEN** `POST /api/mail/send-bulk` is called with multiple addresses and some fail
- **THEN** the system continues sending to remaining addresses, stores each result individually (sent or failed), and returns all results

#### Scenario: Bulk send partial validation failure
- **WHEN** `POST /api/mail/send-bulk` is called with a template referencing `{{ nonexistent }}` which fails for all members
- **THEN** the system returns a 400 error with the validation error and does not create a mail job

#### Scenario: Single send with attachments
- **WHEN** `POST /api/mail/send-bulk` is called with one recipient and `attachment_ids: ["doc-uuid"]`
- **THEN** the system validates the attachments, creates the job and recipient with linked attachments, and returns the MailJob

#### Scenario: Multiple recipients with attachments rejected
- **WHEN** `POST /api/mail/send-bulk` is called with two recipients and `attachment_ids: ["doc-uuid"]`
- **THEN** the system returns a 400 error indicating attachments are only supported for single-recipient sends

### Requirement: Bulk mail batching
The system SHALL process mail recipients sequentially with a configurable interval between sends (default: 36 seconds, configured via `mail_send_interval_seconds`). The worker SHALL NOT automatically retry recipients that have status `failed`. Only an explicit call to `POST /api/mail/jobs/{id}/retry` SHALL reset failed recipients to `pending`.

#### Scenario: Large recipient list
- **WHEN** a mail job is created with 600 recipients
- **THEN** the background worker processes them one at a time, waiting the configured interval between each send

#### Scenario: Failed recipient not retried automatically
- **WHEN** a mail recipient fails with an SMTP error
- **THEN** the recipient status is set to `failed` and the worker does NOT attempt to send to this recipient again unless explicitly retried via the retry endpoint

#### Scenario: Explicit retry resets failed recipients
- **WHEN** `POST /api/mail/jobs/{id}/retry` is called for a job with failed recipients
- **THEN** all `failed` recipients are reset to `pending` and the job status is set to `running`

### Requirement: Sent mail history endpoint
The system SHALL expose `GET /api/mail/jobs` returning all stored MailJob entities ordered by creation time descending. This replaces the previous `GET /api/mail/sent` endpoint.

#### Scenario: List mail jobs
- **WHEN** `GET /api/mail/jobs` is called
- **THEN** the system returns all MailJob entities with their status, counts, and timestamps

#### Scenario: No jobs yet
- **WHEN** `GET /api/mail/jobs` is called and no jobs exist
- **THEN** the system returns an empty list

### Requirement: Empfänger-Darstellung auf der Mail-Seite
Die Mail-Seite MUSS Empfänger in einer skalierbaren Darstellung anzeigen, die sowohl für wenige als auch für hunderte Empfänger funktioniert. Standardmäßig wird eine eingeklappte Zusammenfassung angezeigt, die auf Klick zu einer scrollbaren Detail-Liste aufgeklappt werden kann.

#### Scenario: Eingeklappte Ansicht bei Empfängern
- **WHEN** Empfänger ausgewählt sind
- **THEN** zeigt die Mail-Seite eine eingeklappte Zusammenfassung mit Anzahl der Empfänger und einem Aufklapp-Button

#### Scenario: Warnung bei Empfängern ohne E-Mail
- **WHEN** Empfänger ohne E-Mail-Adresse in der Auswahl sind
- **THEN** zeigt die Zusammenfassung eine Warnung mit der Anzahl der Empfänger ohne E-Mail

#### Scenario: Aufgeklappte Detail-Liste
- **WHEN** der Benutzer die Empfänger-Liste aufklappt
- **THEN** erscheint eine scrollbare Liste (max-h-60) mit Mitgliedsnummer, Name, E-Mail-Adresse und Entfernen-Button pro Zeile

#### Scenario: Einzelnen Empfänger entfernen
- **WHEN** der Benutzer den Entfernen-Button bei einem Empfänger klickt
- **THEN** wird der Empfänger aus der Liste entfernt und die Zusammenfassung aktualisiert

#### Scenario: Leere Empfänger-Liste
- **WHEN** alle Empfänger entfernt wurden
- **THEN** verschwindet die Empfänger-Darstellung und der Senden-Button wird deaktiviert

### Requirement: Vorauswahl aus GlobalSignal übernehmen
Die Mail-Seite MUSS beim Laden prüfen, ob `SELECTED_MEMBER_IDS` Einträge enthält, und diese als initiale Empfänger übernehmen. Nach der Übernahme MUSS das GlobalSignal geleert werden.

#### Scenario: Mail-Seite mit vorausgewählten Empfängern
- **WHEN** die Mail-Seite geladen wird und `SELECTED_MEMBER_IDS` nicht leer ist
- **THEN** werden die IDs als initiale `selected_member_ids` übernommen
- **AND** `SELECTED_MEMBER_IDS` wird auf leer gesetzt

#### Scenario: Mail-Seite ohne Vorauswahl
- **WHEN** die Mail-Seite geladen wird und `SELECTED_MEMBER_IDS` leer ist
- **THEN** startet die Mail-Seite ohne vorausgewählte Empfänger (bestehendes Verhalten)

#### Scenario: Manuelle Suche bleibt verfügbar
- **WHEN** Empfänger aus der Mitgliederliste vorausgewählt wurden
- **THEN** kann der Benutzer weiterhin über die Autocomplete-Suche weitere Empfänger hinzufügen

### Requirement: Predefined mail templates
The predefined mail templates (Formell/Informell) SHALL contain only the salutation/greeting line without a closing formula. The closing formula (e.g. "Mit freundlichen Grüßen", "Viele Grüße") SHALL be provided by the mail footer instead.

#### Scenario: Formal template content
- **WHEN** user selects the "Formell" template
- **THEN** the template contains the formal salutation (e.g. "Sehr geehrter Herr...") followed by empty lines for the body, but no closing formula

#### Scenario: Informal template content
- **WHEN** user selects the "Informell" template
- **THEN** the template contains the informal greeting (e.g. "Lieber/Liebe...") followed by empty lines for the body, but no closing formula

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

### Requirement: Mail compose form
The mail compose form SHALL include a template selector dropdown between the subject field and the body field. The dropdown SHALL offer predefined templates that pre-fill the body with Jinja template syntax for personalized salutations.

#### Scenario: Compose form layout
- **WHEN** the mail compose form is displayed
- **THEN** a template dropdown appears above the body textarea, after the subject field

#### Scenario: Template pre-fills body
- **WHEN** a user selects a template from the dropdown
- **THEN** the body textarea is filled with the template content including Jinja variable placeholders

### Requirement: Bulk mail job supports static document attachments
The `POST /mail/send-bulk` endpoint SHALL accept an optional `static_document_ids` field (array of UUID strings) in the `SendBulkMailRequest`. When non-empty, the system validates that each id refers to an existing, non-deleted static document. The resulting `mail_job` SHALL be linked to the chosen static documents via a join table so that the worker can attach the same files to every outgoing recipient email. Static documents are orthogonal to the existing per-recipient `attachment_ids` (which remain member-bound and restricted to single-recipient jobs); both mechanisms MAY be used together.

#### Scenario: Bulk send with static document attachments
- **WHEN** `POST /mail/send-bulk` is called with multiple recipients and `static_document_ids` referencing two existing static documents
- **THEN** the system creates a mail job, persists a join row per (job, document) pair, and every recipient receives an email with both static documents attached as multipart parts

#### Scenario: Unknown static document id is rejected
- **WHEN** `POST /mail/send-bulk` is called with a `static_document_ids` entry that does not exist or is soft-deleted
- **THEN** the system returns a 400/404 error before creating the mail job

#### Scenario: Empty static_document_ids preserves legacy behaviour
- **WHEN** `POST /mail/send-bulk` is called without `static_document_ids` or with an empty array
- **THEN** the system behaves exactly as before and no join rows are created

### Requirement: Mail job to static document join
The system SHALL maintain a `mail_job_static_attachments` join table with rows `(mail_job_id, static_document_id)`. Rows are inserted when a mail job is created with static documents and SHALL be readable by the mail worker to load the files for sending.

#### Scenario: Join rows inserted on job creation
- **WHEN** a mail job is created with two static document ids
- **THEN** two join rows with the new `mail_job_id` exist

#### Scenario: Worker loads static attachments per job
- **WHEN** the mail worker processes a recipient belonging to a job with static attachments
- **THEN** the worker reads the join rows, loads the file bytes from the configured storage directory, and attaches them to the outgoing multipart message

### Requirement: Load template for mail composition
The system SHALL allow clients to retrieve a saved mail template by ID via `GET /api/mail/templates/{id}` and use its subject and body fields to pre-fill the mail sending form. The mail sending endpoints (`/api/mail/send`, `/api/mail/send-bulk`) remain unchanged — they continue to accept subject and body directly.

#### Scenario: Client loads template before sending
- **WHEN** a client fetches `GET /api/mail/templates/{id}` and then calls `POST /api/mail/send-bulk` with the returned subject and body
- **THEN** the system sends the mail using the template content, exactly as if the subject and body were typed manually
