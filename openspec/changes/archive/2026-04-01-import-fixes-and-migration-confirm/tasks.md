## 1. Fix Excel Import Balance Conversion

- [x] 1.1 Update balance parsing in `genossi_service_impl/src/member_import.rs` to multiply by 100 (Euro → Cent), handling both integer and decimal values
- [x] 1.2 Update unit test for balance parsing to expect cents
- [x] 1.3 Update E2E import tests with correct cent-based balance assertions

## 2. Confirm Migration Backend

- [x] 2.1 Add `confirm_migration` method to `MemberActionService` trait in `genossi_service/src/member_action.rs`
- [x] 2.2 Implement `confirm_migration` in `genossi_service_impl/src/member_action.rs`: count actual non-status actions, set `action_count = actual - 1` on member, call `recalc_migrated()`
- [x] 2.3 Add `POST /api/members/{id}/confirm-migration` endpoint in `genossi_rest/src/member_action.rs`
- [x] 2.4 Register the new route in the REST server

## 3. Confirm Migration Frontend

- [x] 3.1 Add `confirm_migration` API function in `genossi-frontend/src/api.rs`
- [x] 3.2 Add "Confirm" button to pending migration badge in `genossi-frontend/src/page/member_details.rs`
- [x] 3.3 Add i18n key for confirm button label (de/en)

## 4. Tests

- [x] 4.1 Add E2E test: confirm migration resolves action count mismatch → migrated becomes true
- [x] 4.2 Add E2E test: confirm migration when shares mismatch → migrated stays false
- [x] 4.3 Add E2E test: confirm migration for non-existent member → 404
