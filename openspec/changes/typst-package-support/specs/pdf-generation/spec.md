## MODIFIED Requirements

### Requirement: Import resolution
The system SHALL resolve Typst `#import` statements relative to the `TEMPLATE_PATH` directory. Templates in subdirectories SHALL be able to import from parent directories using relative paths. Additionally, the system SHALL resolve package imports in the format `#import "@namespace/name:version"` by loading packages from the package cache.

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

#### Scenario: Import package from registry
- **WHEN** a template contains `#import "@preview/letter-pro:3.0.0": letter-simple`
- **THEN** the system SHALL resolve the package from the package cache (downloading if necessary) and make the package's exports available to the template

#### Scenario: Import package that fails to download
- **WHEN** a template contains `#import "@preview/nonexistent:1.0.0": foo`
- **AND** the package is not cached and cannot be downloaded
- **THEN** the system SHALL return HTTP 400 with an error message indicating the package could not be resolved
