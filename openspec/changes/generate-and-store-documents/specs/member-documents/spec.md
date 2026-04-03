## MODIFIED Requirements

### Requirement: Document types
The system SHALL support the following document types for member documents:
- `join_declaration` (Beitrittserklärung) — singleton per member
- `join_confirmation` (Beitrittsbestätigung) — singleton per member
- `share_increase` (Aufstockung) — multiple per member
- `other` (Sonstige) — multiple per member, requires a description

#### Scenario: Singleton type blocks upload when document exists
- **WHEN** a member already has an active document of type `join_declaration`
- **AND** a new document of type `join_declaration` is uploaded for that member
- **THEN** the system SHALL reject the upload with a 409 Conflict error
- **AND** the existing document SHALL remain unchanged

#### Scenario: Singleton type allows upload when no document exists
- **WHEN** a member has no active document of type `join_declaration`
- **AND** a new document of type `join_declaration` is uploaded for that member
- **THEN** the system SHALL create the document

#### Scenario: Multi type allows multiple active documents
- **WHEN** a member already has an active document of type `share_increase`
- **AND** a new document of type `share_increase` is uploaded for that member
- **THEN** both documents SHALL remain active

#### Scenario: Other type requires description
- **WHEN** a document of type `other` is uploaded without a description
- **THEN** the system SHALL reject the upload with a validation error
