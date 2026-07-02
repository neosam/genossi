---
phase: 22-8bit-shared-mail-body-helper
plan: 02
subsystem: mail
tags: [rust, lettre, smtp, mime, refactor, tdd]

# Dependency graph
requires:
  - phase: 22-01
    provides: MailEncoding enum + SmtpConfig.encoding field
provides:
  - "pub fn genossi_mail::send::build_message — single MIME factory for all outgoing mail"
  - "pub struct genossi_mail::send::LoadedAttachment — in-memory attachment tuple for build_message"
  - "Test-mail (send_test_mail, send_test_mail_with_body) charset=utf-8 fix — MAIL-01 satisfied structurally"
  - "8bit CTE opt-in wired through worker + service paths — MAIL-02 satisfied byte-exact"
  - "Default outgoing byte shape preserved when encoding=QuotedPrintable — MAIL-05 backward-compat"
affects: [23 (multipart/alternative HTML rewire will inject html_body: Option<&str> into build_message)]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Pure-sync MIME factory with async I/O kept in caller (D-01/D-02/D-03)"
    - "Explicit SinglePart::builder() in BOTH encoding branches for one-line-diff CTE visibility"
    - "Whole-crate exit criterion: Message::builder() production-usage lives in exactly one module (send.rs)"

key-files:
  created:
    - genossi_mail/src/send.rs
  modified:
    - genossi_mail/src/lib.rs
    - genossi_mail/src/worker.rs
    - genossi_mail/src/service.rs

key-decisions:
  - "build_message is pub fn (sync), NOT pub async fn — attachment I/O stays in the worker (D-01/D-03)"
  - "LoadedAttachment drops recipient_id/document_id from MailRecipientAttachment — pure MIME input (D-02)"
  - "Both encoding branches use SinglePart::builder() explicitly — no SinglePart::plain fallback (D-09, RESEARCH § Alternatives Considered)"
  - "5 worker.rs MIME-byte tests deleted; equivalent coverage in send.rs::tests via CALLS to build_message (D-11)"
  - "digest.rs NOT edited — transitively inherits the charset fix via send_test_mail_with_body (D-04, cascade check confirmed)"
  - "Test-mail stays synchronous; no MailJob persistence, no DocumentStorage generic on MailServiceImpl (D-05)"

patterns-established:
  - "build_message is the one place in genossi_mail where lettre::Message is constructed"
  - "smtp_config.encoding threads from load_smtp_config → build_message on all three send paths"
  - "Existing 'Invalid from address' / 'Invalid to address' error strings preserved verbatim inside build_message for downstream diagnostics"

requirements-completed:
  - MAIL-01
  - MAIL-02
  - MAIL-05

coverage:
  - id: D1
    description: "All three send paths (worker, send_test_mail, send_test_mail_with_body) construct their lettre::Message through exactly one function crate::send::build_message"
    requirement: MAIL-01
    verification:
      - kind: unit
        ref: "grep -c 'crate::send::build_message' in worker.rs (1) + service.rs (2) = 3 call sites"
        status: pass
      - kind: unit
        ref: "whole-crate exit check: grep 'Message::builder()' in genossi_mail/src/ shows production usage only inside send.rs"
        status: pass
    human_judgment: false
  - id: D2
    description: "MailEncoding::QuotedPrintable yields charset=utf-8 + non-7bit CTE, no 8bit CTE"
    requirement: MAIL-05
    verification:
      - kind: unit
        ref: "send.rs::tests::build_message_qp_has_utf8_charset_and_non_7bit_cte"
        status: pass
    human_judgment: false
  - id: D3
    description: "MailEncoding::EightBit yields charset=utf-8 + Content-Transfer-Encoding: 8bit exactly, and NO QP soft-line-break in body"
    requirement: MAIL-02
    verification:
      - kind: unit
        ref: "send.rs::tests::build_message_8bit_has_utf8_charset_and_8bit_cte"
        status: pass
    human_judgment: false
  - id: D4
    description: "Multipart/mixed text part still carries charset=utf-8 with attachments present"
    requirement: MAIL-01
    verification:
      - kind: unit
        ref: "send.rs::tests::build_message_multipart_text_part_has_utf8_charset"
        status: pass
    human_judgment: false
  - id: D5
    description: "In-Reply-To and References headers populate with bracketed form when in_reply_to=Some; absent when None"
    requirement: MAIL-01
    verification:
      - kind: unit
        ref: "send.rs::tests::build_message_reply_includes_in_reply_to_and_references"
        status: pass
      - kind: unit
        ref: "send.rs::tests::build_message_non_reply_omits_in_reply_to"
        status: pass
    human_judgment: false
  - id: D6
    description: "Auto-generated Message-ID readable via email.headers().get_raw()"
    requirement: MAIL-01
    verification:
      - kind: unit
        ref: "send.rs::tests::build_message_exposes_auto_generated_message_id"
        status: pass
    human_judgment: false
  - id: D7
    description: "Malformed from-address returns SmtpError with 'Invalid from address' message — no panic"
    requirement: MAIL-01
    verification:
      - kind: unit
        ref: "send.rs::tests::build_message_rejects_malformed_from_address"
        status: pass
    human_judgment: false
  - id: D8
    description: "Whole workspace still builds and existing genossi_mail lib tests pass"
    requirement: MAIL-05
    verification:
      - kind: unit
        ref: "cargo build (workspace) — success"
        status: pass
      - kind: unit
        ref: "cargo test -p genossi_mail --lib — 224 passed / 0 failed"
        status: pass
    human_judgment: false

