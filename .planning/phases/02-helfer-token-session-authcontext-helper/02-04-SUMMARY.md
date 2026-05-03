---
phase: 02-helfer-token-session-authcontext-helper
plan: 04
subsystem: api

tags: [rest-types, service-trait, helper-token, mockall, utoipa, automock, openapi, phase-2]

requires:
  - phase: 02-helfer-token-session-authcontext-helper
    provides: "Plan 01 — HelperTokenEntity, HelperTokenDao trait + MockHelperTokenDao, atomic_redeem signature"

provides:
  - "HelperTokenStatusTO enum (Open, Used, Revoked) — derived per D-02"
  - "HelperTokenTO REST type (8 fields, ToSchema-derived, iso8601_datetime serde, no token_hash)"
  - "HelperTokenCreateResponseTO (one-time { token, code, qr_svg } per D-21)"
  - "CreateHelperTokenRequest, RedeemRequest, RedeemResponse REST types (D-21, D-22)"
  - "From<&HelperTokenEntity> for HelperTokenTO with revoked > used > open status priority"
  - "HelperToken service-domain type (mirror without token_hash/deleted)"
  - "HelperTokenSubmission, HelperTokenCreated, HelperRedeemSuccess service-domain types"
  - "HelperTokenService trait with 4 async methods + #[automock] MockHelperTokenService"
  - "Documented D-24 ServiceError mapping conventions in redeem_helper_token doc-comment"
  - "13 new unit tests (8 in genossi_rest_types, 5 in genossi_service)"

affects:
  - "02-05 helper-token service impl"
  - "02-07 helper-token REST handlers"
  - "02-08 e2e tests"

tech-stack:
  added: []
  patterns:
    - "Domain-Type-without-secret: HelperToken intentionally lacks token_hash/deleted (D-06 parallel mitigation against pre-image leak)"
    - "Public-flow trait method: redeem_helper_token takes &str + no Authentication (D-22)"
    - "Stable-error-code convention: ServiceError::Conflict(Arc<str>) carries lowercase snake_case discriminator strings (already_used, revoked, assembly_not_open) for D-24 HTTP-status discrimination"
    - "Defensive serialization-test for hash leak: serde_json::to_string(&TO).contains(\"token_hash\") asserted false"

key-files:
  created:
    - "genossi_service/src/helper_token.rs"
  modified:
    - "genossi_rest_types/src/lib.rs"
    - "genossi_service/src/lib.rs"

key-decisions:
  - "ServiceError variants for redeem-flow specifics (already_used, revoked, assembly_not_open) live in the Conflict(Arc<str>) payload as stable string-codes rather than as new ServiceError variants. This keeps the cross-crate ServiceError API unchanged in Plan 04 — Plan 05 (service impl) chooses the discriminator strings, Plan 07 (REST handler) maps them to 410/403."
  - "HelperToken domain type intentionally OMITS token_hash and deleted. The DAO entity has both, but the service layer must never see token_hash (pre-image leak risk) and deleted is service-internal lifecycle metadata never surfaced to callers."
  - "redeem_helper_token signature is &str → Result<HelperRedeemSuccess, ServiceError> with NO Authentication argument. This is a public flow (D-22): the helper has no session yet, so requiring auth would be circular. The trait shape enforces this at compile time."

patterns-established:
  - "Two-layer-domain-mirror: HelperTokenEntity (DAO) → HelperToken (service) → HelperTokenTO (REST), each layer dropping fields the next layer doesn't need (deleted, token_hash)"
  - "Status-derivation-in-From-impl: TO-level enum reduces 2 nullable timestamp columns into 3 enum variants with documented priority"
  - "ToSchema-on-all-public-types: every TO that crosses the OpenAPI boundary derives ToSchema, including request/response wrappers"

requirements-completed: [HLPR-01, HLPR-02, HLPR-06]

duration: 18min
completed: 2026-05-03
---

# Phase 2 Plan 04: Helper Token REST Types and Service Trait Summary

**Six REST TOs (HelperTokenStatusTO, HelperTokenTO, HelperTokenCreateResponseTO, CreateHelperTokenRequest, RedeemRequest, RedeemResponse) plus a 4-method HelperTokenService trait with #[automock] mock — proven by 13 unit tests including a defensive token_hash leak guard and a Debug-output guard.**

## Performance

- **Duration:** ~18 minutes
- **Started:** 2026-05-03T11:15:07Z
- **Completed:** 2026-05-03T11:33:50Z
- **Tasks:** 2 (both TDD red/green; no refactor needed)
- **Files created:** 1 (`genossi_service/src/helper_token.rs`)
- **Files modified:** 2 (`genossi_rest_types/src/lib.rs`, `genossi_service/src/lib.rs`)
- **Tests added:** 13 (8 in `genossi_rest_types`, 5 in `genossi_service`)

