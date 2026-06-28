---
phase: 19-e-mail-anhaenge-anzeigen
plan: 19-01
subsystem: database
tags: [sqlite, sqlx, dao, automock, async-trait, inbound-mail, attachments]

# Dependency graph
requires:
  - phase: pre-19
    provides: "InboundMail DAO + migration `20260409000001_create_inbound_mails_table.sql` as FK target for inbound_mail_attachments"
provides:
  - "Migration `20260608000000_create_inbound_mail_attachments_table.sql` (8 columns + 1 index)"
  - "Entity `InboundMailAttachment` with `oversized: bool` + `relative_path: Option<Arc<str>>` (D-02)"
  - "Trait `InboundMailAttachmentDao` (4 read-only methods: create / find_by_inbound_mail_id / find_by_id_and_mail / count_for_mail)"
  - "Auto-derived `MockInboundMailAttachmentDao` via `#[automock]` (downstream plans 19-02, 19-04 depend on this)"
  - "SQLite impl `InboundMailAttachmentDaoSqlite` + test-only in-memory schema bootstrap"
affects: [19-02-service-and-rest, 19-03-imap-backfill, 19-04-frontend, 19-05-document-storage-wiring]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Read-only DAO entity: no `version`/`deleted` fields, no `Auditable` impl (D-10)"
    - "T-03 IDOR mitigation via DAO query requiring BOTH attachment_id AND mail_id"
    - "Nullable column round-trip: relative_path `Option<String>` ↔ `Option<Arc<str>>` via `as_deref().map(Arc::from)`"
    - "Boolean column round-trip: SQLite INTEGER 0/1 ↔ Rust `bool` (oversized != 0 / `if x { 1i64 } else { 0i64 }`)"

key-files:
  created:
    - migrations/sqlite/20260608000000_create_inbound_mail_attachments_table.sql
  modified:
    - genossi_mail/src/dao.rs (add InboundMailAttachment entity + DAO trait)
    - genossi_mail/src/dao_sqlite.rs (add SQLite impl + 2 roundtrip/IDOR tests + test schema bootstrap)

key-decisions:
  - "Read-only entity: 4 trait methods only (create / find_by_inbound_mail_id / find_by_id_and_mail / count_for_mail) — no update/delete/dump_all (D-07)"
  - "No Auditable impl (D-10) — direct DAO calls, no `audited_*!` macros (consistent with `MailRecipientAttachment` analog)"
  - "T-03 IDOR guard implemented at DAO layer: `find_by_id_and_mail` query requires both keys → cross-mail enumeration returns None"
  - "D-02 oversized rows encoded via `oversized: bool` + nullable `relative_path` (NULL when oversized=true, file bytes not persisted)"

patterns-established:
  - "Read-only attachment-child entity pattern (mirrors `MailRecipientAttachment` but with own `id` PK instead of composite recipient_id/document_id PK because Phase 19 needs individual attachment download endpoints)"
  - "Test-only schema bootstrap inside `setup_db()` lives next to `inbound_mails` table — new tables piggyback on the existing in-memory pool"
  - "DAO-layer IDOR mitigation as line-of-defense before service-layer permission check"

requirements-completed: []

# Metrics
duration: 9min
completed: 2026-06-07
---

# Phase 19 Plan 01: DAO + Migration Summary

**Read-only InboundMailAttachment DAO with 4 methods (incl. T-03 IDOR-safe `find_by_id_and_mail`), SQLite impl, and idempotent migration for inbound_mail_attachments table.**

## Performance

- **Duration:** ~9 min
- **Started:** 2026-06-07T10:10:56Z
- **Completed:** 2026-06-07T10:19:44Z
- **Tasks:** 2
- **Files modified:** 3 (1 created migration, 2 modified source files)

## Accomplishments

- Migration `20260608000000_create_inbound_mail_attachments_table.sql` creates 8-column table + 1 index on `inbound_mail_id` (idempotent via `CREATE TABLE IF NOT EXISTS`)
- New entity `InboundMailAttachment` in `genossi_mail/src/dao.rs` carries `oversized: bool` + `relative_path: Option<Arc<str>>` to encode D-02 oversized rows
- New trait `InboundMailAttachmentDao` exposes exactly 4 read-only methods, `#[automock]`-decorated so downstream plans (19-02, 19-03, 19-04) get free `MockInboundMailAttachmentDao`
- SQLite impl `InboundMailAttachmentDaoSqlite` round-trips all 8 columns including nullable `relative_path` and bool `oversized`
- Two new unit tests pass (`test_inbound_mail_attachment_roundtrip`, `test_find_by_id_and_mail_wrong_mail_returns_none`) — T-03 IDOR mitigation is enforced at DAO layer
- All existing genossi_mail tests stay green (170 passed including the 2 new ones; was 168 before)

## Task Commits

Each task was committed atomically:

1. **Task 1: Migration + Entity + Trait** — `6cccd5d` (feat)
2. **Task 2: SQLite DAO Impl + Roundtrip Tests** — `7bda8fb` (feat)

## Files Created/Modified

- `migrations/sqlite/20260608000000_create_inbound_mail_attachments_table.sql` — created; 8 columns + 1 index, idempotent DDL
- `genossi_mail/src/dao.rs` — added `InboundMailAttachment` struct + `InboundMailAttachmentDao` trait (+18 LOC for entity, +14 LOC for trait, 0 LOC for Auditable impl per D-10)
- `genossi_mail/src/dao_sqlite.rs` — added `InboundMailAttachmentDb` FromRow struct, `TryFrom` impl, `InboundMailAttachmentDaoSqlite` with all 4 methods, test-only `CREATE TABLE inbound_mail_attachments` inside `setup_db()`, two new tests