# Metrics
duration: ~35min
completed: 2026-07-02
status: complete
---

# Phase 22 Plan 02: 8bit + Shared Mail-Body Helper Summary

**Extracts MIME construction from `worker.rs::send_mail_for_recipient` into a new pure, synchronous `genossi_mail::send::build_message`; rewires both `service.rs::send_test_mail` paths (which the digest inherits) through it; fixes the historic missing-charset bug at all three sites and threads the `MailEncoding` opt-in from `SmtpConfig.encoding` to the ONE place where the Content-Transfer-Encoding is decided.**

## Performance

- **Duration:** ~35 min
- **Completed:** 2026-07-02
- **Tasks:** 3
- **Files created:** 1 (`genossi_mail/src/send.rs`)
- **Files modified:** 3 (`genossi_mail/src/lib.rs`, `genossi_mail/src/worker.rs`, `genossi_mail/src/service.rs`)

## Accomplishments

### New shared MIME factory
- `pub struct LoadedAttachment { file_name: Arc<str>, mime_type: Arc<str>, bytes: Vec<u8> }` — pure MIME input, drops the DAO-only `recipient_id`/`document_id` (D-02).
- `pub fn build_message(from: &str, to: &str, subject: &str, body: &str, attachments: &[LoadedAttachment], in_reply_to: Option<&str>, encoding: MailEncoding) -> Result<Message, MailServiceError>` — single source of MIME construction for the crate. Sync (D-01/D-03), centralised address parsing (D-06), CTE decided in the SinglePart builder based on `encoding` (D-07/D-09).
- `pub mod send;` registered in `lib.rs` in alphabetical order (between `rest_templates` and `service`).

### Worker rewire (Task 2)
- `send_mail_for_recipient` reduced to: `load_smtp_config` → `build_transport` → loop `document_storage.load()` into a `Vec<LoadedAttachment>` → `build_message(..., smtp_config.encoding)` → read auto-generated `Message-ID` → `transport.send`.
- Removed: address `.parse()` blocks, `SinglePart::plain`, `Message::builder()` chain, `in_reply_to` branch, `MultiPart::mixed()` loop, top-of-function `use lettre::message::{Attachment, MultiPart, SinglePart}` import.
- Deleted 5 tests that re-inlined the lettre logic (`plain_mail_body_has_utf8_charset`, `built_message_exposes_message_id_header`, `multipart_mail_body_has_utf8_charset`, `reply_mail_includes_in_reply_to_header`, `non_reply_mail_has_no_in_reply_to_header`) — equivalent coverage in `send::tests` per D-11.
- Retained worker-specific tests: `normalize_message_id_strips_angle_brackets`, `test_build_member_document_entity_status_sent`, `test_build_member_document_entity_status_failed_with_truncation`, and the send-interval / retry / find-repayment-letter tests.