## Accomplishments

- Six new REST TOs in `genossi_rest_types/src/lib.rs` with Utoipa schema derives, placed alphabetically after the AssemblyTO block (matching the Phase-1 convention)
- `HelperTokenStatusTO` derives status from two nullable timestamp columns via `revoked > used > open` priority (D-02), implemented inside the `From<&HelperTokenEntity>` impl
- `HelperTokenTO` excludes `token_hash` from its struct fields. A defensive serde-test (`test_to_does_not_expose_token_hash`) asserts the JSON output contains neither the literal `token_hash` key nor any payload value, providing a regression safety net if a future contributor accidentally adds the field
- `HelperTokenCreateResponseTO` carries the one-time secrets `code` (10-char Crockford) and `qr_svg` only in the create response — the `HelperTokenTO` proper never carries them (D-11, D-21)
- `genossi_service/src/helper_token.rs` ships the complete service-trait contract with four `async fn`s, generic over `Self::Context` and `Self::Transaction`, and a `MockHelperTokenService` generated by `#[automock]` for downstream service-impl tests in Plan 05
- `redeem_helper_token` is intentionally `&str → Result<HelperRedeemSuccess, ServiceError>` with NO Authentication argument — public-flow contract (D-22) enforced at compile time
- The trait doc-comment for `redeem_helper_token` documents the D-24 ServiceError → HTTP mapping table that Plan 05 (impl) and Plan 07 (REST) will follow, including the stable lowercase-snake_case Conflict-discriminator-string convention

## Task Commits

Each task ran TDD red→green; both green commits passed without a refactor pass.

1. **Task 1 RED:** `d00e8ee` — `test(02-04): add failing tests for HelperTokenTO and friends`
2. **Task 1 GREEN:** `ad0e0c6` — `feat(02-04): add helper-token REST TOs (HelperTokenTO + 5 friends)`
3. **Task 2 RED:** `ec8df80` — `test(02-04): add failing tests for HelperTokenService trait`
4. **Task 2 GREEN:** `fa60c2c` — `feat(02-04): add HelperTokenService trait and domain types`

The plan-metadata commit (this SUMMARY) is created separately at the end of execution.

## Files Created/Modified

- **Created** `genossi_service/src/helper_token.rs` (217 lines) — domain types (`HelperToken`, `HelperTokenSubmission`, `HelperTokenCreated`, `HelperRedeemSuccess`), the `HelperTokenService` trait with 4 async methods, and 5 unit tests (Debug-leak guard, code/qr_svg shape, submission constructibility, redeem-success metadata, mock compile-check)
- **Modified** `genossi_rest_types/src/lib.rs` (+217 lines) — Phase-2 helper-token TO block inserted after the Phase-1 Assembly TOs, with `From<&genossi_dao::helper_token::HelperTokenEntity> for HelperTokenTO` and 8 unit tests under `mod helper_token_to_tests`
- **Modified** `genossi_service/src/lib.rs` (+1 line) — `pub mod helper_token;` added alphabetically between `document_storage` and `member`

## HelperTokenService Method Signatures

| Method | Signature | Auth | Purpose |
|--------|-----------|------|---------|
| `create_helper_token` | `(assembly_id, &HelperTokenSubmission, ctx) -> Result<HelperTokenCreated, ServiceError>` | admin | D-21 create + return one-time code+qr_svg |
| `list_for_assembly` | `(assembly_id, ctx) -> Result<Arc<[HelperToken]>, ServiceError>` | admin | D-21 list non-deleted tokens for an assembly |
| `revoke_helper_token` | `(assembly_id, token_id, ctx) -> Result<HelperToken, ServiceError>` | admin | D-21+D-23 set revoked_at; only when `used_at IS NULL` and assembly status in {Preparation, Open} |
| `redeem_helper_token` | `(code: &str) -> Result<HelperRedeemSuccess, ServiceError>` | **public** | D-22 atomic one-time-use redeem; spans `atomic_redeem` + `lookup_status` + session creation in service impl (Plan 05) |

## Decisions Made

