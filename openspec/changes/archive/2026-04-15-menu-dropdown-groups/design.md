## Context

The `TopBar` component (`genossi-frontend/src/component/top_bar.rs`) renders a flat list of up to 10 navigation links. It already has a hamburger toggle for mobile that shows/hides the full list. Privilege checks determine which links are visible per user. The frontend uses Dioxus 0.6.3 with Tailwind CSS and follows a component-first architecture.

## Goals / Non-Goals

**Goals:**
- Group the 10 nav items into 3 logical categories (Mitglieder, Kommunikation, Verwaltung)
- Desktop: click-to-toggle dropdown menus below the top bar
- Mobile: accordion-style expand/collapse within the hamburger menu
- Hide entire groups when the user has no permissions for any item within
- Close dropdowns on navigation and on outside click

**Non-Goals:**
- Hover-based dropdowns (harder to implement in Dioxus, inconsistent with mobile)
- Sidebar navigation or layout changes beyond the top bar
- Changes to routing, permissions, or backend
- Animated transitions (keep it simple, can be added later)

## Decisions

### 1. Reusable `NavGroup` component

Introduce a new `NavGroup` component in `src/component/nav_group.rs` that encapsulates:
- A clickable group label with expand/collapse indicator (▾/▸)
- A list of child links
- Open/close state via a `use_signal`

**Why over inline logic in TopBar:** Keeps TopBar readable, follows the component-first principle, and the three groups share identical behavior. A component avoids tripling the toggle logic.

**Alternative considered:** Putting all logic directly in TopBar with three signals — rejected because it duplicates toggle/close behavior and bloats the component.

### 2. Click-based toggle (not hover)

All dropdowns open/close on click. No hover behavior.

**Why:** Consistent behavior across desktop and mobile. Hover is not available on touch devices and Dioxus does not have built-in hover event handling for this pattern.

### 3. Close-on-outside-click via overlay

When a dropdown is open, render an invisible full-screen `div` behind the dropdown that catches clicks and closes it. This is a common pattern that avoids complex global event listeners.

**Why over global click listener:** Dioxus WASM doesn't easily support document-level event listeners. An overlay div is simple, reliable, and framework-native.

### 4. Single open group at a time

Opening a group closes any other open group. Managed by lifting state: `TopBar` holds a signal `open_group: Signal<Option<&'static str>>` and passes it to each `NavGroup`.

**Why:** Prevents visual clutter from multiple open dropdowns. Simplifies close-on-navigate since only one group can be open.

### 5. Desktop dropdown positioning

Dropdowns are positioned with `absolute` relative to the group label's parent `li` (which gets `relative`). Standard CSS dropdown pattern, no JS positioning needed.

### 6. Mobile accordion within hamburger

On mobile, `NavGroup` renders as an expandable section (vertical list) rather than an absolute-positioned dropdown. Controlled via Tailwind responsive classes (`md:absolute md:...` vs default vertical flow).

## Risks / Trade-offs

- **[Risk] Dropdown overlapping content on desktop** → Dropdowns use `z-50` to float above page content. The TopBar already has `z` handling via its dark background.
- **[Risk] Close-on-navigate might miss edge cases** → Each `Link` in a NavGroup gets an `onclick` that resets `open_group` to `None` and (on mobile) sets hamburger `visible` to `false`.
- **[Trade-off] No animation** → Keeps implementation simple. CSS transitions on `max-height` or `opacity` can be added later without structural changes.
