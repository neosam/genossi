---
phase: 10-massenmail-anbindung-template-variablen
plan: 03
subsystem: mail-pipeline
tags: [service, trait-extension, mail-job, breaking-change, tdd]
requires:
  - MailJob.template_id (Option<Uuid>)        # Plan 10.01
  - MailJob.repayment_phase_id (Option<Uuid>) # Plan 10.01
provides:
  - MailService::create_job(..., template_id, repayment_phase_id)
  - MailServiceImpl::create_job writes both new fields into MailJob
  - MockMailService auto-regenerated with the extended arity (via #[automock])
affects:
  - genossi_mail/src/rest.rs (single-send + bulk-send call-sites now pass None,None — bulk gets TODO(10.04) markers)
  - genossi_service_impl/src/application.rs (application-confirmation-mail call-site passes None,None)
tech-stack:
  added: []
  patterns:
    - "Trait+Impl signature extension with positional append-only Option<Uuid> args"
    - "TDD RED→GREEN: failing-compile tests committed first, then signature extension"
    - "MockMailService via #[automock] — automatic regeneration on signature change"
key-files:
  created: []
  modified:
    - genossi_mail/src/service.rs
    - genossi_mail/src/rest.rs
    - genossi_service_impl/src/application.rs
decisions:
  - "D-12 wired through MailService trait: create_job carries template_id end-to-end"
  - "D-03 wired through MailService trait: create_job carries repayment_phase_id end-to-end"
  - "Single-send (POST /send) and application-confirmation mail pass None,None permanently — they are by design ad-hoc/transactional, not template/phase-driven"
  - "Bulk-send (POST /send-bulk) passes None,None as TODO(10.04) placeholder — Plan 10.04 substitutes parsed UUIDs from the extended SendBulkMailRequest body"
  - "No cross-crate test code uses MockMailService → automock regeneration is silent; no matcher updates needed"
metrics:
  duration_seconds: 0
  duration_human: ""
  completed: ""
  tasks_total: 1
  tasks_completed: 1
  files_created: 0
  files_modified: 3
  tests_added: 2
  tests_total_after: 116
requirements:
  - MAIL-01
  - MAIL-02
---

# Phase 10 Plan 03: MailService::create_job Signature-Erweiterung Summary

`MailService::create_job` (Trait + Impl) takes two new trailing `Option<Uuid>` parameters — `template_id` (D-12) and `repayment_phase_id` (D-03) — that flow directly into the persisted `MailJob` row; three production call-sites (`send_mail`, `send_bulk_mail`, application-confirmation mail) and four in-module test call-sites were updated atomically, mockall regenerated `MockMailService` automatically, and the bulk-send call-site carries explicit `TODO(10.04)` markers where Plan 10.04 will substitute parsed UUIDs from the extended `SendBulkMailRequest` body.

## Tasks Executed

### Task 1 — TDD: create_job-Signatur erweitern (Trait + Impl + MailJob-Init)

- **RED commit:** `a58d553` — added two failing unit tests (`test_create_job_persists_template_id_and_repayment_phase_id`, `test_create_job_with_none_template_and_phase_keeps_null`) that reference the new 7-arg signature. `cargo build -p genossi_mail --tests` failed with `error[E0061]: this method takes 5 arguments but 7 arguments were supplied` (2 occurrences, one per test) — RED proven.
- **GREEN commit:** `82c8515` — extended trait+impl with the two new `Option<Uuid>` positional args, replaced the Plan 10.01 `None,None` placeholder in the `MailJob` struct-init with the parameters, and updated all 4 internal test call-sites (`test_create_job`, `test_create_job_empty_recipients`, `test_create_job_with_attachments_single_recipient`, `test_create_job_attachments_rejected_for_multiple_recipients`) plus 3 cross-module production call-sites (`genossi_mail/src/rest.rs::send_mail`, `genossi_mail/src/rest.rs::send_bulk_mail`, `genossi_service_impl/src/application.rs`).
- **Files modified:**
  - `genossi_mail/src/service.rs` — trait sig +2 params (with doc-comment citing D-12/D-03), impl sig +2 params, MailJob struct-init wires both, plus 2 new tests and 4 existing test-call-site updates.
  - `genossi_mail/src/rest.rs` — `send_mail` passes `None,None` (single-send is ad-hoc, no template/phase tracking — permanent by design); `send_bulk_mail` passes `None,None` with `// TODO(10.04): parse body.template_id` and `// TODO(10.04): parse body.repayment_phase_id` comments — Plan 10.04 will substitute parsed UUIDs.
  - `genossi_service_impl/src/application.rs` — application-confirmation mail (transactional join-payment confirmation) passes `None,None` permanently.
- **Tests:** 6/6 create_job-tests green (`cargo test -p genossi_mail --lib test_create_job` → `test result: ok. 6 passed; 0 failed`). Full workspace test run: 0 failures across all crates (40+279+16+70+61+116+62+35+52+276 = 1007 passed, 2 ignored, 0 failed; doc-tests = 0).
- **Verification (acceptance-criteria greps):**
  - `grep -c "template_id: Option<Uuid>," genossi_mail/src/service.rs` → **2** (trait + impl)
  - `grep -c "repayment_phase_id: Option<Uuid>," genossi_mail/src/service.rs` → **2** (trait + impl)
  - `grep -A20 "let job = MailJob" genossi_mail/src/service.rs | grep -c template_id` → **4** (1 prod constructor + 3 test fixtures all reference `template_id`)
  - `grep -c "TODO(10.04)" genossi_mail/src/rest.rs` → **2** (both bulk-send args marked)
  - `cargo build --workspace` → success (warnings are pre-existing in `genossi_rest`, `genossi_bin`, `genossi_mail/src/rest_templates.rs`; none introduced by Plan 10.03)
  - `rustfmt --check` on touched files → clean (after a one-line comment-alignment fix during impl)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 — Bug] Plan example used wrong field name `to_address` in `RecipientInput`**

