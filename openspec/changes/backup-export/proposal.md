## Why

Users need a way to create backups of member data for archival, auditing, and compliance purposes. Currently there is no export functionality — data can only be viewed in the UI or imported via Excel. A structured export ensures data can be preserved and processed externally.

## What Changes

- New REST endpoints for exporting member data, actions, and documents as downloadable files
- New `export_backup` privilege to restrict backup access to authorized users
- New frontend page (`/backup`) with download controls for each export type
- Member list CSV export supports a cutoff date (Stichtag) with historically accurate share calculation
- All member actions exportable as CSV with member name for readability
- All member documents downloadable as a streaming ZIP archive
- Navigation entry visible only to users with `export_backup` privilege

## Capabilities

### New Capabilities
- `backup-export`: REST API endpoints for exporting member lists (CSV with Stichtag), member actions (CSV), and member documents (streaming ZIP). Includes privilege-based access control and historically accurate share calculation.
- `backup-export-ui`: Dedicated frontend page for triggering and downloading backups with Stichtag date picker and download buttons.

### Modified Capabilities

## Impact

- **New API routes**: `GET /api/backup/members`, `GET /api/backup/actions`, `GET /api/backup/documents`
- **Database**: New migration adding `export_backup` privilege and assigning it to admin role
- **Frontend**: New page component, new route, new nav entry in TopBar
- **Dependencies**: May need `csv` crate for CSV generation and `zip`/`async-zip` crate for streaming ZIP
- **Performance**: Document ZIP endpoint streams large files; no temp files needed on server
