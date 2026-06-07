---
phase: 19-e-mail-anhaenge-anzeigen
plan: 19-02
subsystem: mail-inbox
tags: [mail-parser, imap, attachments, document-storage, async-trait, automock, save-then-db, uidvalidity]

# Dependency graph
requires:
  - phase: 19-01
    provides: "InboundMailAttachment entity + DAO trait + SQLite impl + #[automock] mock"
provides:
  - "ParsedAttachment struct + ParsedMail.attachments field + extract_attachments helper (D-01)"
  - "ATTACHMENT_MAX_BYTES = 10 MB hard cap constant (D-02)"
  - "persist_attachment free fn: save-then-DB pattern with delete rollback on DB-fail (T-07)"
  - "InboxImapClient::fetch_one_by_uid trait method + AsyncImapClient impl (T-06)"
  - "InboxService::find_attachment + list_attachments (consumed by Plan 19-03)"
  - "InboxServiceImpl extended with A: InboundMailAttachmentDao + St: DocumentStorage generic params"
  - "Poll worker persists attachments after mail-create with best-effort warn (D-06)"
affects: [19-03-rest-endpoints, 19-04-backfill-worker, 19-05-frontend-components, 19-06-frontend-page-wiring]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Save-then-DB with delete rollback (mirrors static_document_service.rs:108-120)"
    - "Storage path uses only UUIDs (mail_id + attachment_id) — never attacker-supplied filename (T-02 path-traversal mitigation, D-04)"
    - "Hard cap enforcement BEFORE storage.save: oversized rows bypass DocumentStorage entirely (D-02 + T-01 DoS mitigation)"
    - "UIDVALIDITY drift check in fetch_one_by_uid: caller silent-skips on Err (D-06)"
    - "Generic-param InboxServiceImpl<C, D, I, J, R, A, St> with 'static bound on storage (required by InboxService: 'static)"
    - "Best-effort attachment-persist loop after successful mail-create — single failure does NOT abort the cycle (D-06)"

key-files:
  created: []
  modified:
    - genossi_mail/src/inbox.rs (ParsedAttachment + extract_attachments + persist_attachment + InboxService extension + worker loop)
    - genossi_mail/src/inbox_imap.rs (AsyncImapClient::fetch_one_by_uid impl with UIDVALIDITY drift detection)
    - genossi_bin/src/lib.rs (InboxServiceType type alias + wiring for inbox_attachment_dao + worker fields)

key-decisions:
  - "Generic InboxServiceImpl over A + St (not a trait object) — matches the rest of the codebase's monomorphized service pattern; St: DocumentStorage + 'static is required because InboxService: 'static"
  - "Oversized attachments persist metadata-only WITHOUT calling storage.save (D-02 + T-01) — verified in test_persist_attachment_oversized_skips_storage via storage.expect_save().times(0)"
  - "persist_attachment is a private free fn (not a method) — keeps the worker + service code paths uniform; takes &dyn DocumentStorage + &dyn InboundMailAttachmentDao so both call sites can reuse it without type-parameter plumbing"
  - "fetch_one_by_uid returns Err on UIDVALIDITY drift (not Ok(None)) — caller (Plan 19-04 backfill) treats the Err as a silent-skip per D-06; this keeps the IMAP-drift signal distinct from 'UID does not exist' (which is Ok(None))"

patterns-established:
  - "Two-layer integrity for inbox attachments: (1) save-then-DB-then-rollback at the persist_attachment level guards against orphaned files; (2) best-effort warn-on-fail at the poll-worker level guards against one bad attachment blocking the rest of the cycle"
  - "Mock generic-param testing with MockInboundMailAttachmentDao + MockDocumentStorage: existing inbox-service tests just add `Arc::new(Mock*::new())` to their `InboxServiceImpl::new(...)` calls — automatically covered by #[automock]"

requirements-completed: []

# Metrics
duration: 13min
completed: 2026-06-07
---

# Phase 19 Plan 02: Service + IMAP Summary

