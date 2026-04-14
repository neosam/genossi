## 1. NavGroup Component

- [x] 1.1 Create `NavGroup` component in `genossi-frontend/src/component/nav_group.rs` with props: label, open state signal, group id, and children links
- [x] 1.2 Implement click-to-toggle on group label with ▾/▸ indicator
- [x] 1.3 Implement desktop dropdown layout (absolute positioned, z-50, bg-gray-700)
- [x] 1.4 Implement mobile accordion layout (inline expansion within hamburger menu)
- [x] 1.5 Export `NavGroup` from `genossi-frontend/src/component/mod.rs`

## 2. TopBar Refactor

- [x] 2.1 Add `open_group: Signal<Option<&'static str>>` state to TopBar
- [x] 2.2 Define group structures with their items and permission requirements
- [x] 2.3 Replace flat link list with three `NavGroup` instances (Mitglieder, Kommunikation, Verwaltung)
- [x] 2.4 Implement group visibility logic: hide groups where user has no permissions for any item
- [x] 2.5 Add invisible overlay div to close dropdown on outside click

## 3. Close-on-Navigate

- [x] 3.1 Add onclick handlers on nav links inside NavGroup to reset `open_group` to None
- [x] 3.2 On mobile, also close hamburger menu (set `visible` to false) when a link is clicked

## 4. Testing

- [x] 4.1 Verify all three groups render correctly for admin users
- [x] 4.2 Verify group hiding works for users with limited permissions
- [x] 4.3 Verify dropdown open/close toggle behavior
- [x] 4.4 Verify only one group can be open at a time
- [x] 4.5 Build successfully with `cargo build -p genossi-frontend` and verify no clippy warnings
