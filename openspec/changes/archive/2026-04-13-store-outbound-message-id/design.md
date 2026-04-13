## Context

Outgoing mails flow through `genossi_mail/src/worker.rs`, which builds a `lettre::Message` and sends it via an async SMTP transport. `lettre` auto-generates a `Message-ID` header on build, but the current worker does not read it back; once the mail is sent, the ID is lost. A dedicated `mail_recipients` table tracks per-recipient delivery state. Capturing the Message-ID is a prerequisite for the upcoming `member-inbox` feature, which will correlate inbound replies via `In-Reply-To` headers.

## Goals / Non-Goals

**Goals:**
- Persist the outbound `Message-ID` on `mail_recipients` for every successfully delivered recipient.
- Keep the change minimally invasive — no API, service, or UI surface changes.
- Preserve backward compatibility with existing rows (no data backfill required).

**Non-Goals:**
- Threading or matching of inbound replies (handled by `member-inbox`).
- Exposing `message_id` via REST or frontend.
- Generating or overriding the Message-ID ourselves; we let `lettre` produce it.

## Decisions

### Store on `mail_recipients`, not `mail_jobs`

A mail job fans out to N recipients, and `lettre` generates a fresh Message-ID per `Message`, one per recipient in our worker loop. Storing on the recipient is therefore the only lossless option.

### Read via `message.headers().get_first(...)` after build

`lettre::Message::headers()` exposes the finalized headers after building. We read the `Message-ID` header immediately after constructing the `Message` and before `transport.send(email)`, so the captured value is guaranteed to match what is transmitted.

Alternative considered: generate our own Message-ID via a `lettre::message::MessageIdHeader` before build. Rejected — unnecessary complexity when `lettre` already provides a valid value and we only need to observe it.

### Nullable column, no backfill

Existing rows legitimately have no Message-ID. Making the column nullable avoids a migration that would need to fabricate values and keeps the migration a simple `ALTER TABLE ADD COLUMN`.

### Strip angle brackets for storage

RFC 5322 Message-IDs are wrapped in `<...>` in headers. We store the value **without** surrounding angle brackets to simplify later comparison against `In-Reply-To` values (which are also normalized). This decision is local to the DAO layer.

## Risks / Trade-offs

- **[Risk]** `lettre` could in theory omit a Message-ID for some build path → **Mitigation**: treat capture as best-effort; log a warning and store `NULL` if the header is missing. Do not fail the send.
- **[Risk]** Angle-bracket normalization could diverge from how future inbound parsing normalizes `In-Reply-To` → **Mitigation**: centralize normalization in a small helper used by both sides once `member-inbox` lands.
- **[Trade-off]** Reading headers after build means a tiny extra allocation per mail. Negligible compared to SMTP I/O.

## Migration Plan

1. Ship migration `ALTER TABLE mail_recipients ADD COLUMN message_id TEXT`.
2. Deploy updated worker that populates the column.
3. No rollback risk: column is nullable and unused outside the worker.
