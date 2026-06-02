---
phase: 10-massenmail-anbindung-template-variablen
plan: 04
subsystem: mail-pipeline
tags: [rest, request-body, openapi, uuid-parsing, todo-substitution]
requires:
  - phase: 10-massenmail-anbindung-template-variablen Plan 10.03
    provides: MailService::create_job 7-arg signature with two trailing Option<Uuid> (template_id, repayment_phase_id) and TODO(10.04) placeholders at the bulk-send call-site
provides:
  - SendBulkMailRequest with two new `#[serde(default)] Option<String>` fields (template_id, repayment_phase_id) carrying ToSchema docs that cite D-12 / D-03
  - send_bulk_mail handler parses both strings via uuid::Uuid::parse_str; invalid -> MailServiceError::BadRequest -> HTTP 400 with input-echoed message
  - Bulk-send create_job call-site now receives the parsed UUIDs (TODO(10.04) placeholders from 82c8515 removed)
  - 11 E2E test call-sites updated to pass `template_id: None, repayment_phase_id: None` so the struct extension is non-breaking for downstream
affects:
  - Phase 10 Plan 06 (worker uses MailJob.template_id / repayment_phase_id for context merge + MemberDocument-create)
  - Phase 10 Plan 08 (E2E test for bulk-mail-with-repayment_phase_id end-to-end)
  - Phase 12 (frontend mass-mail page sends both fields when arriving from the repayment-phase detail page)
tech-stack:
  added: []
  patterns:
    - "Append-only Option<String> request-body fields with #[serde(default)] for backward compatibility"
    - "uuid::Uuid::parse_str with match-guard on Some(s) if !s.is_empty() to treat empty strings like absent fields"
    - "MailServiceError::BadRequest with input-echoed message for caller-controlled parse errors (T-10-04-02 accepted)"
    - "Test-call-site fixup as part of GREEN commit when extending a struct that has many downstream literal-initializers"
key-files:
  created: []
  modified:
    - genossi_mail/src/rest.rs
    - genossi_bin/tests/e2e_tests.rs
key-decisions:
  - "Use MailServiceError::BadRequest (already wired to HTTP 400 in error_handler) rather than reusing MailServiceError::NotFound (the static_document_ids pattern). The BadRequest variant exists, the error_handler maps it to 400 already, and the semantic is more precise: a malformed UUID is a request error, not a missing resource."
  - "Echo the user-provided string in the 400 message (`format!(\"Invalid template_id UUID: {}\", s)`). T-10-04-02 disposition `accept`: no PII leak since the caller is echoing their own input, and the diagnostic helps frontend developers debug their payload."
  - "Match-guard `Some(s) if !s.is_empty() => parse(s), _ => None` treats empty-string and absent equally — neither attempts a parse, both yield None. This avoids 400-on-empty-string surprises if a frontend sends `\"\"` as JSON null-substitute."
  - "Fix the 11 downstream E2E call-sites in the GREEN commit (Rule 3 auto-fix) rather than a separate refactor commit. The struct extension is structurally incomplete until all downstream literals compile; splitting would leave the tree in a broken state between the two commits."
patterns-established:
  - "BadRequest-with-input-echoed-message pattern for caller-controlled UUID/UUIDish parsing in genossi_mail REST handlers"
  - "TODO(NN.PP)-comment-marker convention from Plan 10.03 fully resolved in Plan 10.04 (grep -c TODO(10.04) drops from 2 to 0)"
requirements-completed:
  - MAIL-01
  - MAIL-02
duration: 8min
completed: 2026-05-31
---

# Phase 10 Plan 04: REST send-bulk Body-Erweiterung Summary

**SendBulkMailRequest gets two optional `Option<String>` UUID fields (template_id D-12, repayment_phase_id D-03), parsed via `uuid::Uuid::parse_str` with `MailServiceError::BadRequest` -> HTTP 400 echoing the malformed input, replacing the two `TODO(10.04)` placeholders in the bulk-send `create_job(...)` call-site from Plan 10.03's commit 82c8515.**

## Performance

- **Duration:** 8 min
- **Started:** 2026-05-31T16:52:35Z
- **Completed:** 2026-05-31T17:00:10Z
- **Tasks:** 1 (TDD, 2 commits)
- **Files modified:** 2

