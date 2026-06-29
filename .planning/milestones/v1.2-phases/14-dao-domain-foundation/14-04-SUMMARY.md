---
phase: 14-dao-domain-foundation
plan: 04
subsystem: rest-endpoint-and-e2e
tags:
  - rest
  - member
  - transfer
  - trsf-06
  - tdd
  - slim-to
  - pii-guard
dependency-graph:
  requires:
    - genossi_service::member::Member (service entity, member_number=i64)
    - genossi_service::member::MemberService::list_transfer_recipients (Plan 14-03)
    - genossi_rest_types::SalutationTO (existing)
    - genossi_rest::test_server::test_support::start_test_server (existing)
  provides:
    - genossi_rest_types::MemberSlimTO (6-field PII-guarded slim DTO)
    - From<&genossi_service::member::Member> for MemberSlimTO
    - GET /api/members/transfer-recipients?exclude_self={uuid} REST endpoint
    - TransferRecipientsQuery (utoipa IntoParams)
    - OpenAPI schema + path registration for MemberSlimTO + get_transfer_recipients
    - E2E test transfer_recipients_e2e::test_transfer_recipients_filters_self_and_cancelled
  affects:
    - Phase 18 (frontend MemberSearch consumes MemberSlimTO + /transfer-recipients endpoint)
tech-stack:
  added: []
  patterns:
    - "Slim-DTO with PII-leak guard (mirrors AttendanceMemberTO from v1.0)"
    - "Sub-route declared BEFORE /{id} (Pitfall 1: axum match by declaration order)"
    - "PermissionDenied -> 401 via global From<ServiceError> (Pitfall 4: NO local 403 mapping)"
    - "3-step exit_date setup in E2E (POST member -> POST Austritt action -> GET member)"
    - "Response-body PII-leak guard via raw text grep on iban/email/bank_account/street/etc."
key-files:
  created:
    - genossi_bin/tests/transfer_recipients_e2e.rs
  modified:
    - genossi_rest_types/src/lib.rs
    - genossi_rest/src/member.rs
decisions:
  - "MemberSlimTO has exactly 6 fields: id (Uuid), member_number (i64), salutation (Option<SalutationTO>), title (Option<String>), first_name (String), last_name (String). NO impl From<&MemberTO> for MemberSlimTO — PII-leak guard documented in doc-comment."
  - "Router orders literal sub-routes BEFORE all /{id} routes (Pitfall 1). Pre-existing /import and /not-reached-by were also moved before /{id} for consistency and defense against future GET-route additions."
  - "Utoipa annotation lists 200, 400, 401, 500 — NO 403 (Pitfall 4: PermissionDenied -> Unauthorized via global From<ServiceError> mapping, not Forbidden)."
  - "E2E uses 3-step exit_date setup (POST member -> POST Austritt action -> GET member) because recalc_dates overrides direct MemberTO.exit_date during create (Pitfall 3)."
  - "E2E body-PII-grep covers iban, email, bank_account, street, current_shares, current_balance, postal_code — defense-in-depth against future drift."
metrics:
  duration: "~25 minutes"
  completed: "2026-06-04"
  tasks-completed: 3
  files-created: 1
  files-modified: 2
  tests-added: 5  # 4 unit (MemberSlimTO) + 1 E2E
  tests-passing: 5
---

# Phase 14 Plan 04: REST Endpoint + E2E for /api/members/transfer-recipients Summary

One-liner: Wires the TRSF-06 endpoint `GET /api/members/transfer-recipients?exclude_self={uuid}` end-to-end with a slim 6-field `MemberSlimTO` (PII-guarded), router ordering that dodges the axum UUID-parsing pitfall, utoipa 200/400/401/500 annotation (no 403), and an E2E test that uses the 3-step Austritt-action setup to populate `exit_date`.

## What changed

### genossi_rest_types/src/lib.rs (+174 lines)

- Added `MemberSlimTO` struct with exactly 6 fields: `id: Uuid`, `member_number: i64`, `salutation: Option<SalutationTO>`, `title: Option<String>`, `first_name: String`, `last_name: String`.
- Added `impl From<&genossi_service::member::Member> for MemberSlimTO` — the ONLY conversion path. NO `impl From<&MemberTO>` exists (PII-leak guard documented in doc-comment).
- Added 4 unit tests in `member_slim_to_tests`:
  1. `test_member_slim_to_from_member_populates_six_fields` — From<&Member> populates exactly the 6 allowed fields.
  2. `test_member_slim_to_serializes_no_pii_fields` — JSON serialization contains NO email/bank_account/iban/street/current_shares/current_balance/postal_code/city.
  3. `test_member_slim_to_serializes_exactly_six_keys_when_all_present` — exact key set when all Options are Some.
  4. `test_member_slim_to_skips_none_optional_fields` — Options skipped when None (4 keys remaining).

