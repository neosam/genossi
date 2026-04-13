## MODIFIED Requirements

### Requirement: Code editor with Typst support
The template management page SHALL include a code editor (CodeMirror) for editing template files. The editor SHALL provide syntax highlighting suitable for Typst. The CodeMirror bundle (`codemirror-bundle.js`) SHALL be built and included in all deployment outputs, including the Nix build.

#### Scenario: Edit and save template
- **WHEN** a board member modifies content in the code editor and clicks "Speichern" (Save)
- **THEN** the system SHALL send a `PUT` request with the content and display a success confirmation

#### Scenario: Unsaved changes warning
- **WHEN** a board member has unsaved changes in the editor and clicks on a different file
- **THEN** the system SHALL warn about unsaved changes before switching

#### Scenario: Code editor available in Nix-deployed version
- **WHEN** the frontend is built via Nix (`nix build .#frontend`)
- **THEN** the build output SHALL contain `codemirror-bundle.js`
- **AND** the generated `index.html` SHALL load the bundle before the WASM application
- **AND** `window.createTypstEditor` SHALL be available when the WASM app initializes
