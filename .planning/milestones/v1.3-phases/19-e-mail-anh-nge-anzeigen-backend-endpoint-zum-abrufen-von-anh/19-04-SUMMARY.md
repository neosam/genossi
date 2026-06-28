---
phase: 19-e-mail-anhaenge-anzeigen
plan: 19-04
subsystem: mail-inbox
tags: [backfill, worker, imap, one-shot, idempotent, uidvalidity, automock]

# Dependency graph
requires:
  - phase: 19-01
    provides: "InboundMailAttachmentDao::count_for_mail (idempotency filter)"
  - phase: 19-02
    provides: "InboxImapClient::fetch_one_by_uid + persist_attachment + parse_raw_mail (extract_attachments path) + ATTACHMENT_MAX_BYTES cap"
  - phase: 19-03
    provides: "RestStateImpl wiring (worker_inbox_attachment_dao, worker_inbox_storage already on struct; reused unchanged)"
provides:
  - "Free fn `run_attachment_backfill<C, D, A, St, I>` in genossi_mail/src/inbox.rs"
  - "RestStateImpl::start_attachment_backfill_worker() method (tokio::spawn wrapper)"
  - "main.rs spawn-call after start_inbox_worker; one-shot at server boot"
  - "2 unit tests for silent-skip + idempotency-on-restart behavior"
affects: [19-05-frontend-components, 19-06-frontend-page-wiring]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "One-shot tokio::spawn pattern: backfill runs once at boot, exits when candidate list is exhausted (no infinite loop body)"
    - "Restart idempotency via DB-state filter (count_for_mail == 0) — no in-memory state table needed (D-05/D-06)"
    - "Best-effort IMAP refetch: Err / Ok(None) → tracing::warn + skipped++ + continue (D-06, T-06 mitigation transitively guaranteed via fetch_one_by_uid's UIDVALIDITY drift check)"
    - "DAO-method reuse policy: list_active() + in-memory filter on has_attachments — no new DAO method added because the dataset is small and the pass is one-shot"

key-files:
  created: []
  modified:
    - genossi_mail/src/inbox.rs (run_attachment_backfill fn + 2 unit tests; +290 LOC)
    - genossi_bin/src/lib.rs (start_attachment_backfill_worker method; +23 LOC)
    - genossi_bin/src/main.rs (spawn call + tracing::info; +3 LOC)

key-decisions:
  - "Use list_active() instead of adding a new DAO method (e.g. find_with_attachments_flag): backfill is a one-shot pass on a small dataset; adding a trait method + SQLite-impl + mockall extension would expand the surface without measurable benefit. Per Plan 19-04 Step 1 LOCKED rationale."
  - "InboundMailDao does not have dump_all in this codebase (it has list_active). The plan said 'dump_all or equivalent' — list_active is the equivalent and was used verbatim."
  - "Both unit tests use mockall .times(0) assertions on attachment_dao.create + storage.save to verify NO persist happens during silent-skip — mockall verifies these on Drop, so a regression would fail the test loudly."
  - "ConfigMissing on load_imap_config is treated as no-op startup (tracing::debug + return) — mirrors poll_once's existing pattern. Other Err variants log warn + return. This means the backfill silently no-ops when IMAP is not configured (e.g., dev environments without IMAP)."

patterns-established:
  - "One-shot worker pattern: free fn returns when work is done; spawn site wraps in tokio::spawn for fire-and-forget; idempotency guaranteed by DB-state filter, not by in-memory progress tracking."
  - "Plan-DAO-mapping policy: when the plan references a DAO method that doesn't exist by name but does exist by semantics (dump_all vs list_active), prefer the existing method over inventing a new one. Document the rename in the SUMMARY."

requirements-completed: []

# Metrics
duration: ~5min
completed: 2026-06-07
---

# Phase 19 Plan 04: Backfill Worker Summary

**One-shot attachment backfill worker — `run_attachment_backfill` iterates legacy inbound mails (`has_attachments=true` + `count_for_mail==0`), refetches each from IMAP via `fetch_one_by_uid`, and runs the same `persist_attachment` pipeline as the poll worker. Best-effort (D-05/D-06): IMAP-Err / Ok(None) → silent-skip. Idempotent on restart via the `count_for_mail == 0` filter. Spawned at server boot from `genossi_bin/src/main.rs` immediately after the inbox worker.**

