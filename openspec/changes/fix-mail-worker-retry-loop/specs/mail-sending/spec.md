## MODIFIED Requirements

### Requirement: Bulk mail batching
The system SHALL process bulk mail recipients via a background worker that picks up `pending` recipients one at a time. The worker SHALL NOT automatically retry recipients that have status `failed`. Only an explicit call to `POST /api/mail/jobs/{id}/retry` SHALL reset failed recipients to `pending`.

#### Scenario: Failed recipient not retried automatically
- **WHEN** a mail recipient fails with an SMTP error
- **THEN** the recipient status is set to `failed` and the worker does NOT attempt to send to this recipient again unless explicitly retried via the retry endpoint

#### Scenario: Explicit retry resets failed recipients
- **WHEN** `POST /api/mail/jobs/{id}/retry` is called for a job with failed recipients
- **THEN** all `failed` recipients are reset to `pending` and the job status is set to `running`
