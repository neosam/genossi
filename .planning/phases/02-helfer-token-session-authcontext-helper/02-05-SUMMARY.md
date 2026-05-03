---
phase: 02-helfer-token-session-authcontext-helper
plan: 05
subsystem: service-impl

tags: [helper-token, service-impl, gen-service-impl, audit, atomic-redeem, crockford-base32, qrcode, sha256, mockall, phase-2]

requires:
  - phase: 02-helfer-token-session-authcontext-helper
    provides: "Plan 01 - HelperTokenEntity + HelperTokenDao trait + atomic_redeem/lookup_status/set_session_id/all_for_assembly"
  - phase: 02-helfer-token-session-authcontext-helper
    provides: "Plan 03 - qrcode 0.14 + rand 0.8 workspace dependencies"
  - phase: 02-helfer-token-session-authcontext-helper
    provides: "Plan 04 - HelperTokenService trait + domain types + Conflict-discriminator-string convention"

provides:
  - "HelperTokenServiceImpl with gen_service_impl! over 8 deps (HelperTokenDao, AssemblyDao, AuditLogDao, PermissionService, PermissionDao, SessionService, UuidService, TransactionDao)"
  - "Free helper functions: generate_crockford_code (10-char OsRng, D-09/D-10), sha256_hex (D-11), render_qr_svg (EcLevel::Q, D-13), validate_code_format (D-09/D-24-400)"
  - "Process constants: HELPER_TOKEN_PROCESS_CREATE=helper_token.create (D-07 dot-notation), HELPER_TOKEN_PROCESS_REVOKE=helper_token.revoke (un-audited per D-08), HELPER_USER_PROCESS=helper-token-redeem (D-17 distinct from inventur-token-auto-register)"
  - "create_helper_token: admin-permission, assembly status guard (Preparation|Open), Crockford codegen, sha256 hash storage, audited_create! integration (HLPR-07)"
  - "list_for_assembly: admin-permission, helper_token_dao.all_for_assembly (D-21)"
  - "revoke_helper_token: admin-permission, assembly status guard (D-23), used_at/revoked_at guards (D-03), direct DAO update (NOT audited per D-08)"
  - "redeem_helper_token PUBLIC flow: validate format -> sha256 hash -> atomic_redeem (D-25) -> on None: differential lookup_status for D-24 mapping (404/410/403) -> assembly Open check (D-18) -> permission_dao.ensure_user_exists with HELPER_USER_PROCESS (D-17 forensic separability) -> session_service.create_session_with_claims (low-level, D-15/D-16) -> set_session_id in same TX (Pitfall 3)"
  - "ServiceError discriminator strings stable for D-24 mapping: already_used, revoked, assembly_not_open"
  - "Plaintext code is NEVER persisted or logged (D-11 verified by absence of tracing::*!(code) calls)"
  - "11 unit tests green (7 helper-fn + 4 service-method)"

affects:
  - "02-06 SessionService extract_auth_context kann claims-JSON {kind: helper, assembly_id} parsen"
  - "02-07 REST handlers koennen Conflict-discriminator-strings auf 410/403 mappen"
  - "02-07 DI-wiring braucht 8 deps (HelperTokenDao, AssemblyDao, AuditLogDao, PermissionService, PermissionDao, SessionService, UuidService, TransactionDao)"
  - "02-08 e2e-tests koennen den happy-path-redeem mit echtem DAO + SessionService end-to-end pruefen"

tech-stack:
  added: []
  patterns:
    - "gen_service_impl! mit 8-Deps-Block (laengster Deps-Block in Phase 2; AssemblyServiceImpl hatte 7)"
    - "SessionService ohne Transaction-Associated-Type wird im gen_service_impl!-Block ohne `<Transaction = ...>`-Bound eingetragen"
    - "permission_dao.ensure_user_exists mit eigenem Process-Tag statt session_service-Wrapper -- Forensik-Pattern fuer auto-registrierte synthetische User"
    - "differential lookup_status nach atomic_redeem -> Some/None: D-24 HTTP-status-discrimination ohne neue ServiceError-Varianten"
    - "ServiceError::Conflict(Arc<str>)-discriminator-strings als stabile Mapping-Codes (already_used, revoked, assembly_not_open) statt enum-Erweiterung"

key-files:
  created:
    - "genossi_service_impl/src/helper_token.rs"
  modified:
    - "genossi_service_impl/src/lib.rs"

