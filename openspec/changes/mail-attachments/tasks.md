## 1. Database

- [x] 1.1 Create SQLite migration for `mail_recipient_attachments` table (recipient_id BLOB, document_id BLOB, composite PK, FKs)

## 2. DAO Layer

- [x] 2.1 Add `MailRecipientAttachmentEntity` struct and DAO trait in `genossi_mail/src/dao.rs`
- [x] 2.2 Implement SQLite DAO for `mail_recipient_attachments` (create, find_by_recipient_id) in `genossi_mail/src/dao_sqlite.rs`

## 3. Service Layer

- [x] 3.1 Extend `MailService::create_job` to accept optional `attachment_ids` and validate: single recipient only, documents exist, documents belong to recipient's member
- [x] 3.2 Store attachment records in `mail_recipient_attachments` after creating the recipient
- [x] 3.3 Add unit tests for attachment validation (ownership check, single-recipient constraint, missing/deleted documents)

## 4. Mail Worker

- [x] 4.1 Load attachments for each recipient in the worker (query `mail_recipient_attachments` + `MemberDocumentDao`)
- [x] 4.2 Load file bytes via `DocumentStorage` and build `lettre` multipart message with attachments
- [x] 4.3 Keep plain text sending path for recipients without attachments
- [x] 4.4 Handle missing file on disk — mark recipient as failed with descriptive error

## 5. REST API

- [x] 5.1 Add `attachment_ids: Option<Vec<String>>` to `SendBulkMailRequest` in `genossi_mail/src/rest.rs`
- [x] 5.2 Pass attachment_ids through to service layer in the send-bulk handler
- [x] 5.3 Return attachment info in `MailJobDetailTO` recipients (optional: list attached document names)

## 6. Frontend

- [x] 6.1 Add `attachment_ids` field to the `SendBulkMailRequest` in `genossi-frontend/src/api.rs`
- [x] 6.2 Add document multiselect component to `mail_page.rs` — visible only when exactly one member is selected
- [x] 6.3 Fetch member documents via existing `get_member_documents()` when single recipient is selected
- [x] 6.4 Clear attachment selection when recipient selection changes
- [x] 6.5 Pass selected document IDs to `send_bulk_mail()` API call

## 7. Integration Tests

- [x] 7.1 E2E test: send mail with attachment — verify multipart message is accepted
- [x] 7.2 E2E test: reject attachment for wrong member
- [x] 7.3 E2E test: reject attachments with multiple recipients
- [x] 7.4 E2E test: send mail without attachment — verify unchanged behavior
