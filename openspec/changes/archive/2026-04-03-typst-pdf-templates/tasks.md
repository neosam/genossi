## 1. Dependencies & Setup

- [x] 1.1 Add `typst` crate (or `typst-library`, `typst-pdf`) to workspace dependencies
- [x] 1.2 Add a free font family (e.g. Liberation Sans/Serif) to `fonts/` directory in the repo
- [x] 1.3 Create default Typst templates (`_layout.typ`, `join_confirmation.typ`) under `templates/defaults/` as source for `include_bytes!`
- [x] 1.4 Add `TEMPLATE_PATH` environment variable support to application configuration

## 2. Template Storage (Backend)

- [x] 2.1 Implement template filesystem service: read, write, delete files and directories under `TEMPLATE_PATH`
- [x] 2.2 Implement path traversal protection (reject `..` segments, absolute paths, validate resolved path is within `TEMPLATE_PATH`)
- [x] 2.3 Implement default template provisioning on startup: for each embedded default, check if file exists, create if missing
- [x] 2.4 Implement file tree listing (recursive directory walk, return JSON tree with names, types, paths)
- [x] 2.5 Write tests for template storage service (CRUD operations, path validation, default provisioning)

## 3. PDF Generation (Backend)

- [x] 3.1 Implement Typst engine wrapper: compile Typst source to PDF bytes with custom font loading (embedded + `fonts/` directory)
- [x] 3.2 Implement member data to Typst variable mapping (member fields as dictionary, `today` variable)
- [x] 3.3 Implement `#import` resolution using `TEMPLATE_PATH` as root, supporting relative paths in subdirectories
- [x] 3.4 Write tests for PDF generation (successful render, compilation errors, import resolution, variable substitution)

## 4. REST API Endpoints

- [x] 4.1 Add `GET /api/templates` endpoint (file tree listing)
- [x] 4.2 Add `GET /api/templates/{*path}` endpoint (read file content)
- [x] 4.3 Add `PUT /api/templates/{*path}` endpoint (create/update file, create directory with trailing slash)
- [x] 4.4 Add `DELETE /api/templates/{*path}` endpoint (delete file or empty directory)
- [x] 4.5 Add `POST /api/templates/render/{*path}/{member_id}` endpoint (render PDF)
- [x] 4.6 Add board-only permission checks to all template endpoints
- [x] 4.7 Add OpenAPI documentation for all template endpoints
- [x] 4.8 Write E2E tests for template API endpoints (CRUD, rendering, permissions, error cases)

## 5. Frontend: Template Management Page

- [x] 5.1 Add "Templates" navigation item (visible only for board members)
- [x] 5.2 Implement file tree component with collapsible directories
- [x] 5.3 Integrate CodeMirror editor with Typst syntax highlighting
- [x] 5.4 Implement file loading: click file in tree → load content into editor
- [x] 5.5 Implement save functionality: editor content → `PUT /api/templates/{path}`
- [x] 5.6 Implement unsaved changes warning when switching files
- [x] 5.7 Implement "Neue Datei" (New File) dialog with path input
- [x] 5.8 Implement "Neuer Ordner" (New Folder) dialog with path input
- [x] 5.9 Implement delete action on file tree items with confirmation
- [x] 5.10 Implement PDF preview: member selection dropdown + render + display

## 6. Frontend: Member Details Integration

- [x] 6.1 Add "Dokument erstellen" (Generate Document) button/section to member details page
- [x] 6.2 Implement template selection dialog listing available templates
- [x] 6.3 Implement PDF generation and download for selected template + member
