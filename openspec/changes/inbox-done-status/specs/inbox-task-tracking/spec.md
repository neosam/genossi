## ADDED Requirements

### Requirement: Inbound mail data model uses independent boolean fields
The `inbound_mails` table SHALL use three independent boolean fields (`replied`, `done`, `archived`) instead of a single `status` text field. The `assigned_member_id` field SHALL remain unchanged.

#### Scenario: New mail has all flags set to false
- **WHEN** a new inbound mail is stored from IMAP polling
- **THEN** the mail SHALL have `replied = false`, `done = false`, `archived = false`

#### Scenario: Flags are independent
- **WHEN** a mail has `replied = true` and `done = false`
- **THEN** the mail is considered open (not done), even though a reply was sent

### Requirement: Mark inbound mail as done
The system SHALL provide an endpoint `POST /api/inbox/{id}/done` that marks an inbound mail as done.

#### Scenario: Mark mail as done
- **WHEN** `POST /api/inbox/{id}/done` is called with a valid mail ID
- **THEN** the mail's `done` field SHALL be set to `true` and the updated mail SHALL be returned

#### Scenario: Mark non-existent mail as done
- **WHEN** `POST /api/inbox/{id}/done` is called with an unknown ID
- **THEN** the system SHALL return 404

### Requirement: Reply sets replied flag independently
The system SHALL set `replied = true` when a reply is sent, without modifying the `done` or `archived` fields.

#### Scenario: Reply to an open mail
- **WHEN** `POST /api/inbox/{id}/reply` is called
- **THEN** the mail's `replied` field SHALL be set to `true`
- **AND** the `done` and `archived` fields SHALL remain unchanged

### Requirement: Archive sets archived flag independently
The system SHALL set `archived = true` when a mail is archived (moved to IMAP archive), without modifying the `done` or `replied` fields.

#### Scenario: Archive a mail
- **WHEN** `POST /api/inbox/{id}/archive` is called
- **THEN** the mail's `archived` field SHALL be set to `true`
- **AND** the `done` and `replied` fields SHALL remain unchanged

### Requirement: Assign and unassign do not affect flags
The system SHALL only modify `assigned_member_id` when assigning or unassigning a member, without modifying `replied`, `done`, or `archived`.

#### Scenario: Assign member to a mail
- **WHEN** `POST /api/inbox/{id}/assign` is called with a member ID
- **THEN** only `assigned_member_id` SHALL be updated
- **AND** `replied`, `done`, `archived` SHALL remain unchanged

#### Scenario: Unassign member from a mail
- **WHEN** `POST /api/inbox/{id}/unassign` is called
- **THEN** `assigned_member_id` SHALL be set to `None`
- **AND** `replied`, `done`, `archived` SHALL remain unchanged

### Requirement: List active mails filters by done flag
The list endpoint `GET /api/inbox` SHALL return only mails where `done = false`, ordered by `received_at` DESC.

#### Scenario: List excludes done mails
- **WHEN** `GET /api/inbox` is called
- **THEN** only mails with `done = false` SHALL be returned

#### Scenario: Done mail not in list
- **WHEN** a mail has `done = true`
- **THEN** the mail SHALL NOT appear in `GET /api/inbox` results

### Requirement: Ignore endpoint removed
The endpoint `POST /api/inbox/{id}/ignore` SHALL be removed. The `ignored` status no longer exists.

#### Scenario: Ignore endpoint returns 404
- **WHEN** `POST /api/inbox/{id}/ignore` is called
- **THEN** the system SHALL return 404 (route not found)

### Requirement: API response uses boolean fields instead of status string
The `InboundMailTO` and `InboundMailDetailTO` response objects SHALL contain `replied: bool`, `done: bool`, `archived: bool` fields instead of a `status: string` field.

#### Scenario: Response contains boolean flags
- **WHEN** any inbox endpoint returns an `InboundMailTO`
- **THEN** the response SHALL include `replied`, `done`, and `archived` as boolean fields
- **AND** SHALL NOT include a `status` field

### Requirement: Database migration from status to boolean fields
The system SHALL migrate existing data from the `status` field to the new boolean fields.

#### Scenario: Migrate "new" status
- **WHEN** a mail has `status = 'new'`
- **THEN** after migration: `replied = false`, `done = false`, `archived = false`

#### Scenario: Migrate "assigned" status
- **WHEN** a mail has `status = 'assigned'`
- **THEN** after migration: `replied = false`, `done = false`, `archived = false`

#### Scenario: Migrate "replied" status
- **WHEN** a mail has `status = 'replied'`
- **THEN** after migration: `replied = true`, `done = false`, `archived = false`

#### Scenario: Migrate "ignored" status
- **WHEN** a mail has `status = 'ignored'`
- **THEN** after migration: `replied = false`, `done = true`, `archived = false`

#### Scenario: Migrate "archived" status
- **WHEN** a mail has `status = 'archived'`
- **THEN** after migration: `replied = false`, `done = false`, `archived = true`
