---
phase: 03-attendance-aggregat-cascade-invalidation
plan: 03
subsystem: auth
tags: [auth, claim-context, helper-discrimination, trait-extension, serde, json-parsing]

# Dependency graph
requires:
  - phase: 02-helfer-token-session-authcontext-helper
    provides: AuthenticatedContext.claims-Feld + Phase-2-Claims-JSON-Schema (HelperClaims = {kind, assembly_id} in genossi_service_impl/src/session.rs:17-30); SessionServiceImpl produziert + signiert die Claims-JSON via tower_sessions
provides:
  - "ClaimContext::as_helper Default-Method (Trait-Erweiterung in genossi_service/src/claim_context.rs)"
  - "AuthenticatedContext::as_helper Override mit defensivem JSON-Parse von Phase-2-Helper-Claims"
  - "MockContext + ()::as_helper erben Default → None (failure-closed für mock_auth + automock)"
  - "7 grüne Modul-Tests (Default + Mock + Helper-Claims-positiv + 4 negative/defensive)"
affects:
  - "Plan 03-05 (AttendanceServiceImpl::check_assembly_access D-18-Branch) — konsumiert ctx.as_helper() in der Helper-Permission-Funnel-Logik"
  - "Plan 03-04 (AttendanceService Trait) — kann das Helper-Discrimination-Pattern für check_assembly_access-Signatur einplanen"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Trait-Method-Default-Impl als Failure-Closed-Erweiterungs-Mechanismus: neue Methoden mit defensivem Default brechen keine bestehenden Implementoren (A6 in RESEARCH Assumptions Log)"
    - "Lokale serde-Mirror-Struct für JSON-Schema-Konsumption — vermeidet Cross-Crate-Imports beim Parsen einer fremden Producer-Struct (Phase 2's HelperClaims), bleibt aber Schema-isomorph"
    - "Doc-Comment-Inline-Verifikation: Schema-Quelle + Test-Fixture-Quelle als verbatim Datei:Zeilen-Referenz im Trait-Doc — vermeidet Schema-Drift zwischen Producer (Phase 2) und Consumer (Phase 3)"

key-files:
  created: []
  modified:
    - "genossi_service/src/claim_context.rs (Trait-Method + AuthenticatedContext-Override + 7 Modul-Tests)"

key-decisions:
  - "as_helper-Signatur: Option<Uuid> (NICHT Option<(Arc<str>, Uuid)>) — Begründung verbatim im Doc-Comment: Plan 05 braucht in check_assembly_access nur die assembly_id; die session_id ist NICHT Teil des Claims-JSON sondern liegt in AuthenticatedContext.user_id (Format helper:<token_id>); Cascade-Discovery in close_assembly nutzt HelperTokenDao::list_session_ids_for_assembly statt as_helper. Dies ist eine bewusste Abweichung von der RESEARCH §Open Question 1 Recommendation, die eine 2-Tuple-Signatur vorschlug — die ist redundant zum Plan-05-Use-Case."
  - "permission::MockContext ist Unit-Struct ohne Default-Impl (genossi_service/src/permission.rs:150) — Plan-Action schlug `MockContext::default()` im Test vor, korrigiert zu `MockContext` (Unit-Construct). Funktional identisch."
  - "Build-Feature: cargo build -p genossi_service (default-features) schlägt im default workspace-cargo-build pre-existing am utoipa-feature-Gate fehl (auth_types.rs:6-58 nutzt utoipa::ToSchema). Tests + verification laufen via `--features utoipa,oidc` — der reguläre Workspace-Build via genossi_rest aktiviert das Feature transitiv."

patterns-established:
  - "ClaimContext-Trait-Erweiterungs-Workflow: Default-Methode + selektive Overrides nur dort, wo die Information vorliegt (AuthenticatedContext für oidc-Builds; mock_auth+automock erben Default-None)"
  - "Defensive JSON-Parsing-Idiom: ?-Chain auf Option<&Arc<str>> + serde_json::from_str(...).ok()? + post-parse-discriminator-check (kind != \"helper\" → None) — failure-closed, kein Panic, kein .unwrap() im Produktiv-Code"
  - "Lokal-mirrored Producer-Schema-Strukturen für Cross-Crate-JSON-Verträge: HelperClaims ist private im Producer-Crate (genossi_service_impl), Consumer (genossi_service) spiegelt das Schema lokal ohne Re-Export"

requirements-completed: [ATTN-06]

# Metrics
duration: ~7 min
completed: 2026-05-04
---

