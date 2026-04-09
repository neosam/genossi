## ADDED Requirements

### Requirement: InboundMail data model

The system SHALL store inbound mails with the following fields:

- `id` (UUID, system-generated, primary key)
- `created` (DateTime, system-generated)
- `version` (UUID, for optimistic locking)
- `uid_validity` (INTEGER, required): IMAP `UIDVALIDITY` value of the mailbox at fetch time
- `imap_uid` (INTEGER, required): IMAP UID of the message within that validity
- `from_address` (TEXT, required): parsed sender email address
- `subject` (TEXT, required): decoded subject; empty string if none
- `received_at` (DateTime, required): message `Date` header, or fetch time if missing
- `body_text` (TEXT, required): extracted plain-text body, empty string if none
- `has_attachments` (BOOLEAN, required)
- `has_html_body` (BOOLEAN, required)
- `raw_html_body` (TEXT, optional): raw HTML body if any, stored unrendered
- `in_reply_to` (TEXT, optional): value of the `In-Reply-To` header, normalized without angle brackets
- `status` (TEXT, required): one of `new`, `assigned`, `archived`, `ignored`
- `assigned_member_id` (UUID, optional): FK to the `members` table

The combination `(uid_validity, imap_uid)` SHALL be unique.

#### Scenario: Store a newly fetched mail

- **WHEN** the inbox worker fetches an unseen mail from the IMAP server
- **THEN** it inserts a new `InboundMail` row with `status = new`, `assigned_member_id = NULL`, and all header/body fields populated

#### Scenario: Deduplicate on re-fetch

- **WHEN** the inbox worker fetches a mail whose `(uid_validity, imap_uid)` already exists in the database
- **THEN** the worker skips insertion and does not duplicate the row

### Requirement: IMAP polling worker

The system SHALL run a background worker that periodically connects to the shared IMAP mailbox configured in the config store and fetches new messages. The poll interval SHALL be read from config key `imap_poll_interval_seconds` (default 300). The worker SHALL NOT modify server-side message flags or move messages during polling.

#### Scenario: Successful poll with new mails

- **WHEN** the poll interval elapses and the IMAP server has messages with UIDs greater than the highest known `imap_uid` for the current `uid_validity`
- **THEN** the worker fetches those messages, parses them, and stores each as a new `InboundMail` row without changing any IMAP flags

#### Scenario: UIDVALIDITY change

- **WHEN** the server reports a new `UIDVALIDITY` value that differs from the one previously recorded
- **THEN** the worker treats the new validity as a distinct namespace and fetches from UID 1 onward within that namespace

#### Scenario: IMAP connection failure

- **WHEN** the worker fails to connect or authenticate
- **THEN** it logs the error and retries on the next poll interval without crashing

#### Scenario: Missing IMAP configuration

- **WHEN** required IMAP config keys are missing or empty
- **THEN** the worker logs a warning and skips the poll cycle

### Requirement: Body and attachment extraction

The system SHALL extract the plain-text body from each inbound mail, preferring the `text/plain` MIME part over `text/html`. If only an HTML body is present, the system SHALL store the raw HTML in `raw_html_body` and set `has_html_body = true`, but SHALL NOT render or sanitize it in the MVP. The system SHALL count attachments and set `has_attachments` accordingly, but SHALL NOT store attachment contents.

#### Scenario: Plain-text body available

- **WHEN** an incoming mail contains a `text/plain` part
- **THEN** `body_text` contains the decoded plain-text content and `has_html_body` reflects whether an HTML part was also present

#### Scenario: Only HTML body

- **WHEN** an incoming mail contains only an HTML body
- **THEN** `body_text` is an empty string, `has_html_body = true`, and `raw_html_body` contains the raw HTML

#### Scenario: Mail with attachments

- **WHEN** an incoming mail contains one or more attachments
- **THEN** `has_attachments = true` and the attachment contents are discarded

### Requirement: List and view inbox

The system SHALL expose REST endpoints to list inbound mails ordered by `received_at` descending and to fetch a single inbound mail by ID. Each listed item SHALL include sender, subject, received date, status, and assigned member (id and display name) if any.

#### Scenario: List inbox

- **WHEN** `GET /api/inbox` is called
- **THEN** the response contains all `InboundMail` rows except those with status `ignored`, ordered by `received_at` descending, each annotated with its assigned member's name if assigned

#### Scenario: Get mail detail

- **WHEN** `GET /api/inbox/{id}` is called with a valid ID
- **THEN** the response returns the full `InboundMail` including `body_text` and assignment

### Requirement: Assign inbound mail to member

The system SHALL allow users to associate an inbound mail with a member and to remove that association. Upon assignment, the mail's `status` SHALL transition from `new` to `assigned`. Upon unassignment, the status SHALL return to `new` unless it is already `archived` or `ignored`.

#### Scenario: Assign to member

- **WHEN** `POST /api/inbox/{id}/assign` is called with a valid member ID
- **THEN** `assigned_member_id` is set, `status` becomes `assigned`, and the response reflects the update

#### Scenario: Unassign

- **WHEN** `POST /api/inbox/{id}/unassign` is called
- **THEN** `assigned_member_id` is cleared and `status` returns to `new` (if previously `assigned`)

#### Scenario: Sender-based member suggestion

- **WHEN** the frontend opens the assignment UI for an unassigned inbound mail
- **THEN** it offers the member whose email address matches the mail's `from_address` as the preselected suggestion, if any

### Requirement: Mark read mirrors to IMAP

The system SHALL, on explicit user action to mark an inbound mail as read in genossi, set the IMAP `\Seen` flag for the corresponding message on the server. Polling SHALL NOT set this flag implicitly.

#### Scenario: User opens mail in genossi

- **WHEN** `POST /api/inbox/{id}/mark-read` is called
- **THEN** the system connects to IMAP and sets the `\Seen` flag on the message identified by the stored `uid_validity` and `imap_uid`

#### Scenario: IMAP mark-read fails

- **WHEN** the IMAP `\Seen` flag update fails (network, UID no longer valid, etc.)
- **THEN** the REST call returns an error and the local state is not changed

### Requirement: Archive mirrors to IMAP

The system SHALL, on explicit user action to archive an inbound mail, move the message on the IMAP server into the folder configured by `imap_archive_mailbox`, and set the local `status` to `archived`.

#### Scenario: User archives mail

- **WHEN** `POST /api/inbox/{id}/archive` is called
- **THEN** the system moves the message to the configured archive folder via IMAP and updates `status = archived`

#### Scenario: Archive folder missing

- **WHEN** the configured archive folder does not exist on the server
- **THEN** the REST call returns an error and the local `status` is not changed

### Requirement: Ignore inbound mail

The system SHALL allow users to mark an inbound mail as `ignored`, hiding it from the default inbox list without touching the IMAP server.

#### Scenario: Ignore

- **WHEN** `POST /api/inbox/{id}/ignore` is called
- **THEN** `status` becomes `ignored` and the mail no longer appears in `GET /api/inbox` responses

### Requirement: Frontend inbox page

The frontend SHALL provide an `/inbox` page listing inbound mails with sender, subject, received date, and an assignment label showing either the assigned member's name or "nicht zugeordnet". Each entry SHALL open a detail view showing the plain-text body, an indicator if attachments are present, and an assignment control.

#### Scenario: View inbox

- **WHEN** the user navigates to `/inbox`
- **THEN** the page displays all non-ignored inbound mails with assignment labels

#### Scenario: Assign from detail view

- **WHEN** the user selects a member in the detail view and confirms
- **THEN** the frontend calls the assign endpoint and updates the label to the member's name