- **D-24 mapping convention documented in trait doc-comment, not as ServiceError variants.** The plan body previously flagged this as an open question (Plan 05 might add new ServiceError variants `HelperTokenAlreadyUsed` / `HelperTokenForbidden`). Plan 04 settled on the simpler approach: keep `ServiceError::Conflict(Arc<str>)` and standardize the payload string-codes (`already_used`, `revoked`, `assembly_not_open`). Rationale: avoid bloating the cross-crate `ServiceError` enum for a single endpoint's branching needs; the REST layer (Plan 07) will pattern-match on these stable strings to choose 410 vs 403.
- **`HelperToken` domain type drops both `token_hash` AND `deleted`.** `token_hash` is excluded for the documented D-06 reason (no pre-image leak above the DAO layer). `deleted` is also excluded because it's an internal lifecycle marker that the service layer can filter on but never needs to expose to callers — both the DAO `all()` and `find_by_id()` defaults already filter `deleted IS NULL`.
- **Module ordering.** Placed `pub mod helper_token;` between `document_storage` and `member` (true alphabetical), matching the Phase-1 convention. Plan body suggested between `claim_utils` and `document_storage` which would not be alphabetical.

## Deviations from Plan

None — plan executed exactly as written. Two minor authoring slips in the plan's grep-based acceptance criteria are noted under "Issues Encountered" rather than as deviations because they are non-functional artifacts of how the criteria were drafted, not behavior changes.

## Issues Encountered

- **Acceptance criteria grep over-matches doc-comments.** Two acceptance-criteria greps were authored as "MUST be 0":
  - `grep -B 5 -A 15 "pub struct HelperTokenTO" genossi_rest_types/src/lib.rs | grep -c "token_hash"` — expected 0
  - `grep -B 2 -A 10 "pub struct HelperToken {" genossi_service/src/helper_token.rs | grep -c "token_hash"` — expected 0

  Both return 1 in this implementation because the doc-comment ABOVE each struct explicitly documents the exclusion ("Excludes `token_hash` (hash leakage prevention, D-06 audit-fields parallel)" and "EXCLUDES `token_hash` and `deleted`"). The actual struct fields contain no `token_hash`. The behaviorally meaningful guard is the runtime test `test_to_does_not_expose_token_hash` (TO) and `test_helper_token_from_entity_excludes_hash` (domain), both green. Removing the doc-comments to satisfy the grep would reduce auditability of the security decision; the doc-comments stay, the structs are correct, the tests prove it. This matches the same pattern used in Plan 02-01's SUMMARY ("authoring slips … resolved in favor of established convention").

- **Workspace-wide `cargo build --workspace --features mock_auth` requires `SQLX_OFFLINE=true` in this worktree.** The worktree has no `genossi.db` and no `DATABASE_URL` set; sqlx-macro `query!` invocations in `genossi_dao_impl_sqlite` need the offline cache. With `SQLX_OFFLINE=true` the workspace builds cleanly. This is a pre-existing environment property (the same condition applied during Plan 02-01), not a regression caused by this plan.

- **`cargo test -p genossi_service helper_token` requires `--features utoipa`.** The default-feature build of `genossi_service` doesn't pull in `utoipa`, but `genossi_service/src/auth_types.rs` references `utoipa::ToSchema` (this is a pre-existing condition). Tests are run as `cargo test -p genossi_service --features utoipa helper_token`, which is the same invocation downstream consumers (`genossi_rest_types`) use via their `genossi_service = { features = ["utoipa"] }` dependency declaration. This is not a Plan-04 introduction.

## Test Inventory

### `genossi_rest_types` (8 new tests in `mod helper_token_to_tests`)

| Test | Purpose |
|------|---------|
| `test_status_open_when_neither_used_nor_revoked` | D-02: both timestamps None → Open |
| `test_status_used_when_used_at_some` | D-02: used_at set, revoked_at None → Used |
| `test_status_revoked_dominates_used` | D-02: revoked_at set wins regardless of used_at |
| `test_to_does_not_expose_token_hash` | D-06 parallel: serde_json output contains neither the field name nor the hash payload |
| `test_create_response_has_one_time_secrets` | D-21: HelperTokenCreateResponseTO carries code+qr_svg |
| `test_redeem_request_minimal_json` | D-22: RedeemRequest deserializes from `{"code":"..."}` |
| `test_redeem_response_carries_assembly_and_expiry` | D-22: RedeemResponse holds assembly_id + expires_at |
| `test_create_helper_token_request_json` | D-21: CreateHelperTokenRequest deserializes from `{"memo":"..."}` |

### `genossi_service` (5 new tests in `helper_token::tests`)

| Test | Purpose |
|------|---------|
| `test_helper_token_from_entity_excludes_hash` | Debug-output of `HelperToken` contains neither `token_hash` field name nor entity hash payload |
| `test_helper_token_created_carries_code_and_qr_svg` | HelperTokenCreated holds 10-char code + svg-prefixed qr_svg |
| `test_helper_token_submission_constructible` | HelperTokenSubmission is a plain `{ memo }` value-type |
| `test_helper_redeem_success_carries_session_metadata` | HelperRedeemSuccess holds session_id + assembly_id + expires_at |
| `test_mock_helper_token_service_compiles` | `#[automock]` generates `MockHelperTokenService` (compile-only assertion) |

