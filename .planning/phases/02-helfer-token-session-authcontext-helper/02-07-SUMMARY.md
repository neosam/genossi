---
phase: 02-helfer-token-session-authcontext-helper
plan: 07
subsystem: rest+di

tags: [rest-handlers, di-wiring, axum, utoipa, tower-governor, set-cookie, openapi, mock-auth, oidc, phase-2]

requires:
  - phase: 02-helfer-token-session-authcontext-helper
    plan: 04
    provides: "HelperTokenTO + 5 friend TOs + HelperTokenService trait"
  - phase: 02-helfer-token-session-authcontext-helper
    plan: 05
    provides: "HelperTokenServiceImpl with 8 deps + Conflict-discriminator strings"
  - phase: 02-helfer-token-session-authcontext-helper
    plan: 06
    provides: "SessionService helper-claims discriminator + AssemblyStatusProbe + MockSessionServiceImpl::with_probe"

provides:
  - "genossi_rest/src/helper_token.rs: HelperTokenRestState trait + 4 Axum-Handler (3 Vorstand + 1 Public) + 2 Router-Funktionen + 2 ApiDocs"
  - "RestError::Forbidden(String) (403) + RestError::Gone(String) (410) als neue Varianten (D-24)"
  - "error_handler maps 403/410 mit Body"
  - "ApiDoc nestet '/api/assembly/{assembly_id}/helper-tokens' (Vorstand) + PublicApiDoc-Merge für '/api/helper/redeem'"
  - "create_app + start_server type-bound erweitert um helper_token::HelperTokenRestState"
  - "redeem_rate_layer (10/min/IP via tower_governor) auf POST /api/helper/redeem (Pitfall 7)"
  - "Router-nest 'api/assembly/{assembly_id}/helper-tokens' (admin) + 'api/helper' (public, mit redeem_rate_layer)"
  - "test_server.rs: trait-bound erweitert um HelperTokenRestState"
  - "genossi_bin: HelperTokenServiceDependencies + HelperTokenService type-alias + RestStateImpl-Field + DI-Wiring mit 8 Deps"
  - "DbAssemblyStatusProbe (mock_auth-only): production probe für MockSessionServiceImpl::with_probe — HLPR-05 cascade in mock_auth E2E observable"
  - "impl genossi_rest::helper_token::HelperTokenRestState for RestStateImpl"

affects:
  - "02-08 e2e-tests: alle 4 Endpoints sind via TestServer ansprechbar; HLPR-05-cascade fließt durch DbAssemblyStatusProbe"

tech-stack:
  added: []
  patterns:
    - "Public-Endpoint-Pattern mit Pro-IP-Rate-Limit (`generate_public_route` + .layer(redeem_rate_layer)) — Vorlage aus application::generate_public_route"
    - "Differential ServiceError-Mapping im Handler-Body: `Conflict(payload)` Pattern-Match auf Discriminator-Strings (`already_used` -> 410, `revoked`|`assembly_not_open` -> 403) statt globalen From-Impl auf RestError"
    - "Set-Cookie via raw header::SET_COOKIE (kein tower_cookies::Cookies-Extension nötig), weil der Public-Pfad nicht hinter dem CookieManagerLayer liegt — direkter HeaderValue::from_str-Build"
    - "DbAssemblyStatusProbe-Adapter (mock_auth-only) via async-trait: kapselt AssemblyDao + TransactionDao und erfüllt das `AssemblyStatusProbe`-Contract aus Plan 06 — entkoppelt mock_auth von echter SessionServiceImpl-Konstruktion"
    - "assembly_dao Konstruktion vor dem session_service: ein Arc<AssemblyDao> wird zwischen DbAssemblyStatusProbe (mock_auth), SessionServiceImpl (oidc), AssemblyServiceImpl und HelperTokenServiceImpl geteilt"

key-files:
  created:
    - "genossi_rest/src/helper_token.rs"
    - ".planning/phases/02-helfer-token-session-authcontext-helper/02-07-SUMMARY.md"
  modified:
    - "genossi_rest/src/lib.rs"
    - "genossi_rest/src/test_server.rs"
    - "genossi_bin/src/lib.rs"

