---
phase: 15-service-rest-kuendigung-aufstockung
plan: 04
subsystem: api
tags: [rust, axum, rest, e2e-test, openapi, di-wiring, utoipa, sqlx, audit-chain, optimistic-locking]

requires:
  - phase: 15-service-rest-kuendigung-aufstockung
    plan: 03
    provides: "MembershipAdjustServiceImpl<Deps> with cancel_membership + increase_shares fully implemented; 8 service-tests + 12 pure-function tests green; UPGRADE_PROCESS + CANCEL_PROCESS process strings active"
  - phase: 15-service-rest-kuendigung-aufstockung
    plan: 02
    provides: "cancel_membership impl with recalc_dates hook for CANC-04"
  - phase: 14-dao-domain-foundation
    provides: "compute_effective_date pure function (H1/H2 logic)"
provides:
  - "POST /api/members/{id}/cancel — Axum handler with full error_handler + Utoipa annotation (200/400/401/404/409)"
  - "POST /api/members/{id}/increase-shares — Axum handler (200/400/401/404)"
  - "CancelMembershipRequestTO + IncreaseSharesRequestTO + MembershipAdjustResponseTO — request/response TOs with ISO8601-date-required serde (D-15-10/11)"
  - "RestStateDef trait extended with MembershipAdjustService associated type + accessor (D-15-16) — Foundation for Phase 16-17"
  - "DI wiring in genossi_bin RestStateImpl — MembershipAdjustServiceImpl instantiated with 6 shared Arcs"
  - "11 E2E tests (9 active + 2 #[ignore] for mock_auth-admin-bypass) covering Happy-Path-H1/H2, Already-Cancelled (409), Audit-Chain-Verify, Cancelled-Member-Block (400), Datum-Bounds (400)"
  - "Rule-1 bug fix in increase_shares: removed erroneous version-bump that caused 'Version mismatch' on real DB updates"
affects: [16-partial-repayment, 17-transfer-shares, 18-frontend-ui]

tech-stack:
  added: []
  patterns:
    - "REST sub-route registration BEFORE /{id} catch-all (D-14-08 defensive convention) — kept consistent with Phase 14 transfer-recipients pattern"
    - "MembershipAdjustResponseTO single-round-trip pattern (D-15-11) — bundles action + member so frontend skips a follow-up GET"
    - "MockUserService #[ignore] convention for permission-denied E2E — links to service-layer unit tests for the 401 path (same pattern as repayment_letter_e2e.rs)"
    - "Optimistic-locking semantics: pass OLD entity.version to MemberDao::update (DAO generates new version internally) — codified by Plan-04 E2E discovery of version-bump bug"

key-files:
  created:
    - "genossi_rest/src/membership_adjust.rs (cancel_membership + increase_shares Axum handlers + ApiDoc)"
    - "genossi_bin/tests/membership_adjust_e2e.rs (11 E2E tests)"
    - ".planning/phases/15-service-rest-kuendigung-aufstockung/deferred-items.md"
  modified:
    - "genossi_rest_types/src/lib.rs (3 new TOs: CancelMembershipRequestTO, IncreaseSharesRequestTO, MembershipAdjustResponseTO)"
    - "genossi_rest/src/member.rs (registered /{id}/cancel + /{id}/increase-shares sub-routes before /{id} catch-all)"
    - "genossi_rest/src/lib.rs (pub mod membership_adjust + RestStateDef extension + OpenAPI ApiDoc nest)"
    - "genossi_bin/src/lib.rs (MembershipAdjustServiceDependencies struct + RestStateImpl slot + new() construction + Self-init + RestStateDef impl)"
    - "genossi_service_impl/src/membership_adjust.rs (Rule-1 fix: removed erroneous version-bump in increase_shares)"

key-decisions:
  - "Permission-Denied E2E-tests #[ignore]'d (BLOCKER 5 resolution): mock_auth context_extractor always injects DEVUSER which has admin via migration 20250129000001 — 401 path is unit-tested at service layer (genossi_service_impl/src/membership_adjust.rs::service_tests::test_*_permission_denied)"
  - "Sub-routes /{id}/cancel and /{id}/increase-shares live INSIDE member::generate_route (D-15-09) — coexist without conflict with the separate /api/members/{member_id}/actions Top-Level-.nest() in lib.rs:584"
  - "MembershipAdjustResponseTO as named TO (not anonymous JSON) — gives clearer OpenAPI schema and easier frontend builder construction"
  - "Date-fragility-fix in all E2E tests: derived from time::OffsetDateTime::now_utc().date() so the suite doesn't break on year-rollover (today_march_15(), today_august_15(), current_year_dec_31(), next_year_dec_31() helpers)"
  - "Rule-1 deviation (real bug): removed version-bump line in increase_shares — MemberDao::update treats entity.version as the OLD version in WHERE clause + generates new version internally. Bumping it caused 'Version mismatch' on every real DB update. Mock DAOs in service tests don't honor WHERE semantics so didn't catch this."

