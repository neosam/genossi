## MODIFIED Requirements

### Requirement: Timestamp worker WebDAV responsibility
The timestamp worker SHALL NOT perform WebDAV uploads. The .tsr token upload to WebDAV is handled by the backup worker. The timestamp worker only creates timestamps and stores them locally in the database.

#### Scenario: Timestamp created
- **WHEN** the timestamp worker creates a timestamp successfully
- **THEN** the token is stored in the database with `webdav_path = NULL`; WebDAV upload is deferred to the backup worker

### Requirement: AuditTimestampDao additional methods
The AuditTimestampDao SHALL provide methods to support the backup worker's .tsr upload:
- `get_pending_upload()`: Returns all entries where `webdav_path IS NULL AND status = 'success'`
- `update_webdav_path(id, path)`: Sets the `webdav_path` for a given entry

#### Scenario: Query pending uploads
- **WHEN** the backup worker queries for pending uploads
- **THEN** only entries with status "success" and no webdav_path are returned

#### Scenario: Mark as uploaded
- **WHEN** a .tsr file is uploaded successfully
- **THEN** the backup worker calls `update_webdav_path` with the WebDAV path