key-decisions:
  - "ServiceError-Mapping bleibt im Handler-Body (lokaler match), nicht in der globalen `From<ServiceError> for RestError`. Begründung: nur der redeem-Handler braucht die Conflict-Discriminator-Differenzierung; alle anderen Handler-Pfade nutzen weiterhin den 1:1-Mapping (Conflict -> 409). Plan 04+05 hatten die stable strings explizit so dokumentiert (lokale Pattern-Match-Verantwortung des Aufrufers)."
  - "DbAssemblyStatusProbe nutzt `transaction_dao.use_transaction(None)` + `assembly_dao.find_by_id(...)` ohne explizites commit. find_by_id ist read-only; die Transaction wird beim Drop sauber zurückgegeben. Das Pattern matched die Phase-1-AssemblyServiceImpl-Read-Only-Pfade und ist die einfachste DB-Roundtrip-Form für die HLPR-05-Cascade."
  - "assembly_dao wird vor session_service konstruiert (statt erst beim assembly_service). Begründung: sowohl der oidc-SessionServiceImpl als auch die mock_auth-DbAssemblyStatusProbe brauchen die DAO für den D-18-Status-Check. Eine DAO-Instanz, mehrfach geklont über Arc."
  - "RestError::Gone und RestError::Forbidden sind neu, NICHT `RestError::HelperRedeemUsed` o.ä. — die generic-HTTP-Status-Variant-Form passt zur Konvention (NotFound, BadRequest, Conflict). Die Body-Strings tragen die Discriminator-Information für API-Konsumenten."
  - "APP_URL hat keinen fail-fast-Check im genossi_bin (RESEARCH-A4): Plan 05 Service-Layer hat einen Default `'http://localhost:3000/'` via `std::env::var(...).unwrap_or_else(...)`. Der oidc-Build hat einen anderen Pfad mit `expect()`, der bereits striktes fail-fast leistet. Im mock_auth-Build (Tests) ist Default ausreichend."

patterns-established:
  - "Pro-Endpoint-Rate-Limit via separater GovernorLayer (auth/api/join/redeem haben jeweils eigene Configs) — Pattern für zukünftige Public-Endpoints kopierbar"
  - "Multi-fold Cascade-Probe-Adapter: Eine Phase implementiert das Probe-Trait; die nächste Phase wired den Production-Adapter; mock_auth-E2E-Tests können End-to-End-Cascade beobachten ohne reale OIDC-Setup"
  - "Set-Cookie auf Public-Endpoint via raw HeaderValue::from_str — bypassed den CookieManagerLayer, weil Public-Endpoints nicht durch ihn laufen"

requirements-completed: [HLPR-01, HLPR-02, HLPR-06]

duration: ~30min
completed: 2026-05-03
---

# Phase 2 Plan 07: Helper Token REST + DI Wiring Summary

**Vier Axum-Handler (3 Vorstand admin + 1 Public mit Set-Cookie und Pro-IP-Rate-Limit), zwei neue RestError-Varianten (403/410) für die D-24-Differenzierung, vollständiges DI-Wiring in genossi_bin mit DbAssemblyStatusProbe für HLPR-05-Cascade in mock_auth-Builds — proven by 4 grünen Validation-Tests im genossi_rest und 189 grünen workspace-tests in mock_auth + oidc.**

## Performance

- **Duration:** ~30 min
- **Tasks:** 3 (alle ohne RED-fail Refactor-Pass; Tasks 1+2 zwischen abhängig wegen RestError-Varianten)
- **Files created:** 1 (`genossi_rest/src/helper_token.rs`)
- **Files modified:** 3 (`genossi_rest/src/lib.rs`, `genossi_rest/src/test_server.rs`, `genossi_bin/src/lib.rs`)
- **Tests added:** 4 (Validation-Tests in `helper_token::tests`)

## Endpoints + Status-Code-Mapping (D-24)

| Method | Path | Auth | Body | Erfolg | D-24 Mapping (Fehler) |
|--------|------|------|------|--------|------------------------|
| `POST` | `/api/assembly/{assembly_id}/helper-tokens` | admin | `CreateHelperTokenRequest` | 201 + `HelperTokenCreateResponseTO` (one-time qr_svg + code) | 401 unauth, 404 assembly missing, 409 status conflict, 422 validation |
| `GET` | `/api/assembly/{assembly_id}/helper-tokens` | admin | — | 200 + `[HelperTokenTO]` | 401 unauth |
| `POST` | `/api/assembly/{assembly_id}/helper-tokens/{token_id}/revoke` | admin | — | 200 + `HelperTokenTO` | 401 unauth, 404 not-found, 409 status conflict (used/closed) |
| `POST` | `/api/helper/redeem` | **public** | `RedeemRequest` | 200 + `RedeemResponse` + `Set-Cookie: app_session=...` | **400** (`ValidationError`), **404** (`EntityNotFound`), **410** (`Conflict("already_used")`), **403** (`Conflict("revoked"\|"assembly_not_open")`), **429** (rate limit) |

## ServiceError -> HTTP Mapping im Redeem-Handler (D-24, exakt)