patterns-established:
  - "v1.2 REST sub-route pattern for member actions: POST /{id}/{action-name}, request_body = *RequestTO with iso8601_date_required, response_body = MembershipAdjustResponseTO (D-15-09..11). Phase 16 partial_repayment + Phase 17 transfer_shares will follow this template."
  - "RestStateDef extension pattern for incrementally-growing services: add associated type + accessor method + matching impl in genossi_bin/src/lib.rs. Phase 16-17 will extend the SAME MembershipAdjustService trait (D-15-13 inkrementelles Wachsen) — no new RestStateDef slot needed."
  - "E2E audit-hash-chain verification via GET /api/audit/verify after each mutating operation — single shared instance of audit_log_dao yields a single per-process hash chain that stays valid across cancel + upgrade tx commits"

requirements-completed: [CANC-01, CANC-03, CANC-04, CANC-05, UPGD-01, UPGD-02, UPGD-03, UPGD-04, PERM-01, PERM-02, AUDT-01]

metrics:
  duration: 13min
  completed: 2026-06-04
  tasks: 2
  files_modified: 5
  files_created: 3
  tests_added: 11
  tests_passing: 9 active + 2 ignored
---

# Phase 15 Plan 04: REST + E2E + DI-Wiring Summary

**REST-Endpoints fuer cancel_membership + increase_shares exposed (POST /api/members/{id}/cancel + /increase-shares), MembershipAdjustService voll in RestStateDef + RestStateImpl verdrahtet, 9 E2E-Tests gruen inkl. AUDT-01 Audit-Chain-Verify + Datum-Bounds-Edge-Cases, plus Rule-1-Bug-Fix entdeckt durch E2E (version-bump in increase_shares entfernt) — Phase 15 v1.2 ist damit Production-Ready.**

## Performance

- **Duration:** ~13 min
- **Started:** 2026-06-04T12:29:56Z
- **Completed:** 2026-06-04T12:43:10Z
- **Tasks:** 2
- **Files modified:** 5
- **Files created:** 3 (membership_adjust.rs handler, e2e test file, deferred-items.md)

## Accomplishments

- **Task 1: DTOs + REST handlers + Route registration + RestStateDef extension + DI wiring** — Vollstaendige Stack-Konsistenz: 3 neue TOs in `genossi_rest_types`, neue Datei `genossi_rest/src/membership_adjust.rs` mit beiden Handlern (error_handler-wrapped, Utoipa-annotiert, 401 NICHT 403 per D-15-12), Sub-Routes registriert in `member::generate_route` VOR `/{id}` catch-alls (D-14-08 defensive Konvention), RestStateDef-Trait erweitert mit `MembershipAdjustService` associated type + accessor, DI in RestStateImpl (Deps-Struct + Slot + Konstruktion + Self-Init + Trait-Impl). `cargo build --workspace` clean.

- **Task 2: 11 E2E-Tests + Rule-1 Bug-Fix** — Neue Datei `genossi_bin/tests/membership_adjust_e2e.rs` mit 9 aktiven + 2 ignorierten Tests. Date-Fragility-Fix via Helper-Funktionen (today_march_15, today_august_15, current_year_dec_31, next_year_dec_31). Vollstaendiges Coverage: Happy-Path-H1 (Maerz->current-Dec-31) + Happy-Path-H2 (August->next-Dec-31), Already-Cancelled (409 via Conflict), Audit-Chain-Verify (`/api/audit/verify`=valid nach beiden Mutations), Cancelled-Member-Block (UPGD-04 → 400 mit "cancelled"-Body), Datum-Bounds-Edge-Cases (Vorjahr-2 + Uebernaechstes-Jahr → 400), increase_shares Happy-Path (current_shares 1→4). Rule-1-Bug entdeckt: `increase_shares` bumpte `entity.version` per `uuid_service.new_v4()` VOR audited_update!, was zu "Version mismatch" fuehrte — `MemberDao::update` (genossi_dao_impl_sqlite/src/member.rs:209-300) interpretiert `entity.version` als die ALTE Version (WHERE-Klausel) und generiert die NEUE Version INTERN. Bug-Fix: Version-Bump-Zeile entfernt; Pattern matched jetzt `MemberActionService::update`. Alle 20 bestehenden Service-Tests bleiben gruen (Mock-DAOs honorieren WHERE-Semantik nicht).

