## Context

The Excel import reads balance values directly as integers, but the source data is in Euro (e.g., 150) while the system stores cents (e.g., 15000). Additionally, migration status can be stuck on "pending" due to action count mismatches from the legacy data, with no way for users to resolve this.

## Goals / Non-Goals

**Goals:**
- Fix balance import to convert Euro → Cent (multiply by 100)
- Allow users to confirm migration when action counts don't match
- Keep the confirmation simple — one button click

**Non-Goals:**
- Changing how balance is stored (stays in cents)
- Allowing users to override shares mismatches (only action count confirmation)
- Bulk migration confirmation

## Decisions

### 1. Balance conversion approach

**Decision:** Multiply the parsed balance value by 100 in the import parsing code, handling both integer and decimal Euro values (e.g., 150 → 15000, 150.50 → 15050).

**Rationale:** The conversion belongs at the parsing boundary where the external format meets the internal format.

### 2. Migration confirm mechanism

**Decision:** Add a dedicated `POST /api/members/{id}/confirm-migration` endpoint that adjusts `action_count` to match the actual non-status action count minus 1 (so expected_action_count = actual_action_count). This triggers `recalc_migrated()` which sets the flag to `true` (assuming shares match).

**Alternatives considered:**
- *Manual edit of action_count field*: Too indirect, user would need to know the formula.
- *Direct force-migrated flag*: Bypasses the calculation, could hide real issues.

**Rationale:** Adjusting `action_count` works with the existing recalculation system. The user explicitly confirms "my actions are correct, adjust the expectation." If shares still don't match, migrated stays false — this only resolves action count mismatches.

### 3. Frontend UX

**Decision:** Add a "Confirm" button next to the "Soll-Aktionen / Ist-Aktionen" line in the pending migration badge. Only show when action count mismatches but shares match (since the button can only fix action count issues).

**Rationale:** Only offering confirmation when it can actually resolve the issue avoids user confusion.