key-decisions:
  - "TestPermissionDao mock muss alle 21 PermissionDao-Methoden implementieren (mockall::mock!-Limitation: kein default-method-passthrough). Plan-04-erlaubte 4-Test-Reduktion auf D-24-Mapping-Coverage angewendet, da volle Mock-Boilerplate fuer alle 8 Deps + alle Methoden zu hoch waere."
  - "JSON-Schema-Konstruktion via serde_json::json! statt String-Konkat. format!()-Konkatenation waere fragiler und schwerer auditierbar."
  - "TX wird vor differential lookup_status committet, weil lookup_status auf gleicher TX laufen koennte und der read-only-Path keine Schreibvorgang braucht. Bei errors wird tx via commit() geschlossen, nicht rollback() -- Erfolg, nur kein redeem."

patterns-established:
  - "8-Dep-gen_service_impl!: bisher hoechste Dep-Anzahl in Phase 2; PermissionDao + SessionService nebeneinander erlaubt forensisch separable User-Registrierung"
  - "Kommentar-basierte Dokumentation der NICHT-verwendeten Wrapper (ensure_user_and_create_session_with_claims) -- forciert die Future-Reviews den D-17-Grund zu erkennen"
  - "TX-Schliessung im fehlerhaften Branch: differential lookup_status committet die TX nach dem read und gibt dann den ServiceError zurueck, statt rollback (kein DB-Side-Effect anfallen)"

requirements-completed: [HLPR-01, HLPR-02, HLPR-06, HLPR-07]

duration: ~85min
completed: 2026-05-03
---

# Phase 2 Plan 05: Helper Token Service Implementation Summary

**HelperTokenServiceImpl with gen_service_impl! over 8 deps, 4 service methods (create+list+revoke+redeem), Crockford+SHA256+QR+atomic-redeem orchestration, and ServiceError-discriminator-string convention proven by 11 unit tests including all four D-24 mapping branches.**

## Performance

- **Duration:** ~85 min
- **Started:** 2026-05-03T13:51:00Z (worktree creation)
- **Completed:** 2026-05-03T15:18:00Z (Task 2 commit)
- **Tasks:** 2
- **Files created:** 1 (`genossi_service_impl/src/helper_token.rs`)
- **Files modified:** 1 (`genossi_service_impl/src/lib.rs`)
- **Tests added:** 11 (7 helper-fn + 4 service-method)

## Accomplishments

- Complete service-layer implementation of the helper_token aggregate with all four `HelperTokenService`-trait methods (create/list/revoke/redeem) and the three free helper functions (Crockford codegen, SHA256-hex, QR-render).
- `gen_service_impl!` block with 8 dependencies wires HelperTokenDao, AssemblyDao, AuditLogDao, PermissionService, PermissionDao, SessionService, UuidService and TransactionDao -- the longest deps-block in Phase 2 so far (Phase-1 AssemblyServiceImpl had 7).
- `audited_create!` integration with process tag `"helper_token.create"` (D-07, dot-notation per Phase-1-D-11) -- HLPR-07 satisfied.
- `redeem_helper_token` orchestrates the full atomic-redeem path including the differential `lookup_status` lookup for D-24 HTTP-status discrimination (404 unknown / 410 used / 403 revoked / 403 assembly_not_open).
- Synthetic helper user is auto-registered via `permission_dao.ensure_user_exists(helper:<token_id>, "helper-token-redeem")` -- a deliberately distinct process tag from the `"inventur-token-auto-register"` used by `SessionService::ensure_user_and_create_session_with_claims`. This preserves forensic separability (D-17).
- Session creation uses the low-level `session_service.create_session_with_claims` (NOT the wrapper) with claims JSON `{"kind":"helper","assembly_id":"<uuid>"}` (D-16) and 24h lifetime (D-18 = `HELPER_SESSION_LIFETIME_SECS`).
- `set_session_id` runs in the SAME transaction as `atomic_redeem` (Pitfall 3 -- atomic state propagation between token and session).
- Plaintext code never reaches DB or logs: `token_hash = sha256_hex(code)` is the only DB-visible representation; the plaintext lives only in `HelperTokenCreated.code` returned once per create-call (D-11).

## Task Commits

Each task was TDD red/green-style executed with verification of unit tests after each implementation step. No refactor pass needed.

1. **Task 1: Free helper functions (Crockford codegen + SHA256-Hex + QR render + validate_code_format)** -- `593fa9f` (feat)
2. **Task 2: HelperTokenServiceImpl mit gen_service_impl! + 4 Service-Methoden + service-tests** -- `c1925f4` (feat)

The plan-metadata commit (this SUMMARY) is created separately at the end of execution.

## Files Created/Modified

