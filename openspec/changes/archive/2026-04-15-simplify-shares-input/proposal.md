## Why

When creating member actions, users must manually enter negative values for Verkauf and Übertragung Abgabe, which is unintuitive and error-prone. Additionally, the shares_change field is shown for action types where it must always be 0 (e.g., Note), adding unnecessary clutter. The form should guide users by only showing relevant fields and automatically handling the sign of the value.

## What Changes

- Hide the shares_change input field for Note actions (currently only hidden for status actions)
- Always accept positive values from the user and automatically negate them for Verkauf and Übertragung Abgabe before sending to the API
- Show a dynamic, context-appropriate label instead of the generic "SharesChange" (e.g., "Anteile abgeben" for Verkauf)
- Set `min=1` on the shares input to prevent zero or negative entries
- When editing an existing action, display the absolute value in the form (convert negative API values back to positive for display)
- Add i18n keys for the new dynamic labels in all three languages (DE, EN, CS)

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `frontend-member-actions`: The action create/edit form changes its shares_change field behavior — hiding it for Note, using positive-only input with automatic sign handling, and showing dynamic labels per action type.

## Impact

- **Frontend only**: `genossi-frontend/src/page/member_details.rs` (form logic and rendering)
- **Frontend i18n**: `genossi-frontend/src/i18n/mod.rs`, `en.rs`, `de.rs`, `cs.rs` (new translation keys)
- **No backend changes**: The API contract remains unchanged; the frontend handles sign conversion
- **No database changes**
