## Why

The member data currently lives in an Excel spreadsheet. To migrate existing members into the system, we need an import endpoint that reads `.xlsx` files and creates or updates member records. This avoids manual re-entry of potentially hundreds of members.

## What Changes

- New REST endpoint `POST /api/members/import` accepting `.xlsx` file upload (multipart/form-data)
- Parses Excel columns and maps them to existing `MemberEntity` fields
- Upsert logic: if `member_number` already exists, update the record; otherwise create
- Tolerant date parsing supporting `DD.MM.YYYY`, `YYYY-MM-DD`, and Excel serial numbers
- Returns a structured result with counts (imported, updated, skipped) and per-row errors
- Requires `manage_members` privilege
- Adds `calamine` crate dependency for `.xlsx` parsing

## Capabilities

### New Capabilities
- `excel-member-import`: Import members from `.xlsx` files via REST endpoint with upsert semantics, tolerant parsing, and structured error reporting

### Modified Capabilities

## Impact

- **genossi_service**: New import trait method
- **genossi_service_impl**: Import implementation with `calamine` dependency for Excel parsing
- **genossi_rest**: New `/api/members/import` endpoint with multipart handling
- **genossi_rest_types**: Import result response type
- **genossi_bin**: Wire up import service
- **Cargo.toml**: Add `calamine` workspace dependency
- **OpenAPI/Swagger**: New endpoint documented
