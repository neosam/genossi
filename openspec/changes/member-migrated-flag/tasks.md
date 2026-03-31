## 1. Database Migration

- [x] 1.1 Create SQL migration to add `migrated BOOLEAN NOT NULL DEFAULT 0` column to `members` table

## 2. DAO Layer

- [x] 2.1 Add `migrated: bool` field to `MemberEntity` in `genossi_dao/src/member.rs`
- [x] 2.2 Update SQLite member DAO queries (dump_all, create, update) to include `migrated` field in `genossi_dao_impl_sqlite/src/member.rs`
- [x] 2.3 Add `update_migrated(member_id, migrated: bool, tx)` method to `MemberDao` trait and SQLite implementation

## 3. Service Layer - Recalculation Logic

- [x] 3.1 Extract migration status calculation from `MemberActionService::migration_status()` into a shared helper function
- [x] 3.2 Add `recalc_migrated(member_id, tx)` method that uses the shared helper and writes via `MemberDao::update_migrated()`
- [x] 3.3 Call `recalc_migrated()` after action create in `MemberActionService`
- [x] 3.4 Call `recalc_migrated()` after action update in `MemberActionService`
- [x] 3.5 Call `recalc_migrated()` after action delete in `MemberActionService`
- [x] 3.6 Call `recalc_migrated()` after member update in `MemberService` (when `current_shares` or `action_count` change)

## 4. REST Layer

- [x] 4.1 Add `migrated: bool` field to `MemberTO` in `genossi_rest_types/src/lib.rs`
- [x] 4.2 Update `MemberTO` conversion (From impl) to map the `migrated` field

## 5. Frontend

- [x] 5.1 Add `migrated: bool` field to frontend `MemberTO` in `genossi-frontend/rest-types/src/lib.rs`
- [x] 5.2 Add migration status column/badge to the member list table in `genossi-frontend/src/page/members.rs`
- [x] 5.3 Add i18n translations for migration status labels (de/en)

## 6. Tests

- [x] 6.1 Add unit tests for `recalc_migrated()` logic (migrated true/false scenarios)
- [x] 6.2 Add E2E test: create member + actions → verify `migrated` flag in list response
- [x] 6.3 Add E2E test: update member shares → verify `migrated` flag recalculation
