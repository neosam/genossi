---
phase: 02-helfer-token-session-authcontext-helper
plan: 02
subsystem: auth
tags: [auth-context, enum-extension, helper-token, type-safety, rust, axum, mock_auth, oidc]

requires:
  - phase: 01-assembly-aggregat-audit-hardening
    provides: "Assembly entity (FK target for assembly_id in Helper variant)"
provides:
  - "AuthContext::Helper { session_id: Arc<str>, assembly_id: uuid::Uuid } variant ohne cfg-Gate (D-14)"
  - "Type-Sichtbarkeit der Helper-Variante in beiden Feature-Builds (mock_auth, oidc)"
  - "Smoke-Test in genossi_rest, der den D-14-Vertrag (kein cfg-Gate) durch Test schützt"
  - "Befund dokumentiert: keine exhaustiven AuthContext-Match-Stellen vorhanden — Plan ist minimal-invasiv"
affects: [02-03-helper-token-service, 02-06-session-service-extract-auth-context, 02-07-helper-redeem-rest-endpoint, 03-attendance-aggregat]

tech-stack:
  added: []
  patterns:
    - "Enum-Variante ohne cfg-Gate für cross-feature Type-Sichtbarkeit"
    - "Smoke-Test als Test-Wächter für Compile-Zeit-Verträge (D-14)"

key-files:
  created:
    - .planning/phases/02-helfer-token-session-authcontext-helper/02-02-SUMMARY.md
  modified:
    - genossi_service/src/auth_types.rs
    - genossi_rest/src/lib.rs

key-decisions:
  - "D-14: AuthContext::Helper-Variante OHNE #[cfg(...)]-Annotation — Sichtbarkeit in mock_auth UND oidc"
  - "D-19 (Wire-Point-Begründung): Assembly-Status-Check landet in Plan 06 in SessionServiceImpl::extract_auth_context, nicht in der REST-Auth-Middleware (saubere Schicht-Trennung, mockbar via SessionServiceDeps, minimaler Eingriff in auth_middleware.rs)"
  - "D-20 Phase-2-Stub: Helper-Variante kommt nicht durch die existierende Context-Type-Pipeline (PermissionService matched über Authentication<Self::Context>, NICHT über AuthContext direkt) — Phase 2 etabliert nur die Type-Sichtbarkeit"

patterns-established:
  - "Compiler-Cascade-Befund: keine exhaustiven AuthContext-Match-Stellen in der gesamten Codebase — alle Code-Pfade verwenden Authentication<Self::Context> oder konstruieren AuthContext nur"
  - "TDD-Disziplin auch bei trivialen Enum-Erweiterungen: failing test → variant hinzufügen → green test"

requirements-completed: [HLPR-02, HLPR-05]

duration: 19min
completed: 2026-05-03
---

# Phase 02 Plan 02: AuthContext::Helper Variante Summary

**Typsichere `AuthContext::Helper { session_id, assembly_id }`-Variante als Phase-3-Vorbereitung — ohne cfg-Gate verfügbar in mock_auth und oidc, mit zwei Konstruktions-/Distinktheits-Tests und Smoke-Test gegen versehentliche Feature-Gate-Regression.**

## Performance

- **Duration:** ~19 min
- **Started:** 2026-05-03T10:50:00Z
- **Completed:** 2026-05-03T11:08:17Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- `AuthContext::Helper { session_id: Arc<str>, assembly_id: uuid::Uuid }`-Variante in `genossi_service/src/auth_types.rs` erfolgreich hinzugefügt, ohne Feature-Gate (D-14)
- TDD-Doppel-Cycle für Task 1: failing tests (RED) → variant hinzufügen (GREEN) — beide Tests grün in mock_auth UND oidc Build
- Smoke-Test in `genossi_rest/src/lib.rs` etabliert, der D-14 absichert (Variante muss in beiden Builds konstruierbar sein)
- Cascade-Fix-Befund dokumentiert: **0** exhaustive AuthContext-Match-Stellen in der Codebase. Plan 02 ist minimal-invasiv

