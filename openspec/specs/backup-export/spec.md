## ADDED Requirements

### Requirement: Export member list as CSV with Stichtag
The system SHALL provide an endpoint `GET /api/backup/members?date=<ISO-date>` that returns a CSV file containing all members who were active at the given Stichtag. The `date` query parameter is required. The response SHALL have `Content-Type: text/csv` and `Content-Disposition: attachment` headers. The CSV SHALL be UTF-8 encoded with BOM for Excel compatibility.

The CSV SHALL include the following columns: Mitgliedsnummer, Anrede, Titel, Vorname, Nachname, Firma, Strasse, Hausnummer, PLZ, Ort, Email, Bankverbindung, Beitrittsdatum, Austrittsdatum, Anteile bei Beitritt, Anteile am Stichtag, Kommentar.

The "Anteile am Stichtag" column SHALL be calculated as `shares_at_joining + SUM(shares_change)` from all non-deleted actions with `date <= Stichtag`.

A member SHALL appear in the export if: `join_date <= Stichtag` AND (`exit_date IS NULL` OR `exit_date > Stichtag`) AND `deleted IS NULL` AND `status != FehlerhaftErfasst`.

#### Scenario: Export members at current date
- **WHEN** user requests `GET /api/backup/members?date=2026-04-12` with `export_backup` privilege
- **THEN** system returns a CSV file with all currently active members and their current share counts

#### Scenario: Export members at historical date
- **WHEN** user requests `GET /api/backup/members?date=2025-06-01` with `export_backup` privilege
- **THEN** system returns a CSV with members active on 2025-06-01, and "Anteile am Stichtag" reflects only actions up to that date

#### Scenario: Exited member excluded after exit date
- **WHEN** a member has `exit_date = 2025-03-15` and user requests `date=2025-04-01`
- **THEN** that member SHALL NOT appear in the CSV

#### Scenario: Exited member included before exit date
- **WHEN** a member has `exit_date = 2025-03-15` and user requests `date=2025-03-01`
- **THEN** that member SHALL appear in the CSV

#### Scenario: Missing date parameter
- **WHEN** user requests `GET /api/backup/members` without a `date` parameter
- **THEN** system returns HTTP 400 Bad Request

#### Scenario: Unauthorized access
- **WHEN** user without `export_backup` privilege requests the endpoint
- **THEN** system returns HTTP 403 Forbidden

### Requirement: Export all member actions as CSV
The system SHALL provide an endpoint `GET /api/backup/actions` that returns a CSV file containing all non-deleted member actions across all members. The response SHALL have `Content-Type: text/csv` and `Content-Disposition: attachment` headers. The CSV SHALL be UTF-8 encoded with BOM.

The CSV SHALL include the following columns: Mitgliedsnummer, Vorname, Nachname, Aktionstyp, Datum, Anteileaenderung, Uebertragung-Mitgliedsnummer, Wirksamkeitsdatum, Kommentar.

Actions SHALL be ordered by member number, then by action date.

#### Scenario: Export all actions
- **WHEN** user requests `GET /api/backup/actions` with `export_backup` privilege
- **THEN** system returns a CSV file with all non-deleted actions including member names

#### Scenario: Actions include member names
- **WHEN** the CSV is generated
- **THEN** each row SHALL contain the first name and last name of the member the action belongs to

#### Scenario: Unauthorized access
- **WHEN** user without `export_backup` privilege requests the endpoint
- **THEN** system returns HTTP 403 Forbidden

### Requirement: Export all member documents as ZIP
The system SHALL provide an endpoint `GET /api/backup/documents` that returns a streaming ZIP archive containing all non-deleted member documents. The response SHALL have `Content-Type: application/zip` and `Content-Disposition: attachment` headers.

The ZIP SHALL organize files in directories named `<member_number>_<last_name>_<first_name>/` with files named `<document_type>_<file_name>`.

The ZIP SHALL be streamed directly in the HTTP response without creating temporary files on the server.

#### Scenario: Export all documents
- **WHEN** user requests `GET /api/backup/documents` with `export_backup` privilege
- **THEN** system returns a streaming ZIP file with all non-deleted documents organized by member

#### Scenario: Documents organized by member
- **WHEN** member 42 (Hans Mueller) has two documents (beitrittserklaerung.pdf, aufstockung.pdf)
- **THEN** the ZIP contains `042_Mueller_Hans/join_declaration_beitrittserklaerung.pdf` and `042_Mueller_Hans/share_increase_aufstockung.pdf`

#### Scenario: Unauthorized access
- **WHEN** user without `export_backup` privilege requests the endpoint
- **THEN** system returns HTTP 403 Forbidden

### Requirement: Backup export privilege
The system SHALL define a privilege `export_backup` that controls access to all backup endpoints. The `admin` role SHALL be assigned this privilege via database migration.

#### Scenario: Admin has export privilege
- **WHEN** a user with the `admin` role accesses any backup endpoint
- **THEN** access is granted

#### Scenario: User without privilege denied
- **WHEN** a user without `export_backup` privilege accesses any backup endpoint
- **THEN** access is denied with HTTP 403
