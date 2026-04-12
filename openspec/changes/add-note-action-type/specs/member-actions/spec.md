## ADDED Requirements

### Requirement: Note action type
The system SHALL support a `Note` action type that allows recording free-text notes in a member's action history.

#### Scenario: Note action has zero shares_change
- **WHEN** an action of type `Note` is created
- **THEN** `shares_change` SHALL be 0

#### Scenario: Note action requires comment
- **WHEN** an action of type `Note` is created without a `comment` or with an empty `comment`
- **THEN** the system SHALL reject the creation with a validation error

#### Scenario: Note action rejects transfer_member_id
- **WHEN** an action of type `Note` is created with a `transfer_member_id`
- **THEN** the system SHALL reject the creation with a validation error

#### Scenario: Note action rejects effective_date
- **WHEN** an action of type `Note` is created with an `effective_date`
- **THEN** the system SHALL reject the creation with a validation error

#### Scenario: Note action does not affect member dates
- **WHEN** a `Note` action is created, updated, or deleted
- **THEN** the member's `join_date` and `exit_date` SHALL remain unchanged (note actions are ignored during date derivation)

#### Scenario: Note action excluded from migration action count
- **WHEN** migration status is calculated for a member with `Note` actions
- **THEN** `Note` actions SHALL NOT be counted in `actual_action_count`

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

#### Scenario: Note has zero shares_change
- **WHEN** an action of type `Note` is created
- **THEN** `shares_change` SHALL be 0

#### Scenario: Note requires comment
- **WHEN** an action of type `Note` is created without a `comment` or with an empty `comment`
- **THEN** the system SHALL reject the creation with a validation error

#### Scenario: Note rejects transfer_member_id
- **WHEN** an action of type `Note` is created with a `transfer_member_id`
- **THEN** the system SHALL reject the creation with a validation error

#### Scenario: Note rejects effective_date
- **WHEN** an action of type `Note` is created with an `effective_date`
- **THEN** the system SHALL reject the creation with a validation error