## Performance

- **Duration:** ~5 min
- **Started:** 2026-06-07T10:58:40Z
- **Completed:** 2026-06-07T11:03:40Z (approx)
- **Tasks:** 2 (Task 1 TDD RED→GREEN; Task 2 wiring)
- **Files modified:** 3 (`inbox.rs`, `lib.rs`, `main.rs`)

## Accomplishments

- `pub async fn run_attachment_backfill<C, D, A, St, I>` lives in `genossi_mail/src/inbox.rs`, placed AFTER `start_inbox_worker` (line ~786). Generic-param list mirrors `start_inbox_worker` (`Send + Sync + 'static` bounds throughout).
- Config-loading path mirrors `poll_once`: `load_imap_config` → `ConfigMissing` is a no-op startup (`tracing::debug`), other errors log `tracing::warn`. No-IMAP-configured environments silently no-op the backfill.
- Candidate-gathering: `mail_dao.list_active().await` + `.iter().filter(|m| m.has_attachments)` + per-mail `attachment_dao.count_for_mail(mail.id).await == 0` check.
- Per-candidate refetch: `imap_client.fetch_one_by_uid(&imap_cfg, mail.uid_validity, mail.imap_uid)`. Both `Err(...)` and `Ok(None)` log `tracing::warn` + `skipped += 1` + `continue` (D-06).
- On `Ok(Some(fetched))`: `parse_raw_mail(&fetched.raw)` → per-attachment `persist_attachment` (reusing the Plan 19-02 helper with its built-in 10 MB cap, save-then-DB pattern, rollback). Per-attachment failures log `tracing::warn` and continue (no full-cycle abort).
- Per-mail aggregate: `persisted += 1` if ≥1 attachment landed, else `skipped += 1`.
- Logging contract exactly as plan-specified:
  - Start: `inbox_attachment_backfill: starting (N candidates)`
  - End:   `inbox_attachment_backfill: done (Y persisted, Z skipped)`
- One-shot: no `loop {}` body inside the fn; `awk` verification confirms `0` occurrences of `loop {` within the function body.
- `RestStateImpl::start_attachment_backfill_worker()` in `genossi_bin/src/lib.rs` is the spawn-method, placed immediately after `start_inbox_worker`. Clones 5 Arc dependencies (config, mail-dao, attachment-dao, storage, imap-client), `tokio::spawn`s, awaits `genossi_mail::inbox::run_attachment_backfill(...)`.
- `genossi_bin/src/main.rs` spawn-call placed immediately after the existing `rest_state.start_inbox_worker();` + `tracing::info!("Inbox worker started");` block. Adds `rest_state.start_attachment_backfill_worker();` + `tracing::info!("Attachment backfill worker spawned");`. AWK gate confirms ordering: backfill spawn appears AFTER inbox spawn (line numbers monotonically increase).
- **2 new unit tests** all green (175 total in genossi_mail, was 173):
  - `test_run_attachment_backfill_silent_skips_imap_error` — 2 candidate mails, first refetch returns `Err`, second returns `Ok(None)`. `attachment_dao.expect_create().times(0)` + `storage.expect_save().times(0)` enforced by mockall on Drop. T-06 mitigation verified.
  - `test_run_attachment_backfill_skips_already_backfilled` — 1 mail, `count_for_mail` returns `Ok(2)` (already-backfilled). `imap_client.expect_fetch_one_by_uid().times(0)` proves the idempotency filter prevents refetch.

## Task Commits

1. **Task 1 RED:** `34d3822` (test) — 2 failing tests for `run_attachment_backfill`. `cargo check -p genossi_mail --tests` fails with `cannot find function run_attachment_backfill in this scope` × 2.
2. **Task 1 GREEN:** `63cf264` (feat) — Implementation of `run_attachment_backfill`. All 175 genossi_mail tests pass (173 prior + 2 new).
3. **Task 2:** `e4fcf6f` (feat) — `start_attachment_backfill_worker` method on `RestStateImpl` + spawn-call in `main.rs`. `cargo check -p genossi_bin` passes; full workspace check passes.

## Files Created/Modified

