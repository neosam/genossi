---
phase: 04-frontend-component-first-mit-qr-scanner-und-manual-code-fall
plan: 01
subsystem: api
tags: [axum, helper-token, session, cookies, public-route, tdd]

# Dependency graph
requires:
  - phase: 02-helfer-token-session-authcontext-helper
    provides: helper_token-Aggregat, redeem-Flow, app_session-Cookie-Set-Pattern
  - phase: 03-anwesenheits-flow-helfer-attendance-snapshot-und-vorstand-recap
    provides: AssemblyService::AssemblyDao-Wiring, redeem_rate_layer
provides:
  - GET /api/helper/session (D-06 Auto-Redirect-Trigger für Frontend-/helper-Mount)
  - POST /api/helper/logout (D-07 HelperShell-Logout-Action)
  - HelperSessionTO (3-key PII-Whitelist Transfer-Object)
  - HelperTokenService::find_assembly_for_session + DAO::find_assembly_id_for_session (Reverse-Lookup session_id → assembly)
affects:
  - 04-02 (frontend foundation — Frontend ruft beide Endpoints auf)
  - 04-04 (HelperShell-Komponenten — Logout-Button und Auto-Redirect)
  - 04-05 (HelperLandingPage — Initial-Session-Probe)

# Tech tracking
tech-stack:
  added:
    - reqwest "cookies" feature (für Cookie-Jar in E2E-Tests)
  patterns:
    - "Public-Route-Pattern mit selbst-gelesenem Cookie (header::COOKIE) — symmetrisch zu redeem_helper_token Set-Cookie-Pattern"
    - "Append-only-Erweiterung von generate_public_route — kein zweiter .nest('/api/helper', ...) Mount"
    - "Reverse-Lookup im DAO via WHERE session_id = ? AND deleted IS NULL"

key-files:
  created: []
  modified:
    - genossi_rest_types/src/lib.rs (HelperSessionTO + 3-key serialization test)
    - genossi_rest/src/helper_token.rs (read_session_cookie + 2 Handler + erweiterte generate_public_route + PublicApiDoc)
    - genossi_dao/src/helper_token.rs (HelperTokenDao::find_assembly_id_for_session)
    - genossi_dao_impl_sqlite/src/helper_token.rs (SQL Reverse-Lookup-Implementation)
    - genossi_service/src/helper_token.rs (HelperTokenService::find_assembly_for_session + HelperSessionInfo struct)
    - genossi_service_impl/src/helper_token.rs (Service-Impl + Mock-Erweiterung)
    - genossi_service_impl/src/assembly.rs (Mock-Erweiterung TestHelperTokenDao)
    - genossi_bin/tests/e2e_tests.rs (5 neue E2E-Tests + HelperSessionTO-Import)
    - genossi_bin/Cargo.toml (reqwest 'cookies' feature)
    - Cargo.lock (Auto-Update durch neues feature)

key-decisions:
  - "find_assembly_for_session liefert HelperSessionInfo {assembly_id, assembly_name} statt nur assembly_id — vermeidet zweiten DAO/Service-Roundtrip im REST-Handler und umgeht admin-only AssemblyService::get_assembly"
  - "logout invalidiert nur, wenn Cookie sowohl von SessionService verifiziert als auch von helper_token-Tabelle bekannt ist — admin/OIDC-Cookies werden mit 401 abgewiesen (T-04-02)"
  - "Cookie-Override Max-Age=0 mit identischen Attributen wie redeem (Path=/; HttpOnly; SameSite=Strict; Secure) — Browser ersetzt nur bei voller Attributsmatch zuverlässig"

patterns-established:
  - "Symmetric-Cookie-Pattern: redeem SETZT, get_helper_session+helper_logout LESEN per header::COOKIE direkt — beide Pfade öffentlich, Cookie selbst ist die Auth"
  - "Append-only Public-Route: generate_public_route<RestStateDef + HelperTokenRestState>() bleibt der einzige Builder; Phase 4 fügt Routen hinzu statt neuen Builder/Mount"
  - "PII-Whitelist-TO-Pattern (parallel zu Phase 3 AttendanceMemberTO): inline 3-key serialization-Test im Types-Crate sichert Schema gegen versehentliches Feldschnüren"

requirements-completed: []  # Plan-Frontmatter `requirements: []` (D-06/D-07 sind Truths, keine externen Requirements-IDs)

# Metrics
duration: 35min
completed: 2026-05-05
---

# Plan 04-01: Helper-Session + Logout Backend Summary

**GET /api/helper/session und POST /api/helper/logout als append-only Routen im existierenden helper_redeem_router — Frontend kann jetzt Auto-Redirect und Logout ohne /api/attendance-Probe machen.**

