## ADDED Requirements

### Requirement: MemberAction data model
The system SHALL store member actions with the following fields:
- `id` (UUID, system-generated, primary key)
- `member_id` (UUID, foreign key to member, required)
- `action_type` (Enum, required): one of `Eintritt`, `Austritt`, `Todesfall`, `Aufstockung`, `Verkauf`, `UebertragungEmpfang`, `UebertragungAbgabe`
- `date` (Date, required): date the action occurred
- `shares_change` (i32, required): change in shares (0 for status actions, positive/negative for share actions)
- `transfer_member_id` (Optional UUID): reference to the other member in a transfer
- `effective_date` (Optional Date): only for Austritt, the date the exit becomes effective per Satzung
- `comment` (Optional String)
- `created` (DateTime, system-generated)
- `deleted` (Optional DateTime, for soft delete)
- `version` (UUID, for optimistic locking)

#### Scenario: Action stored with all fields
- **WHEN** a member action is created with all fields provided
- **THEN** the system stores the action with a generated UUID, created timestamp, and version UUID

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

#### Scenario: Effective date only for Austritt
- **WHEN** an action of type other than `Austritt` is created with an `effective_date`
- **THEN** the system SHALL reject the creation with a validation error

### Requirement: Create member action
The system SHALL allow authenticated users with `manage_members` privilege to create member actions via `POST /api/members/{member_id}/actions`.

#### Scenario: Successful action creation
- **WHEN** an authenticated user sends a POST request with valid action data for an existing member
- **THEN** the system creates the action, assigns a UUID and version, sets the created timestamp, and returns the created action with HTTP 200

#### Scenario: Member not found
- **WHEN** an action is created for a member_id that does not exist
- **THEN** the system returns HTTP 404

#### Scenario: Insufficient privileges
- **WHEN** a user without `manage_members` privilege attempts to create an action
- **THEN** the system returns HTTP 401 Unauthorized

### Requirement: List member actions
The system SHALL allow authenticated users with `view_members` privilege to list all actions for a member via `GET /api/members/{member_id}/actions`.

#### Scenario: List actions for member
- **WHEN** an authenticated user sends a GET request for a valid member_id
- **THEN** the system returns all non-deleted actions for that member, ordered by date

#### Scenario: Member not found
- **WHEN** actions are requested for a member_id that does not exist
- **THEN** the system returns HTTP 404

### Requirement: Update member action
The system SHALL allow authenticated users with `manage_members` privilege to update a member action via `PUT /api/members/{member_id}/actions/{action_id}`.

#### Scenario: Successful update
- **WHEN** a PUT request is sent with valid updated data and matching version
- **THEN** the system updates the action, generates a new version UUID, and returns the updated action with HTTP 200

#### Scenario: Version conflict
- **WHEN** a PUT request is sent with a version UUID that does not match the stored version
- **THEN** the system returns an error indicating a version conflict

### Requirement: Delete member action (soft delete)
The system SHALL allow authenticated users with `manage_members` privilege to soft-delete a member action via `DELETE /api/members/{member_id}/actions/{action_id}`.

#### Scenario: Successful soft delete
- **WHEN** a DELETE request is sent for an existing action
- **THEN** the system sets the `deleted` timestamp on the action and returns HTTP 204

### Requirement: Migration validation
The system SHALL provide a validation endpoint `GET /api/members/{member_id}/actions/migration-status` that compares actions against imported Excel values.

#### Scenario: Member fully migrated (auto)
- **WHEN** a member has `action_count == 0` AND `shares_at_joining == current_shares` AND has an Eintritt action and an Aufstockung action whose shares_change equals shares_at_joining
- **THEN** the migration status SHALL be `migrated`

#### Scenario: Member fully migrated (manual)
- **WHEN** the sum of all shares_change values equals `current_shares` AND the count of non-Eintritt share actions equals `action_count + 1` (since Excel's action_count excludes the initial Aufstockung)
- **THEN** the migration status SHALL be `migrated`

#### Scenario: Member partially migrated
- **WHEN** the sum of shares_change values does not equal `current_shares` OR the action count does not match
- **THEN** the migration status SHALL be `pending` with details showing expected vs. actual values

#### Scenario: Member has no actions
- **WHEN** a member has no actions at all
- **THEN** the migration status SHALL be `pending`

### Requirement: OpenAPI documentation for actions
The system SHALL expose all member action endpoints in the Swagger UI at `/swagger-ui/` with complete OpenAPI annotations.

#### Scenario: Swagger UI shows action endpoints
- **WHEN** a user navigates to `/swagger-ui/`
- **THEN** the Swagger UI displays all member action endpoints with their schemas
