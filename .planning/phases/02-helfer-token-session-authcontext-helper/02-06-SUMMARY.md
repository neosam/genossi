---
phase: 02-helfer-token-session-authcontext-helper
plan: 06
subsystem: auth

tags: [session-service, auth-context, helper-claims, assembly-status-check, mock_auth, oidc, d-18, d-19, rust]

requires:
  - phase: 02-helfer-token-session-authcontext-helper
    plan: 01
    provides: "AssemblyDao trait + AssemblyEntity + AssemblyStatus (Phase 1 carry-over) — used as injected dep for the D-18 status-check"
  - phase: 02-helfer-token-session-authcontext-helper
    plan: 02
    provides: "AuthContext::Helper { session_id, assembly_id } variant — constructed by the new extract_auth_context branch"

provides:
  - "SessionServiceImpl::extract_auth_context erkennt JSON-Claims mit `kind=\"helper\"` und konstruiert AuthContext::Helper {session_id, assembly_id} (D-15/D-16)"
  - "D-18-Cascade wired in extract_auth_context: bei assembly.status != Open invalidiert die Session via permission_dao.delete_session und liefert Ok(None)"
  - "SessionServiceDeps erweitert um AssemblyDao + TransactionDao (drei Deps total: PermissionDao, AssemblyDao, TransactionDao)"
  - "MockSessionServiceImpl ist refactored zu Field-Struct mit optionalem assembly_status_probe; Default-Konstruktor erhält Backward-Compat"
  - "Cookie-Format `helper:<assembly_uuid>:<token_id>` wird im Mock erkannt und in AuthContext::Helper umgesetzt — RESEARCH-A3 / Pitfall 5 resolved"
  - "Trait AssemblyStatusProbe (async, public) erlaubt Plan 07 + Plan 08 die HLPR-05-Cascade End-to-End in mock_auth-Builds zu üben"
  - "11 neue Unit-Tests (5 Helper-Claims-Discriminator + 6 Mock-Helper-Cookie-Format/Probe)"

affects: ["02-07 helper-redeem REST handler + DI-Wiring", "02-08 e2e tests (HLPR-05 Cascade)", "Phase 3 attendance-aggregat"]

tech-stack:
  added: []
  patterns:
    - "Claims-JSON-Discriminator via serde_json::from_str::<HelperClaims>(claims) — kind-Feld als Tag, parse-Fehler fällt auf User-Session-Pfad zurück (Backward-Compat)"
    - "Pitfall-2 Early-Return: claims.is_none() ⇒ kein extra DB-Roundtrip auf hot path"
    - "Probe-Pattern: optionales async-Trait via Arc<dyn AssemblyStatusProbe> erlaubt mock_auth-Cascade-Test ohne reale DB"
    - "use_transaction(None) + commit als minimal-invasive D-18-Lookup-TX in extract_auth_context"

key-files:
  created:
    - ".planning/phases/02-helfer-token-session-authcontext-helper/02-06-SUMMARY.md"
  modified:
    - "genossi_service_impl/src/session.rs"
    - "genossi_bin/src/lib.rs"

key-decisions:
  - "D-19 Wire-Point: Status-Check landet in SessionServiceImpl::extract_auth_context (Service-Layer), nicht in der REST-Auth-Middleware. Drei Argumente: (1) saubere Schicht-Trennung, (2) mockbar via SessionServiceDeps, (3) auth_middleware.rs:101-134 delegiert bereits korrekt — KEINE Änderung dort nötig"
  - "MockSessionServiceImpl-Refactor zu Field-Struct mit Default — bestehende Use-Sites in genossi_bin migriert auf ::default() (Plan 07 wird das auf ::with_probe(...) umstellen, sobald der Probe-Adapter existiert)"
  - "AssemblyStatusProbe als async-Trait public exportiert — Plan 07 + Plan 08 brauchen den Type, also ist Sichtbarkeit explizit nötig"
  - "genossi_bin/src/lib.rs muss in Plan 06 angepasst werden (nicht erst Plan 07), weil sonst der Workspace-Build bricht: SessionServiceDeps hat zwei neue assoziierte Typen + MockSessionServiceImpl ist kein Unit-Struct mehr"
  - "tx.clone() vor find_by_id + transaction_dao.commit nach dem Lookup — TestTransactionDao erlaubt das No-Op, in Produktion wird ein echter SQLite-Tx aufgemacht und sofort committed (Pattern aus Phase-1 AssemblyServiceImpl::open_assembly übernommen)"

