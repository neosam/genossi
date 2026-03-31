## Context

The member list page already displays join_date and has a text search filter. The `MemberTO` struct includes both `join_date` (required) and `exit_date` (optional). No backend changes are needed — this is a pure frontend feature.

## Goals / Non-Goals

**Goals:**
- Show active/inactive status per member based on a user-selected reference date
- Allow filtering to show only active members
- Default the reference date to today

**Non-Goals:**
- Backend-side filtering (the dataset is small enough for client-side filtering)
- Persisting the selected date or filter state across page navigations

## Decisions

### 1. Active status computation

**Decision:** Compute in the frontend at render time based on `join_date`, `exit_date`, and the selected reference date. A member is active when `join_date <= reference_date AND (exit_date IS NULL OR exit_date > reference_date)`.

**Rationale:** No backend changes needed, the data is already available, and the member list is small enough for client-side computation.

### 2. UI layout

**Decision:** Place the date picker and filter toggle in a row between the search bar and the table. The date picker is an `<input type="date">` defaulting to today. The filter toggle is a checkbox with label.

**Rationale:** Keeps controls grouped logically — search, then date/filter, then results.

### 3. Active column display

**Decision:** Show a colored badge in the table: green "Aktiv" / red "Inaktiv". Place the column after the migration status column.

**Rationale:** Consistent with the existing badge pattern used for migration status.