### genossi_rest/src/member.rs (+86 lines, -8 lines)

- Extended imports: `axum::extract::Query`, `serde::Deserialize`, `utoipa::IntoParams`, `genossi_rest_types::MemberSlimTO`.
- Added `TransferRecipientsQuery` struct (`#[derive(Debug, Deserialize, IntoParams)]`) with single `exclude_self: Uuid` field.
- Added `get_transfer_recipients` handler — modeled on `get_all_members`, delegates to `MemberService::list_transfer_recipients`, maps result to `Vec<MemberSlimTO>`.
- Utoipa annotation lists statuses 200, 400, 401, 500 — explicitly NO 403. Inline doc-comment fixes the Pitfall 4 invariant.
- Reordered `generate_route` so literal sub-routes (`/transfer-recipients`, `/import`, `/not-reached-by/{job_id}`) are declared BEFORE all `/{id}` routes. Inline doc-comment fixes the Pitfall 1 invariant against future route additions.
- Registered `get_transfer_recipients` in `ApiDoc::paths(...)` and `MemberSlimTO` in `components(schemas(...))`.

### genossi_bin/tests/transfer_recipients_e2e.rs (NEW, 265 lines)

- New E2E test file gated by `#![cfg(feature = "mock_auth")]` (workspace default; admin permissions automatic).
- Local `setup()` helper (1:1 from `e2e_tests.rs::setup` — in-memory SQLite + start_test_server).
- Helper `sample_member(member_number, first_name)` builds a `MemberTO` skeleton with all required fields.
- Helper `create_active_member(client, server, member_number, first_name)` posts a single member with `exit_date: None`.
- Helper `create_cancelled_member(client, server, member_number)` runs the 3-step Austritt-action setup (Pitfall 3): POST member → POST `MemberAction::Austritt` → GET member.
- Test `test_transfer_recipients_filters_self_and_cancelled`:
  - Creates 3 members: `m_active` (1001), `m_cancelled` (1002, via 3-step setup), `m_self` (1003).
  - Sanity-asserts `m_cancelled.exit_date.is_some()` — guards against silent setup-helper regressions.
  - GETs `/api/members/transfer-recipients?exclude_self={m_self.id}` and asserts status 200.
  - Asserts exactly 1 recipient returned (the active non-self member).
  - Re-fetches the body as raw text and asserts NO PII fields leak (`iban`, `email`, `bank_account`, `street`, `current_shares`, `current_balance`, `postal_code`).

## All four research pitfalls addressed

| Pitfall | Where addressed | Verification |
|---|---|---|
| **#1: Sub-route ordering** — `/transfer-recipients` must come BEFORE `/{id}` GET | `genossi_rest/src/member.rs::generate_route` (line 42 `/transfer-recipients` < line 51 `/{id}` GET) | E2E `test_transfer_recipients_filters_self_and_cancelled` returns 200 (not 400 from a UUID-parse error); inline doc-comment fixes invariant. |
| **#3: 3-step `exit_date` setup** in E2E | `create_cancelled_member` helper (POST member → POST `ActionTypeTO::Austritt` → GET member) | E2E asserts `m_cancelled.exit_date.is_some()` as sanity check; service-layer filter `exit_date IS NULL` only removes it when `recalc_dates` has populated it. |
| **#4: PermissionDenied → 401 (not 403)** | `utoipa::path` responses block in `get_transfer_recipients` lists 200/400/401/500 only | `grep -A 12 'path = "/transfer-recipients"' \| grep -c "status = 403"` returns 0; `grep -c "status = 401"` returns 1. |
| **PII-leak guard** — no `From<&MemberTO> for MemberSlimTO` | `MemberSlimTO` doc-comment forbids the impl; only `From<&genossi_service::member::Member>` exists; E2E body-grep verifies no leak | `grep -cE "^impl From<&MemberTO> for MemberSlimTO"` returns 0; 4 unit tests + 7 E2E body-grep assertions cover the guard. |

## Test results