- **Found during:** Task 1, while drafting the two TDD tests.
- **Issue:** The plan's `<behavior>` example code referenced `RecipientInput { member_id: ..., to_address: ... }`, but the actual struct (`genossi_mail/src/service.rs:38-41`) defines the field as `address: String`. Using the plan-as-written would have failed to compile.
- **Fix:** Used `address:` in the test code, matching the real struct.
- **Files modified:** none (caught before writing the test).
- **Commit:** `a58d553` (test code uses correct field name).

**2. [Rule 1 — Bug] rustfmt comment alignment**

- **Found during:** Task 1 verification after writing the GREEN signature.
- **Issue:** Trailing inline comments on `template_id,` and `repayment_phase_id,` in the MailJob struct-init had extra padding spaces; rustfmt reformatted them to single-space-before-comment.
- **Fix:** Adjusted whitespace inline; no semantic change.
- **Files modified:** `genossi_mail/src/service.rs`
- **Commit:** Folded into `82c8515` (GREEN gate — no separate refactor commit needed).

### Out-of-scope / Deferred

**Clippy version mismatch (pre-existing infrastructure):** The project's pinned `rustc` is 1.89, but the `cargo-clippy` available on `PATH` (and in the nix store) is 1.90 or 1.93 — running `cargo clippy --workspace` produces a noise burst of `error[E0514]: found crate ... compiled by an incompatible version of rustc` and the E0282 `type annotations needed` errors in `genossi_dao/src/member_document.rs:55,59` (unrelated to this plan — those lines were created by Plan 10.02). `cargo build --workspace` and `cargo test --workspace` both compile and pass on the project's pinned rustc, so the Plan 10.03 code is correct; the clippy gate is a pre-existing tooling issue that should be tracked separately (suggest: pin clippy in `flake.nix` alongside rustc, or upgrade the project to rustc 1.93). Logging as deferred rather than auto-fixing since it touches dev-environment plumbing, not Plan 10.03 code.