patterns-established:
  - "Claims-Discriminator-Parsing: serde_json::from_str::<HelperClaims> + match parsed.kind als Standard für Future-Claim-Tags (z.B. `kind=\"vorstand-impersonation\"` o.ä.)"
  - "Probe-Adapter-Pattern für mock_auth: optionales async-Trait, das eine Service-Layer-Entscheidung in mock_auth-Builds nachstellt — entkoppelt mock_auth-Tests vom realen DAO-Stack"
  - "Backward-Compat-Migration via Default-Impl: Field-Struct mit `#[derive(Default)]` erlaubt bestehende Unit-Konstruktion über `::default()` zu ersetzen, ohne API-Breakage für Konsumenten"

requirements-completed: [HLPR-05]

duration: ~28min
completed: 2026-05-03
---

# Phase 2 Plan 06: SessionService Helper-Claims-Discriminator + D-18 Status-Check Summary

**Helper-Sessions werden im SessionService an `claims.kind=="helper"` erkannt; D-18 invalidiert die Session sofort, wenn die gebundene Assembly nicht mehr `Open` ist — Pitfall 2 Early-Return verhindert DB-Roundtrip auf dem User-Session-Hot-Path. Mock-Variante erkennt Cookie-Format `helper:<uuid>:<tok>` und cascadiert via optionalem AssemblyStatusProbe.**

## Performance

- **Duration:** ~28 Minuten
- **Started:** 2026-05-03T11:15:57Z
- **Completed:** 2026-05-03T11:43:44Z
- **Tasks:** 2
- **Files modified:** 2 (`genossi_service_impl/src/session.rs`, `genossi_bin/src/lib.rs`)

## Accomplishments

