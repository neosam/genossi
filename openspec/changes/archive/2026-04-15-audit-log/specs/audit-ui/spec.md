## ADDED Requirements

### Requirement: Audit log page
The system SHALL provide a frontend page at the route `/audit` that displays audit log entries in a table. The page SHALL require admin privilege.

The table SHALL display columns: Zeitpunkt, Benutzer, Aktion, Entity-Typ, Entity-ID, Feld, Alter Wert, Neuer Wert.

Entries with the same transaction_id SHALL be visually grouped.

#### Scenario: Audit log page displays entries
- **WHEN** an admin navigates to /audit
- **THEN** the page displays audit log entries in a table ordered by timestamp descending

#### Scenario: Empty audit log
- **WHEN** an admin navigates to /audit and no audit entries exist
- **THEN** the page displays an appropriate empty state message

### Requirement: Audit log filtering
The audit log page SHALL provide filter controls for:
- Entity-Typ (dropdown: Member, MemberAction, MemberDocument, Application)
- Benutzer (text input)
- Aktion (dropdown: create, update, delete)
- Zeitraum (date range picker: von/bis)

Filters SHALL be applied via query parameters to the REST API.

#### Scenario: Filter by entity type
- **WHEN** an admin selects "Member" in the Entity-Typ dropdown
- **THEN** the table updates to show only member-related audit entries

#### Scenario: Filter by date range
- **WHEN** an admin sets a from and to date
- **THEN** the table updates to show only entries within the specified range

#### Scenario: Combined filters
- **WHEN** an admin selects entity type "Member" and action "update"
- **THEN** the table shows only update entries for members

### Requirement: Hash chain verification UI
The audit log page SHALL provide a button to verify the hash chain integrity. The result SHALL be displayed prominently.

#### Scenario: Verification success
- **WHEN** an admin clicks the verify button and the chain is intact
- **THEN** the page displays a success message with the total number of verified entries

#### Scenario: Verification failure
- **WHEN** an admin clicks the verify button and the chain is broken
- **THEN** the page displays a warning with the number of broken links and highlights the affected entries

### Requirement: Audit log navigation
The system SHALL provide navigation to the audit log page in the main menu, visible only to admin users.

#### Scenario: Admin sees audit link
- **WHEN** an admin user views the navigation menu
- **THEN** the menu contains a link to the audit log page

#### Scenario: Non-admin does not see audit link
- **WHEN** a non-admin user views the navigation menu
- **THEN** the menu does not contain a link to the audit log page
