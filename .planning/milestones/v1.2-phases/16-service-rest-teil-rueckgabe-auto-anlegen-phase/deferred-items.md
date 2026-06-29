# Phase 16 — Deferred Items

## Out-of-scope failures discovered during Plan 16-04 execution

### 1. `test_mail_preview_repayment_no_entries_does_not_default_to_one` (pre-existing failure)

- **File:** `genossi_bin/tests/e2e_tests.rs:13964`
- **Failure:** `panicked at .. errors must be array` — `json["errors"]` is not an array in the `/api/mail/preview` response.
- **Verified pre-existing:** `git show 6fdc4c4:genossi_bin/tests/e2e_tests.rs` shows identical test code on the Plan-16-04 worktree base (commit `6fdc4c4`). Plan 16-04 only touches `genossi_bin/src/lib.rs` (DI wiring), `genossi_rest/src/member.rs` (sub-route registration), and `genossi_rest/src/membership_adjust.rs` (new handler). None of those touch mail-preview, repayment-letter rendering, or the `/api/mail/preview` handler.
- **Action:** NOT fixed in Plan 16-04 (out of scope per executor SCOPE BOUNDARY rule). To be triaged separately — likely related to Quick-c19 (`62e62b7` "Bulk-Mail mit per-Empfänger-RepaymentLetter") landing earlier on `main`.
