## Why

Users currently see a fixed set of columns in the member list. Many useful fields (email, company, street, postal code, bank account) are only visible on the detail page. Different users need different views depending on their task — an admin checking addresses needs different columns than someone reviewing share balances. Column preferences should persist across sessions and devices via backend storage.

## What Changes

- New `user_preferences` table in the backend for storing per-user settings as JSON values
- REST API endpoints for reading and writing user preferences
- Frontend column configuration UI (dropdown/popover) in the member list toolbar
- Dynamic table rendering based on selected columns instead of hardcoded markup
- Default column set for users without saved preferences

## Capabilities

### New Capabilities
- `user-preferences`: Backend storage and REST API for per-user preference key-value pairs (user_id + key → JSON value). Distinct from the global `config-store` which has no user association.
- `member-list-columns`: Frontend capability for configurable column selection in the member list, including column picker UI, dynamic table rendering, and persistence via user-preferences API.

### Modified Capabilities
<!-- No existing spec-level requirements change. The member list rendering changes but member-management requirements stay the same. -->

## Impact

- **Database**: New `user_preferences` migration (SQLite)
- **Backend**: New DAO, service, and REST layers for user preferences (following existing patterns)
- **Frontend**: Refactor `members.rs` page from hardcoded columns to data-driven rendering
- **API**: New endpoints `GET/PUT /api/user-preferences/{key}`