```
ServiceError::ValidationError(_)               -> 400 BadRequest("invalid_code_format")
ServiceError::EntityNotFound(_)                -> 404 NotFound
ServiceError::Conflict(Arc::from("already_used"))      -> 410 Gone("already_used")
ServiceError::Conflict(Arc::from("revoked"))           -> 403 Forbidden("revoked")
ServiceError::Conflict(Arc::from("assembly_not_open")) -> 403 Forbidden("assembly_not_open")
ServiceError::Conflict(other)                  -> 409 Conflict(other)  // Fallback
ServiceError::* (any other)                    -> via standard From-Impl (PermissionDenied -> 401, etc.)
```

## DI-Wiring-Diff (genossi_bin/src/lib.rs)

### Neue Type-Aliases + Deps

```rust
type HelperTokenDao = genossi_dao_impl_sqlite::helper_token::HelperTokenDaoImpl;

pub struct HelperTokenServiceDependencies;
unsafe impl Send for HelperTokenServiceDependencies {}
unsafe impl Sync for HelperTokenServiceDependencies {}

impl genossi_service_impl::helper_token::HelperTokenServiceDeps for HelperTokenServiceDependencies {
    type Context = Context;
    type Transaction = Transaction;
    type HelperTokenDao = HelperTokenDao;
    type AssemblyDao = AssemblyDao;
    type AuditLogDao = AuditLogDao;
    type PermissionService = PermissionService;
    type PermissionDao = PermissionDao;
    type SessionService = SessionService;
    type UuidService = UuidService;
    type TransactionDao = TransactionDao;
}

type HelperTokenService =
    genossi_service_impl::helper_token::HelperTokenServiceImpl<HelperTokenServiceDependencies>;
```

### DbAssemblyStatusProbe (mock_auth-only)

```rust
#[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
struct DbAssemblyStatusProbe {
    assembly_dao: Arc<AssemblyDao>,
    transaction_dao: Arc<TransactionDao>,
}

#[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
#[async_trait::async_trait]
impl genossi_service_impl::session::AssemblyStatusProbe for DbAssemblyStatusProbe {
    async fn is_open(&self, assembly_id: uuid::Uuid) -> bool {
        // Best-effort: errors and missing assemblies → "not open" (D-18 cascade-safe).
        let Ok(tx) = self.transaction_dao.use_transaction(None).await else { return false; };
        let result = self.assembly_dao.find_by_id(assembly_id, tx).await;
        matches!(result, Ok(Some(a)) if a.status == genossi_dao::assembly::AssemblyStatus::Open)
    }
}
```

### RestStateImpl::new — wichtige Änderungen

- `assembly_dao` wird VOR `session_service` konstruiert (Plan-06-Konsequenz)
- `session_service` (mock_auth-Pfad) wird via `MockSessionServiceImpl::with_probe(probe)` statt `::default()` konstruiert
- `helper_token_service` mit allen 8 Deps konstruiert
- Self-Block-Field `helper_token_service` ergänzt
- `impl genossi_rest::helper_token::HelperTokenRestState for RestStateImpl` am Datei-Ende

### SessionService-Deps-Vergleich

| Build | Vorher (vor Plan 06) | Plan 06 | Plan 07 |
|-------|----------------------|---------|---------|
| `mock_auth` | `MockSessionServiceImpl` (Unit) | `MockSessionServiceImpl::default()` (Field-Struct) | `MockSessionServiceImpl::with_probe(DbAssemblyStatusProbe)` |
| `oidc` | `SessionServiceImpl { permission_dao }` | `+ assembly_dao + transaction_dao` (impl SessionServiceDeps erweitert) | unchanged seit Plan 06 |

## redeem_rate_layer Konfiguration

```rust
let redeem_rate_config = Arc::new(
    GovernorConfigBuilder::default()
        .per_second(6)        // 60s window / 6s per request = 10/min steady-state
        .burst_size(10)       // allow short bursts for re-tries
        .finish()
        .unwrap(),
);
let redeem_rate_layer = GovernorLayer { config: redeem_rate_config };
// ...
let helper_redeem_router =
    helper_token::generate_public_route::<RestState>().layer(redeem_rate_layer);
let app = app
    .nest("/api/public", join_router)
    .nest("/api/helper", helper_redeem_router)
    .with_state(rest_state.clone());
```

**Begründung:** RESEARCH Pitfall 7 — Brute-Force-Schutz auf dem Public-Redeem-Endpoint. 10/min steady-state mit burst 10 erlaubt legitime Retries (z.B. Tipp-Fehler im Code) bei gleichzeitigem Schutz vor Code-Enumeration.

## APP_URL-Default-Verhalten (RESEARCH-A4)

