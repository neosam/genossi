# Deferred Items — Quick 260625-e14

## Pre-existing e2e failure (out of scope)

- **Test:** `genossi_bin::e2e_tests::test_mail_preview_repayment_no_entries_does_not_default_to_one`
- **Status:** FAILED on HEAD, NOT caused by this task.
- **Evidence:** `git diff HEAD genossi_mail/src/rest.rs` shows the only change is a single
  line added inside a `#[cfg(test)]` module (`postal_status: ...Erreichbar`). The production
  `/api/mail/preview` handler is byte-identical to HEAD, so the rendering behavior this test
  asserts on is unchanged by this task.
- **Symptom:** `json["errors"].as_array()` panics because `errors` is omitted from the
  preview response (`#[serde(skip_serializing_if = "Vec::is_empty")]`) — the strict-env render
  did not produce the expected error, i.e. a regression in the Quick-c19 mail-preview path.
- **Action:** Left untouched per executor scope boundary (only auto-fix issues directly caused
  by this task's changes). Needs separate investigation of the mail-preview repayment-context
  resolution, unrelated to PostalStatus.
