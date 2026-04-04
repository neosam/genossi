## Context

The member list (`genossi-frontend/src/page/members.rs`) is currently read-only. Editing requires navigating to the detail page per member. After the `member-list-column-selection` change, the table renders columns dynamically from a column registry where each `ColumnDef` has an `editable` flag. The existing `PUT /api/members/{id}` endpoint with optimistic locking (version field) is sufficient for saving changes — no backend modifications needed.

Editable fields (not set by actions): `member_number`, `first_name`, `last_name`, `email`, `company`, `street`, `house_number`, `postal_code`, `city`, `bank_account`, `comment`, `shares_at_joining`, `current_balance`.

Read-only fields (action-driven or system): `join_date`, `exit_date`, `current_shares`, `action_count`, `migrated`, `active_status`, `created`, `deleted`, `version`.

## Goals / Non-Goals

**Goals:**
- Global edit mode toggle that converts editable cells to input fields
- Autosave per row when focus leaves the row
- Visual feedback: red row highlighting on save errors, toast with error message
- Field-level validation for required fields (first_name, last_name) before submit
- Clean separation between edit mode and normal mode (no checkboxes, no row navigation in edit mode)

**Non-Goals:**
- Cell-level edit (click single cell) — the whole table switches mode
- Undo/redo
- Concurrent edit detection beyond optimistic locking (version conflict already handled by backend)
- Batch save button — saving is automatic per row

## Decisions

### Decision 1: Global edit mode vs. per-row edit

**Choice**: Global toggle — one button switches the entire table between view and edit mode.

**Rationale**: Simpler mental model for the user. Per-row editing creates ambiguity about which row is editable and complicates keyboard navigation. A global mode makes the intent clear: "I'm now editing data."

**Alternative considered**: Per-row edit button — rejected because the primary use case is bulk corrections across many rows.

### Decision 2: Row-level autosave on focus loss

**Choice**: When focus leaves a row (no focused element inside the row), check if any field changed. If yes, send PUT request with full MemberTO. If no changes, do nothing.

**Implementation approach**:
- Track a `focused_row: Signal<Option<Uuid>>` that updates on `focusin` events
- When `focused_row` changes from `Some(old_id)` to `Some(new_id)` or `None`, trigger save for `old_id` if dirty
- Each row maintains a local copy of MemberTO for dirty-checking against the original

**Rationale**: Focus-based triggers are more reliable than blur on individual inputs (which fires between cells within the same row). Using `focusin` event bubbling on the row element captures all child focus changes.

**Alternative considered**: Explicit save button per row — rejected because it adds visual clutter and an extra step for each row edit.

### Decision 3: Error handling with row highlighting + toast

**Choice**: Two-level feedback:
1. **Field-level**: Required fields (first_name, last_name) show red border if empty (frontend validation before PUT)
2. **Row-level**: If PUT fails (validation error, version conflict, network error), the entire row gets a red background and a toast notification shows the error message. The row stays "dirty" so the user can fix and re-trigger save by leaving the row again.

**Rationale**: Row highlighting gives persistent visual indication of which rows have problems. Toast gives the specific error message. Together they cover both "where" and "what."

### Decision 4: State management for edit mode

**Choice**: Three new signals:
- `edit_mode: Signal<bool>` — global toggle
- `row_edits: Signal<HashMap<Uuid, MemberTO>>` — local copies being edited (only populated for rows that the user has touched)
- `row_errors: Signal<HashMap<Uuid, String>>` — error messages per row

**Rationale**: Keeping edits in a separate map (not mutating the main MEMBERS signal) means we can discard all unsaved changes when exiting edit mode, and the original data is always available for dirty-checking.

### Decision 5: Input types per field

**Choice**: Map field types to appropriate HTML input types:
- String fields → `<input type="text">`
- Numeric fields (shares_at_joining, current_balance) → `<input type="number">`
- Optional fields → empty string displayed, saved as `None` if left empty

**Rationale**: Native input types give basic browser validation (number fields reject letters) without custom logic.

## Risks / Trade-offs

- **[Many rows edited, some fail]** → Each row saves independently. Failed rows stay highlighted, successful rows clear. User can scroll through red rows to fix remaining issues.
- **[Version conflict during edit]** → Backend returns 409. Toast shows "Daten wurden von jemand anderem geändert." Row stays dirty. User must refresh to get latest version. Mitigation: Refresh member data from backend after successful save to get new version.
- **[Accidental exit from edit mode with unsaved changes]** → Row saves on focus loss, so leaving edit mode (clicking "Fertig") triggers save for the currently focused row. Remaining unchanged rows need no save. Edge case: user switches browser tab without leaving the row — data not saved. Acceptable trade-off for simplicity.
- **[Performance with large member lists]** → Rendering many input fields is heavier than text. Mitigation: Only rows visible in viewport need input rendering (Dioxus handles this via virtual DOM diffing). For typical list sizes (<1000 members) this is negligible.