### Task 1 — `SessionServiceImpl::extract_auth_context` (D-15/D-16/D-18)
- `SessionServiceDeps` Macro-Block um zwei Deps erweitert: `AssemblyDao<Transaction = Self::Transaction>` + `TransactionDao<Transaction = Self::Transaction>`
- `HelperClaims { kind, assembly_id }` Struct mit `#[derive(Deserialize)]` für JSON-Discriminator-Parsing
- `extract_auth_context` parst die JSON-Claims, erkennt `kind=="helper"` und führt den D-18-Status-Check via `assembly_dao.find_by_id` aus. Bei `assembly.status == Open` returnt sie `AuthContext::Helper {session_id, assembly_id}`; sonst `permission_dao.delete_session` + `Ok(None)` (HLPR-05 SC#4)
- Pitfall-2-Early-Return: `claims.is_none()` ⇒ kein extra DB-Roundtrip auf hot path (User/OIDC-Sessions zahlen den Lookup-Preis nicht)
- Backward-Compat: Sessions ohne `kind`-Feld oder mit invalid-JSON-Claims fallen auf den bestehenden `AuthContext::Mock(...)`-Pfad zurück — getestet
- 5 neue Unit-Tests (`test_extract_auth_context_helper_claims_*`), alle grün
- TDD-Disziplin: RED-Commit (`2f04e6b`) hat 3 failing tests, GREEN-Commit (`b555aac`) macht alle grün

### Task 2 — `MockSessionServiceImpl` Helper-Cookie-Format-Erkennung (RESEARCH-A3, Pitfall 5)
- `MockSessionServiceImpl` von `pub struct MockSessionServiceImpl;` zu Field-Struct mit `assembly_status_probe: Option<Arc<dyn AssemblyStatusProbe>>` refactored
- `#[derive(Default, Clone)]` + `::new()` + `::with_probe(probe)` Konstruktoren
- `pub trait AssemblyStatusProbe { async fn is_open(&self, assembly_id: Uuid) -> bool; }` als public-async-Trait exportiert (Plan 07 + Plan 08 nutzen den Type)
- Cookie-Format-Erkennung in `extract_auth_context`: `sid.strip_prefix("helper:").and_then(split_once(':'))` + `Uuid::parse_str` ⇒ `AuthContext::Helper`. Falls Probe gesetzt UND `probe.is_open(assembly_id) == false`, returnt `Ok(None)` (HLPR-05 Cascade in mock_auth)
- Defensive Fallbacks: helper:<invalid_uuid>:<tok> ⇒ `AuthContext::Mock` (kein Crash)
- 6 neue Unit-Tests in `mock_session_helper_tests`, alle grün
- `genossi_bin/src/lib.rs:450` migrierte Konstruktion `MockSessionServiceImpl` (Unit) zu `MockSessionServiceImpl::default()`

## Task Commits

Plan 06 lief im TDD-Modus für Task 1, daher mehrere Commits:

1. **Task 1 RED** (test): `2f04e6b` — `test(02-06): scaffold helper-claims tests + extend SessionServiceDeps (RED)` — Setup-Erweiterung des Macros, TestDeps + TestAssemblyDao + TestTransactionDao + 5 Tests, davon 3 failing (extract_auth_context returnt noch Mock)
2. **Task 1 GREEN** (feat): `b555aac` — `feat(02-06): wire helper-claims discriminator + D-18 status-check (GREEN)` — Helper-Claims-Branch in extract_auth_context implementiert, alle 13 Session-Tests grün
3. **Task 2** (feat): `670736c` — `feat(02-06): mock helper-cookie-format detection + AssemblyStatusProbe` — MockSessionServiceImpl-Refactor + AssemblyStatusProbe-Trait + 6 mock-helper-Tests + genossi_bin-Migration
4. **rustfmt-Cleanup** (style): `290e10a` — `style(02-06): apply rustfmt to session.rs` — Multi-line method-chain reformat per rustfmt-1.90 (kein Verhaltensänderung)

## Files Created/Modified

### `genossi_service_impl/src/session.rs`
- Imports erweitert: `serde::Deserialize`, `genossi_dao::assembly::{AssemblyDao, AssemblyStatus}`, `genossi_dao::TransactionDao`
- Neuer `HelperClaims`-Struct (D-16-Schema: `{kind, assembly_id}`)
- `gen_service_impl!`-Block um zwei Deps erweitert (AssemblyDao + TransactionDao)
- `extract_auth_context` (Real-Impl) erweitert um Helper-Branch + D-18-Lookup
- `MockSessionServiceImpl` von Unit-Struct zu Field-Struct + `AssemblyStatusProbe`-Trait + Helper-Cookie-Format-Erkennung
- Tests-Modul erweitert um `TestAssemblyDao`, `TestTransactionDao`, `make_assembly`, `make_service_with_assembly`, 5 Helper-Claims-Tests
- Neuer `mod mock_session_helper_tests` mit 6 Tests

### `genossi_bin/src/lib.rs`
- `SessionServiceDependencies` (oidc-Pfad) impl ergänzt um zwei assoziierte Typen: `type AssemblyDao = AssemblyDao;`, `type TransactionDao = TransactionDao;`
- `SessionServiceImpl`-Konstruktion (oidc-Pfad) ergänzt um `assembly_dao` + `transaction_dao`
- `MockSessionServiceImpl` (mock_auth-Pfad) Konstruktion migriert auf `::default()` (statt Unit-Konstruktion) — vorbereitet für Plan 07's `::with_probe(...)`-Migration

## Decisions Made

- **D-19 Wire-Point belassen wie geplant in SessionServiceImpl::extract_auth_context** — `auth_middleware.rs::extract_context_from_headers` (Zeilen 101-134) delegiert bereits korrekt an `session_service.extract_auth_context()`. Drei Argumente:
  1. **Saubere Schicht-Trennung:** Permission/Lifecycle-Logik gehört in den Service-Layer, nicht in die REST-Middleware
  2. **Mockbarkeit:** `SessionServiceDeps` mit `AssemblyDao + TransactionDao` erlaubt Mock-Tests via einfacher Test-Doubles (TestAssemblyDao, TestTransactionDao)
  3. **Minimal-invasiv:** Bestehender User-Session-Pfad ist nicht betroffen (Pitfall-2-Early-Return wenn `claims.is_none()`)
- **Plan 06 muss `genossi_bin/src/lib.rs` mit anpassen** (nicht erst Plan 07): `SessionServiceDeps` hat jetzt zwei neue assoziierte Typen, und `MockSessionServiceImpl` ist kein Unit-Struct mehr — sonst bricht der Workspace-Build und das Acceptance-Criterion `cargo build --workspace` failt. Plan 07 erweitert die `MockSessionServiceImpl::default()`-Konstruktion zu `::with_probe(adapter)` für die D-18-Cascade in mock_auth E2E-Tests.
- **HelperClaims-Struct privat** (nur module-internal-Visibility): Der Discriminator ist eine Service-Implementierungs-Detail, nicht Teil der öffentlichen API. Plan 03 (HelperTokenServiceImpl) und Plan 07 (REST-Handler) konstruieren die JSON-Claims via String-Building (nicht via Serialize), weil sie die Cookie zur Session-Erzeugung schreiben (`SessionService::ensure_user_and_create_session_with_claims`).

## HelperClaims-Schema (D-16)

```json
{
  "kind": "helper",
  "assembly_id": "<uuid-string>"
}
```

- **kind**: String, derzeit nur `"helper"`. Future Discriminators (z.B. `"vorstand-impersonation"`) würden den Trait-Match-Block in `extract_auth_context` erweitern.
- **assembly_id**: UUID-String (Standard-RFC-4122-Format). Plan 03 (HelperTokenService) konstruiert die Claims direkt via `format!("{{\"kind\":\"helper\",\"assembly_id\":\"{}\"}}", uuid)` — kein `serde_json::to_string`, weil die Schema-Form fixiert ist.

**Backward-Compat-Garantie:** Jede `claims`-Form, die diesem Schema NICHT entspricht (parse-error, andere Felder, fehlendes `kind`), fällt auf den bestehenden `AuthContext::Mock` (mock_auth) bzw. den OIDC-User-Pfad (oidc) zurück. **Bestehende Sessions werden nicht invalidiert** — nur explizit als `kind=helper` markierte werden gegen `assembly.status` geprüft.

## Mock-Cookie-Format-Konvention (für Plan 08 E2E-Tests)

Plan 08 setzt im E2E-Test-Cookie das Format:

```
app_session = helper:<assembly_uuid>:<token_id>
```

Beispiel:
```
app_session = helper:550e8400-e29b-41d4-a716-446655440000:tok-abc123
```

`MockSessionServiceImpl::extract_auth_context` parst dieses Format:
- `helper:<valid_uuid>:<tok>` → `AuthContext::Helper { session_id: Arc::from(<full-cookie>), assembly_id: <parsed_uuid> }`
- `helper:<invalid_uuid>:<tok>` → `AuthContext::Mock` (defensive)
- `<anything-without-helper-prefix>` → `AuthContext::Mock` (Backward-Compat für bestehende mock_auth-Tests)

Mit gesetztem `assembly_status_probe` (Plan 07's DI-Wiring) cascade-invalidiert die Mock-Variante Helper-Cookies, wenn die Probe `is_open(assembly_id) == false` antwortet — exakt wie der reale `SessionServiceImpl` im OIDC-Build.

## Liste der neuen Deps in SessionServiceDeps (für Plan 07 DI-Wiring)

`gen_service_impl!`-Block in `genossi_service_impl/src/session.rs`:

```rust
gen_service_impl! {
    struct SessionServiceImpl: SessionService = SessionServiceDeps {
        PermissionDao: PermissionDao = permission_dao,                                  // bestehend
        AssemblyDao: AssemblyDao<Transaction = Self::Transaction> = assembly_dao,       // neu (Plan 06)
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao, // neu (Plan 06)
    }
}
```

`SessionServiceDependencies` impl in `genossi_bin/src/lib.rs:97-104` (oidc-Pfad):

```rust
#[cfg(feature = "oidc")]
impl genossi_service_impl::session::SessionServiceDeps for SessionServiceDependencies {
    type Context = Context;
    type Transaction = Transaction;
    type PermissionDao = PermissionDao;
    type AssemblyDao = AssemblyDao;       // neu (Plan 06)
    type TransactionDao = TransactionDao; // neu (Plan 06)
}
```

Konstruktion in `RestStateImpl::new` (oidc-Pfad, ~Z. 449):

```rust
#[cfg(feature = "oidc")]
let session_assembly_dao = Arc::new(AssemblyDao::new(pool.clone()));

#[cfg(feature = "oidc")]
let session_service = Arc::new(genossi_service_impl::session::SessionServiceImpl {
    permission_dao: permission_dao.clone(),
    assembly_dao: session_assembly_dao,
    transaction_dao: transaction_dao.clone(),
});
```

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocker] genossi_bin/src/lib.rs muss in Plan 06 angepasst werden**
- **Found during:** Task 1 + Task 2
- **Issue:** Plan-Frontmatter listet nur `genossi_service_impl/src/session.rs` als `files_modified`, aber:
  - SessionServiceDeps hat zwei neue assoziierte Typen (`AssemblyDao`, `TransactionDao`) — der bestehende `impl SessionServiceDeps for SessionServiceDependencies` in `genossi_bin/src/lib.rs:97-102` schlug Compile-Fehler "missing associated types"
  - MockSessionServiceImpl ist kein Unit-Struct mehr — die bestehende Konstruktion `Arc::new(MockSessionServiceImpl)` schlug Compile-Fehler "expected value, found struct"
  - Plan 06 sagt: "Plan 07 (DI-Wiring) ist verantwortlich für die genossi_bin-Migration" — aber Plan 07 ist Wave 3 und läuft NACH Plan 06. Der Workspace-Build muss am Ende von Plan 06 grün sein (Acceptance Criterion).
- **Fix:** genossi_bin angepasst:
  - `SessionServiceDependencies` impl ergänzt um `type AssemblyDao = AssemblyDao;` + `type TransactionDao = TransactionDao;`
  - `SessionServiceImpl`-Konstruktion (oidc-Pfad) ergänzt um `assembly_dao` + `transaction_dao`-Felder; neuer Helper `let session_assembly_dao = Arc::new(AssemblyDao::new(pool.clone()));` vor der Session-Service-Konstruktion eingefügt (vor `let assembly_service`-Konstruktion in Z. 498, daher kein Cloning-Konflikt)
  - `MockSessionServiceImpl` (mock_auth-Pfad) auf `::default()` migriert
- **Files modified:** `genossi_bin/src/lib.rs`
- **Commit:** Beigetragen in `2f04e6b` (Task 1) und `670736c` (Task 2)

**2. [Style cleanup] rustfmt-Reformatierung**
- **Found during:** post-Task-2 verification
- **Issue:** `rustfmt --check` meldete Multi-line method-chain Diff in `extract_auth_context` und `TestAssemblyDao::dump_all`
- **Fix:** `rustfmt --edition 2021` über die geänderten Dateien laufen lassen, alle 19 Tests bleiben grün
- **Files modified:** `genossi_service_impl/src/session.rs`
- **Commit:** `290e10a`

## Verification Results

| Check | Command | Result |
|-------|---------|--------|
| genossi_service_impl mock_auth build | `cargo build -p genossi_service_impl --features mock_auth` | exit 0 |
| genossi_service_impl oidc build | `cargo build -p genossi_service_impl --no-default-features --features oidc` | exit 0 |
| Workspace mock_auth build | `cargo build --workspace --features mock_auth` | exit 0 |
| Workspace oidc build | `cargo build --workspace --no-default-features --features oidc` | exit 0 |
| Session-Tests | `cargo test -p genossi_service_impl --features mock_auth --lib session` | 19/19 passed |
| Mock-Helper-Tests speziell | `cargo test -p genossi_service_impl --features mock_auth --lib mock_session_helper_tests` | 6/6 passed |
| Workspace-Lib-Tests | `cargo test --workspace --features mock_auth --lib` | 178 + others passed, 0 failed |
| rustfmt | `rustfmt --check session.rs lib.rs` | exit 0 nach Cleanup-Commit |

## Acceptance Criteria Per Task

### Task 1 (Helper-Claims-Discriminator)
- ✓ `cargo build --workspace --features mock_auth` exit 0
- ✓ `cargo build --workspace --no-default-features --features oidc` exit 0
- ✓ `cargo test -p genossi_service_impl session` exit 0 (8 bestehende + 5 neue Tests grün)
- ✓ `grep -c "AssemblyDao: AssemblyDao" session.rs` = 1
- ✓ `grep -c "TransactionDao: TransactionDao" session.rs` = 1
- ✓ `grep -c "struct HelperClaims" session.rs` = 1
- ✓ `grep -c "kind: String" session.rs` = 1 (im HelperClaims-Struct)
- ✓ `grep -c "AssemblyStatus::Open" session.rs` = 2
- ✓ `grep -c "AuthContext::Helper" session.rs` = 8 (≥ 1 in real, ≥ 1 in mock + Tests)
- ✓ `grep -c "permission_dao.delete_session" session.rs` = 4 (≥ 2 expected)
- ✓ Early-Return-Hint: `grep ... claims.as_deref` = 1
- ✓ `grep -c "test_extract_auth_context_helper_claims" session.rs` = 3 (4 Tests + 5 Klassen-Bezug; Test-Funktionsnamen mit dem Substring sind 3, AC verlangt ≥ 2)

### Task 2 (Mock-Helper-Cookie-Format)
- ✓ `cargo build --workspace --features mock_auth` exit 0
- ✓ `cargo test -p genossi_service_impl mock_session_helper_tests` 6/6 passed
- ✓ `cargo test -p genossi_service_impl session` keine Regressions, alle 19 Tests grün
- ✓ `grep -c 'sid.strip_prefix("helper:")' session.rs` = 1
- ✓ `grep -A 3 'sid.strip_prefix("helper:")' session.rs | grep -c "split_once"` = 1
- ✓ `grep -c "AuthContext::Helper" session.rs` = 8 (≥ 2)
- ✓ `grep -c "trait AssemblyStatusProbe" session.rs` = 1
- ✓ `grep -c "assembly_status_probe" session.rs` = 3 (≥ 2)
- ✓ `grep -c "MockSessionServiceImpl::with_probe\|MockSessionServiceImpl::new" session.rs` = 3 (≥ 2)
- ✓ Test-Trio (`test_mock_helper_cookie_format_returns_helper_context`, `test_mock_normal_cookie_returns_mock_context`, `test_mock_helper_cookie_with_closed_probe_returns_none`) = 3 vorhanden

## Hinweise für nachfolgende Plans

### Plan 07 (DI-Wiring + REST-Handler)
- `MockSessionServiceImpl::default()` in `genossi_bin/src/lib.rs:450` ist der Backward-Compat-Stand. Plan 07 muss den Probe-Adapter implementieren (struct, das AssemblyDao + TransactionDao hält und `AssemblyStatusProbe::is_open` implementiert) und die Konstruktion auf `MockSessionServiceImpl::with_probe(Arc::new(adapter))` umstellen — Plan 02-08 Task 2 braucht das.
- Der oidc-Pfad ist bereits vollständig wired (Plan 06 hat das mit erledigt) — Plan 07 muss nur den mock_auth-Pfad mit dem Probe-Adapter erweitern.

### Plan 03 (HelperTokenServiceImpl) — wenn noch nicht erledigt
- Beim Erzeugen einer Helper-Session muss `claims = Some(format!("{{\"kind\":\"helper\",\"assembly_id\":\"{}\"}}", assembly_id))` an `SessionService::ensure_user_and_create_session_with_claims` übergeben werden. Schema ist exakt das, was `HelperClaims`-Deserialize erwartet — keine Abweichungen erlaubt.

### Plan 08 (E2E-Tests)
- Helper-Cookie-Format für mock_auth: `app_session=helper:<assembly_uuid>:<token_id>` (statt einer reinen UUID).
- HLPR-05-Cascade-Test: nach `close_assembly` über die REST-API einen Helper-Endpoint aufrufen → die `AssemblyStatusProbe` (sofern Plan 07 sie wired) liefert `false` → MockSessionServiceImpl returnt `Ok(None)` → die Auth-Middleware reagiert mit 401.

## TDD Gate Compliance

Plan-Frontmatter hat `type=execute`, also keine Plan-Level-TDD-Gate. Aber Tasks 1 + 2 waren `tdd="true"` und Task 1 hat einen sauberen RED→GREEN-Cycle geliefert:

- **Task 1 RED:** `2f04e6b` (test) — 3 failing tests asserting Helper-Verhalten, das noch nicht implementiert war
- **Task 1 GREEN:** `b555aac` (feat) — Helper-Branch in extract_auth_context, alle Tests grün
- **Task 2:** `670736c` (feat) — Tests + Implementation kombiniert, weil das Struct-Refactoring (Unit→Field) RED-only nicht möglich war (Compile-Bruch). Tests gehen direkt in den GREEN-State, alle 6 grün
- **Style:** `290e10a` (style) — rustfmt-Cleanup, kein Verhaltensänderung

## Threat Surface Scan

Kein neues Threat-Surface eingeführt. Plan 06 implementiert die Mitigations aus dem Plan-Threat-Model:

- **T-02-06-01 (Tampering / forged claims):** mitigate ✓ — claims werden vom Service-Code via `ensure_user_and_create_session_with_claims` gesetzt, Client kann nicht beliebige claims senden. Server-Round-Trip via session-Tabelle.
- **T-02-06-02 (Info-Disclosure / closed assembly):** mitigate ✓ — D-18-Status-Check invalidiert Session sofort, wenn `assembly.status != Open` (verifiziert per `test_extract_auth_context_helper_claims_invalidates_when_assembly_closed`).
- **T-02-06-03 (DoS / extra DB-Roundtrip):** mitigate ✓ — Pitfall-2-Early-Return: `claims.is_none()` ⇒ kein extra Roundtrip. Nur Helper-Sessions zahlen den AssemblyDao-Lookup-Preis.
- **T-02-06-04 (Spoofing / Mock-Cookie in Production):** accept ✓ — MockSessionServiceImpl wird nur im mock_auth-Feature-Build verwendet; Produktion läuft mit oidc-Feature → SessionServiceImpl. Keine Production-Exposure.

## Self-Check: PASSED

Verifikation der SUMMARY-Behauptungen:

- [x] `genossi_service_impl/src/session.rs` existiert (FOUND)
- [x] `genossi_bin/src/lib.rs` existiert (FOUND)
- [x] Commit `2f04e6b` existiert (`git log --oneline | grep 2f04e6b` → FOUND)
- [x] Commit `b555aac` existiert (FOUND)
- [x] Commit `670736c` existiert (FOUND)
- [x] Commit `290e10a` existiert (FOUND)
- [x] `AuthContext::Helper` wird konstruiert (count=8 in session.rs, ≥ 2 unique sites)
- [x] `HelperClaims`-Struct hat genau ein Vorkommen
- [x] `AssemblyStatusProbe`-Trait hat genau ein Vorkommen
- [x] Beide Feature-Builds (mock_auth, oidc) grün
- [x] Workspace-Tests grün (genossi_service_impl: 178 passed, 0 failed; weitere Crates ungebrochen)
- [x] 19/19 Session-Tests grün (8 bestehend + 11 neu)

---
*Phase: 02-helfer-token-session-authcontext-helper*
*Completed: 2026-05-03*
