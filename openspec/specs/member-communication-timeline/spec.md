# Member Communication Timeline

## Purpose

Display a unified chronological timeline of all inbound and outbound mail communications for a member on their detail page.

## Requirements

### Requirement: Communication timeline endpoint
The system SHALL provide an API endpoint `GET /api/members/{member_id}/communications` that returns a chronologically sorted list (newest first) of all inbound and outbound mail communications associated with the given member.

#### Scenario: Member with both inbound and outbound mails
- **WHEN** a GET request is made to `/api/members/{member_id}/communications` for a member who has sent mail recipients linked via `member_id` and inbound mails assigned via `assigned_member_id`
- **THEN** the response SHALL contain entries from both sources merged into a single list, sorted by date descending

#### Scenario: Member with no communications
- **WHEN** a GET request is made to `/api/members/{member_id}/communications` for a member with no linked mail recipients and no assigned inbound mails
- **THEN** the response SHALL be an empty list with HTTP 200

#### Scenario: Invalid member ID
- **WHEN** a GET request is made to `/api/members/{invalid_id}/communications` where the ID is not a valid UUID
- **THEN** the response SHALL return HTTP 400

### Requirement: Communication entry contains direction
Each communication entry SHALL include a `direction` field indicating whether the communication is `inbound` (received mail assigned to the member) or `outbound` (mail sent to the member).

#### Scenario: Inbound mail entry
- **WHEN** an inbound mail is assigned to the member via `assigned_member_id`
- **THEN** the entry SHALL have `direction` set to `inbound` and SHALL include `inbox_id`, `from_address`, and inbound status flags (`done`, `replied`, `archived`)

#### Scenario: Outbound mail entry
- **WHEN** a mail recipient is linked to the member via `member_id`
- **THEN** the entry SHALL have `direction` set to `outbound` and SHALL include `mail_job_id`, `recipient_id`, `to_address`, and the outbound `status` string (`pending`, `sent`, or `failed`)

### Requirement: Communication entries include common fields
Each communication entry SHALL include `date` (timestamp of the communication) and `subject` (mail subject line), regardless of direction.

#### Scenario: Date and subject present on inbound entry
- **WHEN** an inbound mail entry is returned
- **THEN** `date` SHALL be the `received_at` timestamp and `subject` SHALL be the inbound mail's subject

#### Scenario: Date and subject present on outbound entry
- **WHEN** an outbound mail entry is returned
- **THEN** `date` SHALL be the `sent_at` timestamp (or `created` if not yet sent) and `subject` SHALL be the mail job's subject

### Requirement: Chronological sorting
The communication list SHALL be sorted by date in descending order (newest first).

#### Scenario: Mixed inbound and outbound entries sorted
- **WHEN** a member has inbound mail received on April 11 and outbound mail sent on April 9
- **THEN** the April 11 inbound entry SHALL appear before the April 9 outbound entry

### Requirement: Frontend communication section
The member detail page SHALL display a "Kommunikation" section showing the communication timeline for the current member.

#### Scenario: Communication section visible on existing member
- **WHEN** a user views the detail page of an existing member
- **THEN** a "Kommunikation" section SHALL be visible, listing all communications for that member

#### Scenario: Communication section hidden for new members
- **WHEN** a user is creating a new member (no ID yet)
- **THEN** the "Kommunikation" section SHALL NOT be displayed

### Requirement: Deep links to mail details
Each communication entry SHALL link to the detail page of the corresponding mail.

#### Scenario: Inbound entry links to inbox detail
- **WHEN** a user clicks on an inbound communication entry
- **THEN** the browser SHALL navigate to `/inbox/{inbox_id}`

#### Scenario: Outbound entry links to mail job detail
- **WHEN** a user clicks on an outbound communication entry
- **THEN** the browser SHALL navigate to `/mail/jobs/{mail_job_id}`

#### Scenario: Open in new tab
- **WHEN** a user right-clicks or ctrl-clicks a communication entry link
- **THEN** the browser SHALL open the detail page in a new tab (standard anchor behavior)

### Requirement: Soft-deleted entries excluded
The communication timeline SHALL NOT include soft-deleted mail recipients or soft-deleted inbound mails.

#### Scenario: Deleted outbound recipient excluded
- **WHEN** a mail recipient linked to the member has a non-null `deleted` timestamp
- **THEN** that entry SHALL NOT appear in the communication timeline

#### Scenario: Deleted inbound mail excluded
- **WHEN** an inbound mail assigned to the member has been removed from the system
- **THEN** that entry SHALL NOT appear in the communication timeline
