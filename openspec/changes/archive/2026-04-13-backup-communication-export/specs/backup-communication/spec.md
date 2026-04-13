## ADDED Requirements

### Requirement: Communication data included in document backup ZIP

The system SHALL include all member-assigned email communication (inbound and outbound) as .txt files in the `/backup/documents` ZIP download, organized in a `kommunikation/` subfolder per member directory.

#### Scenario: ZIP contains outbound mails for a member
- **WHEN** a member has outbound mails (MailRecipient with member_id set and status "sent")
- **THEN** the ZIP SHALL contain a `kommunikation/` subfolder in that member's directory with one .txt file per sent mail

#### Scenario: ZIP contains inbound mails for a member
- **WHEN** an inbound mail has assigned_member_id set
- **THEN** the ZIP SHALL contain the mail as a .txt file in that member's `kommunikation/` subfolder

#### Scenario: Mails without member assignment are excluded
- **WHEN** an outbound mail has no member_id OR an inbound mail has no assigned_member_id
- **THEN** that mail SHALL NOT appear in the ZIP

### Requirement: Communication file naming convention

Each communication file SHALL be named following the pattern `{YYYY-MM-DD}_{HHmm}_{direction}_{sanitized_subject}.txt` where direction is `ausgehend` or `eingehend`.

#### Scenario: Outbound mail filename
- **WHEN** an outbound mail was sent on 2026-03-15 at 14:30 with subject "Willkommen bei uns"
- **THEN** the filename SHALL be `2026-03-15_1430_ausgehend_Willkommen_bei_uns.txt`

#### Scenario: Subject sanitization
- **WHEN** a mail subject contains special characters, umlauts, or exceeds 50 characters
- **THEN** the filename SHALL have umlauts transliterated (ä→ae, ö→oe, ü→ue, ß→ss), special characters removed, spaces replaced with `_`, and truncated to 50 characters

#### Scenario: Filename collision
- **WHEN** two mails for the same member would produce identical filenames
- **THEN** the system SHALL append the first 8 characters of the mail's UUID to disambiguate

### Requirement: Communication file content format

Each .txt file SHALL contain a header block with metadata followed by a separator line and the mail body.

#### Scenario: Outbound mail content
- **WHEN** rendering an outbound mail
- **THEN** the file SHALL contain:
  - `Richtung: Ausgehend`
  - `Datum: {YYYY-MM-DD HH:mm:ss}`
  - `An: {recipient_email}`
  - `Betreff: {subject}`
  - An empty line and separator (`───────────────────────────────────────`)
  - The mail body text

#### Scenario: Inbound mail content
- **WHEN** rendering an inbound mail
- **THEN** the file SHALL contain:
  - `Richtung: Eingehend`
  - `Datum: {YYYY-MM-DD HH:mm:ss}`
  - `Von: {from_address}`
  - `Betreff: {subject}`
  - An empty line and separator
  - The mail body text

### Requirement: WebDAV communication sync

The WebDAV backup worker SHALL synchronize communication files to a `kommunikation/` subfolder per member within the base backup directory, using an append-only strategy.

#### Scenario: New mails are synced
- **WHEN** a backup cycle runs and new mails exist that have not been synced before
- **THEN** the worker SHALL upload those mails as .txt files and mark them as synced

#### Scenario: Already-synced mails are skipped
- **WHEN** a mail has already been synced in a previous cycle
- **THEN** the worker SHALL NOT re-upload that mail

#### Scenario: Sync tracking persists across restarts
- **WHEN** the server restarts
- **THEN** previously synced mail IDs SHALL still be recognized (stored in database)

### Requirement: Communication backup DAO

The backup DAO SHALL provide a method to retrieve all member-assigned communications with the data needed for file generation and member directory assignment.

#### Scenario: Query returns outbound mails with member info
- **WHEN** `all_communications()` is called
- **THEN** it SHALL return outbound mails joined with member number, first name, and last name from the assigned member, including subject, body, sent_at timestamp, and recipient address

#### Scenario: Query returns inbound mails with member info
- **WHEN** `all_communications()` is called
- **THEN** it SHALL return inbound mails joined with member number, first name, and last name from the assigned member, including subject, body_text, received_at timestamp, and from_address

#### Scenario: Only active members' mails are included
- **WHEN** a mail is assigned to a member that has been soft-deleted
- **THEN** that mail SHALL still be included in the export (member data needed for folder naming)
