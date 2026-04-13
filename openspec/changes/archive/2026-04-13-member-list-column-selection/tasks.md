## 1. Database & Migration

- [x] 1.1 Create SQLite migration for `user_preferences` table (id, user_id, key, value, created, deleted, version, UNIQUE(user_id, key))
- [x] 1.2 Run migration and verify table creation

## 2. Backend DAO Layer

- [x] 2.1 Define `UserPreferenceDao` trait in `genossi_dao` with `dump_all()`, `create()`, `update()` methods
- [x] 2.2 Define `UserPreference` entity struct in `genossi_dao`
- [x] 2.3 Implement `UserPreferenceDaoImpl` in `genossi_dao_impl_sqlite` with SQLx queries
- [x] 2.4 Add `find_by_user_and_key()` method to DAO for looking up preferences by user_id + key
- [x] 2.5 Write unit tests for DAO implementation

## 3. Backend Service Layer

- [x] 3.1 Define `UserPreferenceService` trait in `genossi_service` with `get_by_key()` and `upsert()` methods
- [x] 3.2 Implement `UserPreferenceServiceImpl` in `genossi_service_impl` with upsert logic (create if not exists, update if exists)
- [x] 3.3 Ensure `user_id` is derived from auth context, not from request body
- [x] 3.4 Write unit tests for service implementation using mockall

## 4. Backend REST Layer

- [x] 4.1 Define `UserPreferenceTO` transfer object in `genossi_rest_types`
- [x] 4.2 Implement `GET /api/user-preferences/{key}` endpoint in `genossi_rest`
- [x] 4.3 Implement `PUT /api/user-preferences/{key}` endpoint with upsert behavior
- [x] 4.4 Add OpenAPI documentation via Utoipa
- [x] 4.5 Register routes in the REST server configuration
- [x] 4.6 Write E2E tests for both endpoints (get, upsert, not-found)

## 5. Frontend Column Registry

- [x] 5.1 Create column definition struct (`ColumnDef`) with key, label_key, editable flag, and render function
- [x] 5.2 Define static column registry with all MemberTO display fields
- [x] 5.3 Define default column set constant

## 6. Frontend Preferences API

- [x] 6.1 Add `get_user_preference(key)` and `set_user_preference(key, value)` functions to frontend API module
- [x] 6.2 Add preference state signal for column selection

## 7. Frontend Column Picker UI

- [x] 7.1 Create column picker popover component with checkboxes for all available columns
- [x] 7.2 Add "Spalten" button to member list toolbar
- [x] 7.3 Wire checkbox changes to update column selection state and persist to backend

## 8. Frontend Dynamic Table Rendering

- [x] 8.1 Refactor table header rendering to iterate over selected columns from registry
- [x] 8.2 Refactor table body rendering to dynamically render cells based on selected columns
- [x] 8.3 Load saved column preference on page mount and apply to table
- [x] 8.4 Ensure checkbox column is always prepended in normal mode
- [x] 8.5 Verify fallback to default columns when preference is missing or contains invalid keys
