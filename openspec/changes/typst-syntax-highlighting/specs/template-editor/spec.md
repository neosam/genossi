## MODIFIED Requirements

### Requirement: Code editor with Typst support
The template management page SHALL include a code editor (CodeMirror) for editing template files. The editor SHALL provide syntax highlighting for Typst markup, including keywords, comments, strings, math mode, headings, and code blocks. The highlighting SHALL be powered by the `codemirror-lang-typst` WASM-based parser bundled into the CodeMirror JS bundle.

#### Scenario: Edit and save template
- **WHEN** a board member modifies content in the code editor and clicks "Speichern" (Save)
- **THEN** the system SHALL send a `PUT` request with the content and display a success confirmation

#### Scenario: Unsaved changes warning
- **WHEN** a board member has unsaved changes in the editor and clicks on a different file
- **THEN** the system SHALL warn about unsaved changes before switching

#### Scenario: Typst syntax is highlighted
- **WHEN** a board member opens a `.typ` file containing Typst markup
- **THEN** the editor SHALL display syntax highlighting for Typst keywords (`#let`, `#set`, `#show`, `#import`, `#include`), comments (`//`, `/* */`), strings, headings (`=`), emphasis (`_`, `*`), math mode (`$`), and code blocks

#### Scenario: Incremental re-highlighting on edit
- **WHEN** a board member types new content into the editor
- **THEN** the syntax highlighting SHALL update incrementally without full-document re-parse