No other deviations: the plan was executed exactly as written. Strict TDD discipline (RED `cargo build --tests` fail with E0061 → GREEN signature extension → tests pass), no architectural decisions deferred, no missing critical functionality discovered.

## Auth Gates

None. Plan was fully autonomous (no authentication, server startup, or external service interactions).

## TDD Gate Compliance

Plan-level type is `execute`, but Task 1 carried `tdd="true"`. The git log shows the required sequence:

1. `a58d553` — `test(10-03): add failing tests for create_job template_id+repayment_phase_id` (RED gate). Proof of failure: `cargo build -p genossi_mail --tests` returned `error[E0061]: this method takes 5 arguments but 7 arguments were supplied` twice (lines 959, 1001 of `service.rs` at that snapshot — the two new test bodies).
2. `82c8515` — `feat(10-03): extend MailService::create_job with template_id + repayment_phase_id` (GREEN gate). Proof of pass: `cargo test -p genossi_mail --lib test_create_job` → 6 passed / 0 failed; `cargo test --workspace` → 0 failed total.

No REFACTOR commit was needed — the GREEN diff is minimal and idiomatic (rustfmt alignment fix folded into the same commit).

## Threat Flags

None. The `<threat_model>` is fully covered:

- **T-10-03-01 (Tampering on new args):** `accept` — confirmed. UUIDs persist as opaque BLOB bytes; no DAO lookup happens inside `create_job` (the worker does that in Plan 10.06). REST layer (Plan 10.04) will gate on `Uuid::parse_str` and return 400 BadRequest on malformed strings.
- **T-10-03-02 (Information Disclosure via logs):** `mitigate` — confirmed. `create_job` does not log the new fields; the doc-comment additions describe the parameters but no `tracing::info!`/`debug!`/`error!` references them. The existing `#[instrument(skip(rest_state))]` pattern on REST handlers continues to skip the body, so the new request-body fields don't leak.
- **T-10-03-03 (DoS via hostile UUIDs):** `accept` — confirmed. `create_job` is pure INSERT, no dereferencing of the UUIDs.

No new threat surfaces introduced: the trait signature change is internal API expansion, not a new trust boundary.

## Known Stubs

The `None, None` placeholders in `genossi_mail/src/rest.rs::send_bulk_mail` are **intentional and tracked**:

| Stub | File | Line | Reason |
|------|------|------|--------|
| `None, // TODO(10.04): parse body.template_id` | `genossi_mail/src/rest.rs` | ~382 | Plan 10.04 will extend `SendBulkMailRequest` with `template_id: Option<String>` and replace this with `Uuid::parse_str(...)`-parsed value. |
| `None, // TODO(10.04): parse body.repayment_phase_id` | `genossi_mail/src/rest.rs` | ~383 | Plan 10.04 will extend `SendBulkMailRequest` with `repayment_phase_id: Option<String>` and replace this with `Uuid::parse_str(...)`-parsed value. |

These are explicitly scoped placeholders that Plan 10.04 (next in wave 3) will substitute. The single-send (`send_mail`) and application-confirmation (`application.rs`) call-sites pass `None,None` **permanently** by design — they are not stubs.

## Self-Check: PASSED

**Files claimed modified — existence check:**

- `genossi_mail/src/service.rs` → FOUND (trait sig contains both new params, impl sig contains both, MailJob-init writes them, 6 tests reference create_job — verified via `grep -c`)
- `genossi_mail/src/rest.rs` → FOUND (single-send + bulk-send both contain 7-arg create_job call; bulk-send has 2× TODO(10.04) markers — verified via `grep`)
- `genossi_service_impl/src/application.rs` → FOUND (application-confirmation mail uses 7-arg create_job with permanent None,None — verified via `grep -A 12 "create_job" genossi_service_impl/src/application.rs`)

**Commits claimed — existence check:**

- `a58d553` (RED) → FOUND in `git log --oneline | grep a58d553`
- `82c8515` (GREEN) → FOUND in `git log --oneline | grep 82c8515`

All claims verified.
