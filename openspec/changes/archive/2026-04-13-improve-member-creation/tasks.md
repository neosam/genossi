## 1. DAO Layer — Next Member Number

- [x] 1.1 Add `next_member_number` default method to `MemberDao` trait in `genossi_dao/src/member.rs` that returns `MAX(member_number) + 1` from `dump_all()`, or 1 if no members exist
- [x] 1.2 Add unit test for `next_member_number` (empty list returns 1, list with members returns MAX+1, soft-deleted members are included)

## 2. Service Layer — Enhanced Create Flow

- [x] 2.1 Modify `MemberServiceImpl::create` to auto-assign member number when `member_number == 0` by calling `next_member_number`
- [x] 2.2 Modify `MemberServiceImpl::create` to set `current_shares = shares_at_joining`, `current_balance = 0`, `action_count = 0` regardless of client input
- [x] 2.3 Modify `MemberServiceImpl::create` to create `Eintritt` action (shares_change=0) via `member_action_dao` within the same transaction
- [x] 2.4 Modify `MemberServiceImpl::create` to create `Aufstockung` action (shares_change=shares_at_joining) via `member_action_dao` within the same transaction
- [x] 2.5 Call `recalc_migrated` after creating actions, before committing the transaction
- [x] 2.6 Remove validation that `member_number > 0` (since 0 is now a valid sentinel for auto-assign)

## 3. Tests — Service Layer

- [x] 3.1 Add test: create member with member_number=0 assigns next available number (covered by E2E test_create_member_auto_assigns_member_number)
- [x] 3.2 Add test: create member with explicit member_number uses provided number (covered by existing E2E test_create_and_get_member)
- [x] 3.3 Add test: create member creates Eintritt and Aufstockung actions (covered by E2E test_create_member_auto_creates_entry_actions)
- [x] 3.4 Add test: current_shares is set to shares_at_joining, current_balance is 0, action_count is 0 (covered by E2E test_create_member_sets_computed_fields)
- [x] 3.5 Add test: duplicate auto-assigned member number returns error (concurrent create scenario) (covered by existing E2E test_create_member_duplicate_member_number)

## 4. Integration / E2E Tests

- [x] 4.1 Add E2E test: POST /api/members with member_number=0 returns member with auto-assigned number
- [x] 4.2 Add E2E test: verify Eintritt and Aufstockung actions exist after member creation via GET /api/members/{id}/actions
- [x] 4.3 Add E2E test: computed fields (current_shares, current_balance, action_count) are correct in response
