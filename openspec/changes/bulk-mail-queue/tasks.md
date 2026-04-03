## 1. Database Migration

- [x] 1.1 Create migration to drop `sent_mails` table
- [x] 1.2 Create migration to create `mail_jobs` table (id, created, deleted, version, subject, body, status, total_count, sent_count, failed_count)
- [x] 1.3 Create migration to create `mail_recipients` table (id, created, deleted, version, mail_job_id, to_address, member_id, status, error, sent_at)

## 2. DAO Layer

- [x] 2.1 Define `MailJob` and `MailRecipient` model structs in `genossi_mail`
- [x] 2.2 Define `MailJobDao` trait with methods: `create`, `find_by_id`, `all`, `update`
- [x] 2.3 Define `MailRecipientDao` trait with methods: `create`, `find_by_job_id`, `next_pending`, `update`
- [x] 2.4 Implement `MailJobDao` for SQLite (`MailJobDaoSqlite`)
- [x] 2.5 Implement `MailRecipientDao` for SQLite (`MailRecipientDaoSqlite`)
- [x] 2.6 Remove `SentMailDao` trait and `SentMailDaoSqlite` implementation
- [x] 2.7 Write unit tests for MailJob SQLite DAO
- [x] 2.8 Write unit tests for MailRecipient SQLite DAO

## 3. Service Layer

- [x] 3.1 Update `MailService` trait: replace `send_mail`/`send_mails`/`get_sent_mails` with `create_job`, `get_jobs`, `get_job_with_recipients`, `retry_job`
- [x] 3.2 Implement `create_job` — creates MailJob + MailRecipients in one transaction
- [x] 3.3 Implement `get_jobs` — returns all jobs ordered by created DESC
- [x] 3.4 Implement `get_job_with_recipients` — returns job with all its recipients
- [x] 3.5 Implement `retry_job` — resets failed recipients to pending, updates job counters
- [x] 3.6 Keep `send_test_mail` as synchronous direct send
- [x] 3.7 Write unit tests for all new service methods (using mockall)

## 4. Background Worker

- [x] 4.1 Create `worker.rs` module with `start_mail_worker` function
- [x] 4.2 Implement worker loop: poll for next pending recipient, send, update status, sleep
- [x] 4.3 Implement job completion detection (update job status when all recipients processed)
- [x] 4.4 Read `mail_send_interval_seconds` from ConfigService per iteration (default: 36)
- [x] 4.5 Handle SMTP errors gracefully — mark recipient as failed, continue
- [x] 4.6 Write unit tests for worker logic (mocked DAO and ConfigService)

## 5. REST Endpoints

- [x] 5.1 Update `POST /api/mail/send` — create job with 1 recipient, return 202
- [x] 5.2 Update `POST /api/mail/send-bulk` — create job with N recipients, return 202
- [x] 5.3 Add `GET /api/mail/jobs` — list all jobs with counts
- [x] 5.4 Add `GET /api/mail/jobs/{id}` — job detail with recipients
- [x] 5.5 Add `POST /api/mail/jobs/{id}/retry` — retry failed recipients
- [x] 5.6 Remove old `GET /api/mail/sent` endpoint
- [x] 5.7 Update REST types (MailJobTO, MailRecipientTO, request types)
- [x] 5.8 Update OpenAPI/Utoipa schema annotations

## 6. Binary Integration

- [x] 6.1 Update `RestStateImpl` to wire new DAOs (MailJobDao, MailRecipientDao)
- [x] 6.2 Start background worker in `main.rs` after server setup
- [x] 6.3 Pass required services (ConfigService, MailJobDao, MailRecipientDao, SMTP logic) to worker

## 7. Frontend

- [x] 7.1 Update mail page to show job list with progress (sent_count/total_count)
- [x] 7.2 Add expandable job detail view showing individual recipients
- [x] 7.3 Add retry button for jobs with failed recipients
- [x] 7.4 Update bulk send form to use new endpoint response (202 + job)
- [x] 7.5 Add i18n keys for new UI elements (MailJobs, Progress, Retry, etc.)

## 8. E2E Tests

- [x] 8.1 Write e2e test: create bulk mail job and verify job + recipients created
- [x] 8.2 Write e2e test: verify worker processes pending recipients
- [x] 8.3 Write e2e test: retry failed recipients
- [x] 8.4 Write e2e test: test mail still works synchronously
