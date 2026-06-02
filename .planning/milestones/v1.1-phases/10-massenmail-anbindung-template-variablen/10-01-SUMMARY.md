---
phase: 10-massenmail-anbindung-template-variablen
plan: 01
subsystem: mail-pipeline
tags: [migration, dao, sqlite, mail-job]
requires:
  - migrations/sqlite/20260403000003_create_mail_jobs_table.sql
provides:
  - mail_jobs.template_id (BLOB NULL)
  - mail_jobs.repayment_phase_id (BLOB NULL)
  - MailJob.template_id (Option<Uuid>)
  - MailJob.repayment_phase_id (Option<Uuid>)
affects:
  - genossi_mail/src/service.rs (MailJob constructors default new fields to None — Plan 10.03 wires real values)
  - genossi_mail/src/worker.rs (sample_job test fixture)
  - genossi_mail/src/inbox.rs (reply-flow defaults to None)
tech-stack:
  added: []
  patterns:
    - "ALTER TABLE ADD COLUMN BLOB NULL (forward-only, backward-compat)"
    - "parse_optional_uuid<->Vec<u8> roundtrip via TryFrom<&MailJobDb>"
key-files:
  created:
    - migrations/sqlite/20260601000000_extend_mail_job_template_phase.sql
  modified:
    - genossi_mail/src/dao.rs
    - genossi_mail/src/dao_sqlite.rs
    - genossi_mail/src/service.rs
    - genossi_mail/src/worker.rs
    - genossi_mail/src/inbox.rs
decisions:
  - "D-03 addressed: job-wide repayment_phase_id persisted on mail_jobs row"
  - "D-12 addressed: job-wide template_id persisted on mail_jobs row"
  - "update() does NOT touch template_id/repayment_phase_id — immutable post-create (same semantics as subject/body/created)"
  - "MailJob is NOT Auditable (operational entity, not domain) — new columns excluded from any audit_fields() impl"
metrics:
  duration_seconds: 594
  duration_human: "9min 54s"
  completed: "2026-05-31T16:17:17Z"
  tasks_total: 2
  tasks_completed: 2
  files_created: 1
  files_modified: 5
  tests_added: 2
  tests_total_after: 114
---

# Phase 10 Plan 01: Mail-Job Schema-Erweiterung Summary

JWT-style optional refs `template_id` and `repayment_phase_id` added to `mail_jobs` via forward-only `ALTER TABLE ADD COLUMN BLOB NULL` migration; `MailJob` entity, SQLite mapping, and four downstream constructors updated to round-trip them — Plan 10.03 (`MailService::create_job` signature) will start passing real values, while inbox/test-mail keep them `None`.

## Tasks Executed

### Task 1 — Migration anlegen (template_id, repayment_phase_id)

- **Commit:** `c1b3803`
- **File created:** `migrations/sqlite/20260601000000_extend_mail_job_template_phase.sql`
- **Content:** two `ALTER TABLE mail_jobs ADD COLUMN ... BLOB NULL` statements with ADR-style header explaining D-12 (template tracking), D-03 (job-wide repayment context), FK-as-documentation convention (project does not enable `PRAGMA foreign_keys=ON`), and the forward-only constraint (SQLite < 3.35 cannot remove columns).
- **Verification:** `grep -c "ALTER TABLE mail_jobs ADD COLUMN template_id BLOB NULL"` → 1; `grep -c "repayment_phase_id BLOB NULL"` → 1; `grep -c "DROP COLUMN\|DROP TABLE"` → 0; `cargo build -p genossi_mail` → clean.

### Task 2 — MailJob struct + SQLite-Impl Erweiterung (TDD)

- **RED commit:** `651a63f` — added two failing round-trip tests that referenced not-yet-existing struct fields (compile-fail = 6 × `E0609`).
- **GREEN commit:** `645e6b9` — extended `MailJob` struct, `MailJobDb`, `TryFrom<&MailJobDb>`, INSERT (11 → 13 placeholders), SELECT statements in `find_by_id` + `all`, the test-module `CREATE TABLE mail_jobs (...)`, the `sample_job()` fixture, plus four downstream `MailJob {...}` constructors (`service.rs` × 4, `worker.rs` × 1, `inbox.rs` × 1) — all default the new fields to `None` per the plan's "Zwischen-Stand" guidance.
- **Files modified:**
  - `genossi_mail/src/dao.rs` (struct extension + inline comments tying each field to its decision)
  - `genossi_mail/src/dao_sqlite.rs` (struct + TryFrom + INSERT/SELECT × 2 + setup_db + sample_job + 2 new tests)
  - `genossi_mail/src/service.rs` (3 test fixtures + 1 production `create_job` body)
  - `genossi_mail/src/worker.rs` (1 test fixture)
  - `genossi_mail/src/inbox.rs` (1 production reply-flow body)
