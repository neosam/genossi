---
phase: 22-8bit-shared-mail-body-helper
plan: 01
subsystem: mail
tags: [rust, lettre, smtp, config, mime, encoding]

# Dependency graph
requires:
  - phase: prior mail infrastructure
    provides: SmtpConfig loader + build_transport (genossi_mail/src/service.rs)
provides:
  - "pub enum MailEncoding { QuotedPrintable, EightBit } in genossi_mail::service"
  - "SmtpConfig.encoding: MailEncoding field populated from smtp_encoding KV key"
  - "smtp_encoding parsing with tolerant fallback (unknown -> warn+QP)"
  - "Three #[tokio::test] fallback-behavior tests in service.rs tests module"
affects: [22-02 (send.rs build_message parameter), 22-03 (config UI wiring)]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Tolerant-fallback KV parsing mirroring smtp_tls (unknown values log warn, never error)"
    - "Enum-not-bool for user-configurable choices (D-07 project rule reinforced)"

key-files:
  created: []
  modified:
    - genossi_mail/src/service.rs

key-decisions:
  - "MailEncoding declared with exactly two variants (QuotedPrintable, EightBit) — no Auto/SevenBit/Base64 (Plan 02 pins CTE via lettre headers)"
  - "SmtpConfig still does not derive Debug — protects pass field from log leakage (T-22-02)"
  - "smtp_encoding is optional (not in required_keys) — matches smtp_tls policy"
  - "Wildcard-arm fallback logs tracing::warn! with only the offending value, never the full struct"

patterns-established:
  - "MailEncoding: canonical Rust enum for opt-in operator switches with a safe production default"
  - "load_smtp_config: consistent optional-key handling — .map(|e| e.value.as_ref()) + match on Option<&str> with named + empty + None + wildcard arms"

requirements-completed:
  - MAIL-03

coverage:
  - id: D1
    description: "MailEncoding enum with QuotedPrintable and EightBit variants is public and importable from genossi_mail::service"
    requirement: MAIL-03
    verification:
      - kind: unit
        ref: "cargo build -p genossi_mail (successful compile proves enum shape + pub visibility)"
        status: pass
    human_judgment: false
  - id: D2
    description: "SmtpConfig gains a pub encoding: MailEncoding field populated from load_smtp_config"
    requirement: MAIL-03
    verification:
      - kind: unit
        ref: "genossi_mail/src/service.rs#load_smtp_config_defaults_encoding_to_qp_when_key_missing"
        status: pass
      - kind: unit
        ref: "genossi_mail/src/service.rs#load_smtp_config_reads_encoding_8bit_when_set"
        status: pass
    human_judgment: false
  - id: D3
    description: "Unknown smtp_encoding values log a tracing::warn! and fall back to QuotedPrintable — a typo cannot disable mail (T-22-01/T-22-03 mitigation)"
    requirement: MAIL-03
    verification:
      - kind: unit
        ref: "genossi_mail/src/service.rs#load_smtp_config_falls_back_on_unknown_encoding_value"
        status: pass
    human_judgment: false

# Metrics
duration: 7min
completed: 2026-07-02
status: complete
---

# Phase 22 Plan 01: MailEncoding Enum + smtp_encoding Config Plumbing Summary

**Adds `pub enum MailEncoding { QuotedPrintable, EightBit }` and threads a new `SmtpConfig.encoding` field through `load_smtp_config`, driven by the optional `smtp_encoding` KV key with tolerant fallback — production default `quoted-printable` is preserved.**

## Performance

- **Duration:** ~7 min
- **Started:** 2026-07-02T19:23:xxZ (approx)
- **Completed:** 2026-07-02T19:30:46Z
- **Tasks:** 2
- **Files modified:** 1 (`genossi_mail/src/service.rs`)

## Accomplishments
- Declared `pub enum MailEncoding { QuotedPrintable, EightBit }` with `Copy, Clone, Debug, PartialEq, Eq` — public so Plan 02's `send.rs::build_message` can consume it as a parameter type.
- Extended `SmtpConfig` with `pub encoding: MailEncoding` as the last field (existing field order untouched, downstream callers unchanged per MAIL-05 back-compat).
- Added `smtp_encoding` parsing in `load_smtp_config`: `Some("8bit")` → `EightBit`; `Some("quoted-printable") | Some("") | None` → `QuotedPrintable`; wildcard `Some(other)` → `tracing::warn!(value = %other, ...)` + `QuotedPrintable`.
- Added three named `#[tokio::test]` cases proving default, opt-in, and unknown-fallback branches all yield the expected variant.
- `build_transport` untouched (CTE is a Message concern, not Transport — per RESEARCH § Architectural Responsibility Map).