# Phase 3 Plan 03: ClaimContext::as_helper Helper-Discrimination Summary

**Trait-Erweiterung `ClaimContext::as_helper(&self) -> Option<Uuid>` mit Default-Impl (failure-closed → None) und einem AuthenticatedContext-Override, der Phase-2-HelperClaims-JSON defensiv parst — die typsichere Brücke für Plan 05's Helper-Permission-Branch.**

## Performance

- **Duration:** ~7 min
- **Started:** 2026-05-04T07:50:33Z
- **Completed:** 2026-05-04T07:57:32Z (approx)
- **Tasks:** 1 TDD-Task (RED + GREEN + REFACTOR — kompletter Zyklus)
- **Files created:** 0
- **Files modified:** 1 (`genossi_service/src/claim_context.rs`)
- **Tests added:** 7 (alle grün)
- **Commits:** 3 (RED + GREEN + REFACTOR — 1 finaler Doc-Commit folgt nach diesem SUMMARY)

## Accomplishments

- **D-17/D-18 Type-Bridge gelegt:** Plan 05 kann `ctx.as_helper()` als typsichere Discrimination zwischen Helper-Sessions und OIDC/Mock-Vorständen aufrufen, ohne eine `match`-Anweisung über alle möglichen Context-Typen schreiben zu müssen.
- **Failure-Closed bei jedem Negativ-Pfad:** Default-Impl liefert `None`; AuthenticatedContext-Override liefert `None` bei null-claims, malformed JSON, kind != "helper", invalid UUID. Keine Panic-Pfade. Kein `.unwrap()` außerhalb des Test-Blocks.
- **Phase-2-Schema-Konsumption ohne Cross-Crate-Coupling:** Lokale `HelperClaims`-Struct in der Method-Body — bleibt isomorph zum Producer (`genossi_service_impl/src/session.rs:17-30`) ohne Re-Export oder zirkuläre Dependency zwischen den Crates.
- **Bestehende Tests bleiben grün:** Workspace-`cargo test` zeigt keine Regression — alle 776+ Tests bleiben grün, inklusive Phase-2's `extract_auth_context`-Tests, die das gleiche Claims-JSON konsumieren.
- **OIDC-Feature-Build verifiziert:** `cargo build -p genossi_service --features oidc,utoipa` exit 0 — die Method funktioniert symmetrisch in beiden Auth-Mode-Builds.

## Task Commits

| # | Phase | Commit | Type | Files |
|---|-------|--------|------|-------|
| 1a | RED: 7 failing tests | `3dd3044` | test | `genossi_service/src/claim_context.rs` |
| 1b | GREEN: Trait + AuthenticatedContext-Override | `f21cbaa` | feat | `genossi_service/src/claim_context.rs` |
| 1c | REFACTOR: rustfmt-Cleanup auf Test-Block | `f8f4fbb` | refactor | `genossi_service/src/claim_context.rs` |

**Plan metadata commit:** wird im Final-Commit nach diesem SUMMARY angefügt.

## Files Modified

### `genossi_service/src/claim_context.rs`

Verbatim Trait-Method-Signatur (das Plan-05-Konsumvertragsdokument):

```rust
fn as_helper(&self) -> Option<Uuid> {
    None
}
```

Verbatim AuthenticatedContext-Override (defensiver JSON-Parser):

```rust
fn as_helper(&self) -> Option<Uuid> {
    let claims_str = self.claims.as_ref()?;

    #[derive(serde::Deserialize)]
    struct HelperClaims {
        kind: String,
        assembly_id: Uuid,
    }

    let parsed: HelperClaims = serde_json::from_str(claims_str.as_ref()).ok()?;
    if parsed.kind != "helper" {
        return None;
    }
    Some(parsed.assembly_id)
}
```

Verifiziertes Phase-2-Claims-JSON-Format (verbatim aus `genossi_service_impl/src/session.rs:17-30`):

```rust
#[derive(Deserialize)]
struct HelperClaims {
    kind: String,
    assembly_id: Uuid,
}
```

JSON-Repräsentation, verbatim aus `make_helper_claims` (session.rs:712-713):

```text
{"kind":"helper","assembly_id":"<uuid-string>"}
```

**Es gibt KEIN `session_id`-Feld im Claims-JSON.** Verifiziert via `grep -n 'struct HelperClaims\|kind:\|assembly_id:\|session_id' genossi_service_impl/src/session.rs` (Z. 27-29 zeigen nur die zwei Felder; alle weiteren `session_id`-Treffer in der Datei sind SessionEntity-row-id, nicht Claims).