Plan 05 Service-Layer (`HelperTokenServiceImpl::create_helper_token`) liest `APP_URL` via `std::env::var("APP_URL").unwrap_or_else(|_| "http://localhost:3000/".into())` — kein fail-fast.

Plan 07 fügt **keinen** zusätzlichen Check in `genossi_bin` hinzu. Begründung:
- Der oidc-Build hat in `genossi_rest/src/lib.rs::oidc_config()` bereits `std::env::var("APP_URL").expect(...)` — strikter fail-fast für die Production-Auth-Setup-Umgebung
- Der mock_auth-Build (Tests) profitiert vom Default (kein Setup-Boilerplate für lokale Test-Server)
- CONTEXT D-12 sagt "Plan-Detail" und Plan 05 hat den pragmatischen Default-Pfad gewählt — das ist bewusst getragen

## Task Commits

1. **Task 1** (feat): `7d32092` — `feat(02-07): add helper_token REST handlers + ApiDocs (Task 1)` — Datei-Erstellung mit 4 Handlern + Validation-Tests; standalone-Build fehlerhaft (RestError::Gone/Forbidden noch nicht da)
2. **Task 2** (feat): `678d272` — `feat(02-07): wire helper_token module + RestError 403/410 + redeem rate-limit` — pub mod + RestError-Varianten + error_handler + ApiDoc-Nest + create_app/start_server type-bound + redeem_rate_layer + Router-nest. Build grün.
3. **Task 3** (feat): `d02a702` — `feat(02-07): wire HelperTokenService DI + DbAssemblyStatusProbe (Task 3)` — Type-Aliases, HelperTokenServiceImpl-Konstruktion mit 8 Deps, DbAssemblyStatusProbe für mock_auth, RestStateImpl-Field + Trait-Impl. Workspace-Build + Tests grün in beiden Feature-Builds.
4. **Style** (style): `9e2091e` — `style(02-07): apply rustfmt to plan 07 files` — kein Verhaltensänderung.

Plan-metadata commit (this SUMMARY) wird separat am Ende erzeugt.

## Files Created/Modified

### `genossi_rest/src/helper_token.rs` (created, ~440 Zeilen)
- `HelperTokenRestState`-Trait
- `validate_create_helper_token_request` (memo: required + max 256 Unicode-chars)
- 4 Handler: `create_helper_token` (201), `list_helper_tokens` (200), `revoke_helper_token` (200), `redeem_helper_token` (200 + Set-Cookie)
- `generate_route` (3 admin) + `generate_public_route` (1 public)
- `ApiDoc` (3 admin paths + 4 schemas) + `PublicApiDoc` (1 path + 2 schemas)
- 4 Validation-Tests in `mod tests`

### `genossi_rest/src/lib.rs` (modified)
- `pub mod helper_token;` (alphabetisch zwischen `dev` und `http_util`)
- `RestError::Forbidden(String)` + `RestError::Gone(String)` Varianten
- `error_handler`-Match um zwei neue Arms erweitert (403 + 410)
- ApiDoc-Nest-Liste um `/api/assembly/{assembly_id}/helper-tokens` erweitert
- PublicApiDoc-Merge: `helper_token::PublicApiDoc` nach `application::PublicApiDoc`
- `create_app` + `start_server` type-bound um `helper_token::HelperTokenRestState`
- `redeem_rate_config` + `redeem_rate_layer` analog zum `join_rate_layer`-Pattern
- Router-Build: `.nest("/api/assembly/{assembly_id}/helper-tokens", helper_token::generate_route::<RestState>())`
- Public-Block: `.nest("/api/helper", helper_redeem_router)` mit `redeem_rate_layer`

### `genossi_rest/src/test_server.rs` (modified)
- `start_test_server` trait-bound um `crate::helper_token::HelperTokenRestState` erweitert

### `genossi_bin/src/lib.rs` (modified)
- `type HelperTokenDao = ...HelperTokenDaoImpl`
- `pub struct HelperTokenServiceDependencies` + `impl HelperTokenServiceDeps`
- `type HelperTokenService = ...`
- `DbAssemblyStatusProbe`-Struct + `impl AssemblyStatusProbe` (mock_auth-only)
- `RestStateImpl.helper_token_service: Arc<HelperTokenService>` field
- `RestStateImpl::new(...)` umgestellt:
  - `assembly_dao` jetzt vor `session_service`
  - `session_service` (mock_auth) via `with_probe(...)` mit `DbAssemblyStatusProbe`
  - `helper_token_service` mit 8 Deps konstruiert
  - Self-Block-Field hinzugefügt
- `impl genossi_rest::helper_token::HelperTokenRestState for RestStateImpl` am Ende

## Verification Results

