## ADDED Requirements

### Requirement: MailTemplate data model
The system SHALL store email templates with the following fields:
- `id` (UUID, system-generated, primary key)
- `created` (DateTime, system-generated)
- `deleted` (Optional DateTime, for soft delete)
- `version` (UUID, for optimistic locking)
- `name` (TEXT, required, UNIQUE among non-deleted templates)
- `subject` (TEXT, required) — MiniJinja template for the email subject
- `body` (TEXT, required) — MiniJinja template for the email body

#### Scenario: Template stored with all fields
- **WHEN** a mail template is created with name "Einladung MV", subject "Einladung zur Mitgliederversammlung", and body "Sehr geehrte/r {{ first_name }}..."
- **THEN** the system stores a MailTemplate entity with a generated UUID, created timestamp, version UUID, and the provided name/subject/body

### Requirement: Create mail template
The system SHALL expose `POST /api/mail/templates` accepting a JSON body with `name`, `subject`, and `body` fields. The system generates id, created, and version automatically.

#### Scenario: Successful creation
- **WHEN** `POST /api/mail/templates` is called with `{"name": "Einladung", "subject": "Betreff", "body": "Text"}`
- **THEN** the system creates the template and returns it with status 201

#### Scenario: Duplicate name
- **WHEN** `POST /api/mail/templates` is called with a name that already exists (among non-deleted templates)
- **THEN** the system returns status 409 (Conflict) with an error message indicating the name is taken

#### Scenario: Missing required field
- **WHEN** `POST /api/mail/templates` is called without `name`
- **THEN** the system returns status 422 (Unprocessable Entity)

### Requirement: List mail templates
The system SHALL expose `GET /api/mail/templates` returning all non-deleted mail templates ordered by name ascending.

#### Scenario: List templates
- **WHEN** `GET /api/mail/templates` is called
- **THEN** the system returns all non-deleted MailTemplate entities

#### Scenario: No templates exist
- **WHEN** `GET /api/mail/templates` is called and no templates exist
- **THEN** the system returns an empty array

#### Scenario: Deleted templates excluded
- **WHEN** `GET /api/mail/templates` is called and some templates have been soft-deleted
- **THEN** the system returns only non-deleted templates

### Requirement: Get single mail template
The system SHALL expose `GET /api/mail/templates/{id}` returning a single template by its UUID.

#### Scenario: Template found
- **WHEN** `GET /api/mail/templates/{id}` is called with a valid UUID of an existing non-deleted template
- **THEN** the system returns the template with status 200

#### Scenario: Template not found
- **WHEN** `GET /api/mail/templates/{id}` is called with a UUID that does not exist or is soft-deleted
- **THEN** the system returns status 404

### Requirement: Update mail template
The system SHALL expose `PUT /api/mail/templates/{id}` accepting a JSON body with `name`, `subject`, `body`, and `version` fields. The version field SHALL be used for optimistic locking.

#### Scenario: Successful update
- **WHEN** `PUT /api/mail/templates/{id}` is called with valid data and the correct current version
- **THEN** the system updates the template, generates a new version UUID, and returns the updated template

#### Scenario: Version conflict
- **WHEN** `PUT /api/mail/templates/{id}` is called with an outdated version
- **THEN** the system returns status 409 (Conflict) indicating a version mismatch

#### Scenario: Name conflict on update
- **WHEN** `PUT /api/mail/templates/{id}` is called with a name that is already used by a different non-deleted template
- **THEN** the system returns status 409 (Conflict) with an error message indicating the name is taken

#### Scenario: Template not found on update
- **WHEN** `PUT /api/mail/templates/{id}` is called for a non-existent or soft-deleted template
- **THEN** the system returns status 404

### Requirement: Delete mail template (soft delete)
The system SHALL expose `DELETE /api/mail/templates/{id}` which sets the `deleted` timestamp on the template.

#### Scenario: Successful deletion
- **WHEN** `DELETE /api/mail/templates/{id}` is called for an existing non-deleted template
- **THEN** the system sets the `deleted` timestamp and returns status 204

#### Scenario: Template not found on delete
- **WHEN** `DELETE /api/mail/templates/{id}` is called for a non-existent or already-deleted template
- **THEN** the system returns status 404

### Requirement: Predefined templates seeded on migration
The system SHALL include a database migration that inserts two predefined email templates:
1. **Formelle Anrede**: A formal German salutation template using salutation, title, and last_name
2. **Informelle Anrede**: An informal German salutation template using salutation, title, and first_name

These templates SHALL use fixed UUIDs so the migration is idempotent.

#### Scenario: Fresh database
- **WHEN** migrations run on a fresh database
- **THEN** both predefined templates are present and retrievable via the API

#### Scenario: Re-run migration
- **WHEN** migrations run on a database that already has the predefined templates
- **THEN** the existing templates are not duplicated or overwritten
