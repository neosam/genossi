## ADDED Requirements

### Requirement: Backup page accessible via navigation
The system SHALL provide a dedicated page at route `/backup` accessible via the TopBar navigation. The navigation entry SHALL only be visible to users with the `export_backup` privilege.

#### Scenario: Navigation visible with privilege
- **WHEN** a user with `export_backup` privilege views any page
- **THEN** a "Backup" link appears in the TopBar navigation

#### Scenario: Navigation hidden without privilege
- **WHEN** a user without `export_backup` privilege views any page
- **THEN** no "Backup" link appears in the TopBar navigation

### Requirement: Member list CSV download with date picker
The backup page SHALL display a card for downloading the member list as CSV. The card SHALL contain a date input for the Stichtag and a download button. The date input SHALL default to the current date.

When the user clicks the download button, the system SHALL trigger a file download from `GET /api/backup/members?date=<selected-date>`.

#### Scenario: Download member list with default date
- **WHEN** user navigates to `/backup` and clicks the member list download button without changing the date
- **THEN** a CSV download starts with the current date as Stichtag

#### Scenario: Download member list with custom date
- **WHEN** user selects date 2025-06-01 and clicks the download button
- **THEN** a CSV download starts with `date=2025-06-01`

### Requirement: Actions CSV download
The backup page SHALL display a card for downloading all member actions as CSV. The card SHALL contain a download button.

When the user clicks the download button, the system SHALL trigger a file download from `GET /api/backup/actions`.

#### Scenario: Download actions
- **WHEN** user clicks the actions download button
- **THEN** a CSV file download starts

### Requirement: Documents ZIP download with size warning
The backup page SHALL display a card for downloading all member documents as ZIP. The card SHALL contain a download button and a warning that the download may take several minutes and be several hundred MB in size.

When the user clicks the download button, the system SHALL trigger a file download from `GET /api/backup/documents`.

#### Scenario: Download documents
- **WHEN** user clicks the documents download button
- **THEN** a ZIP file download starts

#### Scenario: Warning displayed
- **WHEN** user views the documents download card
- **THEN** a warning about download size and duration is visible
