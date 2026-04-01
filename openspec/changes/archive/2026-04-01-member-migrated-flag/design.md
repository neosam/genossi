## Context

Members are imported from a legacy system (Excel). After import, their action history needs to be recreated in the new system. The migration status (comparing expected vs actual shares and action counts) is currently only visible on the member detail page via a dedicated endpoint that computes it on the fly.

Users need to see migration status directly in the member list to identify which members still need attention without clicking into each one.

## Goals / Non-Goals

**Goals:**
- Add a persistent `migrated` boolean flag to the Member entity
- Automatically recalculate the flag on every relevant data change
- Display the flag in the frontend member list
- Zero additional API calls needed for the member list to show migration status

**Non-Goals:**
- Replacing the existing `/migration-status` detail endpoint (it continues to provide the expected vs actual breakdown)
- Allowing manual override of the `migrated` flag
- Batch migration status recalculation endpoint (can be added later if needed)

## Decisions

### 1. Persistent boolean flag vs computed field

**Decision:** Store `migrated` as a boolean column in the `members` table.

**Alternatives considered:**
- *Compute on list query via SQL JOIN*: Would require aggregating all actions per member on every list request. Adds query complexity and may degrade with scale.
- *Compute in application layer per request*: N+1 query problem for the list endpoint.

**Rationale:** A stored flag is the simplest to query, adds negligible storage, and the recalculation cost is paid only when data changes (not on every read).

### 2. Recalculation trigger points

**Decision:** Recalculate after every:
- Action create / update / delete (in `MemberActionService`)
- Member update that changes `current_shares` or `action_count` (in `MemberService`)

**Rationale:** The `migrated` status depends on both action data and member fields. Triggering on all relevant mutations ensures the flag is always consistent.

### 3. Recalculation bypasses service-layer update

**Decision:** The `recalc_migrated()` method writes directly via the Member DAO (a dedicated method to update only the `migrated` field), not through the full `MemberService::update()` flow.

**Alternatives considered:**
- *Full service-layer update*: Would trigger another recalculation, causing an infinite loop or requiring loop-detection logic.

**Rationale:** A direct DAO write for a single derived field is simple, avoids recursion, and doesn't need version/optimistic locking since no user-facing conflict is possible.

### 4. Migration status calculation reuse

**Decision:** Extract the existing calculation logic from `MemberActionService::migration_status()` into a shared helper that both the existing endpoint and `recalc_migrated()` can use.

**Rationale:** Avoids duplicating the business rules for what constitutes "migrated" (matching shares and action counts).

### 5. Database default for existing rows

**Decision:** Default `migrated` to `false` for existing rows. A one-time data migration or manual trigger can backfill correct values.

**Rationale:** Defaulting to `false` is safe — it shows members as "pending" until properly evaluated. Backfilling can happen via a simple script or endpoint.

## Risks / Trade-offs

- **[Stale flag on direct DB edits]** → If someone modifies action data directly in the database (bypassing the service layer), the flag won't update. Mitigation: This is acceptable — all normal operations go through the service layer.
- **[Initial backfill needed]** → Existing members will all show as "pending" after migration. Mitigation: Add a one-time backfill in the migration SQL or a startup routine.
- **[Slight write overhead]** → Every action change triggers a recalculation and member update. Mitigation: The calculation is lightweight (sum + count over a small set of actions per member).
