## 1. Edit Mode State Management

- [ ] 1.1 Add `edit_mode: Signal<bool>` global signal for toggling edit mode
- [ ] 1.2 Add `row_edits: Signal<HashMap<Uuid, MemberTO>>` for tracking local edits per row
- [ ] 1.3 Add `row_errors: Signal<HashMap<Uuid, String>>` for tracking error state per row
- [ ] 1.4 Add `focused_row: Signal<Option<Uuid>>` for tracking which row has focus

## 2. Edit Mode Toggle UI

- [ ] 2.1 Add "Bearbeiten" / "Fertig" toggle button to member list toolbar
- [ ] 2.2 Hide checkbox column when edit mode is active
- [ ] 2.3 Disable row click navigation when edit mode is active
- [ ] 2.4 Clear row_edits and row_errors when exiting edit mode

## 3. Inline Input Rendering

- [ ] 3.1 Extend ColumnDef with input type information (text vs number) based on editable flag
- [ ] 3.2 Render editable cells as `<input>` fields in edit mode, using appropriate type attribute
- [ ] 3.3 Render read-only cells as plain text in edit mode (visually distinct, e.g., slightly grayed)
- [ ] 3.4 Populate input fields from row_edits map (or original MemberTO if not yet touched)
- [ ] 3.5 Update row_edits on input change events

## 4. Row Focus Tracking & Autosave

- [ ] 4.1 Add focusin event handler on table rows to track focused_row signal
- [ ] 4.2 Implement save trigger when focused_row changes from Some(old) to Some(new) or None
- [ ] 4.3 Implement dirty-check comparing row_edits entry against original MemberTO
- [ ] 4.4 Send PUT /api/members/{id} with updated MemberTO when row is dirty
- [ ] 4.5 On successful save: update MEMBERS signal with response data (new version), remove from row_edits
- [ ] 4.6 Trigger save for focused row when "Fertig" button is clicked

## 5. Error Handling

- [ ] 5.1 Create or integrate toast notification component for error messages
- [ ] 5.2 On save failure: add entry to row_errors, show toast with member-specific error message
- [ ] 5.3 Apply red background styling to rows present in row_errors
- [ ] 5.4 Clear error from row_errors when the row is successfully saved on retry

## 6. Frontend Validation

- [ ] 6.1 Validate required fields (first_name, last_name) are non-empty before sending PUT
- [ ] 6.2 Show red border on empty required field inputs
- [ ] 6.3 Show toast with validation error and skip PUT request if validation fails
- [ ] 6.4 Clear field-level validation errors when the field is filled in

## 7. Testing

- [ ] 7.1 Test edit mode toggle shows/hides checkboxes and changes button label
- [ ] 7.2 Test that only editable fields become input fields in edit mode
- [ ] 7.3 Test autosave triggers on row focus loss with dirty data
- [ ] 7.4 Test no save request when row data is unchanged
- [ ] 7.5 Test error highlighting and toast on failed save
- [ ] 7.6 Test required field validation prevents empty first_name/last_name