## Accomplishments

- `SendBulkMailRequest` carries two new `#[serde(default)] #[schema(example=...)] pub template_id/repayment_phase_id: Option<String>` fields with doc-comments citing Phase 10 D-12 / D-03 and the 400-BadRequest contract.
- `send_bulk_mail` handler parses both via `uuid::Uuid::parse_str` and routes parse errors to `MailServiceError::BadRequest(format!(\"Invalid {field}_id UUID: {input}\"))` (which the existing `error_handler` already maps to HTTP 400).
- The `None, None` TODO-placeholders from Plan 10.03 are replaced with the parsed `template_id` / `repayment_phase_id` UUIDs in the bulk-send `create_job(...)` call.
- The single-send `send_mail` handler is **unchanged** and continues to pass `None, None` permanently by design.
- 11 `SendBulkMailRequest { ... }` literal-initializers in `genossi_bin/tests/e2e_tests.rs` are extended with `template_id: None, repayment_phase_id: None` so the workspace builds (Rule 3 auto-fix — see Deviations).
- Acceptance-greps:
  - `grep -c \"pub template_id: Option<String>\" genossi_mail/src/rest.rs` -> **1**
  - `grep -c \"pub repayment_phase_id: Option<String>\" genossi_mail/src/rest.rs` -> **1**
  - `grep -c \"TODO Plan 10.04\\|TODO(10.04)\" genossi_mail/src/rest.rs` -> **0** (placeholders fully removed)
  - `grep -c \"Uuid::parse_str\" genossi_mail/src/rest.rs` -> **8** (static_document_ids + template_id + repayment_phase_id + 5 other pre-existing UUID-parse sites for preview/job_detail/retry_job/etc.)
  - `grep -B1 -A12 \"\\.create_job(\" genossi_mail/src/rest.rs | grep -c \"template_id\\|repayment_phase_id\"` -> **4** (two new args present at the bulk call-site, two comments referencing both names)
  - `grep -A20 \"pub async fn send_mail\" genossi_mail/src/rest.rs | grep -c \"None\"` -> **3** (single-send still passes the two trailing None,None permanently)

## Task Commits

Task 1 was executed as a TDD RED-GREEN cycle (Rule: plan-mandated `tdd=\"true\"`):

1. **RED — `25633d8`** `test(10-04): add failing serde tests for SendBulkMailRequest phase 10 fields`
   - Added two `#[test]` functions in `genossi_mail::rest::tests`:
     - `test_send_bulk_mail_request_serde_with_phase10_fields` — asserts deserialization of a JSON payload carrying both UUID strings yields `Some(\"550e8400-...\")` / `Some(\"660e8400-...\")`.
     - `test_send_bulk_mail_request_serde_without_phase10_fields_backward_compat` — asserts a payload without the two new keys yields `None` / `None`.
   - **Proof of RED:** `cargo build -p genossi_mail --tests` failed with **4× E0609 (`no field 'template_id'` / `no field 'repayment_phase_id'` on `rest::SendBulkMailRequest`)** at lines 614, 621, 633, 634 of `rest.rs` at that snapshot — RED proven before any production code change.

