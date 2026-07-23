# Deferred Items — Phase 27

Out-of-scope discoveries logged during execution (SCOPE BOUNDARY rule). NOT fixed.

## Pre-existing e2e test failures (unrelated to plan 27-01)

Discovered during the workspace-wide regression run for plan 27-01. Both tests
depend on `genossi_mail/src/render.rs` (plain-from-HTML derivation), which plan
27-01 does NOT touch. They fail on the untouched baseline as well.

- `genossi_bin/tests/e2e_tests.rs::preview_body_html_round_trips_to_response`
  - Assertion: plain body expected `"Hallo Max"` but got `"Hallo **Max**"`.
  - Cause: HTML→plain derivation leaks Markdown-style `**bold**` markers.
  - Owning code: `genossi_mail/src/render.rs` (`plain_from_html`).

- `genossi_bin/tests/e2e_tests.rs::test_mail_preview_repayment_no_entries_does_not_default_to_one`
  - Failure surfaced with "errors must be array" during the same run.
  - Owning code: mail-preview / repayment aggregation path (genossi_mail).

These belong to the mail-render surface (Phase 27 plan 27-03 hardens sanitize/
render). If still red after 27-03, address there or open a dedicated fix.
