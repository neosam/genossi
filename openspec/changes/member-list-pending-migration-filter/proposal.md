## Why

The member list already shows a migration status badge per member (migrated/pending), but there is no way to filter the list to show only members with pending migrations. When preparing for migration work, users need to quickly see which members still need attention without scrolling through the entire list.

## What Changes

- Add a "Only pending migrations" filter checkbox to the member list filter bar, alongside the existing "Only active" and "Only exited" toggles
- When enabled, the list shows only members where `migrated == false`
- Add i18n translation keys for the new filter label (de/en)

## Capabilities

### New Capabilities

### Modified Capabilities

- `member-management`: Add a filter toggle for pending migration status to the member list page

## Impact

- **Frontend only**: Member list page (`members.rs`) gets a new filter checkbox
- **i18n**: New translation key for the filter toggle label
- **No backend changes**: The `migrated` field is already available in `MemberTO`
