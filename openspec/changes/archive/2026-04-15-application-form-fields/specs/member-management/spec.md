## MODIFIED Requirements

### Requirement: Public join request accepts title
The public join endpoint (`POST /public/join`) SHALL accept an optional `title` field in the request body. The field SHALL be stored on the created application.

#### Scenario: Public join with title
- **WHEN** a public join request includes `title` = "Prof."
- **THEN** the created application SHALL have `title` = "Prof."

#### Scenario: Public join without title
- **WHEN** a public join request does not include a `title` field
- **THEN** the created application SHALL have `title` = NULL

### Requirement: Admin create application form shows salutation and title
The admin application creation form SHALL display:
- An optional salutation dropdown with options: (empty), Herr, Frau, Firma
- An optional title text input field

#### Scenario: Admin creates application with salutation and title
- **WHEN** an admin selects "Frau" as salutation and enters "Dr." as title
- **THEN** the created application SHALL have `salutation` = "Frau" and `title` = "Dr."

#### Scenario: Admin creates application without salutation and title
- **WHEN** an admin leaves salutation and title empty
- **THEN** the created application SHALL have `salutation` = NULL and `title` = NULL
