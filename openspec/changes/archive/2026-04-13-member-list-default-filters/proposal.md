## Why

The member list currently shows all members by default, requiring users to manually enable the "Only active members" filter every time they open the page. In practice, users almost always want to see active members. Additionally, the "Only exited members" filter shows all members who ever exited — which is rarely useful. What users actually need is to see members who exited in a specific year (e.g., for annual reports or the Generalversammlung).

## What Changes

- **Default "Only active" to on**: The "Only active members" checkbox starts checked when the page loads
- **Replace "Only exited" with "Exited in year"**: Instead of showing all ever-exited members, filter by exit_date falling within the year of the current reference date
  - The label dynamically shows the year: "Ausgetreten in 2026"
  - The year is derived from the existing reference date picker — no additional input field needed
  - A member matches if their `exit_date` falls between Jan 1 and Dec 31 of the reference date's year
- **Filters are combinable**: "Only active" and "Exited in year" can both be enabled at the same time (e.g., member is still active today but has an exit_date later this year)

## Capabilities

### New Capabilities

### Modified Capabilities

- `member-management`: Change default filter state and replace exit filter with year-based exit filter on the member list page

## Impact

- **Frontend only**: Member list page (`members.rs`) filter logic and defaults
- **i18n**: Replace translation key for the exited filter label (now includes year)
- **No backend changes**: All data is already available in `MemberTO`