## Task Commits

Plan 02 lief im TDD-Modus, daher mehrere Commits pro Task:

1. **Task 1 RED** (test): `5e83bd3` — failing tests für `AuthContext::Helper` (variant existiert noch nicht)
2. **Task 1 GREEN** (feat): `0ace761` — `AuthContext::Helper`-Variante in auth_types.rs (D-14, kein cfg-Gate)
3. **Task 2** (test): `5a84d43` — Smoke-Test in genossi_rest, der D-14 in beiden Builds prüft

_Hinweis: Task 1 nutzte den klassischen TDD-Cycle (RED → GREEN). Task 2 ist defensiv-präventiv, kein REFACTOR nötig._

## Files Created/Modified

- `genossi_service/src/auth_types.rs` — Neue Enum-Variante `AuthContext::Helper { session_id: Arc<str>, assembly_id: uuid::Uuid }` und `#[cfg(test)] mod tests` mit zwei Tests (`test_auth_context_helper_variant_constructible`, `test_auth_context_helper_distinct_from_mock`)
- `genossi_rest/src/lib.rs` — Neuer Smoke-Test `test_helper_variant_compiles_in_both_features` im bestehenden `tests` mod, der die Variante konstruiert und dadurch in beiden Feature-Builds sicherstellt, dass kein cfg-Gate eingeführt wird

## Decisions Made

- **Cascade-Fix-Strategie minimal:** `grep -rn "AuthContext::"` ergab nur Konstruktionsstellen (`AuthContext::Mock(...)`), keine exhaustiven `match`-Pattern über `AuthContext`. Daher musste KEIN `Helper { .. } => Err(...)`-Arm in irgendeiner anderen Datei eingefügt werden.
- **Smoke-Test im genossi_rest Crate platziert:** Begründung — `genossi_rest` ist ein Konsument von `AuthContext`, der Test wirkt als End-to-End-Sichtbarkeits-Garantie.

## Deviations from Plan

None — plan executed exactly as written. Die Task-2-Action erlaubte explizit "Falls auch nach `grep` keine Compiler-Cascade-Fix-Stellen aufzufinden sind, **dokumentiere das in der SUMMARY** als Befund — das bestätigt, dass Plan 02 minimal-invasiv ist." Genau das ist eingetreten und wird hier dokumentiert.

## Issues Encountered

- **`cargo build -p genossi_service` allein schlägt fehl** (kein `utoipa`-Feature aktiv). Auflösung: Workspace-Build nutzen ODER `--features mock_auth,utoipa` explizit angeben. Das ist eine bekannte Eigenschaft der Crate-Feature-Konfiguration und kein Bug.
- **Whitespace-Drift in `genossi_dao/src/helper_token.rs` und `genossi_dao_impl_sqlite/src/helper_token.rs`** während des Build-Laufs aufgetaucht (vermutlich durch implizit aktivierten rustfmt im Build-Toolchain). Da diese Dateien zu Plan 02-01 gehören und nicht zu Plan 02-02, wurden die Änderungen via `git checkout --` revertiert, bevor der Task-2-Commit erstellt wurde. Plan-02-Scope wurde nicht verletzt.

## Verification Results

