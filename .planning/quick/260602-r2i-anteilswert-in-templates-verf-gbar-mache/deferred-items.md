# Deferred Items — Quick 260602-r2i

## Pre-existing E2E test failure (out of scope)

**Test:** `test_letter_idempotency_d13_08_and_no_status_toggle_d13_09`
**File:** `genossi_bin/tests/repayment_letter_e2e.rs:847`
**Failure:** Second bulk-call returns 409 (expected 200 per D-13-08 idempotency).

### Verification that this is pre-existing

I cloned the worktree to a fresh checkout at the plan's base commit `36d79a5`
(`docs(260602-r2i): pre-dispatch plan for share_value in Repayment templates`)
and ran the test. It already failed with identical output:

```
assertion `left == right` failed: D-13-08: Zweiter Bulk-Call muss 200 sein
(keine Idempotenz-Sperre)
  left: 409
 right: 200
```

The test was therefore broken before this quick-task started.

### Scope decision

Per Rule 4 / scope boundary, this is not caused by the r2i changes and is not
fixable inside the r2i scope. It looks adjacent to the just-shipped quick task
260602-q9l ("idempotent Repayment-Letter regeneration") — a separate quick
task should investigate why the bulk-letter endpoint still returns 409 on the
second call when the regeneration path is supposed to be idempotent.

### What r2i did NOT change

- All Letter-E2E test setup (member/phase/entry construction).
- The bulk-letter REST handler.
- The `audited_create!`/`audited_update!` cascade for MemberDocuments.

The r2i additive change (`share_value` in JSON inputs + a new template line)
is orthogonal to the idempotency-on-second-call concern.

### Other E2E tests in the same suite

6 of 7 active tests in `repayment_letter_e2e.rs` pass after r2i — including
two of the most-relevant ones for r2i's invariants:
- `test_letter_null_iban_renders_ok` (Pitfall #5: NULL-IBAN still renders)
- `test_letter_multi_entry_aggregation_d13_04` (multi-entry aggregation)
- `test_letter_happy_path_3_entries_2_members` (end-to-end PDF flow)