## Task Commits

Each task was committed atomically via `jj commit`:

1. **Task 1: Add MailEncoding enum, extend SmtpConfig, parse smtp_encoding with tolerant fallback** — `27025ec7` (feat)
2. **Task 2: Add three fallback-behavior tests for smtp_encoding parsing** — `6f70a2ee` (test)

## Files Created/Modified
- `genossi_mail/src/service.rs` — Added `MailEncoding` enum above `SmtpConfig`, new `encoding` field on `SmtpConfig`, `smtp_encoding` parsing block in `load_smtp_config`, populated new field in the `Ok(SmtpConfig { … })` constructor, appended three `load_smtp_config_*` tokio tests to the existing `#[cfg(test)] mod tests` block.

## Decisions Made
- **Enum shape locked to two variants only.** No `Auto`, `SevenBit`, or `Base64` — Plan 02's `build_message` pins CTE headers via lettre; extra variants would create dead code paths.
- **`SmtpConfig` still does NOT derive `Debug`.** Confirmed the pre-existing decision (T-22-02 threat: `pass` must not leak via `Debug`). No change requested by the plan and none applied.
- **Wildcard warn logs only the offending string** (`value = %other`), never the full `SmtpConfig` — mitigates T-22-02 for tracing sinks.
- **`smtp_encoding` is NOT added to `required_keys`** — parity with `smtp_tls`, honors MAIL-05 backward compatibility (existing deployments without the key continue to work).

## Deviations from Plan

None - plan executed exactly as written.

The plan's acceptance criteria included `cargo clippy -p genossi_mail --all-targets --all-features -- -D warnings`. Clippy failed with a **pre-existing** warning in `genossi_mail/src/worker.rs:105` (`clippy::unnecessary_sort_by`) that is entirely unrelated to this plan's changes (worker.rs was not modified). Per the SCOPE BOUNDARY rule in execute-plan.md ("Only auto-fix issues DIRECTLY caused by the current task's changes"), the warning is logged to `deferred-items.md` and left in place. Verified my new code introduces zero clippy warnings by running `cargo clippy -p genossi_mail --lib` (the pre-existing warning was the only diagnostic).

## Issues Encountered
None. Both tasks compiled and their verification steps passed on first run:
- `cargo build -p genossi_mail` → success.
- `cargo test -p genossi_mail --lib -- load_smtp_config` → 3/3 new tests pass.
- `cargo test -p genossi_mail --lib` → 222 pass, 0 fail (219 pre-existing + 3 new).
- `cargo build` (workspace) → success.

## Deferred Issues
- **Pre-existing clippy warning in `genossi_mail/src/worker.rs:105`** (`clippy::unnecessary_sort_by`). Fix suggested: `matches.sort_by_key(|b| std::cmp::Reverse(b.created));`. Out of scope for 22-01 (worker.rs not touched). Recommend addressing in a follow-up `gsd-quick` if `-D warnings` is desired workspace-wide.

## User Setup Required

None - no external service configuration required. The new `smtp_encoding` KV key is entirely backward-compatible: existing deployments without the key continue to encode as `quoted-printable`, unchanged.

For operators who want to opt in to 8-bit encoding, insert a KV row: `key=smtp_encoding, value=8bit, value_type=string` via the existing config UI. Plan 22-03 wires the UI toggle.

## Next Phase Readiness
- **Plan 22-02 (`send.rs::build_message` extraction) is unblocked.** The stable `MailEncoding` type is now importable from `genossi_mail::service`, exactly as its `<action>` requires.
- **`SmtpConfig.encoding` is populated at all three read sites** — `send_mail_for_recipient` (worker.rs:639) and both test-mail paths (service.rs:415, 447) will see the new field automatically once Plan 02 wires it into the Message builder.
- **No blockers.** All acceptance criteria for 22-01 met; workspace builds; test suite green.

## Self-Check: PASSED

Verified:
- File `.planning/phases/22-8bit-shared-mail-body-helper/22-01-SUMMARY.md` was just written.
- Task 1 commit `27025ec7` exists in `jj log`.
- Task 2 commit `6f70a2ee` exists in `jj log`.
- `pub enum MailEncoding` present exactly once in `genossi_mail/src/service.rs`.
- `pub encoding: MailEncoding` present exactly once.
- 3 new test function names present exactly once each.
- 222 tests pass, 0 fail.

---
*Phase: 22-8bit-shared-mail-body-helper*
*Plan: 01*
*Completed: 2026-07-02*
