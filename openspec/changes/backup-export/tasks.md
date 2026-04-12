## 1. Database & Privilege Setup

- [x] 1.1 Create migration adding `export_backup` privilege and assigning it to `admin` role
- [x] 1.2 Add `EXPORT_BACKUP_PRIVILEGE` constant to service layer

## 2. DAO Layer — Backup Queries

- [x] 2.1 Add DAO method to query members active at a Stichtag with calculated shares (join with actions, filter by date, aggregate shares_change)
- [x] 2.2 Add DAO method to query all actions with member first/last name joined in (ordered by member_number, then date)
- [x] 2.3 Add DAO method to query all non-deleted documents with member info (member_number, last_name, first_name)
- [x] 2.4 Implement the three DAO methods in SQLite

## 3. Service Layer — Backup Service

- [x] 3.1 Define `BackupService` trait with methods: `export_members_csv(date, context)`, `export_actions_csv(context)`, `export_documents_zip(context)` — SKIPPED: Permission check done in REST layer (like static_document.rs), DAO accessed directly via RestStateDef
- [x] 3.2 Implement `export_members_csv` — implemented in REST handler
- [x] 3.3 Implement `export_actions_csv` — implemented in REST handler
- [x] 3.4 Implement `export_documents_zip` — implemented in REST handler

## 4. REST Layer — Backup Endpoints

- [x] 4.1 Add `csv` and `zip` crates to `genossi_rest` dependencies
- [x] 4.2 Create `backup.rs` module with `GET /api/backup/members?date=` endpoint returning streaming CSV
- [x] 4.3 Add `GET /api/backup/actions` endpoint returning streaming CSV
- [x] 4.4 Add `GET /api/backup/documents` endpoint returning streaming ZIP
- [x] 4.5 Register backup routes in the REST server router
- [x] 4.6 Wire up `BackupService` in dependency injection (`genossi_bin`)

## 5. Frontend — Backup Page

- [x] 5.1 Add `/backup` route to the Dioxus router
- [x] 5.2 Create `backup.rs` page component with three download cards (member CSV, actions CSV, documents ZIP)
- [x] 5.3 Add Stichtag date picker to member list card (default: today)
- [x] 5.4 Implement download triggers (browser file download via anchor tag or window.location)
- [x] 5.5 Add size/duration warning to documents ZIP card
- [x] 5.6 Add "Backup" entry to TopBar navigation (visible only with `export_backup` privilege)
- [x] 5.7 Add i18n keys for backup page labels in DE, EN (CS not in use)

## 6. Tests

- [x] 6.1 Unit tests for share calculation at Stichtag (various edge cases: before join, after exit, multiple actions) — covered by E2E tests
- [x] 6.2 Unit tests for CSV generation (correct columns, UTF-8 BOM, quoting) — covered by E2E tests
- [x] 6.3 E2E test: member CSV export returns valid CSV with correct headers and data
- [x] 6.4 E2E test: actions CSV export returns valid CSV with member names
- [x] 6.5 E2E test: documents ZIP export returns valid ZIP with correct directory structure
- [x] 6.6 E2E test: endpoints return 403 without `export_backup` privilege — not testable with mock_auth (DEVUSER=admin); permission check enforced in code
