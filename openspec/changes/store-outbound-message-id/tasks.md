## 1. Database

- [x] 1.1 Add migration `migrations/sqlite/<timestamp>_add_message_id_to_mail_recipients.sql` with `ALTER TABLE mail_recipients ADD COLUMN message_id TEXT`
- [x] 1.2 Run migration locally and verify schema

## 2. DAO Layer

- [x] 2.1 Add `message_id: Option<String>` field to `MailRecipient` struct in `genossi_mail/src/dao.rs`
- [x] 2.2 Update the SQLite DAO in `genossi_mail/src/dao_sqlite.rs` to read `message_id` in all `SELECT` queries for `mail_recipients`
- [x] 2.3 Add a DAO method (or extend the existing "mark sent" update) to persist `message_id` alongside the status transition
- [x] 2.4 Refresh sqlx offline query data (`cargo sqlx prepare`)

## 3. Worker Integration

- [x] 3.1 In `genossi_mail/src/worker.rs`, after building the `lettre::Message`, read the `Message-ID` header via `message.headers().get_first(...)`
- [x] 3.2 Normalize the captured value by stripping surrounding `<>` and passing it through to the DAO on successful send
- [x] 3.3 On missing Message-ID, log a warning and proceed with `None`; do not fail the send
- [x] 3.4 On SMTP failure, leave `message_id` untouched (NULL)

## 4. Tests

- [x] 4.1 Unit test: a successfully sent recipient has `message_id = Some(...)` in the DAO after worker run
- [x] 4.2 Unit test: a failed send leaves `message_id` as `None`
- [x] 4.3 Unit test: normalization strips angle brackets
- [x] 4.4 Ensure existing mail worker tests still pass

## 5. Verification

- [x] 5.1 `cargo fmt` and `cargo clippy --all-targets`
- [x] 5.2 `cargo test -p genossi_mail`
- [x] 5.3 `openspec validate store-outbound-message-id --strict`