### Service rewire (Task 3)
- `send_test_mail(to)` — the SMTP smoke-test — routes through `build_message(&smtp_config.from, to, "Genossi Test-E-Mail", <fixed body>, &[], None, smtp_config.encoding)`. The fixed body literal is preserved verbatim.
- `send_test_mail_with_body(to, subject, body)` — the Mail-Template test-render sibling — routes through `build_message(..., &[], None, smtp_config.encoding)`. The Quick 260603-jtf privacy-defense block comment is preserved verbatim above the function body.
- Both async signatures on `MailService` are unchanged; no `MailJob` persistence, no `DocumentStorage` DI, no test-mail routing changes at the REST boundary.
- `digest.rs` NOT edited — the digest path already delegates to `send_test_mail_with_body`, so rewiring that method transitively fixes the digest charset (verified via `jj diff --stat genossi_mail/src/digest.rs` = 0 lines).

### Test coverage (send.rs::tests)
Seven MIME-byte tests, all calling `build_message` directly:
1. `build_message_qp_has_utf8_charset_and_non_7bit_cte` — MAIL-05 default byte-shape preserved
2. `build_message_8bit_has_utf8_charset_and_8bit_cte` — MAIL-02 pinned CTE + no QP soft-break
3. `build_message_multipart_text_part_has_utf8_charset` — attachments preserve charset
4. `build_message_reply_includes_in_reply_to_and_references` — bracketed In-Reply-To + References
5. `build_message_non_reply_omits_in_reply_to` — no In-Reply-To when None
6. `build_message_exposes_auto_generated_message_id` — worker Message-ID capture works
7. `build_message_rejects_malformed_from_address` — SmtpError, not panic

## Task Commits

Each task was committed atomically via `jj commit`:

1. **Task 1: add genossi_mail::send::build_message shared MIME factory** — `9d42566e` (feat)
2. **Task 2: rewire worker::send_mail_for_recipient to build_message** — `db6eaf4e` (refactor)
3. **Task 3: rewire service test-mail paths through build_message** — `2c5b4eae` (fix)

## Files Created/Modified

- **Created:** `genossi_mail/src/send.rs` (338 lines total — 112 module + 226 test module; under the 350-LOC soft budget from PLAN task 1 step 4).
- **Modified:** `genossi_mail/src/lib.rs` — added `pub mod send;`.
- **Modified:** `genossi_mail/src/worker.rs` — `send_mail_for_recipient` (was ~94 lines of inline MIME construction) reduced to ~50 lines that load bytes and delegate; 5 redundant MIME-byte tests removed.
- **Modified:** `genossi_mail/src/service.rs` — `send_test_mail` (was 30 lines) reduced to 20; `send_test_mail_with_body` (was 42 lines) reduced to 27; both call `build_message` with `&[], None`.

## Decisions Made

- **Both encoding branches use the explicit `SinglePart::builder()` form** (RESEARCH § Alternatives Considered): one-line diff visibility of the CTE decision; `SinglePart::plain` is avoided in production code (grep confirms 0 occurrences in `send.rs`).
- **`grep '.load(' worker.rs` returns 1 non-test hit at line 647** (the `document_storage.load(...)` continuation) — the async I/O boundary is preserved per D-03; only spans two lines because of Rust method-chain style. Grep for the literal `document_storage.load` returns 0 because of the line-break, but the semantic invariant holds.
- **Whole-crate exit check** (RESEARCH § Pitfall 1): the only remaining `Message::builder()` reference in `genossi_mail/src/` outside `send.rs` is a doc-comment on line 881 of `service.rs` (referring to how the factory works) — allowed per the plan's exit criterion which permits documentation-comment mentions.

## Deviations from Plan

None — plan executed exactly as written.

Grep verification for Task 3's acceptance criterion "`grep -c 'Invalid from address' genossi_mail/src/service.rs` returns `0`" holds because the string now lives in `send.rs` inside `build_message`; if a caller wants the old message text it still ships (structurally preserved for T-22-05 log stability).

## Issues Encountered

None during the three tasks. Every acceptance-criterion grep returned the expected count; every unit and clippy check passed on first attempt for the modified crate.

