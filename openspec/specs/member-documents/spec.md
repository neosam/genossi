# Member Documents

## Purpose

Manage document uploads, storage, listing, downloading, and deletion for cooperative members. Documents are stored on the filesystem and tracked via database records, with support for singleton and multi-instance document types.

## Requirements

### Requirement: Document types
The system SHALL support the following document types for member documents:
- `join_declaration` (Beitrittserklärung) — singleton per member
- `join_confirmation` (Beitrittsbestätigung) — singleton per member
- `share_increase` (Aufstockung) — multiple per member
- `other` (Sonstige) — multiple per member, requires a description

#### Scenario: Singleton type allows only one active document
- **WHEN** a member already has an active document of type `join_declaration`
- **AND** a new document of type `join_declaration` is uploaded for that member
- **THEN** the existing document SHALL be soft-deleted
- **AND** the new document SHALL be created as the active document

#### Scenario: Multi type allows multiple active documents
- **WHEN** a member already has an active document of type `share_increase`
- **AND** a new document of type `share_increase` is uploaded for that member
- **THEN** both documents SHALL remain active

#### Scenario: Other type requires description
- **WHEN** a document of type `other` is uploaded without a description
- **THEN** the system SHALL reject the upload with a validation error

### Requirement: Document upload
The system SHALL allow uploading documents for a member via multipart/form-data POST to `/members/{member_id}/documents`. The upload MUST include the file, the document type, and optionally a description.

#### Scenario: Successful upload
- **WHEN** a board member uploads a PDF file with type `join_declaration` for member with ID `{member_id}`
- **THEN** the system SHALL store the file on the filesystem at `{DOCUMENT_STORAGE_PATH}/{document_uuid}.{extension}`
- **AND** the system SHALL create a database record with the document metadata
- **AND** the system SHALL return the document metadata as JSON with status 201

#### Scenario: Upload exceeds size limit
- **WHEN** a file larger than 50 MB is uploaded
- **THEN** the system SHALL reject the upload with a 400 error

#### Scenario: Upload for non-existent member
- **WHEN** a document is uploaded for a member ID that does not exist
- **THEN** the system SHALL return a 404 error

### Requirement: Document listing
The system SHALL allow listing all active (non-deleted) documents for a member via GET `/members/{member_id}/documents`.

#### Scenario: List documents for a member
- **WHEN** a board member requests the document list for a member
- **THEN** the system SHALL return a JSON array of document metadata (id, member_id, document_type, description, file_name, mime_type, created)
- **AND** soft-deleted documents SHALL NOT be included

#### Scenario: List documents for member with no documents
- **WHEN** a board member requests the document list for a member with no documents
- **THEN** the system SHALL return an empty JSON array

### Requirement: Document download
The system SHALL allow downloading a document via GET `/members/{member_id}/documents/{document_id}`. The response MUST include the raw file bytes with the correct Content-Type header and Content-Disposition header with the original filename.

#### Scenario: Successful download
- **WHEN** a board member requests a document by ID
- **THEN** the system SHALL return the file bytes with the stored MIME type as Content-Type
- **AND** the Content-Disposition header SHALL include the original filename

#### Scenario: Download deleted document
- **WHEN** a board member requests a document that has been soft-deleted
- **THEN** the system SHALL return a 404 error

#### Scenario: Download non-existent document
- **WHEN** a board member requests a document ID that does not exist
- **THEN** the system SHALL return a 404 error

### Requirement: Document deletion
The system SHALL allow soft-deleting a document via DELETE `/members/{member_id}/documents/{document_id}`. The database record SHALL be marked with a deleted timestamp. The file on the filesystem SHALL NOT be deleted.

#### Scenario: Successful deletion
- **WHEN** a board member deletes a document
- **THEN** the system SHALL set the `deleted` timestamp on the database record
- **AND** the file SHALL remain on the filesystem
- **AND** the system SHALL return status 204

#### Scenario: Delete already deleted document
- **WHEN** a board member deletes a document that is already soft-deleted
- **THEN** the system SHALL return a 404 error

### Requirement: Board-only access
All document endpoints SHALL require board member (Vorstand) permissions. Non-board users SHALL receive a 403 error.

#### Scenario: Non-board member attempts upload
- **WHEN** a non-board member attempts to upload a document
- **THEN** the system SHALL return a 403 error

#### Scenario: Board member accesses documents
- **WHEN** a board member accesses any document endpoint
- **THEN** the system SHALL allow the operation

### Requirement: Filesystem storage configuration
The system SHALL read the document storage base path from the `DOCUMENT_STORAGE_PATH` environment variable. If the variable is not set, the system SHALL use a default path of `./documents`.

#### Scenario: Custom storage path
- **WHEN** `DOCUMENT_STORAGE_PATH` is set to `/var/data/genossi/documents`
- **THEN** documents SHALL be stored under `/var/data/genossi/documents/`

#### Scenario: Default storage path
- **WHEN** `DOCUMENT_STORAGE_PATH` is not set
- **THEN** documents SHALL be stored under `./documents/`

#### Scenario: Storage directory creation
- **WHEN** the configured storage directory does not exist
- **THEN** the system SHALL create it on first upload

### Requirement: Frontend document section
The member details page SHALL include a "Dokumente" (Documents) section after the Actions section. It SHALL display a table of documents and provide an upload form.

#### Scenario: Display document list
- **WHEN** a board member views the member details page
- **THEN** the system SHALL display a table with columns: Typ, Dateiname, Hochgeladen, Aktionen (Download, Löschen)

#### Scenario: Upload via frontend
- **WHEN** a board member selects a document type, chooses a file, and clicks upload
- **THEN** the frontend SHALL send a multipart POST request to the backend
- **AND** the document list SHALL refresh after successful upload

#### Scenario: Download via frontend
- **WHEN** a board member clicks the download button on a document
- **THEN** the browser SHALL download the file with its original filename

#### Scenario: Delete via frontend
- **WHEN** a board member clicks the delete button on a document
- **THEN** the frontend SHALL send a DELETE request
- **AND** the document list SHALL refresh after successful deletion