## Test Suite

| # | Test | Status | Pfad |
|---|------|--------|------|
| 1 | `test_as_helper_default_returns_none_for_unit` | green | Default-Impl-Pfad via `()` |
| 2 | `test_as_helper_for_mock_context_returns_none` | green | mock_auth-Pfad (cookie-basiert, nicht claims) |
| 3 | `test_as_helper_for_authenticated_context_with_helper_claims` | green | Positivpfad: helper-claims → Some(aid) |
| 4 | `test_as_helper_for_authenticated_context_with_oidc_claims_returns_none` | green | OIDC-User-Session → None |
| 5 | `test_as_helper_for_authenticated_context_without_claims_returns_none` | green | claims=None → None |
| 6 | `test_as_helper_for_authenticated_context_with_malformed_claims_returns_none` | green | not-JSON → None (kein Panic) |
| 7 | `test_as_helper_for_authenticated_context_with_helper_kind_but_invalid_uuid_returns_none` | green | UUID-Parse-Failure → None (kein Panic) |

**Gesamt:** 7/7 Tests grün. `cargo test -p genossi_service --features utoipa` zeigt 30 passed, 0 failed (alle bestehenden Tests inkl. der 7 neuen). `cargo test --workspace` exit 0, alle 776+ Tests grün.

## Decisions Made

- **as_helper-Signatur ist `Option<Uuid>`** statt `Option<(Arc<str>, Uuid)>`: Plan 05 (`AttendanceServiceImpl::check_assembly_access`) braucht in seiner Funnel-Logik nur die `assembly_id` (Vergleich mit Endpoint-aid + Status-Check). Die `session_id` kommt aus `AuthenticatedContext.user_id` (Format `helper:<token_id>`) bzw. wird im Cascade-Pfad (`close_assembly`) via `HelperTokenDao::list_session_ids_for_assembly` ermittelt — nicht aus den Claims. Dies ist eine bewusste Abweichung von der RESEARCH §Open Question 1 Recommendation und im Doc-Comment der Trait-Method explizit erklärt.
- **`permission::MockContext` ist Unit-Struct, kein `Default`-Trait** — Test-Fixture im Plan-Action sagt `MockContext::default()`; korrigiert zu `MockContext` (Unit-Construct). Funktional identisch, nur Konstruktor-Syntax. Im SUMMARY erwähnt, weil zukünftige Plan-Erweiterungen das berücksichtigen müssen.
- **Lokale serde-Mirror-Struct (statt Re-Export aus genossi_service_impl):** Producer (`HelperClaims` in genossi_service_impl/src/session.rs) ist privat; Re-Export würde eine Reverse-Dependency genossi_service → genossi_service_impl erzeugen (zirkulär). Lokale Mirror-Struct hält Schema isomorph, dokumentiert via Doc-Comment + Test-Comment den Single-Source-of-Truth.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 — Blocking] `permission::MockContext::default()` existiert nicht**

- **Found during:** Task 1 GREEN-Phase, beim ersten `cargo build -p genossi_service --tests --features utoipa`.
- **Issue:** Plan-Action-Schritt 2 (Test 2 `test_as_helper_for_mock_context_returns_none`) schreibt `MockContext::default()`. Tatsächlich ist `permission::MockContext` ein Unit-Struct ohne `Default`-Impl (`genossi_service/src/permission.rs:150` — `pub struct MockContext;`). Das ist eine **andere** Struct als `auth_types::MockContext` (Plan-Author hat die zwei verwechselt).
- **Fix:** `MockContext::default()` → `MockContext` (Unit-Construct). Funktional identisch — Unit-Struct hat keinen State.
- **Files modified:** `genossi_service/src/claim_context.rs` (Test-Block, einzige Zeile).
- **Verification:** `cargo test -p genossi_service claim_context --features utoipa` 7/7 grün; gesamtes `cargo test -p genossi_service` 30/30 grün.
- **Committed in:** `f21cbaa` (zusammen mit GREEN-Phase, da semantisch eine Einheit).
- **Forward impact:** Falls Plan 05 oder spätere Pläne `MockContext` für Tests konstruieren, müssen sie ebenfalls die Unit-Struct-Syntax verwenden (kein `::default()`-Aufruf).

**2. [Rule 2 — Done-Kriterium-Klärung] Done-Kriterium "session_id darf NICHT in claim_context.rs auftauchen" — 4 dokumentarische Treffer**