**Pre-existing e2e test failure (out of scope):** `test_mail_preview_repayment_no_entries_does_not_default_to_one` in `genossi_bin/tests/e2e_tests.rs:14365` fails on the parent commit `4a1d2747` (verified via `jj new` before Task 1). This is entirely unrelated to Phase 22 — it exercises the mail-preview render path (`/api/mail/preview`), not the send path. Per execute-plan.md's SCOPE BOUNDARY rule, out of scope for Plan 22-02 and logged as a pre-existing issue in this SUMMARY. All 300 other e2e tests pass.

## Deferred Issues

- **Pre-existing `test_mail_preview_repayment_no_entries_does_not_default_to_one` failure** in `genossi_bin/tests/e2e_tests.rs:14365` (`errors must be array` panic). Not caused by Phase 22 — reproducible at the parent commit before any Plan 02 changes. Recommend a `gsd-debug` or `gsd-quick` follow-up outside the Phase 22 scope.
- **Pre-existing clippy warning** in `genossi_mail/src/worker.rs:105` (`clippy::unnecessary_sort_by`) — carried over from Plan 22-01 deferred list. Still not touched here (worker.rs edit was scoped to `send_mail_for_recipient` + test module).

## User Setup Required

None — Plan 22-02 is a pure refactor + bug-fix at the code level. No database migrations, no config keys, no restarts. Existing deployments continue to encode as `quoted-printable` (MAIL-05); operators who want 8bit set `smtp_encoding=8bit` via the config UI (Plan 22-03 will wire the UI toggle).

## Next Phase Readiness

- **Phase 22 is ready to verify.** All three plans (22-01 enum + config plumbing, 22-02 shared factory + rewire, 22-03 operator runbook) are complete. Ready for `/gsd-verify-work`.
- **Phase 23 (HTML mail / multipart/alternative) is unblocked.** The signature extension will inject `html_body: Option<&str>` into `build_message` and add a second `SinglePart` inside an `alternative` group — a single-file diff. No cross-crate refactor needed.

## Self-Check: PASSED

Verified:
- `test -f genossi_mail/src/send.rs` → exists (338 LOC).
- `grep -c 'pub mod send;' genossi_mail/src/lib.rs` → 1.
- `grep -c 'pub fn build_message' genossi_mail/src/send.rs` → 1.
- `grep -c 'pub async fn build_message' genossi_mail/src/send.rs` → 0 (sync, D-01).
- `grep -c 'DocumentStorage' genossi_mail/src/send.rs` → 0 (D-05).
- `grep -c 'SinglePart::plain' genossi_mail/src/send.rs` → 0 (explicit builder in both branches).
- 7 tests defined in `send::tests` — all pass.
- `worker.rs`: `Message::builder()` = 0, `SinglePart::plain` = 0, `MultiPart::mixed` = 0, `ContentTransferEncoding` = 0, `Invalid from address` = 0, `LoadedAttachment` = 3, `crate::send::build_message` = 1.
- `service.rs`: non-comment `Message::builder()` = 0, `crate::send::build_message` = 2, `Genossi Test-E-Mail` = 1, `Diese E-Mail bestätigt, dass die SMTP-Konfiguration korrekt ist` = 1, `Quick 260603-jtf` present, `.body(body.to_string())` = 0.
- Whole-crate exit check: `grep -rn 'Message::builder()' genossi_mail/src/ | grep -v '^\s*//' | grep -v test | grep -v 'src/send.rs'` shows 1 line, at `service.rs:881` inside a `///` doc comment (allowed per plan exit criterion).
- `digest.rs` not edited: `jj diff --stat genossi_mail/src/digest.rs` → 0 changed.
- `cargo build -p genossi_mail` → success.
- `cargo test -p genossi_mail --lib` → 224 passed, 0 failed.
- `cargo build` (workspace) → success.
- Task 1 commit `9d42566e`, Task 2 commit `db6eaf4e`, Task 3 commit `2c5b4eae` visible in `jj log`.

---
*Phase: 22-8bit-shared-mail-body-helper*
*Plan: 02*
*Completed: 2026-07-02*