| Check | Command | Result |
|-------|---------|--------|
| genossi_rest mock_auth build | `cargo build -p genossi_rest` | exit 0 |
| genossi_rest helper_token tests | `cargo test -p genossi_rest --lib helper_token` | 4/4 passed |
| genossi_rest full lib tests | `cargo test -p genossi_rest --lib` | 44/44 passed |
| genossi_bin mock_auth build | `cargo build -p genossi_bin --features mock_auth` | exit 0 |
| genossi_bin oidc build | `cargo build -p genossi_bin --no-default-features --features oidc` | exit 0 |
| Workspace mock_auth tests | `cargo test --workspace --features mock_auth --lib` | 189 passed, 0 failed, 2 ignored |
| Workspace oidc tests | `cargo test --workspace --no-default-features --features oidc --lib` | 189 passed, 0 failed, 2 ignored |
| Workspace build | `cargo build --workspace` | exit 0 |
| rustfmt | `rustfmt --check --edition 2021 helper_token.rs lib.rs lib.rs` | exit 0 nach Cleanup-Commit |
| clippy (rust-1.90 toolchain) | `cargo clippy -p genossi_rest -p genossi_bin --all-targets` | exit 0; nur 2 pre-existing warnings (Auditable-import + clone_on_copy in e2e_tests) — KEINE neuen Warnings durch Plan 07 |

`SQLX_OFFLINE=true` ist in dieser Worktree-Umgebung erforderlich (pre-existing condition seit Plan 02-01; siehe Plan-04/05/06 SUMMARY).

## Acceptance Criteria Per Task

### Task 1 (REST handlers + ApiDocs)
- ✓ `pub trait HelperTokenRestState` (1)
- ✓ `pub async fn create_helper_token` (1)
- ✓ `pub async fn list_helper_tokens` (1)
- ✓ `pub async fn revoke_helper_token` (1)
- ✓ `pub async fn redeem_helper_token` (1)
- ✓ `.status(201)` >= 1
- ✓ `.status(200)` >= 3
- ✓ `tag = "Helper Tokens"` >= 3
- ✓ `tag = "Helper Redeem"` >= 1
- ✓ `extract_auth_context` == 3 (3 admin handlers; redeem-handler hat KEINEN call)
- ✓ `SET_COOKIE` >= 1
- ✓ `HttpOnly` >= 1
- ✓ `SameSite=Strict` >= 1
- ✓ `Max-Age=86400` >= 1
- ✓ `RestError::Gone` >= 1
- ✓ `RestError::Forbidden` >= 1
- ✓ `already_used` >= 1
- ✓ `revoked\|assembly_not_open` >= 1
- ✓ `pub fn generate_route` (1)
- ✓ `pub fn generate_public_route` (1)
- ✓ `pub struct ApiDoc` (1)
- ✓ `pub struct PublicApiDoc` (1)
- ✓ 4 Validation-Tests grün

### Task 2 (lib.rs RestError + Module-Wiring + Router-Nest)
- ✓ `cargo build -p genossi_rest` exit 0
- ✓ `cargo test -p genossi_rest` 44/44 passed
- ✓ `pub mod helper_token;` (1)
- ✓ `Forbidden(String)` >= 1
- ✓ `Gone(String)` >= 1
- ✓ `RestError::Forbidden` >= 1
- ✓ `RestError::Gone` >= 1
- ✓ `.status(403)` >= 1
- ✓ `.status(410)` >= 1
- ✓ `/api/assembly/{assembly_id}/helper-tokens` >= 1
- ✓ `helper_token::HelperTokenRestState` >= 2 (create_app + start_server)
- ✓ `helper_token::PublicApiDoc` >= 1
- ✓ `redeem_rate_config\|redeem_rate_layer` >= 2
- ✓ `"/api/helper"` >= 1
- ✓ `helper_token::generate_route\|helper_token::generate_public_route` >= 2

### Task 3 (DI-Wiring in genossi_bin)
- ✓ `cargo build -p genossi_bin --features mock_auth` exit 0
- ✓ `cargo build -p genossi_bin --no-default-features --features oidc` exit 0
- ✓ `cargo build --workspace` exit 0
- ✓ `type HelperTokenDao = ...HelperTokenDaoImpl` (1)
- ✓ `pub struct HelperTokenServiceDependencies` (1)
- ✓ `impl ...HelperTokenServiceDeps for HelperTokenServiceDependencies` (1)
- ✓ `helper_token_service: Arc<HelperTokenService>` >= 1
- ✓ `let helper_token_dao = Arc::new` (1)
- ✓ `HelperTokenServiceImpl {` >= 1
- ✓ `impl genossi_rest::helper_token::HelperTokenRestState for RestStateImpl` (1)
- ✓ `fn helper_token_service` >= 1
- ✓ `type AssemblyDao = AssemblyDao` >= 2 (3 Vorkommen: AssemblyServiceDeps, HelperTokenServiceDeps, SessionServiceDeps)
- ✓ oidc-Build: SessionServiceDeps hat `type AssemblyDao` + `type TransactionDao` (count = 2)
- ✓ mock_auth: `DbAssemblyStatusProbe` + `with_probe` (count = 8 — Struct, impl, Konstruktion, Comments, Plan-Konsequenz-Hinweise)
- ✓ mock_auth: `AssemblyStatusProbe for DbAssemblyStatusProbe` (1)
- ✓ Workspace-Tests grün in beiden Feature-Builds