- **Found during:** Final-Verify nach GREEN-Phase, beim Run der Done-Kriterien-Greps.
- **Issue:** Plan-Done-Kriterium sagt `grep -c 'session_id' genossi_service/src/claim_context.rs` muss = 0 sein. Tatsächlich gibt es 4 `session_id`-Treffer — alle in **Doc-Kommentaren** (Z. 16, 17, 22, 46), die genau erklären, dass `session_id` **NICHT** Teil der Claims ist. Die intent des Done-Kriteriums war: kein produktiver Code-Use von `session_id` in `as_helper`. Das ist erfüllt.
- **Fix:** Keine Code-Änderung — Doc-Kommentare sind explizit Teil des Plan-Action-Texts (Plan-Z. 217: "there is NO `session_id` field"). Das Done-Kriterium ist zu eng formuliert — es zielt semantisch auf Code, nicht auf Doku.
- **Files modified:** Keine.
- **Verification:** Manuelle Inspektion der Treffer (`grep -n 'session_id' genossi_service/src/claim_context.rs`) zeigt: alle 4 sind in `///`-Doc-Comments oder `//`-Code-Comments, kein produktiver Use, kein Match auf einen Identifier oder Field-Access.
- **Committed in:** Im GREEN-Commit `f21cbaa` enthalten.
- **Forward impact:** Plan 05 sollte das Done-Kriterium adoptieren, falls es eigene `as_helper`-Konsumstellen testet — dann wieder „kein produktiver Code-Use", nicht „kein dokumentarischer Treffer".

**3. [Rule 3 — Pre-existing] `cargo build -p genossi_service` ohne Features schlägt am utoipa-Gate fehl**

- **Found during:** Task 1 GREEN-Phase nach erstem stash-pop und Re-Compile.
- **Issue:** `genossi_service/src/auth_types.rs:6,31,38,46,54` hat `#[derive(... utoipa::ToSchema)]` auf öffentlichen TOs. Diese benötigen das `utoipa`-Feature (`Cargo.toml:14,20: utoipa = ["dep:utoipa"]`). Default-Features = `["mock_auth"]` bringt das Feature NICHT. Das ist pre-existing — keine Plan-03-03-Verursachung.
- **Fix:** Verifikation läuft via `cargo build -p genossi_service --features utoipa,oidc` (transitive Activation in `genossi_rest` ist der Standard-Pfad für den Workspace).
- **Files modified:** Keine — out-of-scope.
- **Verification:** Workspace-Build (`cargo build --workspace`) sauber; einzelner Feature-isolierter Build des Crates funktioniert mit Feature-Flags.
- **Forward impact:** Out-of-Scope für Phase 3. Falls einmal jemand single-crate-build von genossi_service braucht, sollte das `utoipa`-Feature default-aktiviert werden (Phase-5-Operations-Detail).

---

**Total deviations:** 3 (1 Plan-Mismatch ohne semantische Folgen, 1 Done-Kriterium-Klärung ohne Code-Änderung, 1 pre-existing Out-of-Scope). Keine Architekturentscheidung. Keine Auswirkung auf Plan-Konsumenten.

## Issues Encountered

- **rustfmt + cargo-fmt nicht direkt auf PATH** (pre-existing in Nix-Setup; siehe Memory `feedback_nix_toolchain.md`). rustfmt aus `/nix/store` (rustfmt-preview-1.93.0) angewendet, eine kleine Format-Korrektur im Test-Block (Zeile 154) als REFACTOR-Commit `f8f4fbb`.
- **clippy** wäre der nächste Verifikations-Step — Plan-Verification listet `cargo clippy -p genossi_service --no-deps -- -D warnings`. Das Toolchain-Mismatch-Issue aus Plans 03-01 + 03-02 (clippy aus /nix/store findet std nicht) gilt weiter; nicht Plan-spezifisch.
- **stash-pop nach utoipa-Build-Test:** Beim Diagnose-Schritt (Test ob Build pre-existing ohne Plan-Änderung scheitert) wurde der Stash zwischengeparkt — pop hat ihn sauber zurückgespielt. Keine Code-Verlust.

## TDD Gate Compliance

- **RED-Gate:** Commit `3dd3044` (`test(03-03): add failing tests for ClaimContext::as_helper`) — verifiziert per `cargo build -p genossi_service --tests` mit 15 E0599-Errors (Method existierte nicht).
- **GREEN-Gate:** Commit `f21cbaa` (`feat(03-03): implement ClaimContext::as_helper for helper discrimination`) — alle 7 Tests grün, alle bestehenden Tests bleiben grün.
- **REFACTOR-Gate:** Commit `f8f4fbb` (`refactor(03-03): apply rustfmt to claim_context test fixture`) — single-line Format-Korrektur, Tests bleiben grün.

