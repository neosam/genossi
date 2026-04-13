## 1. Database Migration

- [x] 1.1 Create SQLite migration that adds `replied`, `done`, `archived` columns (INTEGER NOT NULL DEFAULT 0) to `inbound_mails`
- [x] 1.2 In the same migration, populate new columns from existing `status` field (replied→replied=1, ignored→done=1, archived→archived=1)
- [x] 1.3 Remove the `status` column (recreate table without it, SQLite limitation)
- [x] 1.4 Remove the `idx_inbound_mails_status` index, add `idx_inbound_mails_done` index

## 2. DAO Layer

- [x] 2.1 Update `InboundMail` struct: replace `status: Arc<str>` with `replied: bool`, `done: bool`, `archived: bool`
- [x] 2.2 Update `InboundMailDao` SQLite implementation: adjust INSERT, SELECT, and UPDATE queries for new columns
- [x] 2.3 Change `list_active()` filter from `status != 'ignored'` to `done = 0`

## 3. Service Layer

- [x] 3.1 Remove `ignore()` method from `InboxService` trait and implementation
- [x] 3.2 Add `mark_done(id: Uuid) -> Result<InboundMail, MailServiceError>` to `InboxService` trait and implementation
- [x] 3.3 Update `reply()` to set `mail.replied = true` instead of `mail.status = "replied"`
- [x] 3.4 Update `archive()` to set `mail.archived = true` instead of `mail.status = "archived"`
- [x] 3.5 Simplify `assign_member()` and `unassign()` to only modify `assigned_member_id` (no status changes)
- [x] 3.6 Update `poll_once()` to create new mails with `replied: false, done: false, archived: false` instead of `status: "new"`

## 4. REST Layer

- [x] 4.1 Update `InboundMailTO` and `InboundMailDetailTO`: replace `status: String` with `replied: bool`, `done: bool`, `archived: bool`
- [x] 4.2 Update `to_list_to()` and `to_detail_to()` mapping functions
- [x] 4.3 Remove `ignore_inbox` handler and route
- [x] 4.4 Add `done_inbox` handler at `POST /{id}/done`
- [x] 4.5 Update OpenAPI documentation (`InboxApiDoc`)

## 5. Frontend

- [x] 5.1 Update frontend API types for new boolean fields
- [x] 5.2 Replace `InboxStatusBadge` with individual indicators (replied icon, done badge, archived badge)
- [x] 5.3 Replace "Ignorieren" button with "Erledigt" button
- [x] 5.4 Add filter UI (Offen / Erledigt / Alle) to inbox page

## 6. Tests

- [x] 6.1 Update existing `InboxService` unit tests for new field structure
- [x] 6.2 Add unit test for `mark_done()`
- [x] 6.3 Update E2E tests in `genossi_bin/tests/e2e_tests.rs` for changed API shape
- [x] 6.4 Add E2E test for the done endpoint
