# Template Editor

## Purpose

Frontend template management interface providing a file tree browser, code editor with Typst support, and PDF preview capabilities for board members.

## Requirements

### Requirement: Template management page
The frontend SHALL provide a template management page accessible from the main navigation. The page SHALL display a file tree on the left and a code editor on the right.

#### Scenario: Navigate to template management
- **WHEN** a board member clicks "Templates" in the navigation
- **THEN** the system SHALL display the template management page with the file tree loaded

#### Scenario: Non-board member navigation
- **WHEN** a non-board member accesses the application
- **THEN** the "Templates" navigation item SHALL NOT be visible

### Requirement: File tree display
The template management page SHALL display a tree view of all template files and directories in the left panel. Files and directories SHALL be visually distinguishable.

#### Scenario: Display file tree with nested directories
- **WHEN** the template directory contains `_layout.typ`, `join_confirmation.typ`, and `vorstand/einladung.typ`
- **THEN** the file tree SHALL show files and the `vorstand/` directory with its contents as a collapsible node

#### Scenario: Select file in tree
- **WHEN** a board member clicks on a file in the tree
- **THEN** the file content SHALL be loaded into the code editor

### Requirement: Code editor with Typst support
The template management page SHALL include a code editor (CodeMirror) for editing template files. The editor SHALL provide syntax highlighting suitable for Typst.

#### Scenario: Edit and save template
- **WHEN** a board member modifies content in the code editor and clicks "Speichern" (Save)
- **THEN** the system SHALL send a `PUT` request with the content and display a success confirmation

#### Scenario: Unsaved changes warning
- **WHEN** a board member has unsaved changes in the editor and clicks on a different file
- **THEN** the system SHALL warn about unsaved changes before switching

### Requirement: Create new file
The template management page SHALL provide a button to create a new file. The user SHALL be prompted for the file path (including optional subdirectory).

#### Scenario: Create new file in root
- **WHEN** a board member clicks "Neue Datei" (New File) and enters `custom_letter.typ`
- **THEN** the system SHALL create the file via `PUT /api/templates/custom_letter.typ` with empty content
- **AND** the file tree SHALL refresh and show the new file

#### Scenario: Create new file in subdirectory
- **WHEN** a board member clicks "Neue Datei" and enters `vorstand/protokoll.typ`
- **THEN** the system SHALL create the file (and directory if needed) via `PUT`

### Requirement: Create new directory
The template management page SHALL provide a button to create a new directory. The user SHALL be prompted for the directory path.

#### Scenario: Create new directory
- **WHEN** a board member clicks "Neuer Ordner" (New Folder) and enters `vorstand`
- **THEN** the system SHALL create the directory via `PUT /api/templates/vorstand/`
- **AND** the file tree SHALL refresh and show the new directory

### Requirement: Delete file or directory
The template management page SHALL allow deleting files and empty directories via a delete action in the file tree.

#### Scenario: Delete file
- **WHEN** a board member clicks the delete action on a file
- **THEN** the system SHALL confirm the deletion and send a `DELETE` request
- **AND** the file tree SHALL refresh

#### Scenario: Attempt to delete non-empty directory
- **WHEN** a board member attempts to delete a directory that contains files
- **THEN** the system SHALL display an error message that the directory is not empty

### Requirement: PDF preview
The template management page SHALL provide a preview button that renders the current template with a selected member's data and displays the resulting PDF.

#### Scenario: Preview template
- **WHEN** a board member selects a member from a dropdown and clicks "Vorschau" (Preview)
- **THEN** the system SHALL call `POST /api/templates/{path}/render/{member_id}`
- **AND** the resulting PDF SHALL be displayed inline or in a new tab

#### Scenario: Preview with compilation error
- **WHEN** a board member previews a template that contains Typst errors
- **THEN** the system SHALL display the error messages from the Typst compiler

### Requirement: Generate document from member details
The member details page SHALL provide a button or section to generate documents from available templates.

#### Scenario: Generate document from member page
- **WHEN** a board member clicks "Dokument erstellen" (Generate Document) on a member's detail page
- **THEN** the system SHALL show a list of available templates
- **AND** when a template is selected, the system SHALL render the PDF and offer it for download