**Attachment-Pipeline (parse → 10 MB cap → save-then-DB → rollback) + fetch_one_by_uid (UIDVALIDITY-guard) + InboxService API surface for attachment listing/lookup — all wired through genossi_bin so the existing inbox worker persists attachments automatically after each successful mail-create.**

## Performance

- **Duration:** ~13 min
- **Started:** 2026-06-07T10:23:25Z
- **Completed:** 2026-06-07T10:36:40Z
- **Tasks:** 1 (single-task plan with 11 sub-steps)
- **Files modified:** 3 (inbox.rs, inbox_imap.rs, genossi_bin/src/lib.rs)

## Accomplishments

- `ParsedAttachment { file_name, mime_type, bytes }` struct + `ParsedMail.attachments: Vec<ParsedAttachment>` field added next to existing `has_attachments: bool` (kept for backward compat, now derived from `!attachments.is_empty()`)
- `extract_attachments(&Message) -> Vec<ParsedAttachment>` helper iterates `msg.attachments()`, handles `is_message()` parts as embedded `.eml`, falls back to synthetic filenames + `application/octet-stream` when headers are absent (Pitfall 3 + 4)
- `parse_raw_mail` now calls `extract_attachments(&msg)` — line 208 `msg.attachment_count() > 0` MVP path completely removed (verified: `grep -c "msg.attachment_count() > 0"` returns 0)
- `const ATTACHMENT_MAX_BYTES: u64 = 10 * 1024 * 1024;` (D-02 hard cap, NOT configurable)
- `persist_attachment(...)` free fn with save-then-DB + rollback: oversized → metadata-only, otherwise → `storage.save` → `dao.create` → on DB-fail `storage.delete` (best-effort; `tracing::warn!` if the cleanup itself fails)
- Storage path is `inbound_mail_attachments/{mail_id}/{attachment_id}` — UUIDs only, never the filename (T-02 + D-04 path-traversal mitigation)
- `InboxImapClient::fetch_one_by_uid(config, expected_uid_validity, uid) -> Result<Option<FetchedMessage>, MailServiceError>` added to the trait; `MockInboxImapClient` auto-extended via `#[automock]`
- `AsyncImapClient::fetch_one_by_uid` impl: opens `EXAMINE` session, compares `mailbox.uid_validity` against `expected_uid_validity`, returns `Err("UIDVALIDITY drift: expected X, got Y")` on mismatch (T-06)
- `InboxService::find_attachment(mail_id, attachment_id)` + `InboxService::list_attachments(mail_id)` added; the IDOR-safe `find_by_id_and_mail` DAO from Plan 19-01 provides T-03 mitigation transparently
- `InboxServiceImpl` generic-param list extended with `A: InboundMailAttachmentDao` and `St: DocumentStorage + 'static`; constructor takes two new `Arc`'d dependencies
- Poll worker (`poll_once`) signature extended with `attachment_dao: &A, storage: &St`; after a successful `dao.create(&mail)`, loops `parsed.attachments.iter()` and calls `persist_attachment(...)` for each; per-attachment failures log a `tracing::warn!` and continue (D-06)
- `genossi_bin/src/lib.rs` wired: new `InboundMailAttachmentDaoType` alias, two new `RestStateImpl` fields (`worker_inbox_attachment_dao`, `worker_inbox_storage`), `start_inbox_worker` passes both into `start_inbox_worker(...)`
- **3 new unit tests** all green (173 total in genossi_mail, was 170):
  - `test_parse_raw_mail_extracts_attachments` — multipart raw with inline base64 PNG; asserts 1 attachment with `file_name == "test.png"`, `mime_type == "image/png"`, non-empty bytes
  - `test_persist_attachment_oversized_skips_storage` — `vec![0u8; ATTACHMENT_MAX_BYTES + 1]`; `storage.expect_save().times(0)`; DAO row carries `oversized=true` + `relative_path=None`
  - `test_persist_attachment_rollback_on_db_fail` — `storage.expect_save().times(1)` + `storage.expect_delete().times(1)` + `dao.expect_create().returning(Err(DatabaseError))`; verifies the rollback chain

## Task Commits

Single task, single commit:

