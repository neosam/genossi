## Why

genossi can send mails to members via SMTP, but the mailbox that members reply to is a black hole from the app's perspective. Users must switch to an external mail client to see answers, which fragments the workflow and makes it hard to keep member communication visible. A read-side view of the shared mailbox inside genossi — with the ability to manually associate incoming mails to members — closes this gap with minimal complexity.

## What Changes

- Add an IMAP-based inbox worker that periodically polls the shared member mailbox, downloads new mails, and stores them as `InboundMail` rows. The worker is **read-only by default** and does not modify server-side flags during polling.
- Add a new `InboundMail` entity with: `id`, `created`, `version`, `uid_validity`, `imap_uid`, `from_address`, `subject`, `received_at`, `body_text`, `has_attachments`, `has_html_body`, `raw_html_body` (optional, unrendered), `status` (`new`/`assigned`/`archived`/`ignored`), `assigned_member_id` (optional), `in_reply_to` (optional, captured for future auto-threading, not used in MVP).
- Add an `InboxService` with operations: list inbox, get detail, assign to member, unassign, mark read, archive, ignore.
- On explicit user actions in genossi, mirror to IMAP: "open/mark read" sets the `\Seen` flag on the server; "archive" moves the message to a configurable archive folder.
- Add REST endpoints under `/api/inbox` for list/get/assign/mark-read/archive/ignore.
- Add IMAP configuration keys to the config store: `imap_host`, `imap_port`, `imap_user`, `imap_pass`, `imap_tls`, `imap_mailbox` (default `INBOX`), `imap_archive_mailbox`, `imap_poll_interval_seconds`.
- Add a new frontend page `/inbox` showing a list of inbound mails with sender, subject, received date, assignment label ("Max Mustermann" or "nicht zugeordnet"), and a detail view with text body, "hat Anhänge"-indicator, and a member-assignment dropdown (prefilled by absender-email match).
- Deduplicate stored mails by `(uid_validity, imap_uid)`.

## Capabilities

### New Capabilities

- `member-inbox`: Polling, storing, viewing, and manually associating inbound mails from the shared member mailbox to genossi members.

### Modified Capabilities

- `config-store`: Add new IMAP configuration keys.

## Impact

- **Code**:
  - `genossi_mail/`: new `inbox_worker.rs`, new `InboundMail` DAO + SQLite impl, new `InboxService` + impl, new `/api/inbox` REST routes.
  - `genossi_bin/src/lib.rs`: wire and start the new inbox worker.
  - `genossi-frontend/`: new `inbox_page.rs`, routes, nav entry, REST client bindings, i18n strings.
- **Database**: New migration creating `inbound_mails` table with UNIQUE constraint on `(uid_validity, imap_uid)`.
- **Dependencies**: Add `async-imap` (or similar) and `mail-parser` crates to `genossi_mail`.
- **Configuration**: New IMAP config keys in the config store; no env var plumbing required.
- **Security**: IMAP credentials stored in the same ConfigStore used for SMTP credentials; follow the same handling patterns.
- **Out of scope** (deferred to later changes): automatic threading via `In-Reply-To`, per-member communication history on the member detail page, attachment download/rendering, HTML rendering, multiple mailboxes, sending replies from genossi.