| Check | Command | Result |
|-------|---------|--------|
| genossi_service mock_auth build | `cargo build -p genossi_service --features mock_auth,utoipa` | exit 0 |
| genossi_service oidc build | `cargo build -p genossi_service --no-default-features --features oidc,utoipa` | exit 0 |
| Workspace mock_auth build | `cargo build --workspace` | exit 0 |
| Workspace oidc build | `cargo build --workspace --no-default-features --features oidc` | exit 0 |
| auth_types tests (mock_auth) | `cargo test -p genossi_service --features mock_auth,utoipa auth_types` | 2 passed |
| auth_types tests (oidc) | `cargo test -p genossi_service --no-default-features --features oidc,utoipa auth_types` | 2 passed |
| Smoke test (mock_auth) | `cargo test -p genossi_rest test_helper_variant` | 1 passed |
| Smoke test (oidc) | `cargo test -p genossi_rest --no-default-features --features oidc test_helper_variant` | 1 passed |
| Workspace tests (mock_auth) | `cargo test --workspace` | alle 957+ Tests grün |
| Clippy (mock_auth) | `cargo clippy --workspace --all-targets` | 0 non-exhaustive warnings |
| Clippy (oidc) | `cargo clippy --workspace --all-targets --no-default-features --features oidc` | 0 non-exhaustive warnings |
| rustfmt --check | `rustfmt --edition 2021 --check <changed files>` | exit 0 |

## Acceptance Criteria Per Task

### Task 1
- ✓ `cargo build -p genossi_service --features mock_auth,utoipa` exit 0
- ✓ `cargo build -p genossi_service --no-default-features --features oidc,utoipa` exit 0
- ✓ `cargo test -p genossi_service auth_types` 2 Tests grün
- ✓ `grep -c "Helper {" genossi_service/src/auth_types.rs` = 3 (Definition + 2 Test-Konstruktionen)
- ✓ `grep -c "session_id: Arc<str>" genossi_service/src/auth_types.rs` = 2 (Definition + ein Test)
- ✓ `grep -c "assembly_id: uuid::Uuid" genossi_service/src/auth_types.rs` = 3 (Definition + 2 Test-Konstruktionen)
- ✓ `grep -B 2 "Helper {" ... | grep -c '#\[cfg'` = 0 (D-14 erfüllt: kein cfg-Gate)
- ✓ `grep -c "test_auth_context_helper_variant_constructible"` = 1

### Task 2
- ✓ `cargo build --workspace` exit 0
- ✓ `cargo build --workspace --no-default-features --features oidc` exit 0
- ✓ `cargo test --workspace` alle Tests grün
- ✓ Cascade-Fix-Befund: 0 exhaustive Match-Stellen → keine Stub-Match-Arms hinzugefügt
- ✓ `cargo clippy ...` 0 non-exhaustive Warnings in beiden Builds

## Cascade-Fix-Stellen

**Anzahl: 0**

`grep -rn "AuthContext::" genossi_service/ genossi_service_impl/ genossi_rest/ genossi_bin/ genossi_dao/ genossi_dao_impl_sqlite/ genossi_rest_types/ --include='*.rs' | grep -v test` ergibt drei Konstruktionsstellen, allesamt nicht-exhaustive:

- `genossi_service/src/session.rs:147` — `Ok(Some(AuthContext::Mock(...)))` (Konstruktion)
- `genossi_service_impl/src/session.rs:150` — `Ok(Some(AuthContext::Mock(MockContext { ... })))` (Konstruktion)
- `genossi_service_impl/src/session.rs:587` — `Ok(Some(AuthContext::Mock(MockContext::default())))` (Konstruktion)

Keine `match auth_context { ... }`-Stelle, die alle Varianten enumeriert. Damit erzeugt das Hinzufügen einer dritten Variante in `AuthContext` keine non-exhaustive-Pattern-Compilerfehler. Der `PermissionService` matched über `Authentication<Self::Context>`, NICHT über `AuthContext` direkt — die Helper-Variante kommt über die `Context`-Type-Pipeline (`Context = MockContext` oder `Option<AuthenticatedContext>`) NICHT in den `check_permission`-Match. Plan 06 wird dies in `SessionServiceImpl::extract_auth_context` für den positiven Pfad ergänzen (D-19 Wire-Point: dort und nicht in der Auth-Middleware).

## Hinweise für nachfolgende Plans