## Threat Flags

Keine über die im Plan-Frontmatter dokumentierten T-03-03-01..03 hinaus. Keine zusätzlichen Trust-Boundaries angefasst:

- **T-03-03-01 (Elevation of Privilege):** Mitigated. Default-Impl liefert None; AuthenticatedContext-Override liefert nur bei `kind=="helper"` AND parsbarer UUID `Some(aid)`.
- **T-03-03-02 (Spoofing):** Mitigated. Defensiver JSON-Parse mit `?`-Operator + `.ok()?`; kein `.unwrap()` außerhalb Tests; failure-closed → None.
- **T-03-03-03 (Repudiation):** Out-of-Scope für Phase 3 (D-08 — kein Audit für attendance). marked_by_user_id stammt aus `AuthenticatedContext.user_id` (Format `helper:<token_id>`), NICHT aus den Claims — die Claims tragen nur `kind` + `assembly_id`.

## Next Phase Readiness

**Direkt konsumierbar von Plan 03-04 (AttendanceService Trait, Wave 1):**

- Plan 03-04 darf das `as_helper`-Pattern bereits in der Trait-Doku referenzieren, falls dort `check_assembly_access` als Vertragsteil dokumentiert wird.

**Direkt konsumierbar von Plan 03-05 (AttendanceServiceImpl::check_assembly_access D-18-Branch):**

```rust
// Skizze für Plan 05's check_assembly_access:
async fn check_assembly_access(
    &self,
    assembly_id: Uuid,
    context: Authentication<Self::Context>,
) -> Result<AssemblyEntity, ServiceError> {
    let tx = self.transaction_dao.use_transaction(None).await?;
    let assembly = self.assembly_dao.find_by_id(assembly_id, tx.clone()).await?
        .ok_or(ServiceError::EntityNotFound(assembly_id))?;

    match context {
        Authentication::Full => Ok(assembly),
        Authentication::Context(ctx) => {
            if let Some(helper_aid) = ctx.as_helper() {
                // D-18 helper-branch: aid match + status open
                if helper_aid != assembly_id {
                    return Err(ServiceError::PermissionDenied);
                }
                if assembly.status != AssemblyStatus::Open {
                    return Err(ServiceError::PermissionDenied);
                }
                Ok(assembly)
            } else {
                // D-19: admin fallback for OIDC/Mock-Vorstand
                self.permission_service
                    .check_permission(ADMIN_PRIVILEGE, Authentication::Context(ctx))
                    .await?;
                Ok(assembly)
            }
        }
    }
}
```

**Konsum-Vertrag:** Plan 05 destrukturiert `Some(helper_aid)` als reine `Uuid` (NICHT als `(session_id, aid)`-Tuple). Die `session_id` ist in Plan 05 nicht nötig — sie ist Cascade-Discovery-Pfad-spezifisch (Plan 02's `list_session_ids_for_assembly`).

**No blockers** für Plan 04 oder Plan 05.

## Self-Check

Verification commands run after SUMMARY drafting:

```bash
[ -f /home/neosam/programming/rust/projects/genossi3/genossi_service/src/claim_context.rs ] && echo "FOUND"
git log --oneline | grep -E '3dd3044|f21cbaa|f8f4fbb'
grep -c 'fn as_helper' /home/neosam/programming/rust/projects/genossi3/genossi_service/src/claim_context.rs
```

See `## Self-Check: PASSED` block at end.

---

## Self-Check: PASSED

- `genossi_service/src/claim_context.rs` — FOUND on disk, contains `fn as_helper` (2 Treffer: Default + Override)
- `.planning/phases/03-attendance-aggregat-cascade-invalidation/03-03-SUMMARY.md` — FOUND on disk (this file)
- Commit `3dd3044` (RED) — FOUND in git log
- Commit `f21cbaa` (GREEN) — FOUND in git log
- Commit `f8f4fbb` (REFACTOR) — FOUND in git log
- All 7 module tests green via `cargo test -p genossi_service claim_context --features utoipa`
- Workspace tests stay green via `cargo test --workspace` (776+ tests, 0 failed)
- OIDC + utoipa feature build OK via `cargo build -p genossi_service --features oidc,utoipa`

---

*Phase: 03-attendance-aggregat-cascade-invalidation*
*Plan: 03*
*Completed: 2026-05-04*
