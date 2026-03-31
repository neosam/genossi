## Context

The member list page already has a filter bar with "Only active members" and "Only exited members" checkbox toggles. The `migrated` boolean field is already present in `MemberTO` and displayed as a badge in the list table. Users need a quick way to filter down to members with pending migrations.

## Goals / Non-Goals

**Goals:**
- Allow users to filter the member list to show only members with pending migrations (`migrated == false`)
- Follow the existing filter pattern (checkbox toggle in the filter bar)

**Non-Goals:**
- No filter for "only migrated" members (inverse) — not requested
- No backend query filtering — the data is already fully loaded client-side
- No changes to the migration status badge display

## Decisions

**1. Client-side filter only**
The member list is already fully loaded into the frontend state. Adding a client-side `.filter()` is consistent with how the existing active/exited filters work. No backend changes needed.

**2. Checkbox toggle pattern**
Use the same checkbox pattern as the existing `only_active` and `only_exited` toggles. The new toggle is independent of the active/exited filters — they can be combined (e.g., show only active members with pending migrations).

**3. i18n key placement**
Add a single new translation key `OnlyPendingMigration` to the existing i18n system (de, en).

## Risks / Trade-offs

- [Minimal risk] This is a small, self-contained frontend change with no backend or data model impact.