2. **GREEN — `6e0d1d1`** `feat(10-04): extend SendBulkMailRequest with template_id + repayment_phase_id`
   - Added two `Option<String>` fields with `#[serde(default)]` + `#[schema(example=...)]` + doc-comments citing D-12 / D-03.
   - Added the two `Uuid::parse_str` blocks in `send_bulk_mail` with `MailServiceError::BadRequest` mapping (which the existing `error_handler` already routes to HTTP 400 — verified at `rest.rs:215-221`).
   - Replaced the two `// TODO(10.04): parse body.template_id` / `// TODO(10.04): parse body.repayment_phase_id` placeholder-comments with `template_id, // Phase 10 D-12` and `repayment_phase_id, // Phase 10 D-03` arg-substitutions.
   - Updated 11 SendBulkMailRequest literal-initializer sites in `genossi_bin/tests/e2e_tests.rs` with `template_id: None, repayment_phase_id: None` (Rule 3 auto-fix; details in Deviations).
   - **Proof of GREEN:** `cargo test -p genossi_mail --lib test_send_bulk_mail_request_serde` -> `2 passed; 0 failed`; `cargo test --workspace --lib` -> 730 lib tests pass (40+0+16+70+61+118+62+35+52+276), 2 ignored, 0 failed (net +2 vs. Plan 10.03's 116 in genossi_mail); `cargo test --test e2e_tests` -> 279 passed, 0 failed (identical to Plan 10.03 — no regression).

**Plan metadata:** (this commit follows after self-check) — see Final Commit section.

## Files Created/Modified

- `genossi_mail/src/rest.rs` (+39 lines net):
  - Struct `SendBulkMailRequest` (Z. 112-135 post-edit): +2 fields with utoipa schema docs.
  - Function `send_bulk_mail` (Z. 307-403 post-edit): +2 UUID-parse blocks, +2 arg substitutions in `create_job(...)` call, -2 TODO-placeholders.
  - `#[cfg(test)] mod tests` at end of file: +2 serde-roundtrip tests with doc-comments citing D-12 / D-03 and backward-compat rationale.
- `genossi_bin/tests/e2e_tests.rs` (+22 lines net): 11 SendBulkMailRequest `{ ... }` literal-initializers extended with `template_id: None, repayment_phase_id: None`. No semantic change — the existing assertions and call-paths are unchanged.

## Decisions Made

1. **BadRequest variant over NotFound for UUID parse-error mapping.** The plan's `<action>` allowed either, conditioned on `grep -n \"BadRequest\" genossi_mail/src/service.rs`. The variant exists at `genossi_mail/src/service.rs:20` (`BadRequest(Arc<str>)`) and the `error_handler` in `rest.rs:215-221` already maps it to HTTP 400 with the message echoed in the JSON body. This is semantically more precise than reusing `NotFound` (which the static_document_ids pattern uses) and yields the contract documented in the schema-doc-comment: `\"Must be a valid Uuid string; invalid -> 400 BadRequest.\"`
2. **Empty-string treated as absent.** Match-guard `Some(s) if !s.is_empty() => parse(s), _ => None` ensures that a frontend sending `\"\"` (JSON null-substitute) gets the same behavior as omitting the field entirely — no 400 on empty input. Defensive against frontend frameworks that turn `null` into `\"\"` during JSON serialization.
3. **Echo input in the error message.** `format!(\"Invalid template_id UUID: {}\", s)` echoes the caller-provided string. T-10-04-02 disposition `accept`: this is the caller's own input being returned, no PII leak, and the diagnostic helps frontend developers debug payload-construction bugs.
4. **Fix 11 downstream E2E literal-initializers in the same GREEN commit (Rule 3 auto-fix).** The struct extension is structurally incomplete until every literal-initializer in the workspace compiles. Splitting the fixup into a separate refactor commit would leave the tree red-build between commits — bad git hygiene. Documented as a deviation; not surprising scope since the field-literal pattern is the project-wide convention.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 — Blocking] Extended 11 SendBulkMailRequest literal-initializers in genossi_bin/tests/e2e_tests.rs**

- **Found during:** Task 1 GREEN-phase, after the struct extension passed `cargo build -p genossi_mail --tests` but the workspace test build failed with `error[E0063]: missing fields 'repayment_phase_id' and 'template_id' in initializer of 'SendBulkMailRequest'` at 11 sites in `genossi_bin/tests/e2e_tests.rs` (lines 3211, 3296, 3344, 3522, 3607, 3909, 3967, 4009, 4066, 4556, 4592).
- **Issue:** The existing E2E tests construct `SendBulkMailRequest { ... }` via field-literals (not `..Default::default()`), so any new required field — even one with `#[serde(default)]` (which affects deserialization, not in-Rust construction) — breaks the literal-initializers. This is the standard Rust struct-extension consequence; the plan's `<action>` Schritt 1 did not mention it explicitly but the canonical pattern is to update downstream sites in the same commit.
- **Fix:** Used `Edit` with `replace_all` to add `template_id: None, repayment_phase_id: None` after `static_document_ids: vec![],` (matched 9 sites in one operation), then two targeted `Edit` calls for the remaining two sites (which had `static_document_ids: vec![doc_id]` and `static_document_ids: vec![uuid::Uuid::new_v4().to_string()]` — different right-hand-sides, so they needed separate matches). A final `replace_all` covered three more sites that had `attachment_ids: vec![doc_id.to_string()],` as the anchor.
- **Files modified:** `genossi_bin/tests/e2e_tests.rs` (11 literal-initializers).
- **Verification:** `cargo build --tests --workspace` -> clean; `cargo test --test e2e_tests` -> **279 passed, 0 failed** (identical to the post-Plan-10.03 baseline — no regression).
- **Committed in:** `6e0d1d1` (folded into the GREEN commit per Decision 4 above).