## Decisions Made

1. **ServiceError-Mapping bleibt im Handler-Body** statt globaler `From<ServiceError> for RestError`-Erweiterung. Begründung: Nur der redeem-Handler braucht die Conflict-Discriminator-Differenzierung; alle anderen Handler-Pfade in der Codebase verwenden die Standard-Mapping (Conflict -> 409). Eine Änderung der globalen From-Impl würde die Bedeutung des `Conflict`-Payloads für ALLE Handler verschieben — das wollten wir nicht. Plan 04+05 hatten die stable strings explizit als Discriminator-Codes für die REST-Layer-spezifische Mapping-Verantwortung dokumentiert.

2. **DbAssemblyStatusProbe ist mock_auth-only** (nicht im oidc-Build). Begründung: Der oidc-Build verwendet `SessionServiceImpl` mit eigenem AssemblyDao + TransactionDao-Wiring (Plan 06); die Probe wird nur gebraucht, um `MockSessionServiceImpl::with_probe(...)` zu füttern — was im oidc-Build nicht existiert. `#[cfg(all(feature = "mock_auth", not(feature = "oidc")))]` hält das sauber.

3. **`assembly_dao` wird vor `session_service` konstruiert.** Begründung: sowohl der oidc-SessionServiceImpl als auch die mock_auth-DbAssemblyStatusProbe brauchen die DAO. Eine Instanz, mehrfach geklont über Arc — DAOs sind stateless wrappers über den SqlitePool.

4. **APP_URL-Check NICHT im genossi_bin.** Begründung: Der oidc-Build hat bereits einen strikten `expect()`-Pfad in `genossi_rest::oidc_config()`. Der mock_auth-Build profitiert vom Service-Layer-Default ("http://localhost:3000/"). CONTEXT D-12 erlaubt diese Plan-Detail-Entscheidung.

5. **RestError::Gone und RestError::Forbidden sind generic-HTTP-Status-Varianten** (nicht `HelperRedeemUsed` o.ä.). Begründung: Konsistenz mit den existierenden `NotFound`/`BadRequest`/`Conflict`-Varianten — einer pro HTTP-Status-Code. Die Body-Strings tragen die Discriminator-Information für API-Konsumenten.

## Deviations from Plan

**1. [Style] HelperTokenServiceImpl-Deps-Reihenfolge nach Plan-05-Output korrigiert**
- **Found during:** Task 3 setup
- **Issue:** Plan-Action listet 7 Deps für die HelperTokenServiceImpl-Konstruktion, aber Plan 05 (gemerged) hat 8 Deps (zusätzlich `permission_dao` zwischen `permission_service` und `session_service`).
- **Fix:** HelperTokenServiceDeps-Impl + Konstruktion mit allen 8 Deps verkabelt: `helper_token_dao`, `assembly_dao`, `audit_log_dao`, `permission_service`, `permission_dao`, `session_service`, `uuid_service`, `transaction_dao` — exakt wie das `gen_service_impl!` in `genossi_service_impl/src/helper_token.rs:138-148` deklariert.
- **Files modified:** `genossi_bin/src/lib.rs`
- **Commit:** `d02a702`

**2. [Rule 3 - Blocker] genossi_rest/src/test_server.rs braucht trait-bound-Erweiterung**
- **Found during:** Task 2 verification
- **Issue:** `start_test_server` hat dieselben trait-bounds wie `create_app` und musste analog erweitert werden um `helper_token::HelperTokenRestState`. Plan-Action hat das nicht explizit erwähnt.
- **Fix:** trait-bound-Liste in test_server.rs ergänzt; Build wieder grün.
- **Files modified:** `genossi_rest/src/test_server.rs`
- **Commit:** `678d272`

