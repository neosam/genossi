## ADDED Requirements

### Requirement: Generate and store document
The system SHALL allow generating a PDF from a Typst template and storing it as a MemberDocument via `POST /api/members/{member_id}/documents/generate/{document_type}`. The `document_type` SHALL be mapped to a template file and a `DocumentType` enum value.

#### Scenario: Successful generation and storage
- **WHEN** a board member sends `POST /api/members/{member_id}/documents/generate/join_confirmation`
- **AND** the member exists
- **AND** the member has no existing `JoinConfirmation` document
- **AND** the template `join_confirmation.typ` exists
- **THEN** the system SHALL render the template with the member's data
- **AND** store the resulting PDF as a MemberDocument with type `JoinConfirmation`
- **AND** store the file on the filesystem at `{DOCUMENT_STORAGE_PATH}/{document_uuid}.pdf`
- **AND** return the document metadata as JSON with status 201

#### Scenario: Document of that type already exists
- **WHEN** a board member sends `POST /api/members/{member_id}/documents/generate/join_confirmation`
- **AND** the member already has an active `JoinConfirmation` document
- **THEN** the system SHALL return HTTP 409 Conflict

#### Scenario: Non-existent member
- **WHEN** a board member sends the generate request for a member ID that does not exist
- **THEN** the system SHALL return HTTP 404

#### Scenario: Unknown document type
- **WHEN** a board member sends the generate request with an unrecognized document type
- **THEN** the system SHALL return HTTP 400

#### Scenario: Template not found
- **WHEN** a board member sends the generate request for a valid document type
- **AND** the corresponding template file does not exist on disk
- **THEN** the system SHALL return HTTP 404

#### Scenario: Template compilation error
- **WHEN** a board member sends the generate request
- **AND** the Typst template contains syntax errors
- **THEN** the system SHALL return HTTP 400 with the error messages

### Requirement: Template-to-DocumentType mapping
The system SHALL maintain a fixed mapping from document type identifiers to template file paths and `DocumentType` enum values:

| Identifier           | Template File              | DocumentType       |
|----------------------|----------------------------|--------------------|
| `join_confirmation`  | `join_confirmation.typ`    | JoinConfirmation   |
| `join_declaration`   | `join_declaration.typ`     | JoinDeclaration    |

#### Scenario: Map join_confirmation to template
- **WHEN** a generate request uses document type `join_confirmation`
- **THEN** the system SHALL use template `join_confirmation.typ` and DocumentType `JoinConfirmation`

#### Scenario: Map join_declaration to template
- **WHEN** a generate request uses document type `join_declaration`
- **THEN** the system SHALL use template `join_declaration.typ` and DocumentType `JoinDeclaration`

### Requirement: Board-only access
The generate endpoint SHALL require board member (Vorstand) permissions.

#### Scenario: Non-board member attempts generation
- **WHEN** a non-board member sends a generate request
- **THEN** the system SHALL return HTTP 403

### Requirement: Frontend generate button
The member details page SHALL display a "Generieren" button for each document type that has a template mapping and for which no document of that type exists yet.

#### Scenario: No join confirmation exists
- **WHEN** a board member views a member's detail page
- **AND** the member has no `JoinConfirmation` document
- **THEN** the page SHALL display a "Beitrittsbestätigung generieren" button

#### Scenario: Join confirmation already exists
- **WHEN** a board member views a member's detail page
- **AND** the member already has a `JoinConfirmation` document
- **THEN** the "Beitrittsbestätigung generieren" button SHALL NOT be displayed

#### Scenario: Button triggers generation
- **WHEN** a board member clicks the "Beitrittsbestätigung generieren" button
- **THEN** the frontend SHALL send `POST /api/members/{member_id}/documents/generate/join_confirmation`
- **AND** refresh the document list on success
