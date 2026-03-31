## Context

Members are currently managed in an Excel spreadsheet. The system already has a full CRUD API for members (`MemberEntity`). We need to bridge the gap by allowing bulk import from `.xlsx` files via the existing REST API.

The existing architecture uses a layered pattern: DAO → Service → REST. The import follows this pattern, with Excel parsing logic living in the service layer.

## Goals / Non-Goals

**Goals:**
- Import members from `.xlsx` files via a single REST endpoint
- Upsert semantics: create new members, update existing ones (matched by `member_number`)
- Tolerant date parsing (multiple formats)
- Structured error reporting per row
- Accessible via Swagger UI for manual imports

**Non-Goals:**
- Importing columns that don't map to existing `MemberEntity` fields (9 columns ignored)
- Extending the data model for unmapped columns
- `.xls` (legacy format) support
- Scheduled/automated imports
- Export functionality

## Decisions

### 1. Excel parsing with `calamine`

**Decision**: Use `calamine` crate in `genossi_service_impl`.

**Rationale**: `calamine` is the most mature Rust crate for reading Excel files. It supports `.xlsx`, `.xls`, and `.ods`. It reads from `&[u8]`, so we can pass the uploaded bytes directly without temp files.

**Alternatives considered**:
- `umya-spreadsheet`: More focused on writing Excel files
- Requiring CSV upload: Adds manual step for the user

### 2. Column matching by header name

**Decision**: Match columns by their header text in the first row, not by position.

**Rationale**: The Excel file has specific German column names (`Nachname`, `Vorname(n)`, etc.). Matching by name is resilient to column reordering and makes the mapping explicit. Unknown columns are silently ignored.

### 3. Upsert via `member_number`

**Decision**: Use `member_number` (Excel column `ID1`) as the match key. If a member with that number exists, update all fields. Otherwise create a new member.

**Rationale**: `member_number` is the natural business key and has a unique index. UUID-based matching is not practical since UUIDs don't exist in the source spreadsheet.

### 4. Date parsing strategy

**Decision**: Try multiple formats in order:
1. Excel serial number (float, e.g. `44927.0`)
2. `DD.MM.YYYY` (German format)
3. `YYYY-MM-DD` (ISO format)

**Rationale**: Excel internally stores dates as serial numbers, but `calamine` may return them as strings or floats depending on cell formatting. Supporting all three covers the common cases.

### 5. Error handling: continue on row errors

**Decision**: Collect per-row errors and continue processing. Return all errors in the response alongside success counts.

**Rationale**: A single bad row shouldn't block importing hundreds of valid members. The caller can review errors and fix the source data.

### 6. Transaction scope

**Decision**: Run the entire import in a single database transaction.

**Rationale**: Either the whole import succeeds or nothing changes. This prevents partial imports that are hard to debug. If any database error occurs, the entire transaction rolls back.

**Trade-off**: Parse errors are collected and those rows are skipped - only rows that parse successfully are included in the transaction. A database-level error (e.g. constraint violation) rolls back everything.

### 7. Service trait design

**Decision**: Add `import_members` method to a new `MemberImportService` trait in `genossi_service`, taking `&[u8]` (the raw file bytes).

**Rationale**: Keeps the import concern separate from the existing `MemberService` CRUD trait. The REST layer handles multipart extraction and passes raw bytes to the service.

## Risks / Trade-offs

- **Large files**: No explicit size limit. Very large spreadsheets could use significant memory since `calamine` loads the whole sheet. → Mitigation: Axum's multipart has configurable size limits. For the expected member count (hundreds, not millions), this is fine.
- **Encoding**: Excel files should handle UTF-8 correctly via `calamine`, but unusual encodings in cell values could cause issues. → Mitigation: `calamine` handles Excel's internal encoding; unlikely to be a problem.
- **Concurrent imports**: Two simultaneous imports could conflict on member_number upserts. → Mitigation: Single transaction with unique constraint handles this at the DB level.
