## Why

Outgoing mails are sent via `lettre`, which auto-generates an RFC 5322 `Message-ID` header, but genossi discards it. Without the Message-ID we cannot later correlate inbound replies to the original recipient (for threading), debug delivery issues, or handle bounces reliably. Capturing it now is trivial and lays the groundwork for the upcoming `member-inbox` feature (auto-threading via `In-Reply-To`).

## What Changes

- Extract the `Message-ID` from the constructed `lettre::Message` before sending and persist it on the corresponding `MailRecipient` row.
- Add a `message_id` column (nullable TEXT) to the `mail_recipients` table.
- Expose `message_id` on the DAO struct `MailRecipient`.
- Update the mail worker to write the captured `Message-ID` when transitioning a recipient to `Sent`.
- No UI or REST surface changes in this change. No breaking changes.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `mail-sending`: Add requirement that each successfully sent recipient must record the outbound `Message-ID` used during SMTP delivery.

## Impact

- **Code**: `genossi_mail/src/worker.rs` (extract and pass Message-ID), `genossi_mail/src/dao.rs` (`MailRecipient` struct), `genossi_mail/src/dao_sqlite.rs` (read/write column).
- **Database**: New migration adding `message_id TEXT` column to `mail_recipients` (nullable for backward compatibility with existing rows).
- **Dependencies**: None. `lettre` already exposes `Message::headers()` / `Message::message_id()`.
- **Backward compatibility**: Existing rows keep `message_id = NULL`; no data migration required.
