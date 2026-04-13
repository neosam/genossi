## 1. Database Migration

- [x] 1.1 Create SQLite migration adding `salutation TEXT` and `title TEXT` columns to the `member` table

## 2. Backend DAO Layer

- [x] 2.1 Add `Salutation` enum to `genossi_dao/src/member.rs` with variants `Herr`, `Frau`, `Firma` and `as_str()`/`from_str()` methods
- [x] 2.2 Add `salutation: Option<Salutation>` and `title: Option<Arc<str>>` fields to `MemberEntity`
- [x] 2.3 Update SQLite DAO implementation (`genossi_dao_impl_sqlite/src/member.rs`) to read/write the new fields

## 3. Backend Service Layer

- [x] 3.1 Add `Salutation` enum and fields `salutation`/`title` to `Member` in `genossi_service/src/member.rs`
- [x] 3.2 Update `From` implementations between `MemberEntity` and `Member`

## 4. Backend REST Layer

- [x] 4.1 Add `SalutationTO` enum to `genossi_rest_types/src/lib.rs` with Serialize/Deserialize/ToSchema derives
- [x] 4.2 Add `salutation: Option<SalutationTO>` and `title: Option<String>` to `MemberTO`
- [x] 4.3 Update `From` implementations between service types and REST types
- [x] 4.4 Verify OpenAPI schema includes the new fields in Swagger UI

## 5. Frontend REST Types

- [x] 5.1 Add `SalutationTO` enum to `genossi-frontend/rest-types/src/lib.rs` with Serialize/Deserialize and helper methods (`all()`, `as_str()`, `from_str()`)
- [x] 5.2 Add `salutation` and `title` fields to frontend `MemberTO`

## 6. Frontend Member List

- [x] 6.1 Add `salutation` column to `ALL_COLUMNS` in `genossi-frontend/src/columns.rs` as select/dropdown type
- [x] 6.2 Add `title` column to `ALL_COLUMNS` as editable text type
- [x] 6.3 Implement dropdown rendering for salutation in the member list inline editing

## 7. Frontend Member Detail Page

- [x] 7.1 Add salutation dropdown field to the member detail form
- [x] 7.2 Add title text input field to the member detail form

## 8. Tests

- [x] 8.1 Add/update unit tests for Salutation enum (DAO layer: `as_str`/`from_str` roundtrip, invalid value handling)
- [x] 8.2 Add/update unit tests for service layer `From` conversions with salutation and title
- [x] 8.3 Add/update E2E tests: create member with salutation/title, read back, update, verify persistence
