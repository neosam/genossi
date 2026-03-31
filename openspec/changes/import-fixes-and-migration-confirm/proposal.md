## Why

Two issues found during real-world usage of the Excel import and migration tracking:

1. **Balance import bug**: The Excel file contains Euro amounts (e.g., 150.00), but the system stores balance in cents (e.g., 15000). The import reads the value as-is without converting Euro to cents, resulting in incorrect balances (150 cents instead of 15000 cents).

2. **No way to confirm migration when action counts differ**: When the "Soll-Aktionen" (expected) differs from "Ist-Aktionen" (actual), the member stays "pending" even if the user has verified that the actions are correct. The user needs a way to confirm migration is complete despite the count mismatch — effectively saying "the imported action_count from Excel was wrong, the actual actions are correct."

## What Changes

- Fix the Excel import to multiply the balance value by 100 (Euro → Cent conversion)
- Add a "Confirm migration" button on the member detail page (visible when migration status is "pending")
- The confirm action adjusts `action_count` on the member to match the actual action count (minus 1, since expected = action_count + 1), causing `migrated` to recalculate to `true` (assuming shares already match)

## Capabilities

### New Capabilities

- `migration-confirm`: Allow users to confirm migration completion by adjusting the expected action count to match reality

### Modified Capabilities

- `member-management`: Fix Excel import balance conversion (Euro → Cent)

## Impact

- **Import service**: Change balance parsing to multiply by 100
- **REST layer**: New endpoint or use existing member update for confirming migration
- **Frontend**: Add confirm button to migration status badge on member detail page
- **E2E tests**: Fix existing balance assertions, add tests for confirm flow
