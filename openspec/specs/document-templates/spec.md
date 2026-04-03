# Document Templates

## Purpose

Template filesystem management for Typst document templates, including storage configuration, default template provisioning, and CRUD operations via REST API.

## Requirements

### Requirement: Template storage configuration
The system SHALL read the template storage base path from the `TEMPLATE_PATH` environment variable. If the variable is not set, the system SHALL use a default path of `./templates`.

#### Scenario: Custom template path
- **WHEN** `TEMPLATE_PATH` is set to `/var/data/genossi/templates`
- **THEN** templates SHALL be read from and written to `/var/data/genossi/templates/`

#### Scenario: Default template path
- **WHEN** `TEMPLATE_PATH` is not set
- **THEN** templates SHALL be read from and written to `./templates/`

### Requirement: Default template provisioning
The system SHALL embed default templates via `include_bytes!` in the binary. On application startup, the system SHALL check for each embedded default template whether the corresponding file exists in `TEMPLATE_PATH`. If the file does not exist, the system SHALL create it with the embedded content. Existing files SHALL NOT be overwritten.

#### Scenario: First startup with empty template directory
- **WHEN** the application starts and `TEMPLATE_PATH` is empty or does not exist
- **THEN** the system SHALL create the directory and write all default templates (e.g., `join_confirmation.typ`, `_layout.typ`)

#### Scenario: Startup with existing customized template
- **WHEN** the application starts and `join_confirmation.typ` already exists in `TEMPLATE_PATH`
- **THEN** the system SHALL NOT overwrite the existing file

#### Scenario: New default template added in later version
- **WHEN** a new application version includes an additional default template `leave_declaration.typ`
- **AND** the user's `TEMPLATE_PATH` already contains `join_confirmation.typ` and `_layout.typ`
- **THEN** the system SHALL create only `leave_declaration.typ` and leave existing files untouched

### Requirement: List template file tree
The system SHALL allow listing all templates as a recursive file tree via `GET /api/templates`. The response SHALL include files and directories with their types.

#### Scenario: List templates with nested directories
- **WHEN** a board member requests `GET /api/templates`
- **AND** the template directory contains `_layout.typ`, `join_confirmation.typ`, and `vorstand/einladung.typ`
- **THEN** the system SHALL return a JSON tree structure containing all files and directories with their names, types (`file` or `directory`), and relative paths

#### Scenario: List templates with empty directory
- **WHEN** a board member requests `GET /api/templates`
- **AND** the template directory is empty
- **THEN** the system SHALL return an empty JSON array

### Requirement: Read template file
The system SHALL allow reading a template file's content via `GET /api/templates/{*path}` where `{*path}` is the relative path within `TEMPLATE_PATH`.

#### Scenario: Read existing template
- **WHEN** a board member requests `GET /api/templates/join_confirmation.typ`
- **THEN** the system SHALL return the file content as plain text with status 200

#### Scenario: Read template in subdirectory
- **WHEN** a board member requests `GET /api/templates/vorstand/einladung.typ`
- **THEN** the system SHALL return the file content from `TEMPLATE_PATH/vorstand/einladung.typ`

#### Scenario: Read non-existent template
- **WHEN** a board member requests a template path that does not exist
- **THEN** the system SHALL return HTTP 404

### Requirement: Create or update template file
The system SHALL allow creating or updating a template file via `PUT /api/templates/{*path}`. The request body SHALL contain the file content as plain text. If the parent directory does not exist, the system SHALL create it.

#### Scenario: Create new template
- **WHEN** a board member sends `PUT /api/templates/custom_letter.typ` with Typst content in the body
- **THEN** the system SHALL create the file at `TEMPLATE_PATH/custom_letter.typ` and return status 200

#### Scenario: Create template in new subdirectory
- **WHEN** a board member sends `PUT /api/templates/vorstand/einladung.typ` with content
- **AND** the `vorstand/` directory does not exist
- **THEN** the system SHALL create the directory and the file

#### Scenario: Update existing template
- **WHEN** a board member sends `PUT /api/templates/join_confirmation.typ` with new content
- **THEN** the system SHALL overwrite the existing file with the new content

### Requirement: Delete template file or directory
The system SHALL allow deleting a template file or empty directory via `DELETE /api/templates/{*path}`.

#### Scenario: Delete template file
- **WHEN** a board member sends `DELETE /api/templates/custom_letter.typ`
- **THEN** the system SHALL delete the file and return status 204

#### Scenario: Delete empty directory
- **WHEN** a board member sends `DELETE /api/templates/vorstand/` and the directory is empty
- **THEN** the system SHALL delete the directory and return status 204

#### Scenario: Delete non-empty directory
- **WHEN** a board member sends `DELETE /api/templates/vorstand/` and the directory contains files
- **THEN** the system SHALL return HTTP 400 with an error message

#### Scenario: Delete non-existent template
- **WHEN** a board member sends `DELETE /api/templates/nonexistent.typ`
- **THEN** the system SHALL return HTTP 404

### Requirement: Create directory
The system SHALL allow creating an empty directory via `PUT /api/templates/{*path}/` (trailing slash indicates directory).

#### Scenario: Create new directory
- **WHEN** a board member sends `PUT /api/templates/vorstand/`
- **THEN** the system SHALL create the directory at `TEMPLATE_PATH/vorstand/` and return status 200

#### Scenario: Create already existing directory
- **WHEN** a board member sends `PUT /api/templates/vorstand/` and the directory already exists
- **THEN** the system SHALL return status 200 (idempotent)

### Requirement: Path traversal protection
The system SHALL validate all template paths from the API. Paths containing `..` segments or absolute paths SHALL be rejected with HTTP 400. The resolved path MUST be within `TEMPLATE_PATH`.

#### Scenario: Path traversal attempt
- **WHEN** a request includes the path `../../etc/passwd`
- **THEN** the system SHALL return HTTP 400

#### Scenario: Absolute path attempt
- **WHEN** a request includes the path `/etc/passwd`
- **THEN** the system SHALL return HTTP 400

### Requirement: Board-only access
All template endpoints SHALL require board member (Vorstand) permissions. Non-board users SHALL receive HTTP 403.

#### Scenario: Non-board member attempts to list templates
- **WHEN** a non-board member requests `GET /api/templates`
- **THEN** the system SHALL return HTTP 403

#### Scenario: Board member accesses templates
- **WHEN** a board member accesses any template endpoint
- **THEN** the system SHALL allow the operation
