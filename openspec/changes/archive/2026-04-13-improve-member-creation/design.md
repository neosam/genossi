## Context

The member creation flow (`MemberServiceImpl::create`) currently requires the client to provide all fields including `member_number`, `current_shares`, `current_balance`, and `action_count`. No entry actions are created. The Excel import (`member_import.rs`) already implements the correct pattern: creating `Eintritt` + `Aufstockung` actions automatically. The `MemberServiceImpl` already has access to `MemberActionDao` via its dependency injection.

## Goals / Non-Goals

**Goals:**
- Auto-assign next member number when client sends 0
- Automatically create Eintritt + Aufstockung actions on member creation
- Set computed fields (`current_shares`, `action_count`) automatically
- Keep everything in a single transaction

**Non-Goals:**
- Config table for share values (future change)
- Computing `current_balance` from actions (future change — set to 0 for now)
- Changing the Excel import flow
- Frontend changes for the create form (may follow separately)

## Decisions

### 1. Next member number via DAO default implementation

Add a `next_member_number` default method on `MemberDao` that uses `dump_all()` to find `MAX(member_number) + 1`. This follows the project convention where default implementations use `dump_all()` and can be overridden in specific DAO implementations for optimization later.

**Alternative considered**: Adding a dedicated SQL query in `MemberDaoImpl`. Rejected because the convention is defaults via `dump_all()` first, optimize later.

### 2. Member number = 0 triggers auto-assignment

When `member_number` is 0, the service assigns the next available number. When non-zero, the existing uniqueness validation applies. This allows the import flow to continue setting explicit numbers while the create flow can use 0 for auto-assignment.

**Alternative considered**: Making `member_number` `Option<i64>`. Rejected because it would require migration changes and the existing integer field with a sentinel value of 0 is simpler.

### 3. Action creation inside MemberServiceImpl::create

The `MemberServiceImpl` already has `member_action_dao` as a dependency. The Eintritt and Aufstockung actions are created directly via `member_action_dao` within the same transaction — no need to call through `MemberActionService` which would start its own transaction.

This mirrors the pattern used in `member_import.rs` (lines 421-453).

### 4. Ignore current_balance

Set `current_balance` to 0 on creation. The field exists but is not computed. A future change will introduce the config table with share values and compute balance from actions.

## Risks / Trade-offs

- **[Risk] Race condition on member number**: Two concurrent creates could get the same MAX+1. → Mitigated by the existing UNIQUE index on `member_number` — one transaction will fail with a DAO error. The client can retry.
- **[Risk] Eintritt validation requires shares_change=0**: The action validation in `member_action.rs` enforces this. Since we create actions via DAO directly (not through MemberActionService), we must ensure correct values ourselves. → Mitigated by using hardcoded correct values, same as the import does.
- **[Trade-off] current_balance = 0 is technically incorrect**: New members will show 0 balance even though they paid for shares. → Acceptable as a temporary state until the config/balance computation change is implemented.