## Task Commits

Each task was committed atomically via `gsd-sdk query commit` (jj-aware):

1. **Task 1: REST handlers + DI wiring** — `ec9489f` (feat)
2. **Task 2: 11 E2E tests + Rule-1 version-bump fix** — `948324e` (feat)
3. **Deferred-items doc** — `e814ea8` (docs)

## Files Created/Modified

- `genossi_rest/src/membership_adjust.rs` (new, 124 LOC) — 2 Axum handlers + ApiDoc with Utoipa schemas
- `genossi_bin/tests/membership_adjust_e2e.rs` (new, 384 LOC) — 11 E2E tests + setup + helpers + body-builders
- `.planning/phases/15-service-rest-kuendigung-aufstockung/deferred-items.md` (new) — out-of-scope mail-preview test failure tracked
- `genossi_rest_types/src/lib.rs` (+45 LOC) — 3 new TOs after MemberActionTO impls
- `genossi_rest/src/member.rs` (+25 LOC) — Sub-route registration + extensive doc comment about coexistence with /actions nest
- `genossi_rest/src/lib.rs` (+10 LOC) — pub mod membership_adjust + RestStateDef extension + ApiDoc nest under /api/members
- `genossi_bin/src/lib.rs` (+30 LOC) — Deps struct + type alias + RestStateImpl slot + new() construction + Self-init + RestStateDef impl
- `genossi_service_impl/src/membership_adjust.rs` (-1 LOC, +9 LOC of doc comment) — Removed `updated_entity.version = uuid_service.new_v4().await` bug, replaced with explanatory Rule-1-fix comment

## Decisions Made

See `key-decisions` in frontmatter (5 decisions). Most important:

1. **Permission-Denied E2E #[ignore]**: mock_auth context_extractor injects DEVUSER (admin via migration) — non-admin not E2E-reproducible. 401-path is unit-tested at service layer. Same pattern as `repayment_letter_e2e.rs`.
2. **Rule-1 fix on version-bump**: MemberDao::update treats `entity.version` as OLD-version for WHERE clause. Removing the bump aligns with `MemberActionService::update` pattern.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Removed erroneous version-bump in increase_shares**
- **Found during:** Task 2 (E2E test execution exposed "Version mismatch" Conflict on POST /api/members/{id}/increase-shares against in-memory SQLite)
- **Issue:** Plan 03's `increase_shares` Service-Impl included `updated_entity.version = self.uuid_service.new_v4().await` BEFORE `audited_update!`. But `MemberDao::update` (genossi_dao_impl_sqlite/src/member.rs:209-300) treats `entity.version` as the OLD version (WHERE clause) and generates the NEW version internally. Bumping it caused every real-DB update to return `DaoError::ConflictError("Version mismatch")` -> ServiceError::Conflict -> RestError::Conflict -> HTTP 409 instead of 200. The Mock DAOs used in Plan 03 service-unit-tests didn't honor WHERE-semantics so missed the bug.
- **Fix:** Removed the version-bump line. Pattern now matches `MemberActionService::update` (member_action.rs:399-408): pass entity unchanged to `audited_update!`. Added 8-line doc comment explaining the DAO contract for future readers.
- **Files modified:** `genossi_service_impl/src/membership_adjust.rs`
- **Verification:** All 20 existing service-tests still pass (`cargo test -p genossi_service_impl --lib membership_adjust`); E2E `test_increase_shares_happy_path` + `test_increase_shares_audit_chain_verify` now both pass.
- **Committed in:** `948324e` (Task 2 commit, bundled with E2E test addition)

---

**Total deviations:** 1 auto-fixed (Rule-1 bug).
**Impact on plan:** Rule-1 fix essential for correctness — without it, `POST /api/members/{id}/increase-shares` would have returned 409 on every call. Plan 03's implementation looked superficially correct (and unit-tests-green) but had a DAO-contract mismatch only catchable via real-DB integration. Plan 04's E2E suite caught it before Phase 16/17 builds on the same pattern. No scope creep.

## Issues Encountered

**Pre-existing unrelated test failure** (out of scope per SCOPE BOUNDARY rule): `test_mail_preview_repayment_no_entries_does_not_default_to_one` in `genossi_bin/tests/e2e_tests.rs:13964` fails with `errors must be array`. The failure is in the mail-template-preview JSON shape, completely unrelated to Phase 15's REST handlers, DTOs, and DI wiring. None of the files Phase 15 modified touch the mail-preview render pipeline. Other 293/294 tests in `e2e_tests` pass. Tracked in `deferred-items.md` for triage in a separate fix.