1. **Task 1: Full pipeline + InboxService extension + bin wiring + 3 tests** — `519eeac` (feat)

## Files Created/Modified

- `genossi_mail/src/inbox.rs` — +362 / -18 LOC
  - Added `use crate::dao::{InboundMailAttachment, InboundMailAttachmentDao, ...}` + `use genossi_service::document_storage::DocumentStorage`
  - Added `pub struct ParsedAttachment` + `pub attachments: Vec<ParsedAttachment>` on `ParsedMail`
  - Added `const ATTACHMENT_MAX_BYTES` + `fn extract_attachments` + `async fn persist_attachment`
  - Extended `InboxImapClient` trait with `fetch_one_by_uid`
  - Extended `InboxService` trait with `find_attachment` + `list_attachments`
  - Extended `InboxServiceImpl` with generic params `A, St` + corresponding `Arc<A>` / `Arc<St>` fields
  - Extended `poll_once` + `start_inbox_worker` signatures with `attachment_dao` + `storage`
  - Updated all 5 existing `InboxServiceImpl::new(...)` call sites in tests to pass `MockInboundMailAttachmentDao` + `MockDocumentStorage`
  - Updated all 3 existing `poll_once(...)` call sites in tests with the two new args
  - Added 3 new unit tests (`test_parse_raw_mail_extracts_attachments`, `test_persist_attachment_oversized_skips_storage`, `test_persist_attachment_rollback_on_db_fail`)
- `genossi_mail/src/inbox_imap.rs` — +37 / 0 LOC
  - `impl InboxImapClient for AsyncImapClient` gains `fetch_one_by_uid` with UIDVALIDITY drift check + single-UID `uid_fetch`
- `genossi_bin/src/lib.rs` — +29 / -4 LOC
  - New `InboundMailAttachmentDaoType` alias
  - Extended `InboxServiceType` alias with the two new generic args (`InboundMailAttachmentDaoType, DocumentStorage`)
  - Added `worker_inbox_attachment_dao` + `worker_inbox_storage` fields on `RestStateImpl`
  - Wired `inbox_attachment_dao` + `document_storage` into `InboxServiceImpl::new` call
  - Wired `attachment_dao` + `storage` into `start_inbox_worker` call

## Decisions Made

- **`St: DocumentStorage + 'static` is required, not just `St: DocumentStorage`:** the `InboxService` trait carries a `'static` bound (`pub trait InboxService: Send + Sync + 'static`); without the explicit `+ 'static` on `St`, rustc rejects the impl block with E0310. Documented inline in the impl block.
- **Storage path is UUID-only — never filename:** `format!("inbound_mail_attachments/{}/{}", inbound_mail_id, id)` with `id = Uuid::new_v4()`. This is the T-02 path-traversal mitigation and the D-04 invariant. Filename only ever flows into the DB column + (later) Content-Disposition header (sanitized by Plan 19-03).
- **Oversized rows bypass storage.save COMPLETELY (no truncation):** `if let Some(ref rel_path) = relative_path { storage.save(...).await? }` is gated on `relative_path.is_some()` which itself is gated on `!oversized`. The 10 MB cap is a hard reject (D-02), not a truncate. The DAO row still gets created so the frontend (Plan 19-05) can show "Attachment too big — not stored" to the user.
- **Generic-param InboxServiceImpl, not trait-object:** matches the rest of the codebase (MemberServiceImpl, MailServiceImpl, etc.) — monomorphized over a `Deps`-style generic list. The two new params (`A`, `St`) sit at the end of the list to minimize churn in already-existing call sites.

## Deviations from Plan

None — the plan executed exactly as written. Three minor mechanical adjustments worth recording:

