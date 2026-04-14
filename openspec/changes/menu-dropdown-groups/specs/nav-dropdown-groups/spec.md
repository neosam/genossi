## ADDED Requirements

### Requirement: Navigation items are grouped into dropdown categories
The TopBar SHALL display navigation items in three groups instead of a flat list: **Mitglieder** (Members, Validation, Templates, Applications), **Kommunikation** (Mail, Posteingang), and **Verwaltung** (Config, Dokumente, Backup, Permissions).

#### Scenario: Admin user sees all three groups
- **WHEN** an admin user views the navigation bar
- **THEN** all three group labels (Mitglieder, Kommunikation, Verwaltung) are visible

#### Scenario: User with only view_members sees one group
- **WHEN** a user with only the `view_members` privilege views the navigation bar
- **THEN** only the "Mitglieder" group is visible
- **AND** it contains only Members and Validation links

### Requirement: Groups expand on click
Each group label SHALL toggle its dropdown open/closed when clicked. The dropdown SHALL display the group's navigation links.

#### Scenario: Click to open a group
- **WHEN** a user clicks on a group label (e.g., "Mitglieder")
- **THEN** the dropdown for that group opens and shows its navigation links

#### Scenario: Click to close an open group
- **WHEN** a user clicks on a group label that is already open
- **THEN** the dropdown for that group closes

#### Scenario: Opening one group closes another
- **WHEN** a user clicks on "Kommunikation" while "Mitglieder" is open
- **THEN** "Mitglieder" closes and "Kommunikation" opens

### Requirement: Dropdown closes on navigation
When a user clicks a navigation link inside a dropdown, the dropdown SHALL close and the page SHALL navigate to the selected route.

#### Scenario: Click a link in the dropdown
- **WHEN** a user clicks "Members" inside the Mitglieder dropdown
- **THEN** the app navigates to the Members page
- **AND** the dropdown closes

#### Scenario: Mobile hamburger closes on navigation
- **WHEN** a user on mobile clicks a link inside an expanded group
- **THEN** the app navigates to the selected page
- **AND** both the group accordion and the hamburger menu close

### Requirement: Dropdown closes on outside click
When a dropdown is open, clicking anywhere outside the dropdown SHALL close it.

#### Scenario: Click outside an open dropdown
- **WHEN** a user clicks on the page content area while a dropdown is open
- **THEN** the dropdown closes

### Requirement: Empty groups are hidden
A group SHALL NOT be rendered if the current user has no permissions for any of its items.

#### Scenario: Non-admin without export_backup sees no Verwaltung
- **WHEN** a user has `view_members` and `manage_members` privileges but not `admin` or `export_backup`
- **THEN** the "Verwaltung" group is not rendered

#### Scenario: Non-admin without admin sees no Kommunikation
- **WHEN** a user does not have the `admin` privilege
- **THEN** the "Kommunikation" group is not rendered (all items require admin)

### Requirement: Desktop layout uses absolute-positioned dropdowns
On screens at or above the `md` breakpoint, group dropdowns SHALL be rendered as absolute-positioned panels below the group label.

#### Scenario: Desktop dropdown positioning
- **WHEN** a user on a desktop-sized screen opens a group
- **THEN** the dropdown appears directly below the group label, overlaying page content

### Requirement: Mobile layout uses accordion-style expansion
On screens below the `md` breakpoint, group contents SHALL expand inline within the hamburger menu as an accordion.

#### Scenario: Mobile accordion expansion
- **WHEN** a user on a mobile screen opens the hamburger menu and clicks a group label
- **THEN** the group's links appear inline below the label, pushing other groups down
