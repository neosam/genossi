## 1. Database Migration

- [ ] 1.1 Create SQL migration to add `migrated BOOLEAN NOT NULL DEFAULT 0` column to `members` table

## 2. DAO Layer

- [ ] 2.1 Add `migrated: bool` field to `MemberEntity` in `genossi_dao/src/member.rs`
- [ ] 2.2 Update SQLite member DAO queries (dump_all, create, update) to include `migrated` field in `genossi_dao_impl_sqlite/src/member.rs`
- [ ] 2.3 Add `update_migrated(member_id, migrated: bool, tx)` method to `MemberDao` trait and SQLite implementation

## 3. Service Layer - Recalculation Logic

- [ ] 3.1 Extract migration status calculation from `MemberActionService::migration_status()` into a shared helper function
- [ ] 3.2 Add `recalc_migrated(member_id, tx)` method that uses the shared helper and writes via `MemberDao::update_migrated()`
- [ ] 3.3 Call `recalc_migrated()` after action create in `MemberActionService`
- [ ] 3.4 Call `recalc_migrated()` after action update in `MemberActionService`
- [ ] 3.5 Call `recalc_migrated()` after action delete in `MemberActionService`
- [ ] 3.6 Call `recalc_migrated()` after member update in `MemberService` (when `current_shares` or `action_count` change)

## 4. REST Layer

- [ ] 4.1 Add `migrated: bool` field to `MemberTO` in `genossi_rest_types/src/lib.rs`
- [ ] 4.2 Update `MemberTO` conversion (From impl) to map the `migrated` field

## 5. Frontend

- [ ] 5.1 Add `migrated: bool` field to frontend `MemberTO` in `genossi-frontend/rest-types/src/lib.rs`
- [ ] 5.2 Add migration status column/badge to the member list table in `genossi-frontend/src/page/members.rs`
- [ ] 5.3 Add i18n translations for migration status labels (de/en)

## 6. Tests

- [ ] 6.1 Add unit tests for `recalc_migrated()` logic (migrated true/false scenarios)
- [ ] 6.2 Add E2E test: create member + actions → verify `migrated` flag in list response
- [ ] 6.3 Add E2E test: update member shares → verify `migrated` flag recalculation
