## Why

Editing member data currently requires navigating to the detail page for each member individually. For bulk data corrections (e.g., updating addresses, fixing names after an import), this is tedious. A global edit mode in the member list allows users to modify directly editable fields inline, saving significant time for batch operations.

## What Changes

- Global edit mode toggle in the member list toolbar
- Editable fields rendered as input fields in edit mode; read-only (action-driven) fields remain as plain text
- Autosave per row when focus leaves the row (PUT to existing member update endpoint)
- Row-level error highlighting (red background) when save fails
- Toast notifications for save errors
- Checkbox column hidden in edit mode (selection and editing are mutually exclusive)

## Capabilities

### New Capabilities
- `member-list-inline-edit`: Frontend capability for inline editing of member data in the member list table, including edit mode toggle, input rendering for editable fields, autosave on row blur, and error feedback.

### Modified Capabilities
<!-- No existing spec-level requirements change. The member update API (PUT /api/members/{id}) is already in place and sufficient. -->

## Impact

- **Frontend**: Major refactor of member list row rendering to support edit mode with conditional input/text cells
- **Frontend**: New state management for edit mode, dirty tracking per row, and error state per row
- **Frontend**: Toast notification system (may already exist or need creation)
- **Backend**: No changes needed — existing `PUT /api/members/{id}` endpoint is sufficient
- **Dependency**: Builds on `member-list-column-selection` change (dynamic column rendering)