- **Tests:** `test_mail_job_roundtrip_with_template_and_phase_ids` (Some/Some) + `test_mail_job_roundtrip_with_null_template_and_phase_ids` (None/None) — both green; full `cargo test -p genossi_mail --lib` = 114 passed / 0 failed.
- **Verification:** all acceptance-criteria greps satisfied (struct field count, SQLite mention count ≥ 4, INSERT INTO mail_jobs contains template_id, `cargo build -p genossi_mail` clean, `cargo fmt --check -p genossi_mail` clean, `cargo clippy -p genossi_mail --all-targets` produces no new warnings/errors beyond pre-existing).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 — Bug] Acceptance-criterion conflict on `grep DROP COLUMN` returns 0**

- **Found during:** Task 1 verification.
- **Issue:** The ADR-style header comment in the migration originally contained the literal substring `"DROP COLUMN"` (in the sentence "No down-migration: SQLite < 3.35 has no DROP COLUMN."). The plan acceptance criterion required `grep -c "DROP COLUMN\|DROP TABLE"` → 0, even though that match was a comment, not a destructive operation.
- **Fix:** Rephrased the comment to "No down-migration: SQLite < 3.35 cannot remove columns." Same semantic content, satisfies the acceptance gate.
- **Files modified:** `migrations/sqlite/20260601000000_extend_mail_job_template_phase.sql`
- **Commit:** `c1b3803` (initial commit; the rewording was done before the commit was created — no second commit required).

No other deviations: the plan was executed exactly as written. Inline TDD discipline (RED then GREEN), no auto-fixes for bugs or missing functionality, no architectural decisions deferred.

## Auth Gates

None. Plan was fully autonomous (no authentication, server startup, or external service interactions).

## TDD Gate Compliance

Plan-level type is `execute` (not `tdd`), but Task 2 carried `tdd="true"`. The git log shows the required sequence:

1. `c1b3803` — `feat(10-01): add mail_jobs migration ...` (Task 1, not TDD)
2. `651a63f` — `test(10-01): add failing roundtrip tests ...` (Task 2 RED gate)
3. `645e6b9` — `feat(10-01): persist template_id + repayment_phase_id on MailJob` (Task 2 GREEN gate)

RED proved failure via `cargo build -p genossi_mail --tests` showing 6 × `E0609` `no field` errors. GREEN proved pass via `cargo test -p genossi_mail --lib test_mail_job_roundtrip` → 2 passed. No REFACTOR commit was required — the GREEN diff is already minimal and idiomatic (no duplicated logic, no obvious extract-method opportunity).

## Threat Flags

None. All new columns map directly to existing audited-entity references (`mail_template.id`, `repayment_phase.id`) — they are routing pointers, not new trust surface. The plan's `<threat_model>` is fully covered by the implementation: parameterized SQLx binds (T-10-01-01), `parse_optional_uuid` validation on read (T-10-01-03), `mail_jobs` is operational, not Auditable (T-10-01-04).

## Known Stubs

None. Both new fields are real persisted columns; the `None` defaults in downstream constructors are explicitly scoped placeholders that Plan 10.03 will replace with real values from the extended `create_job` signature. Inline comments mark the temporary defaults.

## Self-Check: PASSED

**Files claimed created/modified — existence check:**

- `migrations/sqlite/20260601000000_extend_mail_job_template_phase.sql` → FOUND
- `genossi_mail/src/dao.rs` → modified, MailJob struct has `template_id` and `repayment_phase_id` (verified via `grep`)
- `genossi_mail/src/dao_sqlite.rs` → modified, `MailJobDb` + INSERT + SELECT + setup_db all updated (verified via `grep`)
- `genossi_mail/src/service.rs` → 4 MailJob constructors default new fields to None (verified via `grep -c "template_id: None,"` → 4 occurrences in service.rs)
- `genossi_mail/src/worker.rs` → 1 MailJob fixture updated
- `genossi_mail/src/inbox.rs` → 1 MailJob constructor updated

**Commits claimed — existence check:**

- `c1b3803` → FOUND in `git log --oneline --all | grep c1b3803`
- `651a63f` → FOUND in `git log --oneline --all | grep 651a63f`
- `645e6b9` → FOUND in `git log --oneline --all | grep 645e6b9`

All claims verified.
