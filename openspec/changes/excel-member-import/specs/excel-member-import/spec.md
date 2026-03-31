## ADDED Requirements

### Requirement: Import members from Excel file
The system SHALL allow authenticated users with `manage_members` privilege to import members from an `.xlsx` file via `POST /api/members/import` using multipart/form-data.

#### Scenario: Successful import of new members
- **WHEN** a user uploads a valid `.xlsx` file containing member rows that do not exist in the system
- **THEN** the system creates a new `MemberEntity` for each valid row and returns a result with the count of imported members

#### Scenario: Insufficient privileges
- **WHEN** a user without `manage_members` privilege attempts to import
- **THEN** the system returns HTTP 401 Unauthorized

### Requirement: Column mapping by header name
The system SHALL match Excel columns to member fields by their header text in the first row. The mapping SHALL be:

| Excel Column       | MemberEntity Field  |
|--------------------|---------------------|
| ID1                | member_number       |
| Nachname           | last_name           |
| Vorname(n)         | first_name          |
| Straße             | street              |
| Nr#                | house_number        |
| PLZ                | postal_code         |
| Ort                | city                |
| Beitritt           | join_date           |
| Anteile Beitritt   | shares_at_joining   |
| Anteile aktuell    | current_shares      |
| Guthaben aktuell   | current_balance     |
| Austritt           | exit_date           |
| Email              | email               |
| Firma              | company             |
| Kommentar          | comment             |
| Bankverbindung     | bank_account        |

Unknown columns SHALL be silently ignored.

#### Scenario: Columns in different order
- **WHEN** the Excel file has the mapped columns in a non-standard order
- **THEN** the system correctly maps each column by its header text

#### Scenario: Extra columns present
- **WHEN** the Excel file contains columns not listed in the mapping (e.g., "Anzahl Aktionen", "BE HiDrive")
- **THEN** those columns are silently ignored and the import proceeds normally

#### Scenario: Missing required column
- **WHEN** the Excel file is missing the `ID1`, `Nachname`, `Vorname(n)`, or `Beitritt` column header
- **THEN** the system returns an error indicating the missing required column

### Requirement: Upsert by member number
The system SHALL use `member_number` (Excel column `ID1`) as the match key. If a member with that number already exists, all mapped fields SHALL be updated. If no member with that number exists, a new member SHALL be created.

#### Scenario: New member created
- **WHEN** a row has a `member_number` that does not exist in the system
- **THEN** a new `MemberEntity` is created with a generated UUID and version

#### Scenario: Existing member updated
- **WHEN** a row has a `member_number` that already exists in the system
- **THEN** the existing member's fields are updated and a new version UUID is generated

### Requirement: Tolerant date parsing
The system SHALL parse date fields (`Beitritt`, `Austritt`) supporting multiple formats:
1. Excel serial numbers (float values)
2. `DD.MM.YYYY` (German date format)
3. `YYYY-MM-DD` (ISO format)

#### Scenario: Date as Excel serial number
- **WHEN** a date cell contains a float value like `44927.0`
- **THEN** the system converts it to the correct calendar date

#### Scenario: Date in German format
- **WHEN** a date cell contains `01.01.2023`
- **THEN** the system parses it as January 1, 2023

#### Scenario: Date in ISO format
- **WHEN** a date cell contains `2023-01-01`
- **THEN** the system parses it as January 1, 2023

#### Scenario: Empty exit date
- **WHEN** the `Austritt` cell is empty
- **THEN** the `exit_date` field is set to `None`

### Requirement: Empty row handling
The system SHALL skip rows where all cells are empty.

#### Scenario: Empty row in middle of data
- **WHEN** the Excel file contains an empty row between data rows
- **THEN** the empty row is skipped and processing continues with the next row

### Requirement: Row-level error handling
The system SHALL collect parsing errors per row and continue processing remaining rows. Rows with errors SHALL NOT be imported.

#### Scenario: Invalid data in one row
- **WHEN** one row has an unparseable date and other rows are valid
- **THEN** the valid rows are imported and the error row is reported in the response with its row number and error message

### Requirement: Structured import result
The system SHALL return a JSON response with the import result:
- `imported`: count of newly created members
- `updated`: count of updated existing members
- `skipped`: count of skipped empty rows
- `errors`: array of objects with `row` (number) and `error` (message)

#### Scenario: Mixed import result
- **WHEN** an import file contains 10 valid new members, 3 existing members, 1 empty row, and 1 error row
- **THEN** the response contains `imported: 10`, `updated: 3`, `skipped: 1`, and `errors` with 1 entry

### Requirement: Transaction atomicity
The system SHALL execute all database operations (creates and updates) within a single transaction. If a database error occurs, the entire import SHALL be rolled back.

#### Scenario: Database error causes rollback
- **WHEN** a database error occurs during import after some members have been created
- **THEN** all changes are rolled back and no members are created or updated

### Requirement: OpenAPI documentation
The import endpoint SHALL be documented in the Swagger UI with request/response schemas.

#### Scenario: Swagger UI shows import endpoint
- **WHEN** a user navigates to `/swagger-ui/`
- **THEN** the `POST /api/members/import` endpoint is visible with its multipart request format and response schema
