## MODIFIED Requirements

### Requirement: WebDAV backup worker
The backup worker SHALL perform two additional steps after the existing backup cycle:

1. Export the audit log as `audit-log.csv` in the backup root directory
2. Upload pending .tsr timestamp tokens to `audit-timestamps/` subdirectory

#### Scenario: Backup cycle with audit data
- **WHEN** a backup cycle runs successfully
- **THEN** the worker exports `audit-log.csv` and uploads any pending .tsr files, then reports the results in the status message

#### Scenario: .tsr upload updates database
- **WHEN** a .tsr file is successfully uploaded to WebDAV
- **THEN** the worker updates the `webdav_path` field of the corresponding `audit_timestamp` record

#### Scenario: .tsr upload failure
- **WHEN** a .tsr file upload fails
- **THEN** the worker logs a warning and continues with the next file; the record's `webdav_path` remains NULL for retry on the next cycle

#### Scenario: No pending .tsr files
- **WHEN** all audit_timestamp records already have a webdav_path set
- **THEN** the worker skips the .tsr upload step
