# Migration Confirm

## Purpose

Provides functionality to confirm member migration by adjusting action counts so that expected and actual action counts match, enabling the `migrated` flag to become `true`.

## Requirements

### Requirement: Confirm migration endpoint
The system SHALL provide a `POST /api/members/{id}/confirm-migration` endpoint that adjusts the member's `action_count` so that the expected action count matches the actual action count.

#### Scenario: Successful confirmation when shares match
- **WHEN** a user calls `POST /api/members/{id}/confirm-migration` and the member's actual shares equal expected shares
- **THEN** the system adjusts `action_count` to `actual_non_status_action_count - 1`, recalculates `migrated` to `true`, and returns HTTP 200

#### Scenario: Confirmation when shares don't match
- **WHEN** a user calls `POST /api/members/{id}/confirm-migration` and actual shares do not equal expected shares
- **THEN** the system adjusts `action_count` but `migrated` remains `false` (shares still mismatch), and returns HTTP 200

#### Scenario: Member not found
- **WHEN** a user calls `POST /api/members/{id}/confirm-migration` with a non-existent member ID
- **THEN** the system returns HTTP 404

### Requirement: Confirm button in frontend
The frontend member detail page SHALL show a "Confirm migration" button in the pending migration status badge when the action count does not match.

#### Scenario: Button visible on action count mismatch
- **WHEN** a member's migration status is "pending" and the action counts differ
- **THEN** a confirm button is displayed next to the action count line

#### Scenario: Button triggers confirmation
- **WHEN** the user clicks the confirm button
- **THEN** the system calls `POST /api/members/{id}/confirm-migration` and refreshes the migration status display
