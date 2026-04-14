## Why

The top navigation bar currently shows all pages as a flat list of 10 items. On desktop this already feels crowded, and on mobile the hamburger menu produces a long vertical list that is hard to scan. Grouping related pages into dropdown menus will make the navigation more compact and easier to use on all screen sizes.

## What Changes

- Replace the flat nav link list in `TopBar` with three dropdown groups: **Mitglieder**, **Kommunikation**, **Verwaltung**
- Each group expands on click (desktop: dropdown below the bar, mobile: accordion inside the hamburger menu)
- Groups are hidden entirely when the user has no permissions for any item in that group
- Clicking a nav link closes the dropdown and (on mobile) the hamburger menu
- Clicking outside an open dropdown closes it

### Group assignment

| Group          | Pages                                    |
|----------------|------------------------------------------|
| Mitglieder     | Members, Validation, Templates, Applications |
| Kommunikation  | Mail, Posteingang                        |
| Verwaltung     | Config, Dokumente, Backup, Permissions   |

## Capabilities

### New Capabilities
- `nav-dropdown-groups`: Grouped dropdown navigation in the TopBar component with click-to-toggle behavior and responsive layout (desktop dropdowns / mobile accordion)

### Modified Capabilities
<!-- No existing spec-level requirements are changing -->

## Impact

- **Frontend only** — no backend or API changes
- `genossi-frontend/src/component/top_bar.rs` is the primary file affected
- May introduce a new reusable `DropdownGroup` component under `src/component/`
- Tailwind CSS classes handle all styling — no additional CSS dependencies
- Existing privilege checks remain unchanged, just reorganized into groups
