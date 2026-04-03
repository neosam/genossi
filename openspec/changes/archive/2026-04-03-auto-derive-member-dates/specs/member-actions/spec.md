## ADDED Requirements

### Requirement: Automatic date derivation from actions
The system SHALL automatically derive `join_date` and `exit_date` on the Member entity from MemberActions after every action create, update, or delete operation, and after member creation (which auto-creates an Eintritt action).

#### Scenario: join_date derived from Eintritt action
- **WHEN** a member has an Eintritt action with a given date
- **THEN** the member's `join_date` SHALL be set to that Eintritt action's date

#### Scenario: exit_date derived from Austritt action
- **WHEN** a member has an Austritt action with an effective_date
- **THEN** the member's `exit_date` SHALL be set to the Austritt action's effective_date

#### Scenario: exit_date derived from Todesfall action
- **WHEN** a member has a Todesfall action with a given date
- **THEN** the member's `exit_date` SHALL be set to the Todesfall action's date (immediately effective)

#### Scenario: exit_date cleared when no exit action
- **WHEN** a member has no Austritt or Todesfall action (e.g. after deleting an exit action)
- **THEN** the member's `exit_date` SHALL be set to None

#### Scenario: Austritt takes precedence over Todesfall
- **WHEN** a member has both an Austritt and a Todesfall action
- **THEN** the member's `exit_date` SHALL be derived from the Austritt action's effective_date

#### Scenario: No Eintritt action preserves existing join_date
- **WHEN** recalc_dates runs for a member that has no Eintritt action (e.g. imported member not yet migrated)
- **THEN** the member's `join_date` SHALL remain unchanged

#### Scenario: Dates recalculated on action create
- **WHEN** a new MemberAction is created
- **THEN** the system SHALL recalculate join_date and exit_date for the affected member

#### Scenario: Dates recalculated on action update
- **WHEN** a MemberAction is updated
- **THEN** the system SHALL recalculate join_date and exit_date for the affected member

#### Scenario: Dates recalculated on action delete
- **WHEN** a MemberAction is soft-deleted
- **THEN** the system SHALL recalculate join_date and exit_date for the affected member

## MODIFIED Requirements

### Requirement: Action type constraints
The system SHALL enforce the following constraints on action types:

#### Scenario: Status actions have zero shares_change
- **WHEN** an action of type `Eintritt`, `Austritt`, or `Todesfall` is created
- **THEN** `shares_change` SHALL be 0

#### Scenario: Aufstockung has positive shares_change
- **WHEN** an action of type `Aufstockung` is created
- **THEN** `shares_change` SHALL be greater than 0

#### Scenario: Verkauf has negative shares_change
- **WHEN** an action of type `Verkauf` is created
- **THEN** `shares_change` SHALL be less than 0

#### Scenario: UebertragungEmpfang has positive shares_change
- **WHEN** an action of type `UebertragungEmpfang` is created
- **THEN** `shares_change` SHALL be greater than 0 and `transfer_member_id` SHALL be set

#### Scenario: UebertragungAbgabe has negative shares_change
- **WHEN** an action of type `UebertragungAbgabe` is created
- **THEN** `shares_change` SHALL be less than 0 and `transfer_member_id` SHALL be set

#### Scenario: Transfer requires transfer_member_id
- **WHEN** an action of type `UebertragungEmpfang` or `UebertragungAbgabe` is created without `transfer_member_id`
- **THEN** the system SHALL reject the creation with a validation error

#### Scenario: Effective date required for Austritt
- **WHEN** an action of type `Austritt` is created or updated without `effective_date`
- **THEN** the system SHALL reject the operation with a validation error indicating that effective_date is required for Austritt actions

#### Scenario: Effective date only for Austritt
- **WHEN** an action of type other than `Austritt` is created with an `effective_date`
- **THEN** the system SHALL reject the creation with a validation error
