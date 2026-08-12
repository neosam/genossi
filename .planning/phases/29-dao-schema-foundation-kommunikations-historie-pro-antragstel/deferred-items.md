# Deferred Items — Phase 29

Out-of-scope discoveries logged during execution. NOT fixed here (scope boundary:
only issues directly caused by the current plan's changes are auto-fixed).

## Pre-existing e2e failures (discovered during 29-02, unrelated to this plan)

Two `genossi_bin` e2e tests fail on the **baseline commit `4f7940e`** (verified in an
isolated worktree) — i.e. they were already red BEFORE plan 29-02 and are unrelated to
the Antragsteller-Historie / carry-over work (they exercise `/api/mail/preview`
markdown + minijinja strict-env rendering, code untouched by 29-02):

1. `preview_body_html_round_trips_to_response` (genossi_bin/tests/e2e_tests.rs)
   - Fails: `plain body must render member first_name` — left `"Hallo **Max**"`, right `"Hallo Max"`.
   - Markdown bold (`**Max**`) is not stripped to plain text in the preview body path.

2. `test_mail_preview_repayment_no_entries_does_not_default_to_one` (genossi_bin/tests/e2e_tests.rs)
   - Fails: `errors must be array` — the `/api/mail/preview` response `errors` field is not an array
     in the no-repayment-entries case.

Both reproduce at `4f7940e` (pre-29-02). Left for a dedicated fix (likely a mail-preview
render/serialization change). All 29-02 tasks and their tests pass; the rest of the e2e
suite (315 tests) is green.