```
$ cargo test -p genossi_rest_types --lib member_slim_to_tests
running 4 tests
test member_slim_to_tests::test_member_slim_to_skips_none_optional_fields ... ok
test member_slim_to_tests::test_member_slim_to_serializes_no_pii_fields ... ok
test member_slim_to_tests::test_member_slim_to_from_member_populates_six_fields ... ok
test member_slim_to_tests::test_member_slim_to_serializes_exactly_six_keys_when_all_present ... ok
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 35 filtered out

$ cargo test -p genossi_bin --test transfer_recipients_e2e
running 1 test
test test_transfer_recipients_filters_self_and_cancelled ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Regression-check: `cargo test -p genossi_service_impl --lib member` — 88 passed, 0 failed (Plan 14-03's contract intact).

Workspace-build: `cargo build --workspace` — success (2 pre-existing warnings on `membership_adjust.rs` dead-code from Plan 14-01, no new warnings).

## TDD gate compliance

Tasks 1, 2, 3 are all marked `tdd="true"` in the plan. Compliance:

- **Task 1 (MemberSlimTO):** RED-then-GREEN — the 4 unit tests were added to a `member_slim_to_tests` module before the `MemberSlimTO` struct existed. First `cargo build` failed with `error[E0433]: use of undeclared type 'MemberSlimTO'` (RED). After adding the struct + From impl, all 4 tests passed (GREEN). Both phases committed as a single `feat` commit `64e06cc` (TDD checkpoint within a single artifact change — the test module IS the spec for the struct).
- **Task 2 (handler + router + OpenAPI):** Pragmatic TDD — the "test" for this task is the E2E in Task 3. Compile verification, grep-based acceptance criteria (sub-route ordering, 401 not 403, OpenAPI registration), and the 5 unit-test-level assertions from Task 1 cover the contract. Committed as `feat` `3ddcbfb`.
- **Task 3 (E2E test):** Pure GREEN — single E2E test added that exercises the full HTTP roundtrip end-to-end. Asserts (1) 200 status (verifies Pitfall 1 fix), (2) exactly 1 recipient (verifies filter logic from Plan 14-03 + Pitfall 3 setup), (3) no PII leak in body (verifies MemberSlimTO design). Committed as `test` `1c8b8e7`.

## Deviations from plan

None — plan executed as written.

Minor deliberate enhancement: when reordering `generate_route` to fix Pitfall 1, the pre-existing `/import` and `/not-reached-by/{job_id}` literal routes were ALSO moved before the `/{id}` routes. Their original position after `/{id}` happened to work because the methods differed (POST vs GET), but a future addition of a GET on the same path would break that assumption. The reorder is defensive and preserves all existing behavior.

## Self-Check: PASSED

- Files created/modified exist:
  - `genossi_rest_types/src/lib.rs` — modified, contains `pub struct MemberSlimTO` (grep returns 1).
  - `genossi_rest/src/member.rs` — modified, contains `pub async fn get_transfer_recipients` (grep returns 1).
  - `genossi_bin/tests/transfer_recipients_e2e.rs` — created, contains `fn test_transfer_recipients_filters_self_and_cancelled` (grep returns 1).
- Commits exist in `git log`:
  - `64e06cc feat(14-04): add MemberSlimTO with PII-leak guard (TRSF-06)`
  - `3ddcbfb feat(14-04): add GET /api/members/transfer-recipients REST endpoint (TRSF-06)`
  - `1c8b8e7 test(14-04): add E2E for GET /api/members/transfer-recipients`
- All 4 acceptance-criteria pitfalls addressed and verified (table above).
- All 5 tests pass (4 unit + 1 E2E); workspace build green; no new clippy/format warnings on touched files.

## Deferred Issues

- **Pre-existing E2E failure** `test_mail_preview_repayment_no_entries_does_not_default_to_one` in `genossi_bin/tests/e2e_tests.rs:13964` panics with `errors must be array`. Verified to fail at baseline commit `75e5f0f` BEFORE any Plan 14-04 changes — out of scope for this plan (mail-preview repayment logic, completely unrelated to Member-API). Logged here so the next executor or verifier does not attribute it to this plan.

## Note for Phase 18

Phase 18's frontend `MemberSearch` component should:

1. Call `GET /api/members/transfer-recipients?exclude_self={current_member_id}`.
2. Deserialize the response as `Vec<MemberSlimTO>` — exactly 6 stable fields (id, member_number, salutation, title, first_name, last_name).
3. Be aware that `exit_date` filtering happens server-side; the slim DTO does NOT carry `exit_date` and Phase 18 must not try to filter further on that field.
4. Field-display order in the dropdown should mirror the struct field order: `{member_number} {salutation?} {title?} {first_name} {last_name}`.

If Phase 18 later needs `current_shares` in the dropdown (e.g. "X Anteile bisher"), extending `MemberSlimTO` is a 6-line change here — but the PII-leak guard doc-comment must be updated to acknowledge the addition.