**3. [Style] rustfmt-Differenzen nach manueller Edit**
- **Found during:** post-Task-3 verification
- **Issue:** rustfmt-Diff: `.route("/{token_id}/revoke", post(...))` als single-line statt 4-line; `as Arc<dyn ...>` cast auf eigene Zeile.
- **Fix:** `rustfmt --edition 2021` über die drei geänderten Dateien laufen lassen, alle Tests bleiben grün.
- **Files modified:** `genossi_rest/src/helper_token.rs`, `genossi_bin/src/lib.rs`
- **Commit:** `9e2091e`

## Issues Encountered

- **`type AssemblyDao = AssemblyDao` count = 3 statt 2**: das dritte Vorkommen ist die SessionServiceDeps-Erweiterung aus Plan 06 — daher hat das acceptance-criterion ">= 2" auch 3 erfüllt. Kein Bug.

- **Acceptance-criterion `DbAssemblyStatusProbe\|MockSessionServiceImpl::with_probe` = 8** (statt erwartete 2): Doppelter Match wegen Comments + Plan-Konsequenz-Hinweisen + tatsächlicher Verwendung. Mehr-als-2 erfüllt das ">= 2".

- **Hauptrepo-Toolchain ist 1.89, clippy braucht 1.90**: Workaround via `PATH="/nix/store/97dzaavh6cz7h27y3hcldm42933a2p6f-rust-default-1.90.0/bin:$PATH"`. Memory-Lektion `feedback_nix_toolchain` angewendet (rustfmt/clippy nicht sofort aufgeben → /nix/store durchsuchen).

- **`SQLX_OFFLINE=true` für Workspace-Builds erforderlich**: pre-existing environment property der worktree, gleiche Bedingung wie Plan 02-04/05/06.

## User Setup Required

Keine.

## Hint for Plan 08 (E2E-Tests)

- Server hat alle 4 Helper-Token-Endpoints registriert. `start_test_server` baut den vollen Router via `create_app`, inkl. der mock_auth-DbAssemblyStatusProbe.
- Bei der HLPR-05-Cascade-Test (assembly closed -> helper-cookie invalid):
  1. Test öffnet Assembly via `POST /api/assembly/{id}/open` (Vorstand-Endpoint, Phase-1)
  2. Test ruft `POST /api/helper/redeem` mit dem Code → erhält `app_session=...` Cookie
  3. Test ruft Helper-Endpoint → 200 (cascade noch aktiv weil assembly Open)
  4. Test ruft `POST /api/assembly/{id}/close` → Assembly status = Closed
  5. Test ruft Helper-Endpoint mit gleichem Cookie → 401 Unauthorized (cascade fired via DbAssemblyStatusProbe)
- Helper-Cookie-Format für mock_auth E2E ist NICHT `helper:<assembly_uuid>:<token_id>` — sondern direkt der `app_session=<session_id>`-Wert, den der Server im Set-Cookie-Header schickt. Plan 06 hat die `helper:<uuid>:<tok>`-Convention für *manuell konstruierte* Test-Cookies geschaffen; Plan 07 setzt im Production-Pfad `app_session=<UUID>` und Plan-Detail welche Session-IDs MockSessionServiceImpl im mock_auth-Build erkennt liegt bei Plan 08.
- Rate-Limit auf `/api/helper/redeem` ist 6 req/sec mit burst 10. E2E-Tests, die viele Redeems hintereinander ausführen, müssen ggf. zwischen-pausieren oder unterschiedliche source-IPs nachstellen.
- Differential ServiceError-Mapping ist im Handler-Body. Tests können also via API-Response-Status verifizieren, dass:
  - Falscher Code (z.B. "ABC") → 400
  - Unbekannter aber gültig-formatierter Code → 404
  - Bereits redeemed → 410
  - Revoked → 403
  - Assembly closed nach Token-Erzeugung → 403

## Threat Surface Scan

Kein neues Threat-Surface eingeführt. Plan 07 implementiert die Mitigations aus dem Plan-Threat-Model:

- **T-02-07-01 (Spoofing / forged session-cookie via XSS):** mitigate ✓ — `HttpOnly`, `Secure`, `SameSite=Strict` Set-Cookie-Attribute verifiziert per code-grep (Max-Age=86400 ergänzt für 24h-Lifetime D-18).
- **T-02-07-02 (DoS / Brute-Force):** mitigate ✓ — `redeem_rate_layer` mit 10/min/IP konfiguriert + im Public-Block mit `.layer(redeem_rate_layer)` aktiviert.
- **T-02-07-03 (Info-Disclosure / RestError-Body leakt internals):** mitigate ✓ — Body-Strings sind stable error-codes (`"already_used"`, `"revoked"`, `"assembly_not_open"`, `"invalid_code_format"`); kein internal-State-Leak.
- **T-02-07-04 (Tampering / Helfer-Cookie für Vorstand-Endpoints):** mitigate ✓ — Vorstand-Endpoints rufen `extract_auth_context` + Service-Layer-Permission-Check; Helper-Sessions liefern PermissionDenied (D-20-Stub aus Plan 02; im oidc-Build gilt das `Context = AuthenticatedContext` und Helper-Sessions kommen NICHT durch die OIDC-Auth-Pipeline).
- **T-02-07-05 (DoS / massive QR-SVG-Generierung):** accept ✓ — Vorstand-Pfad ist admin-protected, kein public-DoS-Risiko.
- **T-02-07-06 (EoP / Public-Redeem bypassed Vorstand-Permissions):** mitigate ✓ — `redeem_helper_token`-Service (Plan 05) gibt nur `HelperRedeemSuccess` zurück (eine GV-spezifische Helper-Session); keine admin-Privilegien werden vergeben.

