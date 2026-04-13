## MODIFIED Requirements

### Requirement: Code editor with Typst support
The template management page SHALL include a CodeMirror 6 editor for editing template files. The editor SHALL provide Typst-specific syntax highlighting, line numbers, bracket matching, and search/replace functionality. The editor SHALL replace the previous textarea-based editing.

#### Scenario: Edit and save template
- **WHEN** a board member modifies content in the CodeMirror editor and clicks "Speichern" (Save)
- **THEN** the system SHALL read the current content from CodeMirror via `getEditorContent()`
- **AND** send a `PUT` request with the content and display a success confirmation

#### Scenario: Unsaved changes warning
- **WHEN** a board member has unsaved changes in the editor and clicks on a different file
- **THEN** the system SHALL compare the CodeMirror content with the last saved content
- **AND** warn about unsaved changes before switching

#### Scenario: Load file into editor
- **WHEN** a board member selects a file in the file tree
- **THEN** the system SHALL load the file content and call `setEditorContent()` to update the editor
- **AND** the editor SHALL display the content with Typst syntax highlighting

#### Scenario: Editor lifecycle
- **WHEN** the templates page is mounted
- **THEN** the system SHALL create a CodeMirror instance via `createTypstEditor()` in an empty container div
- **WHEN** the templates page is unmounted
- **THEN** the system SHALL destroy the CodeMirror instance via `destroyEditor()`

#### Scenario: Typst syntax highlighting
- **WHEN** a template contains Typst syntax like `#let`, `#import`, `#show`, `#set`, or markup like `*bold*`
- **THEN** the editor SHALL display these tokens with appropriate syntax highlighting colors
