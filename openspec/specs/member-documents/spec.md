## MODIFIED Requirements

### Requirement: Create member document
The system SHALL perform member document creation via `audited_create!` macro, logging all non-None document fields to the audit log.

#### Scenario: Document upload audit
- **WHEN** an authenticated user uploads a document for a member
- **THEN** the system creates the document record and creates audit log entries for all non-None document fields with action "create"

### Requirement: Update member document
The system SHALL perform member document updates via `audited_update!` macro, logging only changed fields.

#### Scenario: Document metadata update audit
- **WHEN** an authenticated user updates a document's description
- **THEN** the system updates the document and creates an audit log entry for the description field

### Requirement: Delete member document
The system SHALL perform member document soft-deletes via `audited_delete!` macro, logging all audited field values.

#### Scenario: Document deletion audit
- **WHEN** an authenticated user deletes a member document
- **THEN** the system soft-deletes the document and creates audit log entries with action "delete" for all audited fields