### Für Plan 03 (HelperTokenServiceImpl) und Plan 06 (SessionServiceImpl)
- `AuthContext::Helper { session_id, assembly_id }` ist **fertig zum Verbrauch** — Konstruktion via direkter Struct-Literal-Syntax, beide Felder Required.
- `session_id` ist `Arc<str>`, `assembly_id` ist `uuid::Uuid` — direkt zuweisbar aus `SessionEntity.id`/`Uuid`-Werten.

### Für Plan 06 (D-19 Wire-Point)
- **Status-Check landet in `SessionServiceImpl::extract_auth_context`**, nicht in der REST-Auth-Middleware. Begründung:
  - **Saubere Schicht-Trennung:** Permission/Lifecycle-Logik gehört in den Service-Layer, nicht in die REST-Middleware
  - **Mockability:** `SessionServiceDeps` mit injiziertem `AssemblyDao` erlaubt Mock-Tests via `mockall`
  - **Minimaler Eingriff:** `genossi_rest/src/auth_middleware.rs:101-134` (`extract_context_from_headers`) delegiert bereits korrekt an `session_service.extract_auth_context()` — KEINE Änderung nötig
- Plan 06 muss `SessionServiceDeps` um `AssemblyDao` und `TransactionDao` erweitern (siehe RESEARCH §Pattern 2).

### Für Plan 07 (Helper-Redeem-Endpoint)
- Helper-Redeem-Endpoint muss ÖFFENTLICH (ohne `extract_auth_context`-Aufruf) sein — er nutzt den `extract_auth_context` von `genossi_rest/src/lib.rs:51-73` NICHT, weil der Helfer noch keine Session hat.
- Beim erfolgreichen Redeem konstruiert der Endpoint einen Session-Eintrag mit `claims = JSON({"kind":"helper","assembly_id":"..."})` — `AuthContext::Helper` wird beim NÄCHSTEN Request (mit dem neuen Cookie) durch `SessionServiceImpl::extract_auth_context` (Plan 06) konstruiert.

## TDD Gate Compliance

Plan-Frontmatter hat `type=execute`, also keine Plan-Level-TDD-Gate. Aber Tasks waren `tdd="true"` und der Cycle wurde eingehalten:

- **Task 1:** `5e83bd3` (test, RED) → `0ace761` (feat, GREEN). Korrekte Sequenz, RED-Phase tatsächlich gefailt (E0599: variant not found).
- **Task 2:** Reine `test`-Hinzufügung, da kein Implementierungs-Code nötig (Smoke-Test prüft nur Compile-Zeit-Sichtbarkeit der bereits existierenden Variante). Kein REFACTOR-Commit nötig.

## Threat Surface Scan

Kein neues Threat-Surface eingeführt. Plan 02 fügt nur eine Enum-Variante hinzu, die in Phase 2 ausschließlich `PermissionDenied`/`Unauthorized` (Stub) liefert — keine Privilege-Escalation-Pfade. STRIDE-Threats T-02-02-01..T-02-02-03 aus dem Plan-Threat-Model bleiben mitigated/accept wie geplant.

## Self-Check: PASSED

Verifikation der SUMMARY-Behauptungen:

- [x] `genossi_service/src/auth_types.rs` existiert (FOUND)
- [x] `genossi_rest/src/lib.rs` existiert (FOUND)
- [x] Commit `5e83bd3` existiert (`git log --oneline | grep 5e83bd3` → FOUND)
- [x] Commit `0ace761` existiert (FOUND)
- [x] Commit `5a84d43` existiert (FOUND)
- [x] `AuthContext::Helper`-Variante hat keine cfg-Annotation (grep -B 2 ... → 0)
- [x] Beide Feature-Builds (mock_auth, oidc) grün
- [x] Workspace-Tests grün (957+ passed, 0 failed)

---
*Phase: 02-helfer-token-session-authcontext-helper*
*Completed: 2026-05-03*