- **`St: DocumentStorage + 'static`** — the plan said `St: DocumentStorage` without the `'static` bound. rustc E0310 forced the addition because `InboxService` itself carries `'static`. This is a pure compile-fix, no semantic change.
- **`InboxServiceImpl` generic-param order:** the plan stated "Add two new type parameters to the generic list" but did not specify the order. I placed `A` and `St` at the end of the generic list (`<C, D, I, J, R, A, St>`) to avoid renumbering the existing five params in every `MockX::new()` call site — this keeps the diff small in tests + genossi_bin wiring.
- **Constructor argument order matches generic-param order:** the plan stated "Extend the `pub fn new(...)` constructor to accept + assign both new dependencies" — I placed `attachment_dao` then `storage` as the 6th and 7th params (after `recipient_dao`), keeping the convention "generic param order == constructor arg order == struct field order".

## Issues Encountered

- **First compile attempt hit E0310 on `St`:** the trait bound `'static` is required because `InboxService` itself is `'static`. Diagnosed from rustc help-text: "consider adding an explicit lifetime bound: `St: DocumentStorage + 'static`". One-line fix, no spec impact.
- **The plan's `<verify>` step uses `cargo test -p genossi_mail X Y Z -- --nocapture` (multiple TESTNAME positional args):** known to be rejected by the cargo CLI (only one TESTNAME accepted). Replaced with `cargo test -p genossi_mail -- X Y Z --nocapture` which filters the test binary. All 3 new tests passed verbose.

## User Setup Required

None — the existing IMAP config (`imap_host`/`imap_user`/`imap_pass`) keeps the poll worker running; the attachment persistence is a pure additive behavior. The first poll cycle after deployment will start populating `inbound_mail_attachments` for any newly-arrived mails with attachments. Legacy mails (already in `inbound_mails` without rows in `inbound_mail_attachments`) are NOT backfilled by this plan — Plan 19-04 handles that case explicitly.

## Next Phase Readiness

- **Ready for Plan 19-03 (REST endpoints):** `InboxService::find_attachment` + `InboxService::list_attachments` are wired through the impl + the `RestStateImpl`. The REST layer can call `state.inbox_service.find_attachment(mail_id, attachment_id)` and stream the file via `state.document_storage.load(&attachment.relative_path)` (for non-oversized rows).
- **Ready for Plan 19-04 (Backfill worker):** `InboxImapClient::fetch_one_by_uid(config, expected_uid_validity, uid)` is the exact contract the backfill worker needs. Use the UIDVALIDITY-drift `Err` as a silent-skip signal per D-06.
- **Ready for Plan 19-05 + 19-06 (Frontend):** Backend-only plan; no direct dependency.

## Self-Check: PASSED

- `pub struct ParsedAttachment`: 1 occurrence in `genossi_mail/src/inbox.rs`
- `pub attachments: Vec<ParsedAttachment>`: 1 occurrence
- `fn extract_attachments`: 1 occurrence
- `const ATTACHMENT_MAX_BYTES`: 1 occurrence (= 10 * 1024 * 1024)
- `fn persist_attachment`: 1 definition + 1 call site inside the poll worker + 2 call sites in tests (total grep: 4)
- `msg.attachment_count() > 0`: 0 occurrences (old MVP path fully removed, D-01)
- `fetch_one_by_uid`: 1 occurrence in `inbox.rs` (trait), 1 `fn fetch_one_by_uid` in `inbox_imap.rs` (impl)
- `UIDVALIDITY drift`: 1 occurrence in `inbox_imap.rs` (T-06 mitigation)
- `async fn find_attachment`: 2 occurrences (trait decl + impl)
- `async fn list_attachments`: 2 occurrences (trait decl + impl)
- `audited_create|audited_update|audited_delete`: 0 occurrences in `inbox.rs` (D-10 enforced — InboundMailAttachment is NOT auditable)
- `cargo check -p genossi_mail`: exits 0
- `cargo check -p genossi_mail --tests`: exits 0
- `cargo check -p genossi_bin`: exits 0
- `cargo build --workspace`: exits 0
- `cargo test -p genossi_mail`: 173 passed / 0 failed (170 prior + 3 new)
- `cargo test -p genossi_service_impl`: 406 passed / 0 failed (no regression)
- Commit `519eeac` exists: confirmed via `git log`

---
*Phase: 19-e-mail-anhaenge-anzeigen*
*Plan: 19-02-service-and-imap*
*Completed: 2026-06-07*