- `genossi_mail/src/inbox.rs` — +289 / 0 LOC
  - `pub async fn run_attachment_backfill<C, D, A, St, I>` with doc comment (~50 LOC for body)
  - `test_run_attachment_backfill_silent_skips_imap_error` (~85 LOC)
  - `test_run_attachment_backfill_skips_already_backfilled` (~45 LOC)
- `genossi_bin/src/lib.rs` — +23 / 0 LOC
  - `pub fn start_attachment_backfill_worker(&self)` with doc comment
- `genossi_bin/src/main.rs` — +3 / 0 LOC
  - `rest_state.start_attachment_backfill_worker();`
  - `tracing::info!("Attachment backfill worker spawned");`

## Decisions Made

- **Use `list_active()` instead of a new DAO method:** Plan stated "use `dump_all` + in-memory filter" — `InboundMailDao` has `list_active()` (not `dump_all`). Same semantics for the backfill use-case (all active mails). No new DAO trait method added; no migration; no surface expansion. **Per the plan's own Step 1 LOCKED rationale.**
- **`ConfigMissing` is a no-op startup, not an error:** Mirrors `poll_once`. If IMAP is not configured (e.g., dev/test environment), the backfill silently exits with `tracing::debug` instead of `tracing::warn`. Distinguishes "no config" from "config load actually failed".
- **`InboundMail` field-access verified pre-write:** `mail.has_attachments` / `mail.uid_validity` / `mail.imap_uid` / `mail.id` exist on the struct (confirmed via grep against `genossi_mail/src/dao.rs:255-274`). Plan called this out as a risk; no rework needed.
- **`parse_raw_mail` returns `ParsedMail` directly (not Result):** Plan template assumed `parse_raw_mail(...) -> Result<ParsedMail, _>`. Actual signature is `pub fn parse_raw_mail(raw: &[u8]) -> ParsedMail`. The implementation drops the match-on-Err arm and uses the value directly. No semantic change — parse errors don't exist in the actual API surface.
- **TDD gate for Task 2:** Task 2 is pure wiring (spawn-method + spawn-call). The behavior under test is already covered by Task 1's unit tests on `run_attachment_backfill`. Task 2 verification relied on `cargo check -p genossi_bin` + grep gates + awk-ordering gate — same depth as similar wiring-only commits in earlier plans (Plan 19-02 used the same approach for the `worker_inbox_attachment_dao` field).

## Deviations from Plan

**Two minor mechanical adjustments, both documented:**

1. **[Rule 3 - DAO-API mismatch] `list_active()` substituted for `dump_all()`**
   - **Found during:** Task 1 Step 1 (Discover DAO API)
   - **Issue:** Plan said "find the method that returns all `InboundMail` rows (e.g. `dump_all` or equivalent)". Grep showed `InboundMailDao` exposes `list_active()`, not `dump_all()`. Plan's wording "or equivalent" was the explicit escape hatch.
   - **Fix:** Used `mail_dao.list_active().await` everywhere `dump_all` was referenced. Test stub mocks `expect_list_active()` correspondingly. Semantics identical for the backfill use-case (all non-soft-deleted mails).
   - **Files modified:** None (plan-template change applied during implementation, not a post-hoc fix)
   - **Commit:** `63cf264` (GREEN)

