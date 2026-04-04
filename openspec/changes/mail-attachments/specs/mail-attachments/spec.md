## ADDED Requirements

### Requirement: Mail recipient attachment data model
The system SHALL store mail recipient attachments in a `mail_recipient_attachments` table with the following fields:
- `recipient_id` (BLOB, NOT NULL, FK to `mail_recipients.id`)
- `document_id` (BLOB, NOT NULL, FK to `member_document.id`)

The composite `(recipient_id, document_id)` SHALL be the primary key.

#### Scenario: Attachment record created
- **WHEN** a mail is sent with an attachment
- **THEN** the system stores a row in `mail_recipient_attachments` linking the recipient to the document

#### Scenario: Multiple attachments per recipient
- **WHEN** a mail is sent with two attachments
- **THEN** the system stores two rows in `mail_recipient_attachments` for that recipient

### Requirement: Attachment ownership validation
The system SHALL validate that every attachment belongs to the recipient's member. For each `document_id` in the request, the system SHALL load the `MemberDocument` and verify that its `member_id` matches the recipient's `member_id`. If any attachment fails this check, the request SHALL be rejected with a 400 error.

#### Scenario: Attachment belongs to recipient's member
- **WHEN** a mail is created with `attachment_ids: ["doc-1"]` and `doc-1.member_id` equals the recipient's `member_id`
- **THEN** the system accepts the request and creates the attachment record

#### Scenario: Attachment belongs to a different member
- **WHEN** a mail is created with `attachment_ids: ["doc-1"]` and `doc-1.member_id` does NOT equal the recipient's `member_id`
- **THEN** the system rejects the request with a 400 error indicating the document does not belong to the recipient's member

#### Scenario: Attachment does not exist
- **WHEN** a mail is created with `attachment_ids: ["nonexistent-id"]`
- **THEN** the system rejects the request with a 404 error

#### Scenario: Attachment is soft-deleted
- **WHEN** a mail is created with `attachment_ids: ["doc-1"]` and `doc-1` has a non-null `deleted` timestamp
- **THEN** the system rejects the request with a 404 error

### Requirement: Attachments only for single-recipient sends
The system SHALL only allow `attachment_ids` when the mail has exactly one recipient. If `attachment_ids` is non-empty and `to_addresses` contains more than one entry, the system SHALL reject the request with a 400 error.

#### Scenario: Single recipient with attachments
- **WHEN** a mail is sent to one recipient with `attachment_ids: ["doc-1"]`
- **THEN** the system accepts the request

#### Scenario: Multiple recipients with attachments
- **WHEN** a mail is sent to three recipients with `attachment_ids: ["doc-1"]`
- **THEN** the system rejects the request with a 400 error indicating attachments are only supported for single-recipient sends

#### Scenario: Multiple recipients without attachments
- **WHEN** a mail is sent to three recipients with no `attachment_ids`
- **THEN** the system accepts the request (existing behavior)

### Requirement: Multipart mail with attachments
The mail worker SHALL build a multipart MIME message when a recipient has attachments. For each attachment, the worker SHALL load the file bytes from `DocumentStorage`, and attach them using the document's `file_name` and `mime_type`. When no attachments are present, the worker SHALL send a plain text message as before.

#### Scenario: Send mail with one attachment
- **WHEN** the worker processes a recipient with one attachment (e.g., `report.pdf`, `application/pdf`)
- **THEN** the worker builds a multipart message with the text body and one file attachment named `report.pdf` with content type `application/pdf`

#### Scenario: Send mail with multiple attachments
- **WHEN** the worker processes a recipient with two attachments
- **THEN** the worker builds a multipart message with the text body and both file attachments

#### Scenario: Send mail without attachments
- **WHEN** the worker processes a recipient with no attachments
- **THEN** the worker sends a plain text message (unchanged behavior)

#### Scenario: Attachment file missing from filesystem
- **WHEN** the worker processes a recipient with an attachment but the file is not found on disk
- **THEN** the worker marks the recipient as `failed` with an error message indicating the file is missing

### Requirement: Frontend attachment selector
The mail compose page SHALL display a document multiselect when exactly one member is selected as recipient. The multiselect SHALL list all active (non-deleted) documents of that member, displaying the document type and file name. When the recipient selection changes, the attachment selection SHALL be cleared.

#### Scenario: One recipient selected — show selector
- **WHEN** exactly one member is selected on the mail compose page
- **THEN** the system fetches the member's documents and displays a multiselect with checkboxes

#### Scenario: Multiple recipients selected — hide selector
- **WHEN** more than one member is selected on the mail compose page
- **THEN** the attachment selector is hidden and any previous selection is cleared

#### Scenario: No recipients selected — hide selector
- **WHEN** no members are selected on the mail compose page
- **THEN** the attachment selector is not displayed

#### Scenario: Recipient changes — clear selection
- **WHEN** the user changes the selected recipient from member A to member B
- **THEN** the previously selected attachments are cleared and member B's documents are loaded

#### Scenario: Member has no documents
- **WHEN** exactly one member is selected but that member has no documents
- **THEN** the attachment selector is displayed but empty, with a hint that no documents are available
