## Why

Users need to know whether a member is active (i.e., currently a member of the cooperative) on a given date. Currently, the member list shows all non-deleted members without indicating whether they have already exited. This makes it hard to answer questions like "who was a member on date X?" or "show me only current members."

## What Changes

- Add a date picker ("Stichtag") above the member list table, defaulting to today
- Add an "Active" column to the member list showing whether each member is active on the selected date
- A member is active on a date when: `join_date <= date` AND (`exit_date` is null OR `exit_date > date`)
- Add a filter toggle "Only active members" that filters the list to show only members active on the selected date
- All logic is purely frontend — the data (`join_date`, `exit_date`) is already available in `MemberTO`

## Capabilities

### New Capabilities

### Modified Capabilities

- `member-management`: Add active status column, date picker, and filter toggle to the member list page

## Impact

- **Frontend only**: Member list page (`members.rs`) gets a date picker, active column, and filter toggle
- **i18n**: New translation keys for date picker label, active/inactive status, and filter toggle
- **No backend changes**: All data needed is already in the `MemberTO` response