## Performance

- **Duration:** ca. 35 min
- **Tasks:** 2 (TDD: RED → GREEN)
- **Files modified:** 9 (+ Cargo.lock auto-update)

## Accomplishments

- Zwei neue öffentliche Helper-Endpoints, die das `app_session`-Cookie selbst aus `header::COOKIE` lesen (kein `Extension<Context>`) — symmetrisch zum bestehenden `redeem_helper_token` Set-Cookie-Pattern.
- `HelperSessionTO` (3-key PII-Whitelist) — verhindert versehentliches Leaken von token_id, memo, oder Member-Daten.
- DAO + Service `find_assembly_for_session` — Reverse-Lookup session_id → assembly, plus assembly_name für die TO. Damit kommt der public Handler ohne admin-only `AssemblyService::get_assembly` aus.
- `generate_public_route` append-only erweitert — kein zweiter `.nest("/api/helper", ...)` in `lib.rs`.
- 5 neue E2E-Tests + 1 inline Unit-Test, alle grün; gesamte E2E-Suite weiterhin grün (239 passed, +5 vs. vorher).

## Task Commits

Beide Tasks atomar committed (TDD-Strict):

1. **Task 1: RED — failing E2E tests** — `e3471e7` (test)
   - HelperSessionTO + 3-key serialization-Test (grün)
   - 5 E2E-Tests (alle 404 → fail, weil Endpoints noch nicht existieren)
   - reqwest 'cookies' feature aktiviert
2. **Task 2: GREEN — implementation** — `24b91f8` (feat)
   - read_session_cookie helper
   - get_helper_session + helper_logout Handler
   - generate_public_route erweitert + PublicApiDoc erweitert
   - find_assembly_for_session auf DAO + Service-Trait + Service-Impl
   - Mock-Erweiterungen in TestHelperTokenDao (zwei Stellen)

## Files Created/Modified

- `genossi_rest_types/src/lib.rs` — neuer `HelperSessionTO` struct + inline Unit-Test
- `genossi_rest/src/helper_token.rs` — `read_session_cookie` Helper, `get_helper_session` Handler, `helper_logout` Handler, `generate_public_route` um `/session` + `/logout` erweitert, `PublicApiDoc` mit beiden Pfaden + `HelperSessionTO`
- `genossi_dao/src/helper_token.rs` — neue Trait-Methode `find_assembly_id_for_session(session_id, tx) -> Option<Uuid>`
- `genossi_dao_impl_sqlite/src/helper_token.rs` — SQL-Implementation `SELECT assembly_id FROM helper_token WHERE session_id = ? AND deleted IS NULL`
- `genossi_service/src/helper_token.rs` — neue Trait-Methode `find_assembly_for_session(session_id) -> Option<HelperSessionInfo>` + `HelperSessionInfo` struct
- `genossi_service_impl/src/helper_token.rs` — Implementation joint helper_token-Lookup + assembly-find_by_id im selben tx; In-tree `MockTestHelperTokenDao` um die neue Methode ergänzt
- `genossi_service_impl/src/assembly.rs` — `MockTestHelperTokenDao` an zweiter Stelle ebenfalls ergänzt
- `genossi_bin/tests/e2e_tests.rs` — 5 neue Tests:
  - `helper_session_returns_200_after_redeem`
  - `helper_session_returns_401_without_cookie`
  - `helper_session_returns_401_for_admin_cookie` (T-04-02)
  - `helper_logout_invalidates_session`
  - `helper_logout_returns_401_without_cookie`
- `genossi_bin/Cargo.toml` — reqwest `"cookies"` feature aktiviert (für `cookie_store(true)`)

## Decisions Made

- **HelperSessionInfo statt nur Uuid zurückgeben:** Der Plan-Vorschlag implizierte `find_assembly_for_session -> Option<Uuid>` und einen separaten Lookup für `assembly.name` über `AssemblyService::get_assembly`. Letzteres ist allerdings admin-only (Permission-Check). Statt einen unauthentifizierten Pfad zu öffnen oder eine zweite public Service-Methode zu schaffen, gibt `find_assembly_for_session` jetzt `HelperSessionInfo {assembly_id, assembly_name}` zurück — der Service hat ohnehin `assembly_dao` als Dep und kann den Name innerhalb derselben Read-only-TX mitlesen.
- **`SessionService::invalidate_session` statt `invalidate`:** Der Plan-Snippet referenzierte `SessionService::invalidate` — die existing-Trait-Methode heißt aber `invalidate_session`. Trivial, aber dokumentiert.
- **Logout: Pre-Check via `find_assembly_for_session`:** Damit weisen wir auch admin/OIDC-Cookies mit 401 ab — sonst hätte ein Angreifer einen Probe-Vektor (logout würde 204 für jede gültige Session liefern). T-04-02-konsistent.