## Threat Flags

None — no new security-relevant surface beyond what the plan's `<threat_model>` enumerated (T-15-04-01..08 all addressed: mock_auth gating via feature-flag, sub-route ordering documented, audit-chain end-to-end verified, ServiceError->RestError mapping leaks no PII, i32 bounds on shares via serde, Path<Uuid> strict parsing, OpenAPI exposes only contracted fields).

## User Setup Required

None — pure REST-layer + DI-wiring + E2E test additions. No new environment variables, external services, or configuration changes.

## Verification Gates (alle erfuellt)

- `cargo build --workspace` exit 0
- `cargo test -p genossi_bin --test membership_adjust_e2e` — 9 passed, 2 ignored (mock_auth-admin-bypass), 0 failed
- `cargo test -p genossi_bin --test transfer_recipients_e2e` — 1 passed, 0 failed (Phase 14 regression check)
- `cargo test -p genossi_service_impl --lib membership_adjust` — 20 passed, 0 failed (Plan 01-03 regression)
- `cargo test -p genossi_service_impl --lib` — 370 passed, 2 ignored, 0 failed
- `cargo test -p genossi_rest --lib` — 78 passed, 0 failed
- **Acceptance criteria (from PLAN.md):**
  - 3 TOs defined with ISO8601-date-required serde ✓
  - Sub-routes registered VOR `/{id}` catch-all (cancel line 65 < get_member line 73) ✓
  - `pub mod membership_adjust;` registered in lib.rs ✓
  - `membership_adjust_service: Arc<MembershipAdjustService>` slot in RestStateImpl ✓
  - `MembershipAdjustServiceDependencies` referenced 5x (struct + Send + Sync + Deps impl + type alias) ✓
  - 9+ E2E tests green ✓
  - Audit-Hashchain valid after both mutations (verify endpoint returns `valid=true`, 0 broken_links) ✓
  - Permission-denied tests assert StatusCode::UNAUTHORIZED (in #[ignore] body) ✓

## Next Phase Readiness

- **Phase 16 (Teil-Rueckgabe)** kann sofort starten:
  - `MembershipAdjustService` trait + impl ist die Foundation; Phase 16 ergaenzt `partial_repayment` als 3. Methode
  - REST-Layer-Pattern (POST /api/members/{id}/{action} + RequestTO + MembershipAdjustResponseTO) ist als wiederverwendbares Template etabliert
  - DI-Wiring-Pattern: kein neuer `RestStateDef`-Slot fuer Phase 16/17 noetig — `MembershipAdjustService` waechst inkrementell (D-15-13)
  - Plan-04 Rule-1-Fix dokumentiert das Optimistic-Locking-Contract fuer kuenftige `MemberDao::update`-Konsumenten
- **Phase 17 (Voll-Uebertrag + Teil-Uebertrag)** kann ebenfalls auf demselben Service + REST-Pattern aufbauen + AUDT-02 via shared `member-adjust.transfer`-Process-String etablieren
- **Phase 18 (Frontend)** hat klare REST-Vertraege:
  - `POST /api/members/{id}/cancel` mit `{willensbekundung_date: ISO8601-date}` -> `{action, member}` (member.exit_date frisch fuer Re-Render)
  - `POST /api/members/{id}/increase-shares` mit `{willensbekundung_date, shares}` -> `{action, member}` (member.current_shares frisch fuer Re-Render)
  - OpenAPI-Schemas in Swagger UI verfuegbar fuer Typegen
- Keine bekannten Blocker

## Self-Check: PASSED

Verified:
- `genossi_rest/src/membership_adjust.rs` (FOUND, 124 LOC, 2 handlers + ApiDoc)
- `genossi_bin/tests/membership_adjust_e2e.rs` (FOUND, 384 LOC, 11 tests)
- `.planning/phases/15-service-rest-kuendigung-aufstockung/deferred-items.md` (FOUND)
- Commits `ec9489f` (Task 1), `948324e` (Task 2), `e814ea8` (deferred-items doc) all present in `jj log`
- All 9 active E2E tests pass (`cargo test -p genossi_bin --test membership_adjust_e2e`)
- All 20 service-tests still pass after Rule-1-fix
- No regression in Phase 14 transfer_recipients_e2e
- Workspace build clean

---
*Phase: 15-service-rest-kuendigung-aufstockung*
*Completed: 2026-06-04*
