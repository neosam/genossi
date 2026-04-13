# Member Status

## Purpose

Define the member status enum and its behavioral effects on member visibility, counting, and validation.

## Requirements

### Requirement: Member status enum
The system SHALL support a `status` field on each member with the following enum values:
- `Normal` — Standard member, status determined by actions and dates
- `FehlerhaftErfasst` — Incorrectly registered, was never a real member

The enum SHALL be extensible for future status values.

#### Scenario: Default status for new members
- **WHEN** a member is created without specifying a status
- **THEN** the system SHALL set the status to `Normal`

#### Scenario: Status set at creation
- **WHEN** a member is created with `status` set to `FehlerhaftErfasst`
- **THEN** the system SHALL store the member with status `FehlerhaftErfasst`

#### Scenario: Status update
- **WHEN** an existing member's status is updated to `FehlerhaftErfasst`
- **THEN** the system SHALL persist the new status and the member SHALL no longer count as active

### Requirement: Erroneously registered members excluded from active count
Members with status `FehlerhaftErfasst` SHALL NOT be counted as active members, regardless of their join_date, exit_date, or actions.

#### Scenario: Active member count excludes erroneously registered
- **WHEN** the system counts active members for a reference date
- **THEN** members with status `FehlerhaftErfasst` SHALL be excluded from the count

#### Scenario: Active member list excludes erroneously registered
- **WHEN** the system lists active members
- **THEN** members with status `FehlerhaftErfasst` SHALL NOT appear in the active list

### Requirement: Erroneously registered members visible in full list
Members with status `FehlerhaftErfasst` SHALL be visible when listing all members (unfiltered) and SHALL be clearly distinguishable by their status field.

#### Scenario: All members list includes erroneously registered
- **WHEN** the system lists all members without active-only filter
- **THEN** members with status `FehlerhaftErfasst` SHALL appear with their status field set accordingly

### Requirement: Member number retention
Members with status `FehlerhaftErfasst` SHALL retain their assigned member number. The member number SHALL NOT be reused or freed.

#### Scenario: Member number preserved after status change
- **WHEN** a member's status is changed to `FehlerhaftErfasst`
- **THEN** the member's `member_number` SHALL remain unchanged and unique
