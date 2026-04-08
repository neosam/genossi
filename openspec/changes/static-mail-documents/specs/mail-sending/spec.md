## ADDED Requirements

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
