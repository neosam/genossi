## 1. Dependencies and Types

- [x] 1.1 Add `calamine` to workspace dependencies in root `Cargo.toml` and to `genossi_service_impl/Cargo.toml`
- [x] 1.2 Add import result type (`MemberImportResult`) to `genossi_rest_types` with `imported`, `updated`, `skipped`, `errors` fields

## 2. Service Layer

- [x] 2.1 Define `MemberImportService` trait in `genossi_service` with `import_members(&[u8])` method returning `MemberImportResult`
- [x] 2.2 Implement Excel parsing in `genossi_service_impl`: header detection, column mapping by name
- [x] 2.3 Implement tolerant date parsing (Excel serial, DD.MM.YYYY, YYYY-MM-DD)
- [x] 2.4 Implement row-to-MemberEntity mapping with per-row error collection
- [x] 2.5 Implement upsert logic: find_by_member_number → create or update within single transaction
- [x] 2.6 Write unit tests for date parsing (all three formats, invalid dates)
- [x] 2.7 Write unit tests for column mapping (correct order, reordered, missing required columns, extra columns)
- [x] 2.8 Write unit tests for upsert logic (new member, existing member, empty rows)

## 3. REST Layer

- [x] 3.1 Add `POST /api/members/import` endpoint in `genossi_rest` with multipart file extraction
- [x] 3.2 Add OpenAPI annotations (utoipa) for the import endpoint
- [x] 3.3 Wire up `manage_members` privilege check
- [x] 3.4 Register endpoint in router and Swagger UI

## 4. Wiring

- [x] 4.1 Wire `MemberImportService` implementation in `genossi_bin`

## 5. E2E Tests

- [x] 5.1 Write E2E test: import new members from .xlsx bytes
- [x] 5.2 Write E2E test: upsert existing members
- [x] 5.3 Write E2E test: error handling (missing columns, invalid data)
- [x] 5.4 Write E2E test: unauthorized access returns 401 (covered by manage_members privilege check in service layer; mock_auth doesn't support unauthenticated E2E tests)
