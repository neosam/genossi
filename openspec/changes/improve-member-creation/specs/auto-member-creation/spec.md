## ADDED Requirements

### Requirement: Auto-assign member number
The system SHALL automatically assign the next available member number when a member is created with `member_number` set to 0. The next member number SHALL be `MAX(member_number) + 1` across all members (including soft-deleted ones).

#### Scenario: Auto-assign when member_number is 0
- **WHEN** a member is created with `member_number` set to 0
- **THEN** the system assigns `MAX(existing member_numbers) + 1` as the member number

#### Scenario: First member gets number 1
- **WHEN** no members exist and a member is created with `member_number` set to 0
- **THEN** the system assigns member number 1

#### Scenario: Explicit member number still works
- **WHEN** a member is created with `member_number` set to a positive value (e.g. 42)
- **THEN** the system uses the provided member number (existing uniqueness validation applies)

#### Scenario: Soft-deleted members included in MAX
- **WHEN** a soft-deleted member has the highest member number (e.g. 100)
- **THEN** the next auto-assigned number SHALL be 101

### Requirement: Automatic entry actions on member creation
The system SHALL automatically create an `Eintritt` action and an `Aufstockung` action when a new member is created.

#### Scenario: Eintritt action created
- **WHEN** a member is created with `join_date` of 2025-03-15
- **THEN** the system creates a `MemberAction` with `action_type=Eintritt`, `date=2025-03-15`, `shares_change=0`

#### Scenario: Aufstockung action created
- **WHEN** a member is created with `join_date` of 2025-03-15 and `shares_at_joining` of 3
- **THEN** the system creates a `MemberAction` with `action_type=Aufstockung`, `date=2025-03-15`, `shares_change=3`

#### Scenario: Actions and member in same transaction
- **WHEN** a member creation fails (e.g. duplicate member number)
- **THEN** no actions SHALL be persisted (all-or-nothing)

### Requirement: Computed fields on member creation
The system SHALL set computed fields automatically during member creation, ignoring client-provided values for these fields.

#### Scenario: current_shares set from shares_at_joining
- **WHEN** a member is created with `shares_at_joining` of 5
- **THEN** `current_shares` SHALL be set to 5

#### Scenario: current_balance set to 0
- **WHEN** a member is created
- **THEN** `current_balance` SHALL be set to 0

#### Scenario: action_count set to 0
- **WHEN** a member is created
- **THEN** `action_count` SHALL be set to 0

#### Scenario: migrated flag calculated
- **WHEN** a member is created with the automatic entry actions
- **THEN** the `migrated` flag SHALL be recalculated based on the created actions