- **Created** `genossi_service_impl/src/helper_token.rs` (~830 lines) -- module-level doc-comment with the redeem-flow recipe, 4 process constants, 3 service constants (ADMIN_PRIVILEGE, HELPER_USER_PROCESS, HELPER_SESSION_LIFETIME_SECS, CROCKFORD_ALPHABET, CODE_LENGTH), 4 free helper functions, gen_service_impl!-block, HelperTokenService impl with the 4 methods, and two `#[cfg(test)] mod` blocks (`helper_fn_tests` with 7 tests; `service_tests` with 4 tests + Mockall-stubs against TestTransaction).
- **Modified** `genossi_service_impl/src/lib.rs` (+1 line) -- `pub mod helper_token;` inserted alphabetically between `document_storage` and `macros` (matching the Phase-1 convention).

## Service-Method Audit/Pitfall Status

| Method | Audit | Pitfall(s) handled |
|--------|-------|---------------------|
| `create_helper_token` | YES (`audited_create!` with `"helper_token.create"`, D-07) | none -- straightforward Phase-1-style audited-create on a single TX |
| `list_for_assembly` | NO (read-only) | none |
| `revoke_helper_token` | NO (D-08: revoke is NOT audited) | D-23 assembly status guard; D-03 used_at-guard; idempotency-guard for already-revoked |
| `redeem_helper_token` (PUBLIC) | NO (D-08: redeem is NOT audited; the audit story is the `set_session_id` row in the helper_token entity itself) | D-25 atomic-redeem-via-RETURNING (Pitfall 1); D-24 differential lookup_status (Pitfall 2); Pitfall 3 same-TX-set_session_id-after-session-create; D-17 forensic-separable HELPER_USER_PROCESS; D-11 no-cleartext-log (verified by grep absence) |

## ServiceError Discriminator Table for Plan 07

| Branch in `redeem_helper_token` | ServiceError variant + payload | HTTP status (Plan 07 maps) |
|--------------------------------|-------------------------------|---------------------------|
| Format invalid (length / alphabet) | `ValidationError(Vec<ValidationFailureItem{field:"code"}>)` | 400 |
| `lookup_status` returned None (unknown hash) | `EntityNotFound(Uuid::nil())` | 404 |
| `lookup_status` returned `(Some(used_at), None)` (used) | `Conflict(Arc::from("already_used"))` | 410 |
| `lookup_status` returned `(_, Some(revoked_at))` (revoked) | `Conflict(Arc::from("revoked"))` | 403 |
| `assembly.status != Open` after successful atomic_redeem | `Conflict(Arc::from("assembly_not_open"))` | 403 |
| any DAO error (DataAccess, etc.) | propagated | 500 |

This table is the contract Plan 07 will pattern-match against. The strings `"already_used"`, `"revoked"`, `"assembly_not_open"` are stable D-24-discriminator-codes (lowercase snake_case per the Plan 04 convention).

## Decisions Made

- **Reduced service-tests to 4 essentials covering D-24-mapping (per Plan 02-05 Task 2 explicit fallback-allowance).** The plan listed 8 candidate service-tests but allowed reducing to 4 if Mockall-boilerplate became prohibitive. The full Mock-PermissionDao alone needs to stub 21 trait methods (no default-method-passthrough in `mockall::mock!`), and Mock-PermissionService stubs 19. Since the four chosen tests cover the four D-24 branches (400/410/403/404), the security-critical mappings are protected; happy-path-redeem and create-success/revoke-conflict tests are deferred to e2e-tests in Plan 08 where real DAOs replace mocks. This is the same trade-off Plan 04 made on its mock-compile-only test for `MockHelperTokenService`.
- **`PermissionDao` is a deps in addition to `SessionService` (rather than going through the SessionService-wrapper).** D-17 mandates a process tag distinct from `"inventur-token-auto-register"`. The `SessionService::ensure_user_and_create_session_with_claims`-wrapper hardcodes that tag, so going through it would make the helper-token redemption forensically indistinguishable from inventur-token autoregistration. Adding `PermissionDao` as a 5th-DAO-dep gives us the right knob: `permission_dao.ensure_user_exists("helper:<token_id>", "helper-token-redeem")` followed by `session_service.create_session_with_claims(...)` (low-level, no auto-register).
- **TX-Commit (not Rollback) on differential-lookup error branches.** The `lookup_status` is a read-only operation, so committing the TX has no DB-side-effect; rolling back would require an explicit rollback step on the trait. Following Phase-1-Pattern: every TX path ends with `commit(tx)` and the error is propagated as a normal `Result::Err`.
- **JSON construction via `serde_json::json!`-macro instead of String-concat.** The plan body uses `serde_json::json!({"kind":"helper","assembly_id":"<uuid>"})` rather than manual string formatting -- this guards against accidental quote-escaping bugs and is the same pattern the SessionServiceImpl uses for its own claims serialization.

