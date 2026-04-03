## ADDED Requirements

### Requirement: MailJob data model
The system SHALL store mail jobs with the following fields:
- `id` (UUID, system-generated, primary key)
- `created` (DateTime, system-generated)
- `deleted` (Optional DateTime, for soft delete)
- `version` (UUID, for optimistic locking)
- `subject` (TEXT, required)
- `body` (TEXT, required)
- `status` (TEXT, required): one of `pending`, `running`, `done`, `failed`
- `total_count` (INTEGER, required): total number of recipients
- `sent_count` (INTEGER, required, default 0): number of successfully sent mails
- `failed_count` (INTEGER, required, default 0): number of failed mails

#### Scenario: Job created for bulk send
- **WHEN** a bulk mail request is submitted with 600 recipients
- **THEN** the system creates a MailJob with status `running`, total_count `600`, sent_count `0`, failed_count `0`

#### Scenario: Job completed successfully
- **WHEN** all 600 recipients have been processed (580 sent, 20 failed)
- **THEN** the system updates the MailJob to status `done`, sent_count `580`, failed_count `20`

#### Scenario: Job with all failures
- **WHEN** all recipients fail (e.g. SMTP config invalid)
- **THEN** the system updates the MailJob to status `failed`

### Requirement: MailRecipient data model
The system SHALL store mail recipients with the following fields:
- `id` (UUID, system-generated, primary key)
- `created` (DateTime, system-generated)
- `deleted` (Optional DateTime, for soft delete)
- `version` (UUID, for optimistic locking)
- `mail_job_id` (UUID, required, foreign key to mail_jobs)
- `to_address` (TEXT, required)
- `member_id` (Optional UUID, foreign key to persons)
- `status` (TEXT, required): one of `pending`, `sent`, `failed`
- `error` (Optional TEXT): SMTP error message on failure
- `sent_at` (Optional DateTime): timestamp when mail was sent

#### Scenario: Recipient pending
- **WHEN** a mail job is created
- **THEN** all recipients are stored with status `pending`, error `NULL`, sent_at `NULL`

#### Scenario: Recipient sent successfully
- **WHEN** the worker sends a mail to a recipient
- **THEN** the recipient status is updated to `sent` with `sent_at` set to the current timestamp

#### Scenario: Recipient send failed
- **WHEN** the worker fails to send a mail to a recipient
- **THEN** the recipient status is updated to `failed` with `error` set to the SMTP error message

### Requirement: Background mail worker
The system SHALL start a background worker (Tokio task) on server startup that processes the mail queue. The worker SHALL:
1. Query the database for the next pending recipient of a running job (ordered by job creation time, then recipient creation time)
2. Send the mail using existing SMTP logic
3. Update recipient status and job counters
4. Sleep for the configured interval before processing the next recipient
5. If no pending recipients are found, sleep for 5 seconds before polling again

#### Scenario: Worker processes queue
- **WHEN** there are 3 pending recipients in a running job
- **THEN** the worker sends each mail one at a time with the configured interval between sends

#### Scenario: Worker idle
- **WHEN** there are no pending recipients in any running job
- **THEN** the worker sleeps for 5 seconds before checking again

#### Scenario: Worker resumes after restart
- **WHEN** the server restarts while a job has pending recipients
- **THEN** the worker picks up remaining pending recipients and continues sending

#### Scenario: Worker encounters SMTP failure
- **WHEN** the worker fails to send a mail to a recipient
- **THEN** the worker marks the recipient as failed and continues with the next recipient

### Requirement: Configurable send interval
The system SHALL read the send interval from the config store key `mail_send_interval_seconds` (type: int, default: 36). The worker SHALL read this value before each send operation to allow runtime changes.

#### Scenario: Default interval
- **WHEN** `mail_send_interval_seconds` is not configured
- **THEN** the worker uses a default interval of 36 seconds between sends

#### Scenario: Custom interval
- **WHEN** `mail_send_interval_seconds` is set to `60`
- **THEN** the worker waits 60 seconds between each mail send

#### Scenario: Interval changed at runtime
- **WHEN** `mail_send_interval_seconds` is changed from `36` to `10` while the worker is running
- **THEN** the worker uses the new interval starting from the next send cycle

### Requirement: Job completion detection
The system SHALL mark a job as `done` when `sent_count + failed_count == total_count`. The system SHALL mark a job as `failed` only when all recipients have failed (`failed_count == total_count`).

#### Scenario: All recipients sent
- **WHEN** a job with 100 recipients has sent_count 100 and failed_count 0
- **THEN** the job status is set to `done`

#### Scenario: Mixed results
- **WHEN** a job with 100 recipients has sent_count 95 and failed_count 5
- **THEN** the job status is set to `done`

#### Scenario: All recipients failed
- **WHEN** a job with 100 recipients has sent_count 0 and failed_count 100
- **THEN** the job status is set to `failed`

### Requirement: Retry failed recipients
The system SHALL expose `POST /api/mail/jobs/{id}/retry` which resets all `failed` recipients of a job to `pending`, resets `failed_count` to 0, and sets the job status to `running`.

#### Scenario: Retry a job with failures
- **WHEN** `POST /api/mail/jobs/{id}/retry` is called for a job with 5 failed recipients
- **THEN** the 5 failed recipients are set to `pending`, the job's failed_count is set to 0, the job status is set to `running`, and the worker picks them up

#### Scenario: Retry a job with no failures
- **WHEN** `POST /api/mail/jobs/{id}/retry` is called for a job with 0 failed recipients
- **THEN** no changes are made

### Requirement: Mail job list endpoint
The system SHALL expose `GET /api/mail/jobs` returning all mail jobs ordered by creation time descending, including their counts (total, sent, failed).

#### Scenario: List mail jobs
- **WHEN** `GET /api/mail/jobs` is called
- **THEN** the system returns all mail jobs with their status and counts

### Requirement: Mail job detail endpoint
The system SHALL expose `GET /api/mail/jobs/{id}` returning a single mail job with all its recipients.

#### Scenario: Get job with recipients
- **WHEN** `GET /api/mail/jobs/{id}` is called for a job with 3 recipients
- **THEN** the system returns the job with all 3 recipient entries including their individual statuses
