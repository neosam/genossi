## Why

Creating a new member currently requires the client to manually provide the member number, current shares, current balance, and action count. No entry actions (Eintritt + Aufstockung) are created automatically. This makes the creation flow incomplete and error-prone — the Excel import already handles this correctly, but the REST API create endpoint does not.

## What Changes

- Auto-assign the next available member number when `member_number` is 0 (MAX + 1 from existing members)
- Automatically create an `Eintritt` action (shares_change=0) and an `Aufstockung` action (shares_change=shares_at_joining) when a member is created
- Set `current_shares` to `shares_at_joining` automatically
- Set `current_balance` to 0 (ignored for now, will be computed from actions in a future change)
- Set `action_count` to 0
- Calculate and set the `migrated` flag after creating the actions
- All operations happen within a single transaction

## Capabilities

### New Capabilities
- `auto-member-creation`: Automatic member number assignment and entry action creation during member creation

### Modified Capabilities
- `member-management`: The create member requirement changes — member_number becomes optional (0 = auto-assign), and the service automatically creates Eintritt + Aufstockung actions and sets computed fields

## Impact

- `genossi_service_impl/src/member.rs`: Main changes in `MemberServiceImpl::create`
- `genossi_rest_types/src/lib.rs`: `MemberTO.member_number` may need adjusted documentation/defaults
- `genossi_dao/src/member.rs`: Needs a `next_member_number` default method (MAX + 1 over dump_all)
- Existing tests in `genossi_service_impl` and `genossi_rest` need updates
- The Excel import flow is NOT affected (it continues to provide explicit member numbers)