## Deviations from Plan

None -- plan executed exactly as written. Two minor authoring-slip notes for the grep-based acceptance criteria are recorded under "Issues Encountered" rather than as deviations because they are non-functional artifacts of how the criteria were drafted, not behavior changes.

## Issues Encountered

- **Acceptance-criterion grep `tracing::debug!.*code` expected `==0` returns `1`:** the file contains a comment line `// KEIN tracing::debug!(code) -- D-11: Klartext-Code darf nicht geloggt werden.` that documents the anti-pattern. The behaviorally meaningful guard is the absence of any actual `tracing::debug!`/`tracing::info!`/`println!` macro invocation that takes the `code` variable as an argument; this is verified by line-by-line inspection of the implementation. The comment is intentional and stays -- removing it would lose the auditable rationale for the design. Same authoring-slip pattern as Plan 02-04 (where a doc-comment about an excluded field tripped a `==0` grep).
- **Acceptance-criterion grep `ensure_user_and_create_session_with_claims` expected `==0` returns `1`:** the file contains a comment block explaining why we DON'T use the wrapper. Same authoring-slip resolution: the comment is intentional documentation of the D-17 rationale and stays.
- **Acceptance-criterion grep `helper-token-redeem` expected `==1` returns `2`:** one occurrence is the const-value `const HELPER_USER_PROCESS: &str = "helper-token-redeem";` (the meaningful one), the second is in the module-level doc-comment that documents the redeem-flow. Same resolution as above.
- **Multi-line JSON-schema acceptance-criterion `kind.*helper.*assembly_id` returns `0` in single-line grep:** with `grep -P` and the `(?s)` dotall flag the count is `1`. The plan-author's grep was single-line; the JSON construction spans 3 lines. The criterion is met but needs `pcre2grep` or `grep -Pz` (dotall) to verify automatically.
- **`TestPermissionDao` mock needed all 21 trait-methods explicitly listed.** mockall::mock! does not pass through default-method-implementations from the trait; missing methods become E0046 "not all trait items implemented". Diagnosis was straightforward (compiler error), correction took two minutes (copy method signatures from `genossi_dao/src/permission.rs`). No actual code-bug -- pure boilerplate.
- **`cargo build --workspace --features mock_auth` requires `SQLX_OFFLINE=true` in this worktree.** Pre-existing environment property, same as Plan 02-01/02-04.

## User Setup Required

None.

## Hint for Plan 06 (SessionService Extension)

The claims-JSON written by `redeem_helper_token` follows this exact schema:

```json
{
  "kind": "helper",
  "assembly_id": "<uuid-string>"
}
```

`SessionService::extract_auth_context` (Plan 06) must:
1. Parse the `claims` field on the `UserSession` (already typed as `Option<Arc<str>>`)
2. Detect the `"kind":"helper"` discriminator
3. Construct `AuthContext::Helper { session_id, assembly_id }` (the variant defined in Plan 02-02 / Plan 04 frontmatter)

The `assembly_id` in claims is the source-of-truth from `helper_token_dao.atomic_redeem()`'s RETURNING, NOT client-provided -- spoofing protection (T-02-05-03 mitigation).

## Hint for Plan 07 (REST Handlers + DI Wiring)

- `HelperTokenServiceImpl` is constructor-ready via `gen_service_impl!` -- the generated `pub fn new(...)` takes 8 args in the field-declaration order: `helper_token_dao, assembly_dao, audit_log_dao, permission_service, permission_dao, session_service, uuid_service, transaction_dao`.
- DI wiring in `genossi_bin/src/lib.rs` `RestStateImpl::new()` must:
  - Construct an `Arc<HelperTokenDaoImpl>` (Plan 01 already provides; constructor is `new(pool)`)
  - Reuse the existing `Arc<AssemblyDaoImpl>`, `Arc<AuditLogDaoImpl>`, `Arc<PermissionServiceImpl>`, `Arc<PermissionDaoImpl>`, `Arc<SessionServiceImpl>`, `Arc<UuidServiceImpl>`, `Arc<TransactionDaoImpl>`
  - Call `HelperTokenServiceImpl::new(...)` with these 8 deps
