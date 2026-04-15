## MODIFIED Requirements

### Requirement: Create or update template file
The system SHALL allow creating or updating a template file via `PUT /api/templates/{*path}`. The request body SHALL contain the file content as raw bytes. Both text files and binary files (images, fonts, etc.) SHALL be accepted. If the parent directory does not exist, the system SHALL create it.

#### Scenario: Create new text template
- **WHEN** a board member sends `PUT /api/templates/custom_letter.typ` with Typst content in the body
- **THEN** the system SHALL create the file at `TEMPLATE_PATH/custom_letter.typ` and return status 200

#### Scenario: Create template in new subdirectory
- **WHEN** a board member sends `PUT /api/templates/vorstand/einladung.typ` with content
- **AND** the `vorstand/` directory does not exist
- **THEN** the system SHALL create the directory and the file

#### Scenario: Update existing template
- **WHEN** a board member sends `PUT /api/templates/join_confirmation.typ` with new content
- **THEN** the system SHALL overwrite the existing file with the new content

#### Scenario: Upload binary file
- **WHEN** a board member sends `PUT /api/templates/logo.png` with PNG image data in the body
- **THEN** the system SHALL create the file at `TEMPLATE_PATH/logo.png` with the binary content and return status 200

#### Scenario: Upload image in subdirectory
- **WHEN** a board member sends `PUT /api/templates/images/header.jpg` with JPEG data
- **AND** the `images/` directory does not exist
- **THEN** the system SHALL create the directory and write the binary file

### Requirement: Read template file
The system SHALL allow reading a template file's content via `GET /api/templates/{*path}` where `{*path}` is the relative path within `TEMPLATE_PATH`. For text files, the content SHALL be returned as `text/plain`. For binary files, the content SHALL be returned as `application/octet-stream`.

#### Scenario: Read existing text template
- **WHEN** a board member requests `GET /api/templates/join_confirmation.typ`
- **THEN** the system SHALL return the file content as plain text with status 200

#### Scenario: Read binary file
- **WHEN** a board member requests `GET /api/templates/logo.png`
- **THEN** the system SHALL return the binary file content with `Content-Type: application/octet-stream` and status 200

#### Scenario: Read template in subdirectory
- **WHEN** a board member requests `GET /api/templates/vorstand/einladung.typ`
- **THEN** the system SHALL return the file content from `TEMPLATE_PATH/vorstand/einladung.typ`

#### Scenario: Read non-existent template
- **WHEN** a board member requests a template path that does not exist
- **THEN** the system SHALL return HTTP 404

## ADDED Requirements

### Requirement: Upload file button in template editor
The frontend template editor SHALL provide a file upload button in the toolbar. When clicked, the user SHALL be able to select a file from their device. The file SHALL be uploaded to the template directory via the `PUT /api/templates/{path}` endpoint using the file's binary content as the request body.

#### Scenario: Upload image file via button
- **WHEN** the user clicks the upload button in the template editor toolbar
- **AND** selects a file named `logo.png`
- **THEN** the system SHALL upload the file to `PUT /api/templates/logo.png` with the binary content
- **AND** refresh the file tree to show the new file

#### Scenario: Upload file into selected directory
- **WHEN** the user has a file selected in a subdirectory (e.g., `vorstand/einladung.typ`)
- **AND** clicks the upload button and selects `logo.png`
- **THEN** the system SHALL upload to `PUT /api/templates/logo.png` (root level)

#### Scenario: File tree shows non-Typst files
- **WHEN** the file tree contains binary files (e.g., `logo.png`)
- **THEN** the file tree SHALL display them but they SHALL NOT be openable in the code editor
