## ADDED Requirements

### Requirement: Render template to PDF
The system SHALL allow rendering a Typst template with member data to PDF via `POST /api/templates/{*path}/render/{member_id}`. The response SHALL be the generated PDF file with `Content-Type: application/pdf`.

#### Scenario: Successful PDF generation
- **WHEN** a board member sends `POST /api/templates/join_confirmation.typ/render/{member_id}`
- **AND** the template and member exist
- **THEN** the system SHALL render the template with the member's data and return the PDF with `Content-Type: application/pdf` and `Content-Disposition: attachment; filename="join_confirmation.pdf"`

#### Scenario: Render with non-existent template
- **WHEN** a board member requests rendering of a template that does not exist
- **THEN** the system SHALL return HTTP 404

#### Scenario: Render with non-existent member
- **WHEN** a board member requests rendering with a member ID that does not exist
- **THEN** the system SHALL return HTTP 404

#### Scenario: Render with Typst compilation error
- **WHEN** a board member requests rendering of a template that contains Typst syntax errors
- **THEN** the system SHALL return HTTP 400 with the Typst error messages

### Requirement: Member data as Typst variables
The system SHALL pass member data to the Typst engine as variables accessible in the template. The following member fields SHALL be available:

- `member.first_name` (String)
- `member.last_name` (String)
- `member.member_number` (Integer)
- `member.email` (String or none)
- `member.company` (String or none)
- `member.street` (String or none)
- `member.house_number` (String or none)
- `member.postal_code` (String or none)
- `member.city` (String or none)
- `member.join_date` (String, formatted as date)
- `member.exit_date` (String or none, formatted as date)
- `member.shares_at_joining` (Integer)
- `member.current_shares` (Integer)
- `member.current_balance` (Integer, in cents)
- `member.comment` (String or none)
- `today` (String, current date formatted)

#### Scenario: Template accesses member name
- **WHEN** a template contains `#member.first_name` and `#member.last_name`
- **AND** the member's name is "Max Mustermann"
- **THEN** the rendered PDF SHALL contain "Max" and "Mustermann"

#### Scenario: Template accesses optional field that is not set
- **WHEN** a template contains `#member.email`
- **AND** the member has no email set
- **THEN** the value SHALL be `none` (Typst's none type)

#### Scenario: Template accesses today's date
- **WHEN** a template contains `#today`
- **THEN** the rendered PDF SHALL contain the current date

### Requirement: Import resolution
The system SHALL resolve Typst `#import` statements relative to the `TEMPLATE_PATH` directory. Templates in subdirectories SHALL be able to import from parent directories using relative paths.

#### Scenario: Import layout from same directory
- **WHEN** a template contains `#import "_layout.typ": *`
- **AND** `_layout.typ` exists in `TEMPLATE_PATH`
- **THEN** the system SHALL resolve the import and include the layout

#### Scenario: Import from subdirectory using relative path
- **WHEN** a template at `vorstand/einladung.typ` contains `#import "../_layout.typ": *`
- **AND** `_layout.typ` exists in `TEMPLATE_PATH`
- **THEN** the system SHALL resolve the relative path and include the layout

#### Scenario: Import of non-existent file
- **WHEN** a template contains `#import "nonexistent.typ": *`
- **THEN** the system SHALL return HTTP 400 with a Typst error message about the missing file

### Requirement: Font provisioning
The system SHALL provide fonts to the Typst engine without relying on system font directories. Fonts SHALL be either embedded in the binary or loaded from a fonts directory in the repository. Typst's built-in fonts SHALL also be available.

#### Scenario: Template uses embedded font
- **WHEN** a template specifies `#set text(font: "Liberation Sans")`
- **AND** Liberation Sans is embedded or available in the fonts directory
- **THEN** the system SHALL render the PDF using that font

#### Scenario: Template uses Typst default font
- **WHEN** a template does not specify a font
- **THEN** the system SHALL render using Typst's default font (New Computer Modern)

### Requirement: Board-only access for rendering
The render endpoint SHALL require board member (Vorstand) permissions. Non-board users SHALL receive HTTP 403.

#### Scenario: Non-board member attempts to render
- **WHEN** a non-board member requests `POST /api/templates/join_confirmation.typ/render/{member_id}`
- **THEN** the system SHALL return HTTP 403