2. **[Rule 3 - parse_raw_mail signature]**
   - **Found during:** Task 1 Step 2 (Implement run_attachment_backfill)
   - **Issue:** Plan template assumed `match parse_raw_mail(&fetched.raw) { Ok(p) => ..., Err(e) => ... }`. Actual signature is `pub fn parse_raw_mail(raw: &[u8]) -> ParsedMail` — infallible (mail-parser's `Message::parse` returns Option, and `parse_raw_mail` handles None internally by returning a `ParsedMail` with empty fields).
   - **Fix:** Dropped the `match` and used `let parsed = parse_raw_mail(&fetched.raw);` directly. Error-handling branch removed (would never execute).
   - **Commit:** `63cf264` (GREEN)

Plan executed correctly otherwise. The plan's `<acceptance_criteria>` block had one over-counted grep (`grep -c "test_run_attachment_backfill_silent_skips_imap_error" genossi_mail/src/inbox.rs` returns ≥ 2) — actual count is 1 because the test name only appears in the `fn` declaration. The test does exist, compile, and pass — the spirit of the criterion is satisfied even though the literal grep count is 1.

## Issues Encountered

- **No real issues during execution.** Both tasks compiled on first attempt; tests passed on first run.

## User Setup Required

None — the backfill worker is a pure additive behavior. The first server start after deployment will:
1. Read `imap_host`/`imap_user`/`imap_pass` config (existing).
2. Walk `inbound_mails` table, find rows with `has_attachments=true` AND no entries in `inbound_mail_attachments`.
3. Refetch each via IMAP, persist attachments where possible.
4. Log start + end summary lines.

Legacy mails whose IMAP UID is no longer fetchable (mailbox rotated, UIDVALIDITY drifted, mail was deleted) will permanently show the "attachment received before Phase 19" hint that Plans 19-05/06 will render in the frontend.

## Next Phase Readiness

- **Ready for Plan 19-05 (Frontend components):** No direct dependency. Backfill is a server-side worker; frontend reads through the existing `GET /api/inbox/{id}` endpoint that Plan 19-03 extended. Once backfill runs on a real server, the `attachments` field in the response will populate for previously-empty legacy mails.
- **Ready for Plan 19-06 (Frontend page wiring):** Same as Plan 19-05 — no dependency.

## Threat Flags

No new security-relevant surface introduced. T-06 (UIDVALIDITY drift) mitigation is transitive via the `fetch_one_by_uid` UIDVALIDITY check delivered by Plan 19-02. Backfill propagates the `Err` as silent-skip without ever invoking `persist_attachment` on a mismatched mailbox.

## Self-Check: PASSED

- `pub async fn run_attachment_backfill` in `genossi_mail/src/inbox.rs`: 1 occurrence
- `inbox_attachment_backfill: starting` log string: 2 occurrences (info line + tracing macro arg)
- `inbox_attachment_backfill: done` log string: 2 occurrences
- `fetch_one_by_uid` in `inbox.rs`: 8 occurrences (trait method, automock decl, real impl reference, 2 backfill call sites, 2 test expect_*, plus references in tests for Plan 19-02's existing tests)
- `persist_attachment` in `inbox.rs`: 11 occurrences (fn def, poll-worker call, backfill call, multiple test references)
- `test_run_attachment_backfill_silent_skips_imap_error`: 1 occurrence (test fn definition)
- `test_run_attachment_backfill_skips_already_backfilled`: 1 occurrence (test fn definition)
- `cargo test -p genossi_mail`: 175 passed / 0 failed (173 prior + 2 new) ✓
- `cargo check -p genossi_mail`: exits 0 ✓
- `cargo check -p genossi_bin`: exits 0 ✓
- `cargo check --workspace --exclude genossi-frontend`: exits 0 ✓
- `pub fn start_attachment_backfill_worker` in `genossi_bin/src/lib.rs`: 1 occurrence
- `genossi_mail::inbox::run_attachment_backfill` in `genossi_bin/src/lib.rs`: 1 occurrence
- `rest_state.start_attachment_backfill_worker` in `genossi_bin/src/main.rs`: 1 occurrence
- `Attachment backfill worker spawned` in `genossi_bin/src/main.rs`: 1 occurrence
- Awk line-ordering gate (backfill spawn AFTER inbox spawn in main.rs): OK ✓
- `loop {` inside `run_attachment_backfill` body: 0 occurrences (one-shot verified)
- Commits `34d3822`, `63cf264`, `e4fcf6f` all present in `git log` ✓

## TDD Gate Compliance

- **RED gate:** `34d3822` (`test(19-19-04): add failing tests for run_attachment_backfill`) — `cargo check -p genossi_mail --tests` fails with `error[E0425]: cannot find function 'run_attachment_backfill'` × 2.
- **GREEN gate:** `63cf264` (`feat(19-19-04): implement run_attachment_backfill free fn`) — All 175 genossi_mail tests pass (173 prior + 2 new).
- **REFACTOR gate:** Not required — implementation is intentionally minimal, mirrors the established poll-worker patterns, no cleanup necessary.

Plan 19-04 RED/GREEN gates correctly sequenced in git log.

---
*Phase: 19-e-mail-anhaenge-anzeigen*
*Plan: 19-04-backfill-worker*
*Completed: 2026-06-07*
