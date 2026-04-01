## Why

The migration status of members is currently only visible when navigating to individual member details, requiring a separate API call per member. Users need to see at a glance which members are fully migrated and which still need attention, directly in the member list view.

## What Changes

- Add a `migrated` boolean field to the Member entity and database table
- Automatically recalculate the `migrated` flag whenever a relevant change occurs:
  - After any Action create/update/delete
  - After any Member update that affects `current_shares` or `action_count`
- The recalculation goes directly through the DAO layer to avoid triggering recursive member update cycles
- Include the `migrated` field in `MemberTO` so the member list endpoint returns it
- Display a migration status indicator in the frontend member list
- The existing `/migration-status` detail endpoint remains unchanged (continues to provide expected vs actual breakdowns)

## Capabilities

### New Capabilities

- `member-migrated-flag`: Persistent boolean flag on the Member entity that tracks whether migration is complete, automatically recalculated on relevant data changes

### Modified Capabilities

- `member-management`: The Member entity gains a new `migrated` field, exposed via the REST API and displayed in the frontend member list

## Impact

- **Database**: New migration to add `migrated` column to `members` table
- **DAO layer**: `MemberEntity` gains `migrated: bool` field, DAO queries updated
- **Service layer**: New `recalc_migrated()` method; called from both `MemberActionService` and `MemberService`
- **REST layer**: `MemberTO` gains `migrated` field, automatically included in list/detail responses
- **Frontend**: Member list component updated to show migration status badge/indicator
