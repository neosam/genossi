## Purpose

Static document management -- upload, storage, listing, downloading, and soft-deletion of files (PDFs, images) that can be attached to mail jobs or referenced elsewhere in the system.

## Requirements

### Requirement: StaticDocument data model
The system SHALL store static documents as entities with the following fields:
- `id` (UUID, system-generated, primary key)
- `created` (DateTime, system-generated)
- `deleted` (Optional DateTime, for soft delete)
- `version` (UUID, for optimistic locking)
- `name` (TEXT, required): human-readable name shown in UI
- `filename` (TEXT, required): original filename at upload time
- `content_type` (TEXT, required): MIME type
- `size_bytes` (INTEGER, required): file size in bytes

File bytes SHALL be stored on the filesystem under `<STATIC_DOCUMENTS_PATH>/<id>`, where `<id>` is the UUID rendered without extension. `STATIC_DOCUMENTS_PATH` is an environment variable with default `./data/static_documents`.

#### Scenario: Upload persists metadata and file
- **WHEN** a valid document is uploaded
- **THEN** the system writes the file to `<STATIC_DOCUMENTS_PATH>/<id>` and stores a StaticDocument entity with all metadata fields populated

#### Scenario: Missing storage directory
- **WHEN** the server starts and `STATIC_DOCUMENTS_PATH` does not exist and cannot be created
- **THEN** the system fails to start with a descriptive error

### Requirement: Upload REST endpoint
The system SHALL expose `POST /api/static-documents` accepting `multipart/form-data` with a `file` field and optional `name` field. The endpoint requires authenticated board-level permissions.

#### Scenario: Successful upload
- **WHEN** an authorized user uploads a valid PDF via `POST /api/static-documents`
- **THEN** the system stores the file, creates the StaticDocument entity, and returns the entity as JSON

#### Scenario: Upload without authorization
- **WHEN** an unauthenticated or insufficiently privileged user calls `POST /api/static-documents`
- **THEN** the system returns a 401 or 403 error and does not store anything

#### Scenario: Upload with missing file field
- **WHEN** `POST /api/static-documents` is called without a `file` part
- **THEN** the system returns a 422 validation error

### Requirement: Upload validation
The system SHALL validate uploads before persisting. The content type MUST be on the allow-list (`application/pdf`, `image/png`, `image/jpeg`). The file size MUST NOT exceed the configured maximum (default 10 MB, overridable via `STATIC_DOCUMENTS_MAX_BYTES`).

#### Scenario: Disallowed content type
- **WHEN** a user uploads a file with content type `application/x-msdownload`
- **THEN** the system rejects the upload with a validation error and stores nothing

#### Scenario: File too large
- **WHEN** a user uploads a file larger than the configured maximum
- **THEN** the system rejects the upload with a validation error and stores nothing

#### Scenario: Valid upload within limits
- **WHEN** a user uploads a 2 MB PDF
- **THEN** the system accepts and stores the document

### Requirement: List REST endpoint
The system SHALL expose `GET /api/static-documents` returning all non-deleted StaticDocument entities, ordered alphabetically by `name`.

#### Scenario: List documents
- **WHEN** `GET /api/static-documents` is called
- **THEN** the system returns a JSON array of StaticDocument entities where `deleted` is NULL

#### Scenario: Empty list
- **WHEN** `GET /api/static-documents` is called and no documents exist
- **THEN** the system returns an empty array

### Requirement: Download REST endpoint
The system SHALL expose `GET /api/static-documents/{id}` returning the file bytes with the stored content type and `Content-Disposition: attachment; filename="<filename>"` header.

#### Scenario: Download existing document
- **WHEN** `GET /api/static-documents/{id}` is called with a valid id
- **THEN** the system streams the file content with the correct content type and attachment disposition

#### Scenario: Download non-existing document
- **WHEN** `GET /api/static-documents/{id}` is called with an unknown id
- **THEN** the system returns 404

#### Scenario: Download soft-deleted document
- **WHEN** `GET /api/static-documents/{id}` is called with an id whose `deleted` field is set
- **THEN** the system returns 404

### Requirement: Delete REST endpoint
The system SHALL expose `DELETE /api/static-documents/{id}` performing a soft delete by setting the `deleted` timestamp. The endpoint requires board-level permissions. The file on the filesystem SHALL remain in place.

#### Scenario: Soft delete succeeds
- **WHEN** an authorized user calls `DELETE /api/static-documents/{id}` with a valid id
- **THEN** the system sets `deleted` on the entity and the document no longer appears in list results

#### Scenario: Delete without authorization
- **WHEN** an unauthorized user calls `DELETE /api/static-documents/{id}`
- **THEN** the system returns 401 or 403

### Requirement: Storage directory bootstrap
On server startup, the system SHALL ensure the configured `STATIC_DOCUMENTS_PATH` directory exists and is writable, creating it if necessary.

#### Scenario: Directory created on startup
- **WHEN** the server starts and the configured directory does not exist
- **THEN** the system creates the directory before accepting requests

#### Scenario: Directory not writable
- **WHEN** the server starts and the configured directory exists but is not writable
- **THEN** the server fails to start with a descriptive error