## Deviations from Plan

**1. HelperSessionInfo-Erweiterung (Plan-Action 5)**
- **Found during:** Task 2 (Implementation)
- **Issue:** Plan-Snippet rief `state.assembly_service().find_by_id(assembly_id)` direkt im REST-Handler — diese Methode existiert nicht; `get_assembly` existiert, ist aber admin-only.
- **Fix:** `find_assembly_for_session` gibt jetzt `Option<HelperSessionInfo>` zurück (assembly_id + assembly_name aus dem-selben tx), und der REST-Handler braucht kein `AssemblyRestState`-Bound mehr. Damit ist der Handler-Trait-Bound einfacher: `RestState: RestStateDef + HelperTokenRestState` (statt `+ AssemblyRestState`).
- **Files modified:** genossi_service/src/helper_token.rs, genossi_service_impl/src/helper_token.rs, genossi_rest/src/helper_token.rs
- **Verification:** Alle 5 E2E-Tests grün, kein `+ AssemblyRestState` für die Public-Route nötig.
- **Committed in:** 24b91f8

**2. SessionService::invalidate_session (Plan-Action 1, Handler 2)**
- **Found during:** Task 2 (Implementation)
- **Issue:** Plan-Snippet rief `state.session_service().invalidate(...)` — die existierende Trait-Methode heißt `invalidate_session`.
- **Fix:** Korrekten Methodennamen verwendet.
- **Committed in:** 24b91f8

**3. Mock-Erweiterungen in zwei Service-Impl-Tests-Modulen**
- **Found during:** Task 2 GREEN (`cargo test -p genossi_service_impl`)
- **Issue:** In-tree `mock!`-Definitionen von `TestHelperTokenDao` in `helper_token.rs:649` UND `assembly.rs:505` mussten beide um `find_assembly_id_for_session` ergänzt werden, sonst E0046 "missing trait method".
- **Fix:** Methode in beiden Mocks ergänzt.
- **Committed in:** 24b91f8

---

**Total deviations:** 3 (1 Plan-Snippet-Korrektur (assembly-name-Joining), 1 Methodenname, 1 implizit benötigte Mock-Wartung).
**Impact on plan:** Keine; der externe Vertrag (HelperSessionTO 3-key, beide Endpoints, Status-Codes) ist exakt wie geplant. Die Korrekturen sind interne Implementierungsdetails.

## Issues Encountered

- Keine. RED-State korrekt erreicht (alle 5 Tests scheiterten mit 404 vor Implementation), GREEN-State im ersten Anlauf erreicht (5/5 grün), volle E2E-Suite ohne Regression (239 passed).

## Verification Commands

```bash
# Inline TO unit test
cargo test -p genossi_rest_types helper_session_to_serializes_exactly_three_keys
# => 1 passed

# 5 new E2E tests (excludes the existing cascade test that also matches the filter)
cargo test --test e2e_tests -- helper_session helper_logout
# => 6 passed (5 new + test_close_assembly_cascade_invalidates_helper_sessions)

# Full E2E regression check
cargo test --test e2e_tests
# => 239 passed (was 234 pre-plan)

# Audit chain regression
cargo test --test e2e_tests audit_verify
# => 2 passed (test_audit_verify_empty_chain + test_audit_verify_after_operations)

# Sanity-Greps
grep -c '\.nest("/api/helper"' genossi_rest/src/lib.rs   # => 1 (single mount preserved)
grep -n "fn read_session_cookie" genossi_rest/src/helper_token.rs  # => helper exists
grep -n "Max-Age=0" genossi_rest/src/helper_token.rs       # => logout cookie override
grep -E 'audit|audited_' genossi_rest/src/helper_token.rs | grep -E 'session|logout'
# => empty (no audit calls in new handlers)
```

## Next Phase Readiness

- Beide Endpoints sind dokumentiert (Utoipa-Schemas in `PublicApiDoc`) und einsatzbereit für das Phase-4-Frontend.
- Plan 04-04 (HelperShell-Komponenten) kann den Logout-Button gegen `/api/helper/logout` verdrahten.
- Plan 04-05 (HelperLandingPage) kann beim Mount `/api/helper/session` proben, um Auto-Redirect zu triggern (D-06).
- Keine offenen Sicherheitsfragen; T-04-01..05 alle mitigiert (siehe Threat-Model im Plan).

---
*Phase: 04-frontend-component-first-mit-qr-scanner-und-manual-code-fall*
*Plan: 01*
*Completed: 2026-05-05*
