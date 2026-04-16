## MODIFIED Requirements

### Requirement: Audit log page
The system SHALL provide a frontend page at the route `/audit` that displays a paginated slice of audit log entries in a table. The page SHALL require admin privilege.

The table SHALL display columns: Zeitpunkt, Benutzer, Aktion, Entity-Typ, Entity-ID, Feld, Alter Wert, Neuer Wert.

The page SHALL provide a page-size selector offering the values 25, 50, 100, 200, 500 (default 50). Changing the page size SHALL reset the current page to 0.

The page SHALL provide classic page-number navigation controls including "first", "previous", numbered page buttons, "next", and "last", together with a display of the current page, total page count, and total entry count. Navigation SHALL trigger a backend request for the requested page using the active filters and current page size.

Entries with the same `transaction_id` SHALL be visually grouped via row background color. The chosen color for a transaction SHALL be derived deterministically from the `transaction_id` itself (e.g. via a stable hash) so that the visual grouping for a given transaction is consistent across page boundaries and across reloads.

#### Scenario: Audit log page displays first page
- **WHEN** an admin navigates to `/audit`
- **THEN** the page requests `GET /api/audit?page=0&size=50` and displays the returned entries in a table ordered by timestamp descending

#### Scenario: Page navigation
- **WHEN** an admin clicks "Next" on page 0 of 5
- **THEN** the page requests page 1 with the current size and active filters and replaces the table content with the returned entries

#### Scenario: Jump to last page
- **WHEN** an admin clicks "Last" while on page 0 of 5
- **THEN** the page requests page 4 and renders the final page of entries

#### Scenario: Change page size
- **WHEN** an admin selects "100" in the page-size selector while on page 3
- **THEN** the page resets to page 0, requests `size=100`, and re-renders the table

#### Scenario: Total count visible
- **WHEN** the audit log page has loaded a response
- **THEN** the page shows the total number of entries matching the current filter and the current page index relative to total pages

#### Scenario: Empty audit log
- **WHEN** an admin navigates to `/audit` and no audit entries exist
- **THEN** the page displays an appropriate empty state message and the navigation controls do not advance past page 0

#### Scenario: Stable transaction zebra-striping
- **WHEN** a transaction's entries are split across two pages
- **THEN** the row background color for those entries is identical on both pages because it is derived from the `transaction_id`

### Requirement: Audit log filtering
The audit log page SHALL provide filter controls for:
- Entity-Typ (dropdown: Member, MemberAction, MemberDocument, Application)
- Benutzer (text input)
- Aktion (dropdown: create, update, delete)
- Zeitraum (date range picker: von/bis)

Filters SHALL be applied via query parameters to the REST API. Submitting a filter change SHALL reset the current page to 0 before issuing the request.

#### Scenario: Filter by entity type
- **WHEN** an admin selects "Member" in the Entity-Typ dropdown and submits
- **THEN** the table updates to show only member-related audit entries on page 0 and the total count reflects only matching entries

#### Scenario: Filter by date range
- **WHEN** an admin sets a from and to date and submits
- **THEN** the table updates to show only entries within the specified range on page 0

#### Scenario: Combined filters
- **WHEN** an admin selects entity type "Member" and action "update" and submits
- **THEN** the table shows only update entries for members on page 0 with the total reflecting the combined filter

#### Scenario: Filter change resets page
- **WHEN** an admin is on page 5 and changes the Entity-Typ filter
- **THEN** the page resets to page 0 before requesting the new filtered result
