# Phase 15 Deferred Items

## Out-of-scope test failures discovered

### `test_mail_preview_repayment_no_entries_does_not_default_to_one` (e2e_tests.rs:13964)

**Discovered during:** Phase 15 Plan 04 regression-run of `cargo test -p genossi_bin --test e2e_tests`.

**Failure:** `panicked at 'errors must be array'` at line 13964 — the mail preview response body for `POST /api/mail/preview` does not contain an `errors` array field as the test expects (parsing the response as `json["errors"].as_array()` returns `None`).

**Scope check:** Phase 15 Plan 04 only modified `genossi_rest_types/src/lib.rs` (3 new TOs added), `genossi_rest/src/{membership_adjust.rs,member.rs,lib.rs}` (new module + sub-route registration + RestStateDef extension), `genossi_bin/src/lib.rs` (DI wiring), and added `genossi_bin/tests/membership_adjust_e2e.rs`. None of these touch the mail-preview render pipeline (`genossi_mail/src/template.rs`, `genossi_rest/src/mail/...`) or the minijinja strict-env template rendering. The failure is in an unrelated subsystem (mail-preview-render-error JSON shape).

**Disposition:** Out-of-scope per SCOPE BOUNDARY rule. The other 293 tests in `e2e_tests` pass; this single failure is pre-existing and unrelated to Phase 15. To be triaged in a separate fix/debug task (likely in a mail-template-related phase or a quick).

**No regression introduced by Phase 15** — all newly-touched files compile clean and all Phase 15 E2E tests + Service unit tests + Plan-14 transfer_recipients_e2e regression pass.
