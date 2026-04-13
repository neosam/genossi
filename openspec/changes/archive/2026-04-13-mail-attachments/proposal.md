## Why

When sending emails to individual members (e.g., join confirmations, declarations), board members currently need to send the document separately from the email. The system already stores per-member documents, but there is no way to attach them to outgoing emails. This creates a disconnected workflow where documents and emails are managed independently.

## What Changes

- Add support for attaching member documents to email recipients
- New `mail_recipient_attachments` join table linking recipients to member documents
- REST API accepts optional `attachment_ids` when sending mail
- Backend validates that each attachment belongs to the recipient's member
- Mail worker builds multipart MIME messages with file attachments via lettre
- Frontend shows a document multiselect on the mail compose page when exactly one member is selected

## Capabilities

### New Capabilities
- `mail-attachments`: Attach existing member documents to outgoing emails. Covers the join table, validation, multipart mail building, and frontend document selection.

### Modified Capabilities
- `mail-sending`: The send-bulk endpoint accepts optional `attachment_ids` per recipient. The mail worker produces multipart messages when attachments are present.

## Impact

- **Database**: New `mail_recipient_attachments` migration table
- **Backend**: `genossi_mail` crate (service, worker, rest, dao) gains attachment awareness
- **Frontend**: `mail_page.rs` gains document multiselect; `api.rs` gains attachment fields
- **Dependencies**: No new crates — lettre already supports multipart via the `builder` feature
