## 1. DAO Layer

- [x] 1.1 Define `MemberStatus` enum (`Normal`, `FehlerhaftErfasst`) in `genossi_dao/src/member.rs`
- [x] 1.2 Add `status: MemberStatus` field to `MemberEntity` with default `Normal`
- [x] 1.3 Implement SQLite TEXT serialization/deserialization for `MemberStatus` in `genossi_dao_impl_sqlite`
- [x] 1.4 Update `count_active` query to exclude members where `status != 'Normal'`
- [x] 1.5 Create SQLite migration adding `status TEXT NOT NULL DEFAULT 'Normal'` to member table

## 2. Service Layer

- [x] 2.1 Pass `status` field through service layer create/update operations
- [x] 2.2 Update any active-member filtering logic in service layer to respect `status`

## 3. REST Layer

- [x] 3.1 Add `status` field to `MemberTO` in REST types with serde serialization
- [x] 3.2 Add `MemberStatus` to OpenAPI schema via utoipa
- [x] 3.3 Ensure create/update endpoints accept `status` field (optional, defaults to `Normal`)

## 4. Frontend

- [x] 4.1 Add `status` field to frontend member model/REST types
- [x] 4.2 Display member status in member list with visual marker for `FehlerhaftErfasst`
- [x] 4.3 Add status selection to member create/edit form

## 5. Tests

- [x] 5.1 Unit tests for `MemberStatus` enum serialization/deserialization
- [x] 5.2 Test that `count_active` excludes `FehlerhaftErfasst` members
- [x] 5.3 E2E test: create member with `FehlerhaftErfasst` status, verify not in active count
- [x] 5.4 E2E test: update existing member status to `FehlerhaftErfasst`, verify exclusion
- [x] 5.5 E2E test: create member without status, verify default `Normal`