## Threat Flags

Keine — alle neu eingeführten Surfaces sind im plan-internen `<threat_model>` aufgeführt und entsprechend mitigiert.

## Next Phase Readiness

**Ready for Plan 08 (E2E-Tests):**
- Alle 4 Endpoints laufen via `create_app` und sind durch `start_test_server` ansprechbar
- DI-Kette ist komplett: `HelperTokenServiceImpl` mit 8 Deps, MockSessionServiceImpl mit DbAssemblyStatusProbe
- HLPR-05 Cascade ist über die DbAssemblyStatusProbe in mock_auth-Builds beobachtbar
- ServiceError-Discriminator-Strings stable und im Handler-Body gemappt
- redeem_rate_layer ist aktiv — Plan 08 muss eventuell rate-burst-Tests planen

**No blockers.** All workspace-tests grün in beiden Feature-Builds; Plan 02-04/05/06 bleibt unverändert; Phase-1-E2E-Tests laufen weiter durch (pre-existing 178-test-Korpus + Phase-2-Plan-04/05/06-Tests = 189 Tests).

## TDD Gate Compliance

Plan-Frontmatter hat `type=execute`, also keine Plan-Level-TDD-Gate. Tasks 1-3 waren `tdd="true"` deklariert; tatsächlich verlief der Workflow als RED-via-cross-task-dependency:

- **Task 1** (feat-only commit): Datei mit Validation-Tests + Handler-Skeleton; standalone-Build fehlerhaft (RestError::Gone/Forbidden noch nicht da) — implizit RED-Phase auf Crate-Level.
- **Task 2** (feat): RestError-Varianten + Module-Wiring → Build grün → 4 Validation-Tests grün — GREEN-Phase.
- **Task 3** (feat): DI-Wiring → Workspace-Build grün → 189 lib-tests grün — GREEN auf Workspace-Level.
- **Style** (style): rustfmt cleanup, kein Verhaltensänderung.

Diese Cross-Task-RED→GREEN-Form ist im Plan explizit vorgesehen ("Plan-07-Task-2 muss vorher die RestError-Varianten Gone+Forbidden ergänzen — Build wird nach Task 2 grün") und nicht als Deviation zu werten.

## Self-Check: PASSED

- [x] All 3 task commits exist in git (`7d32092`, `678d272`, `d02a702`) + style cleanup (`9e2091e`)
- [x] Created file present on disk: `genossi_rest/src/helper_token.rs`
- [x] Modified files updated: `genossi_rest/src/lib.rs`, `genossi_rest/src/test_server.rs`, `genossi_bin/src/lib.rs`
- [x] All 4 helper_token validation tests pass green
- [x] HLPR-01 + HLPR-02 + HLPR-06 satisfied (REST + DI + Routing)
- [x] D-21 admin-protection on Vorstand-endpoints via extract_auth_context (3 occurrences)
- [x] D-22 public flow on /api/helper/redeem (no extract_auth_context call)
- [x] D-22 + D-18 Set-Cookie attributes (HttpOnly, SameSite=Strict, Secure, Max-Age=86400)
- [x] D-24 differential ServiceError-Mapping in redeem-handler (400/404/410/403)
- [x] Pitfall 7: redeem_rate_layer (~10/min/IP) on /api/helper/redeem
- [x] Plan 06 consequence: DbAssemblyStatusProbe + MockSessionServiceImpl::with_probe wired
- [x] cargo build --workspace exit 0 (mock_auth + oidc, with SQLX_OFFLINE=true)
- [x] cargo test --workspace --lib: 189 passed, 0 failed in beiden Feature-Builds
- [x] rustfmt --check exit 0 nach Cleanup-Commit
- [x] cargo clippy: keine NEUEN Warnings durch Plan 07 (2 pre-existing warnings in genossi_bin)

---
*Phase: 02-helfer-token-session-authcontext-helper*
*Completed: 2026-05-03*