### Out-of-scope / Deferred

**Pre-existing rustfmt drift in genossi_bin/tests/e2e_tests.rs.** Running `rustfmt --check --edition 2021 genossi_bin/tests/e2e_tests.rs` reports a multi-line `assert_ne!` formatting diff around line ~10645 (a CR-01-Regression assertion from Plan 8.10). This is **pre-existing** — Plan 10.04 did not touch that block. Scope-Boundary applies: documenting here, not fixing. (The touched literal-initializer blocks themselves are rustfmt-clean — verified post-edit.)

**Pre-existing warnings.** `cargo build` reports 2 warnings in `genossi_mail/src/rest_templates.rs` (unused imports `axum::routing::{delete, put}` and dead-code `format_datetime`) and 1 in `genossi_rest`/1 in `genossi_bin`. None introduced by Plan 10.04; all pre-existing.

---

**Total deviations:** 1 auto-fixed (Rule 3 — blocking downstream literal-initializers).
**Impact on plan:** Standard Rust struct-extension consequence; fix was mechanical and well within the spirit of the plan's `<action>` Schritt 2 (\"In `send_bulk_mail` ... ergaenze ... das Parsing\" — implicitly: keep the workspace building). No scope creep.

## Issues Encountered

None beyond the deviation above. `cargo fmt` (no-subcmd-warning) was sidestepped via the Nix-store search pattern (`find /nix/store -name rustfmt -executable`) per project memory — found `rustfmt-preview-1.93.0` and used it directly. Tests stayed green after rustfmt-reformatting touched two short blocks in the new UUID-parse code; the reformat is idempotent (`rustfmt --check` exit 0 on a second run).

## Auth Gates

None. Plan was fully autonomous (no SMTP/IMAP/Nextcloud touched; pure struct + handler edit).

## TDD Gate Compliance

Plan-level type is `execute` but Task 1 carried `tdd=\"true\"`. The git log shows the required RED-GREEN sequence:

1. `25633d8` — `test(10-04): add failing serde tests for SendBulkMailRequest phase 10 fields` (RED gate). Proof of failure: `cargo build -p genossi_mail --tests` returned `error[E0609]: no field 'template_id' on type 'rest::SendBulkMailRequest'` 2x + `no field 'repayment_phase_id'` 2x = 4 errors total.
2. `6e0d1d1` — `feat(10-04): extend SendBulkMailRequest with template_id + repayment_phase_id` (GREEN gate). Proof of pass: `cargo test -p genossi_mail --lib test_send_bulk_mail_request_serde` -> 2/2 passed; full workspace lib-tests 730 passed / 0 failed; E2E tests 279/279 passed.

No REFACTOR commit needed — rustfmt-driven whitespace adjustments (post-rustfmt re-run) were idempotent within the GREEN diff.

## Threat Flags

None. The `<threat_model>` in the plan is fully covered:

- **T-10-04-01 (Tampering — malformed UUID injection):** `mitigate` — confirmed. `Uuid::parse_str` rejects non-UUIDs and returns `Err`; the handler maps to `MailServiceError::BadRequest` -> HTTP 400 long before reaching the DAO. Serde-roundtrip tests cover the deserialization path; the parse-path is exercised by 4× existing `Uuid::parse_str` sites in the same handler that already follow the same pattern.
- **T-10-04-02 (Information Disclosure — error echoes input):** `accept` — confirmed. `format!(\"Invalid template_id UUID: {}\", s)` echoes only the caller's own input string; no DB lookup happens (parse fails before any DAO touch); no PII leak.
- **T-10-04-03 (Elevation of Privilege — bulk endpoint missing OIDC):** `mitigate` — confirmed. POST /api/mail/send-bulk is already OIDC-protected at the `Router` level upstream of this handler. The two new fields enrich an already-authenticated request; they do **not** create a new code path that bypasses auth (the handler signature `pub async fn send_bulk_mail<S: MailRestState>(state: State<S>, axum::Json(body): axum::Json<SendBulkMailRequest>) -> Response` is unchanged at the routing level).
- **T-10-04-04 (DoS — large invalid UUIDs):** `accept` — confirmed. `Uuid::parse_str` works on fixed-size 36-char input; any string is bounded by JSON parser limits upstream.
- **T-10-04-05 (Repudiation — audit trail of who-sent-what):** `mitigate` — confirmed. The REST handler does no audit (just routes the request); the worker (Plan 10.06) will create the MemberDocument with `template_id` captured for traceability. Phase 10.04 keeps the contract aligned with that future state.

No new threat surfaces were introduced — the two new fields are passive request-body data flowing through an already-existing call-stack to MailService::create_job (extended in Plan 10.03).

## Known Stubs

None. The `TODO(10.04)` placeholders from Plan 10.03's commit `82c8515` are fully removed:

```
$ grep -c \"TODO(10.04)\\|TODO Plan 10.04\" genossi_mail/src/rest.rs
0
```

The `send_mail` (single-send) and `application.rs` (application-confirmation mail) call-sites still pass `None, None` to `create_job` — these are **permanent by design** (single-send is ad-hoc, transactional join-confirmation is not template/phase-tracked) and explicitly documented in Plan 10.03's SUMMARY as such, not as stubs.

## User Setup Required

None. Plan was a pure code change with no external service configuration or auth gates.

## Next Phase Readiness

- **Plan 10.05** (template-repayment-context-helper, Wave 2) can proceed without dependency on this plan — it operates on `genossi_mail/src/template.rs` and is independent of the REST body schema.
- **Plan 10.06** (worker-repayment-context-und-audited-create, Wave 3) now has the full chain available: REST sends the two UUIDs into `MailService::create_job`, which persists them on `MailJob`, which the worker (Plan 10.06) reads via `MailJob.template_id` / `MailJob.repayment_phase_id`. The worker can now confidently expect both fields to be valid UUIDs (parse-gated at the REST boundary) or `None` (omitted).
- **Plan 10.08** (e2e-bulk-mail-und-audit-chain, Wave 4) can construct end-to-end test payloads with the two new fields; the 11 existing E2E SendBulkMailRequest sites in `e2e_tests.rs` already pass `template_id: None, repayment_phase_id: None` and are now ready for Plan 10.08 to add new sites that exercise the populated path.
- **Phase 12** (frontend) will populate the two body fields when the user comes from a `RepaymentPhase` detail page (D-02); the schema docs in `SendBulkMailRequest` cite D-03/D-12 explicitly so the OpenAPI client-generator will surface them in the frontend's typed mail-client.

## Self-Check: PASSED

**Files claimed modified — existence + content check:**

- `genossi_mail/src/rest.rs` -> FOUND. `grep -c \"pub template_id: Option<String>\" genossi_mail/src/rest.rs` = 1; `grep -c \"pub repayment_phase_id: Option<String>\"` = 1; `grep -c \"TODO(10.04)\"` = 0; `grep -c \"Uuid::parse_str\"` = 8.
- `genossi_bin/tests/e2e_tests.rs` -> FOUND. `grep -c \"template_id: None,\" genossi_bin/tests/e2e_tests.rs` = 11; `grep -c \"repayment_phase_id: None,\"` = 11.

**Commits claimed — existence check:**

- `25633d8` (RED) -> FOUND via `git log --oneline | grep 25633d8`.
- `6e0d1d1` (GREEN) -> FOUND via `git log --oneline | grep 6e0d1d1`.

All claims verified.

---

*Phase: 10-massenmail-anbindung-template-variablen*
*Completed: 2026-05-31*
