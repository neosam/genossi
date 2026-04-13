## 1. DAO Layer

- [x] 1.1 Add `Note` variant to `ActionType` enum in `genossi_dao/src/member_action.rs`
- [x] 1.2 Add validation methods for `Note` (is_note, shares_change = 0, no transfer, no effective_date)
- [x] 1.3 Update SQLite serialization/deserialization for `Note` in `genossi_dao_impl_sqlite/src/member_action.rs`

## 2. Service Layer

- [x] 2.1 Add `Note` validation in `genossi_service_impl/src/member_action.rs` (shares_change = 0, comment required, no transfer_member_id, no effective_date)
- [x] 2.2 Exclude `Note` from migration action count calculation
- [x] 2.3 Ensure date derivation ignores `Note` actions

## 3. REST Layer

- [x] 3.1 Add `Note` variant to `ActionTypeTO` in `genossi_rest_types/src/lib.rs`

## 4. Frontend

- [x] 4.1 Add `Note` to action type selector and display in the member actions UI

## 5. Tests

- [x] 5.1 Add unit tests for `Note` validation (valid note, missing comment, non-zero shares, with transfer_member_id, with effective_date)
- [x] 5.2 Add integration/e2e test for creating and retrieving a `Note` action via REST API
- [x] 5.3 Verify migration status excludes `Note` actions from count
