## Context

genossi already has a complete outbound mail pipeline (`genossi_mail` crate) with SMTP config in the config store, a `MailJob`/`MailRecipient` model, a background worker, and a typed REST layer. There is no inbound side: replies to the shared member mailbox can only be read via an external client. The mailbox is **shared** — other people and tools (e.g. Thunderbird) access the same IMAP account concurrently, so genossi must behave as a polite, read-mostly guest. This change adds a read side that surfaces inbound mails inside genossi and lets users manually associate them with members.

## Goals / Non-Goals

**Goals:**
- Provide a read-only inbox view of the shared mailbox inside genossi with a manual member-assignment workflow.
- Avoid interfering with other IMAP clients on the same mailbox by default.
- Reuse existing architectural patterns: ConfigStore for credentials, `genossi_mail` crate for domain code, DAO/Service/REST layering, background worker analogous to the send worker.
- Lay groundwork for later automatic threading (`in_reply_to` captured but unused in MVP).

**Non-Goals:**
- Automatic threading via `In-Reply-To` (future change; `store-outbound-message-id` is the prerequisite).
- Per-member communication history on the member detail page.
- Downloading, storing, or rendering attachments.
- Rendering HTML bodies.
- Supporting multiple mailboxes simultaneously.
- Sending replies from within genossi.
- A separate credentials vault; IMAP creds live in the same ConfigStore as SMTP creds.

## Decisions

### Polling over IMAP IDLE

IMAP IDLE would provide near-real-time updates but requires a long-lived TCP connection, server-side support, and more failure handling. A periodic poll (default 300s, configurable) is simpler, resilient to disconnects, and more than enough for member-reply latency expectations. Alternatives considered: IDLE (rejected: complexity), webhook-style delivery (rejected: not available for generic IMAP).

### Dedup via `(UIDVALIDITY, UID)`

IMAP guarantees UIDs are stable within a `UIDVALIDITY` epoch. Storing both values and enforcing a UNIQUE constraint makes re-fetches idempotent and correctly handles mailbox recreation (UIDVALIDITY change → fresh namespace). Alternatives considered: dedup by `Message-ID` header (rejected: not all mails have one, and the header can be spoofed or duplicated).

### Read-mostly polling, write only on explicit user action

The shared-mailbox constraint dictates that polling must not alter server state. `\Seen` is set only when the user explicitly opens a mail in genossi; archive is only on explicit user action. This keeps Thunderbird and friends functional and predictable. Alternatives considered: always mark read on fetch (rejected: breaks parallel clients).

### Store raw HTML, defer rendering

`mail-parser` gives us MIME decoding essentially for free. Storing `raw_html_body` without rendering is cheap (few hundred KB per mail at most) and unblocks a later change to add safe HTML rendering without a second parse. We explicitly do not render HTML in the MVP to avoid XSS considerations.

### Do not store attachment contents

Attachments can be large and frequently contain sensitive material (PII, contracts). Storing only a boolean indicator is sufficient for the MVP; a later change can introduce per-attachment download-on-demand straight from IMAP.

### IMAP operations on user action run synchronously in the request handler

Mark-read and archive both touch the IMAP server. In the MVP these run inline in the REST handler, returning an error to the user if IMAP is unreachable. This keeps the state model simple (no "pending mirror" queue). Alternatives considered: enqueue IMAP side-effects into a mirror worker for eventual consistency (rejected: more moving parts without clear benefit at this scale).

### Library choice: `async-imap` + `mail-parser`

`async-imap` is the maintained async IMAP client for the Rust ecosystem and pairs naturally with the tokio runtime already in use. `mail-parser` handles decoding, MIME traversal, and header normalization without a separate C dependency. Alternatives considered: writing a custom client (rejected: scope), synchronous `imap` crate (rejected: would block the async runtime).

### Crate boundaries

The inbox code lives in `genossi_mail` alongside the send worker. Symmetry is valuable: future features (auto-threading) will need both sides, and splitting crates now would force premature abstraction.

## Risks / Trade-offs

- **[Risk]** Polling misses a mail if UIDVALIDITY changes between polls and the worker crashes mid-fetch → **Mitigation**: fetch all UIDs greater than the highest stored UID for the current validity on each poll; idempotent upsert on `(uid_validity, imap_uid)`.
- **[Risk]** IMAP credentials in the ConfigStore could be dumped via the dump endpoint → **Mitigation**: follow whatever protection already applies to SMTP `secret`-typed entries; do not add new exposure.
- **[Risk]** Poll picks up non-member mails (spam, automated notifications) → **Mitigation**: accept in MVP; the `ignored` status provides manual cleanup. Auto-filtering is a later concern.
- **[Risk]** `\Seen` flag setting races with another client reading the same mail → **Mitigation**: acceptable — both clients converge to "seen".
- **[Risk]** Inbox table grows unbounded → **Mitigation**: archive/ignore semantics let users curate; a retention policy can be added later if needed.
- **[Trade-off]** Synchronous IMAP operations in REST handlers can make the request hang on a slow server → acceptable in MVP, revisit if users complain.

## Migration Plan

1. Ship migration creating the `inbound_mails` table with the UNIQUE constraint.
2. Deploy backend with the inbox worker disabled until IMAP config is present (worker skips cycles with warning).
3. Operator sets IMAP config via the existing config UI.
4. Frontend `/inbox` page and nav entry become available; no data migration required.
5. Rollback: stop the worker, hide the nav entry; the table and rows remain inert. The migration does not drop anything.

## Open Questions

- None blocking; HTML rendering, attachment access, and auto-threading are explicit follow-ups.