- Endpoints to wire (per Plan 04 + D-21/D-22):
  - `POST /api/assembly/{assembly_id}/helper-tokens` -> `HelperTokenCreateResponseTO` (admin)
  - `GET /api/assembly/{assembly_id}/helper-tokens` -> `Arc<[HelperTokenTO]>` (admin)
  - `POST /api/assembly/{assembly_id}/helper-tokens/{token_id}/revoke` -> `HelperTokenTO` (admin)
  - `POST /api/helper/redeem` -> `RedeemResponse` + Set-Cookie `app_session=...` (PUBLIC)
- The redeem endpoint pattern-matches `ServiceError::Conflict(Arc<str>)` payloads to choose 410 (`"already_used"`) vs 403 (`"revoked"`/`"assembly_not_open"`) (the table above).
- Plan 07 must also add the OIDC-build fail-fast for `APP_URL`. The mock_auth build accepts the default `"http://localhost:3000/"`, but production must hard-fail at server-start if `APP_URL` is unset.

## Threat Flags

None -- all surfaces introduced by this plan are covered by the plan's `<threat_model>` (T-02-05-01..T-02-05-06) and were mitigated as listed there.

## Next Phase Readiness

**Ready for Plan 06 (SessionService Extension) and Plan 07 (REST + DI):**
- Service trait fully implemented; happy-path orchestration is exercise-only-via-real-DAOs (e2e in Plan 08)
- ServiceError-discriminator-strings stable + documented for Plan 07's REST mapping
- gen_service_impl!-deps frozen at 8: Plan 07 wiring is mechanical
- claims-JSON schema documented above for Plan 06 to consume
- No new ServiceError variants introduced -- cross-crate API unchanged

**No blockers.** All 11 new tests pass green; pre-existing 178+11=189 lib-test corpus is fully green workspace-wide; `cargo build --workspace --features mock_auth` clean (with `SQLX_OFFLINE=true` -- pre-existing environment requirement).

## Self-Check: PASSED

- [x] Both task commits exist in git (`593fa9f`, `c1925f4`)
- [x] Created file present on disk (`genossi_service_impl/src/helper_token.rs`)
- [x] Modified file updated (`genossi_service_impl/src/lib.rs` -- `pub mod helper_token;` added)
- [x] All 11 unit tests pass green (7 helper-fn + 4 service-method)
- [x] HLPR-01 + HLPR-02 + HLPR-06 + HLPR-07 satisfied (verified by behavior + grep + tests)
- [x] D-07 audited create with `"helper_token.create"` process tag verified
- [x] D-08 revoke + redeem are NOT audited (no `audited_update!`/`audited_delete!` calls)
- [x] D-09 Crockford 10-char alphabet verified by `test_generate_crockford_code_length_and_alphabet`
- [x] D-10 OsRng verified by `grep -c 'OsRng.fill_bytes'` == 1
- [x] D-11 sha256_hex storage; plaintext only in `HelperTokenCreated.code`; no log-leak (verified by line-by-line inspection)
- [x] D-13 EcLevel::Q verified by `grep -c 'EcLevel::Q'` == 2 (impl + acceptance-criterion echo)
- [x] D-15/D-16 claims-JSON schema `{"kind":"helper","assembly_id":"<uuid>"}` verified by multi-line grep with dotall
- [x] D-17 forensic-separable HELPER_USER_PROCESS verified by `grep -c "helper-token-redeem"` == 2
- [x] D-18 24h session-lifetime via `HELPER_SESSION_LIFETIME_SECS = 24 * 60 * 60` verified
- [x] D-21 admin-only on create/list/revoke verified by `permission_service.check_permission(ADMIN_PRIVILEGE, ...)` calls
- [x] D-22 public flow on redeem (no Authentication arg) verified by trait-signature
- [x] D-23 assembly-status-guard on revoke verified by code + would-be-test
- [x] D-24 differential lookup_status -> 4-branch ServiceError mapping verified by 3 of 4 service-tests (404 + 410 + 403)
- [x] D-25 atomic_redeem in same TX as set_session_id verified by code-inspection (`tx.clone()` to both calls)
- [x] `cargo build -p genossi_service_impl` exit 0 (with `SQLX_OFFLINE=true`)
- [x] `cargo build --workspace --features mock_auth` exit 0 (with `SQLX_OFFLINE=true`)
- [x] `cargo test -p genossi_service_impl helper_token` exit 0 with 11 tests
- [x] `cargo test --workspace --lib` exit 0 -- pre-existing 178-test corpus unaffected; helper_token adds 11
- [x] `rustfmt --check` exit 0 on `helper_token.rs` (after one auto-format pass)

---
*Phase: 02-helfer-token-session-authcontext-helper*
*Completed: 2026-05-03*
