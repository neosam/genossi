## Context

Genossi3 has a working mail system (`genossi_mail` crate) with background worker-based sending, and a separate member document system (`member_document` DAO/service) with filesystem storage. Both are fully operational but currently disconnected — there is no way to include a member's documents as email attachments.

The mail system uses `lettre` with the `builder` feature, which already supports `MultiPart` messages and `Attachment` construction. Member documents are stored on the filesystem at `{DOCUMENT_STORAGE_PATH}/{uuid}.{ext}` with metadata (file_name, mime_type, relative_path) tracked in SQLite.

## Goals / Non-Goals

**Goals:**
- Allow attaching one or more existing member documents to an outgoing email
- Validate that attachments belong to the recipient's member
- Build multipart MIME messages when attachments are present
- Show a document selector in the frontend when exactly one recipient is selected

**Non-Goals:**
- Uploading new files directly in the mail compose form (use existing document upload)
- Generating documents on-the-fly during mail send
- Attachment support for bulk sends to multiple recipients (future extension)
- Inline images or HTML email body

## Decisions

### 1. Join table `mail_recipient_attachments` instead of JSON array or column on recipient

**Choice:** Separate join table with `(recipient_id, document_id)` composite.

**Alternatives considered:**
- JSON array column on `mail_recipients` — simpler schema but no FK constraints, harder to query
- Single `attachment_id` column — limits to one attachment

**Rationale:** Follows the existing relational pattern in the codebase. Enables future queries like "which documents were sent by mail". No new crate dependencies. Naturally extends to bulk sends later since it's per-recipient, not per-job.

### 2. Validation at job creation time, not at send time

**Choice:** When creating the mail job, validate that every `attachment_id` exists, is not deleted, and its `member_id` matches the recipient's `member_id`. Reject the request if validation fails.

**Rationale:** Fail-fast is better UX — the user gets immediate feedback. The worker doesn't need to handle "attachment disappeared" errors. Documents are soft-deleted, not physically removed, so the file remains accessible even if the document is soft-deleted between creation and send.

### 3. Worker loads files from DocumentStorage at send time

**Choice:** The worker reads attachment bytes via `DocumentStorage::load()` when building each message, not at job creation time.

**Rationale:** Avoids storing potentially large file data in the database. Files are already on disk. Loading at send time keeps memory usage bounded per message.

### 4. Frontend shows document selector only for single-recipient sends

**Choice:** The attachment multiselect is conditionally rendered when `selected_member_ids.len() == 1`. When the selection changes, the document list is fetched or cleared.

**Rationale:** Matches the agreed scope. The existing `get_member_documents()` API call is reused. No new endpoints needed for the frontend.

### 5. `attachment_ids` on the bulk send request, applied to all recipients

**Choice:** The `SendBulkMailRequest` gains an optional `attachment_ids: Vec<String>`. Since attachments are only supported for single-recipient sends, the API validates that `attachment_ids` is empty when `to_addresses.len() > 1`.

**Rationale:** Keeps the API simple — one endpoint for both single and bulk. The constraint is enforced server-side so the frontend doesn't need special routing.

## Risks / Trade-offs

- **Large attachments → slow sends**: If a member has a 50 MB document, each mail takes longer. → The worker already has configurable send intervals; no change needed. Lettre handles streaming.
- **Document deleted between validation and send**: Soft-delete removes the DB record from active queries but the file remains on disk. → Worker loads by `relative_path` directly, which still works. If the file is physically missing (manual deletion), the recipient is marked `failed`.
- **Frontend document list stale**: If documents change between page load and send. → Acceptable risk, validation catches it server-side.
