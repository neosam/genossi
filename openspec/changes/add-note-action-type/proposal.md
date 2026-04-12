## Why

Members need a way to record free-text notes as part of their action history — for example, documenting that an email address was corrected on a specific date. Currently, all action types are tied to membership status changes or share transactions, so there is no way to log general-purpose notes.

## What Changes

- Add a new `Note` variant to the `ActionType` enum across all layers (DAO, Service, REST, Frontend)
- `Note` actions require a non-empty `comment` field and have `shares_change = 0`
- `Note` actions do not affect member dates, share counts, or migration status

## Capabilities

### New Capabilities

_(none — this extends an existing capability)_

### Modified Capabilities

- `member-actions`: Add `Note` as a new action type with its own validation constraints

## Impact

- **DAO layer**: `ActionType` enum gains `Note` variant; SQLite storage/retrieval updated
- **Service layer**: Validation rules extended for `Note` (shares_change = 0, no transfer_member_id, no effective_date, comment required)
- **REST layer**: `ActionTypeTO` enum gains `Note` variant
- **Frontend**: Action type selector and display updated to include `Note`
- **Migration status**: `Note` actions excluded from action count (like status actions)
- **Date derivation**: `Note` actions do not affect join_date or exit_date