## Decisions Made

- **Read-only entity (no `version`, no `deleted`):** consistent with how `MailRecipientAttachment` is modeled. Inbound mail attachments are write-once at IMAP-poll time + soft-delete-by-parent-deletion (cascade not implemented this plan).
- **`find_by_id_and_mail` requires BOTH keys at DAO level (not service-layer-only check):** Defense-in-depth for T-03 IDOR. Even if a future service-layer change forgets the cross-tenant guard, the DAO query itself returns None for cross-mail lookups.
- **`Option<Arc<str>>` for `relative_path`:** Encodes D-02 oversized rows (oversized=true → relative_path=NULL because file bytes are not persisted past the 10 MB cap). The single-Arc allocation matches the rest of the codebase's `Arc<str>` convention for string-like DAO fields.
- **No Auditable impl (D-10):** Inbound mail attachments are NOT audit-logged. They are derived from IMAP-pulled messages; the inbound mail itself isn't auditable either. Direct DAO calls only; no `audited_*!` macros.

## Deviations from Plan

None — plan executed exactly as written. Two minor mechanical adjustments:

- The plan's `<verify>` block uses `cargo test -p genossi_mail X Y -- --nocapture`, which the actual cargo CLI rejects (only one TESTNAME positional arg accepted). Replaced with `cargo test -p genossi_mail -- X Y --nocapture` to filter the test binary directly. Both tests passed; full `cargo test -p genossi_mail` shows 170 passed / 0 failed.
- Migration file uses `CREATE TABLE IF NOT EXISTS` as the plan specified; the analog `inbound_mails` migration uses plain `CREATE TABLE`. I followed the plan's explicit DDL verbatim.

## Issues Encountered

- **Worktree topology surprise:** The agent's CWD was the worktree directory (`/home/neosam/.../worktrees/agent-…`) but `git rev-parse --show-toplevel` resolved to the main repo (`/home/neosam/programming/rust/projects/genossi3`). Both directories had separate physical copies of `genossi_mail/src/dao.rs` etc. — they were NOT a git-tracked worktree (no `.git` link-file in the worktree dir). Initial edits via the Edit tool went to the worktree copy, but `git status`/`git add` operated against the main repo. Resolved by copying the modified files (`migrations/sqlite/20260608…sql`, `genossi_mail/src/dao.rs`, `genossi_mail/src/dao_sqlite.rs`) from worktree to main repo before `git add`, then doing all subsequent edits directly in main-repo paths. Both commits (`6cccd5d`, `7bda8fb`) include the correct file content. cargo check/test were also run against main-repo paths to ensure the committed code is what was tested. **Note for orchestrator:** the worktree's copies of these files are no longer in sync with the main repo; the main-repo commits are the source of truth.

## User Setup Required

None — no external service configuration required. Migration auto-runs on next `cargo run --bin genossi` or `cargo test` via `sqlx::migrate!()` in `genossi_bin/src/main.rs`.

## Next Phase Readiness

- **Ready for Plan 19-02 (Service + REST):** `MockInboundMailAttachmentDao` is auto-derived and available for service-layer unit tests. The 4-method trait surface matches exactly what Plan 19-02 expects per the interfaces stated in the plan frontmatter.
- **Ready for Plan 19-03 (IMAP backfill):** The `oversized` flag + nullable `relative_path` give the backfill worker the exact shape it needs (oversized=true rows have no file persisted but still appear in attachment listings — D-02 visibility).
- **Ready for Plan 19-04 (Frontend):** N/A directly; depends on Plan 19-02 REST endpoints.
- **Ready for Plan 19-05 (Document Storage wiring):** `relative_path: Option<Arc<str>>` is the contract Plan 19-05 will use to resolve file bytes on disk.

## Self-Check: PASSED

- Migration file exists: `migrations/sqlite/20260608000000_create_inbound_mail_attachments_table.sql` — confirmed via `ls`
- Commit `6cccd5d` exists: confirmed via `git log`
- Commit `7bda8fb` exists: confirmed via `git log`
- `pub struct InboundMailAttachment`: 1 occurrence in `genossi_mail/src/dao.rs`
- `pub trait InboundMailAttachmentDao`: 1 occurrence in `genossi_mail/src/dao.rs`
- `Auditable for InboundMailAttachment`: 0 occurrences (D-10 enforced)
- `pub struct InboundMailAttachmentDaoSqlite`: 1 occurrence in `genossi_mail/src/dao_sqlite.rs`
- `impl InboundMailAttachmentDao for InboundMailAttachmentDaoSqlite`: 1 occurrence
- `INSERT INTO inbound_mail_attachments`: 1 occurrence
- `CREATE TABLE inbound_mail_attachments` (test-only schema): 1 occurrence
- Two new tests present and green: `test_inbound_mail_attachment_roundtrip`, `test_find_by_id_and_mail_wrong_mail_returns_none`
- `cargo check -p genossi_mail`: exits 0
- `cargo test -p genossi_mail`: 170 passed / 0 failed (168 prior + 2 new)
- `cargo check --workspace --exclude genossi-frontend`: exits 0

---
*Phase: 19-e-mail-anhaenge-anzeigen*
*Plan: 19-01-dao-and-migration*
*Completed: 2026-06-07*
