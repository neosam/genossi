## MODIFIED Requirements

### Requirement: Create member
The system SHALL allow authenticated users with `manage_members` privilege to create new members via `POST /api/members`. When `member_number` is 0, the system SHALL auto-assign the next available number. The system SHALL automatically create `Eintritt` and `Aufstockung` actions and set `current_shares` from `shares_at_joining`, `current_balance` to 0, and `action_count` to 0. When the member status is `FehlerhaftErfasst`, the system SHALL NOT create automatic Eintritt and Aufstockung actions, and SHALL set `current_shares` to 0 regardless of `shares_at_joining`. The create operation SHALL be performed via `audited_create!` macro, logging all non-None member fields to the audit log.

#### Scenario: Successful member creation with auto member number
- **WHEN** an authenticated user with `manage_members` privilege sends a POST request with `member_number` set to 0, `join_date`, `shares_at_joining`, and status `Normal`
- **THEN** the system creates the member with the next available member number, creates Eintritt and Aufstockung actions, sets computed fields, returns the created member with HTTP 200, and creates audit log entries for all member fields and action fields

#### Scenario: Successful member creation with explicit member number
- **WHEN** an authenticated user with `manage_members` privilege sends a POST request with a positive `member_number` and status `Normal`
- **THEN** the system creates the member with the provided member number, creates Eintritt and Aufstockung actions, sets computed fields, returns the created member with HTTP 200, and creates audit log entries

### Requirement: Update member
The system SHALL allow authenticated users with `manage_members` privilege to update existing members via `PUT /api/members/{id}`. The update operation SHALL be performed via `audited_update!` macro, which loads the old entity, performs the update, computes the diff, and logs only changed fields to the audit log.

#### Scenario: Member update with changes
- **WHEN** an authenticated user updates a member changing first_name and email
- **THEN** the system updates the member and creates exactly 2 audit log entries for first_name and email

#### Scenario: Member update without data changes
- **WHEN** an authenticated user submits an update with identical data
- **THEN** the system performs the update but creates no audit log entries

### Requirement: Delete member
The system SHALL allow authenticated users with `manage_members` privilege to soft-delete members via `DELETE /api/members/{id}`. The delete operation SHALL be performed via `audited_delete!` macro, logging all audited field values.

#### Scenario: Member deletion audit
- **WHEN** an authenticated user deletes a member
- **THEN** the system soft-deletes the member and creates audit log entries with action "delete" for all audited fields
