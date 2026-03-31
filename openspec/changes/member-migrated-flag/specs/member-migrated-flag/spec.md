## ADDED Requirements

### Requirement: Migrated flag on Member entity
The Member entity SHALL include a `migrated` boolean field (default `false`) that indicates whether the member's action history has been fully migrated from the legacy system.

#### Scenario: New member has migrated false by default
- **WHEN** a new member is created
- **THEN** the `migrated` field SHALL be `false`

#### Scenario: Migrated flag included in API responses
- **WHEN** a member is retrieved via `GET /api/members` or `GET /api/members/{id}`
- **THEN** the response SHALL include the `migrated` boolean field

### Requirement: Automatic recalculation on action changes
The system SHALL recalculate the `migrated` flag for a member whenever an action belonging to that member is created, updated, or deleted.

#### Scenario: Action created triggers recalculation
- **WHEN** a new action is created for a member
- **THEN** the system recalculates the `migrated` flag based on current action data and updates the member record

#### Scenario: Action updated triggers recalculation
- **WHEN** an existing action is updated
- **THEN** the system recalculates the `migrated` flag for the action's member

#### Scenario: Action deleted triggers recalculation
- **WHEN** an action is deleted
- **THEN** the system recalculates the `migrated` flag for the action's member

### Requirement: Automatic recalculation on member changes
The system SHALL recalculate the `migrated` flag whenever a member's `current_shares` or `action_count` fields are updated.

#### Scenario: Member shares updated triggers recalculation
- **WHEN** a member's `current_shares` or `action_count` is updated via `PUT /api/members/{id}`
- **THEN** the system recalculates the `migrated` flag based on current action data

### Requirement: Migrated calculation logic
The `migrated` flag SHALL be `true` when both conditions are met:
- The sum of all action `shares_change` values equals `member.current_shares`
- The count of non-status actions (excluding Eintritt, Austritt, Todesfall) equals `member.action_count + 1`

#### Scenario: All conditions met
- **WHEN** actual shares match expected shares AND actual action count matches expected action count
- **THEN** the `migrated` flag SHALL be set to `true`

#### Scenario: Shares mismatch
- **WHEN** the sum of action shares_change does not equal member.current_shares
- **THEN** the `migrated` flag SHALL be set to `false`

#### Scenario: Action count mismatch
- **WHEN** the count of non-status actions does not equal member.action_count + 1
- **THEN** the `migrated` flag SHALL be set to `false`

### Requirement: Recalculation bypasses service-layer update
The recalculation of the `migrated` flag SHALL write directly to the DAO layer without triggering a full member update, to avoid recursive update cycles.

#### Scenario: No recursive update
- **WHEN** the system recalculates and writes the `migrated` flag
- **THEN** no additional member update event is triggered
