## 1. i18n Keys

- [x] 1.1 Add new i18n keys to `genossi-frontend/src/i18n/mod.rs`: `SharesAdd`, `SharesRemove`, `SharesReceive`, `SharesTransfer`
- [x] 1.2 Add German translations in `de.rs`
- [x] 1.3 Add English translations in `en.rs`
- [x] 1.4 ~~Add Czech translations in `cs.rs`~~ Skipped: no Czech locale exists

## 2. Action Type Helper

- [x] 2.1 Add a `needs_shares_input()` method or equivalent on `ActionTypeTO` in `genossi-frontend/rest-types/src/lib.rs` that returns false for status actions and Note

## 3. Form Field Visibility

- [x] 3.1 Update the shares_change field visibility condition in `member_details.rs` to use the new `needs_shares_input()` check instead of `!is_status`
- [x] 3.2 Use the dynamic i18n label based on action type instead of the static `SharesChange` key
- [x] 3.3 Add `min: "1"` to the shares input element

## 4. Sign Conversion

- [x] 4.1 Update the submit handler to negate shares_change for Verkauf and UebertragungAbgabe before building the `MemberActionTO`
- [x] 4.2 Update the edit load handler to set `action_shares_change` to `shares_change.abs()` when populating the form from an existing action
- [x] 4.3 Clamp the `oninput` handler to ensure parsed values are at least 1

## 5. Testing

- [x] 5.1 Verify the frontend compiles and all existing tests pass (`cargo test` in workspace, `cargo clippy` in frontend)
