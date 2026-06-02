# Deferred Items — Quick 260602-sgp

Out-of-scope discoveries logged per CLAUDE.md SCOPE BOUNDARY rule.

## Pre-existing failing E2E test

- **Test:** `test_letter_idempotency_d13_08_and_no_status_toggle_d13_09`
- **File:** `genossi_bin/tests/repayment_letter_e2e.rs:794`
- **Symptom:** Second POST `/api/repayment-phase/{id}/letters/generate` returns
  409 Conflict instead of expected 200 OK.
- **Verified:** Test was already failing on base commit `4f9cc1f` BEFORE any
  Quick 260602-sgp changes were applied (verified by `git checkout 4f9cc1f --
  ...` + re-running the test in isolation; identical failure mode).
- **Reason for not fixing in this quick:** Outside the scope of "Bulk-Download
  of persisted RepaymentLetter PDFs"; relates to the existing
  `POST /letters/generate` idempotency path (D-13-08), not the new
  `GET /letters/download` endpoint.
- **Suggested follow-up:** a separate quick task or hotfix targeting the
  idempotent `generate()` path. Likely cause: shared state in the test server
  setup or a race between the first `/letters/generate` call's commit and the
  Phase-status check funnel reading a stale (cached?) status. Worth comparing
  to similar regression in the audit-chain test that ran cleanly during this
  task.
