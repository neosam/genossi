## Why

The top-right user area of the navigation bar currently shows the username, a "Sessions beenden" button, and a "Abmelden" link as flat, equally-weighted items. The "Sessions beenden" button visually clutters the menu bar and looks out of place as a top-level navigation item — it's a rarely-used administrative action that shouldn't have the same visual prominence as logout. Users have reported that this layout "looks terrible."

## What Changes

- **Replace flat user area with a dropdown menu**: The displayed username becomes a clickable element that opens a dropdown submenu.
- **Move "Sessions beenden" into the dropdown**: The revoke-sessions button moves from the top-level navigation bar into the user dropdown.
- **Move "Abmelden" into the dropdown**: The logout link also moves into the user dropdown.
- **Username as dropdown trigger**: The username text gets a visual indicator (e.g., chevron) showing it's interactive.
- **Consistent dropdown behavior**: The user dropdown follows the same interaction patterns as the existing `NavGroup` dropdowns (click to toggle, close on outside click, close on navigation).

## Capabilities

### New Capabilities
- `user-menu-dropdown`: A dropdown component attached to the username in the top bar, containing user-related actions (revoke sessions, logout). Follows the same open/close behavior as existing nav group dropdowns.

### Modified Capabilities
- `nav-dropdown-groups`: The user dropdown must integrate with the existing dropdown system — opening the user menu should close any open nav group dropdown, and vice versa.

## Impact

- **Frontend components affected**:
  - `genossi-frontend/src/component/top_bar.rs` — restructure the right-side user area (lines 199–213) to use the new dropdown instead of flat items
  - `genossi-frontend/src/component/revoke_sessions_button.rs` — may need minor adjustments to work inside a dropdown context instead of as a standalone `<li>`
  - New component: `genossi-frontend/src/component/user_menu.rs` — the dropdown component itself
- **No backend changes required** — this is a pure frontend/UI restructuring
- **No API changes** — same revoke-sessions and logout endpoints are used
- **Mobile**: On mobile, the user menu items should appear inline in the hamburger menu, consistent with how nav groups work on mobile