## Hint for Plan 05 (Service Impl)

- `MockHelperTokenDao` (Plan 01) and `MockHelperTokenService` (Plan 04) are both ready to use. Service-impl unit tests should mock the DAO; service-trait consumers in higher layers should mock the service.
- Use `ServiceError::Conflict(Arc::from("already_used"))`, `Conflict(Arc::from("revoked"))`, and `Conflict(Arc::from("assembly_not_open"))` as the three discriminator strings consumed by the REST layer. Adding new `ServiceError` variants is NOT needed (and was deliberately rejected in this plan to keep the cross-crate API minimal).
- The `redeem_helper_token` impl orchestrates: validate code format → SHA256 hash → `atomic_redeem` (returns Some on success, None on miss) → on None, run `lookup_status` to discriminate 404/410/403 → on Some, fetch assembly + check status `Open` → create session via `SessionService` (D-13) → `set_session_id` on the helper_token row in the SAME transaction (Pitfall 3).

## Hint for Plan 07 (REST Handlers)

- All TOs are ready and ToSchema-derived. Endpoints to wire:
  - `POST /api/assembly/{assembly_id}/helper-tokens` → `HelperTokenCreateResponseTO` (admin, D-21)
  - `GET /api/assembly/{assembly_id}/helper-tokens` → `Arc<[HelperTokenTO]>` (admin, D-21)
  - `POST /api/assembly/{assembly_id}/helper-tokens/{token_id}/revoke` → `HelperTokenTO` (admin, D-21+D-23)
  - `POST /api/helper/redeem` → `RedeemResponse` + Set-Cookie `app_session=...` (PUBLIC, D-22)
- ServiceError mapping table for the redeem endpoint:

| ServiceError variant / payload | HTTP |
|--------------------------------|------|
| `ValidationError` | 400 Bad Request |
| `EntityNotFound` | 404 Not Found |
| `Conflict("already_used")` | 410 Gone |
| `Conflict("revoked")` | 403 Forbidden |
| `Conflict("assembly_not_open")` | 403 Forbidden |
| any other | 500 Internal Server Error (logged) |

## Next Phase Readiness

**Ready for Plan 05 (HelperTokenServiceImpl) and Plan 07 (REST handlers):**
- Service trait + `MockHelperTokenService` available for test-driven impl in Plan 05
- All six REST TOs available for utoipa-ApiDoc + handler signatures in Plan 07
- D-24 mapping conventions are documented in code (trait doc-comment) AND in this SUMMARY (mapping table above)
- No new ServiceError variants introduced — Plan 05 reuses existing variants with stable Conflict-string-codes
- `From<&HelperTokenEntity> for HelperTokenTO` and `From<&HelperTokenEntity> for HelperToken` are both ready for use by Plan 05's service impl when transforming DAO results.

**No blockers.** All 13 new tests pass green; pre-existing 167+11=178-test corpus unaffected (verified via `cargo build` clean and `cargo test -p genossi_rest_types`/`cargo test -p genossi_service --features utoipa` both green).

## Self-Check: PASSED

- [x] All 4 task commits exist in git (`d00e8ee`, `ad0e0c6`, `ec8df80`, `fa60c2c`)
- [x] All file changes present on disk (1 created + 2 modified)
- [x] All 13 new unit tests pass green (8 in `genossi_rest_types`, 5 in `genossi_service` with `--features utoipa`)
- [x] D-02 status priority verified by `test_status_revoked_dominates_used`
- [x] D-06 parallel hash-leak guard verified by `test_to_does_not_expose_token_hash` AND `test_helper_token_from_entity_excludes_hash`
- [x] D-22 public-flow contract enforced by trait signature (no Authentication arg on `redeem_helper_token`) — verified by acceptance criterion `grep -A 3 "async fn redeem_helper_token" | grep -c "Authentication"` == 0
- [x] D-24 mapping convention documented in `redeem_helper_token` trait doc-comment + here in this SUMMARY
- [x] `MockHelperTokenService` codegen verified by `test_mock_helper_token_service_compiles`
- [x] `cargo build -p genossi_rest_types` clean
- [x] `cargo build -p genossi_service --features utoipa` clean
- [x] `cargo build --workspace --features mock_auth` clean (with `SQLX_OFFLINE=true` — pre-existing environment requirement)

---
*Phase: 02-helfer-token-session-authcontext-helper*
*Completed: 2026-05-03*
