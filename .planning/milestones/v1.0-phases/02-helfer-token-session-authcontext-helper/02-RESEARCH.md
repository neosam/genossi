# Phase 2: Helfer-Token + Session + AuthContext::Helper - Research

**Researched:** 2026-05-03
**Domain:** Rust/Axum backend — One-Time-Use QR-Token, atomic SQLx UPDATE...RETURNING redeem, typed AuthContext extension, audit-logged helper-token aggregate
**Confidence:** HIGH (alle kritischen Bereiche durch Code-Inspektion + Context7-äquivalente Source-Lookups verifiziert)

## Summary

Phase 2 baut ein neues Aggregat `helper_token` nach exakt demselben Layered-Pattern wie Phase 1 (`assembly`): DAO-Trait + SQLite-Impl, Service mit `gen_service_impl!` und Audit-Macro für Create, Axum-Handler mit `error_handler`-Wrapper, Migration-File. Drei spezifische Neuerungen erweitern die Bestands-Architektur: (1) ein atomares SQL-Statement `UPDATE ... RETURNING ...` für den Race-sicheren Redeem-Pfad; (2) eine neue typsichere `AuthContext::Helper`-Variante, die in `extract_auth_context` über JSON-Claims-Parsing rekonstruiert wird; (3) zwei neue externe Crates (`qrcode` 0.14.1 und ein Crockford-Base32-Mechanismus). Alle Patterns für DAO/Service/REST/E2E-Test sind bereits in der Codebase etabliert und kopierbar.

**Primary recommendation:** Folge dem Phase-1-`assembly`-Aggregat als Strukturtemplate 1:1. Die einzigen drei Stellen, an denen Phase 2 vom Template abweicht, sind: (a) atomarer Redeem-SQL als `query_as::<_, RedeemRow>` mit `fetch_optional` (Workaround für bekannten SQLx-RETURNING-NULL-Bug), (b) `helper_token`-spezifischer SHA256-Token-Hash (`sha2` 0.10 ist bereits im Stack), (c) `AuthContext::Helper`-Match-Branch in jedem Permission-Service-Aufruf-Handler (Compiler enforced, Phase 2 stubbt mit `Err(PermissionDenied)`).

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Token-Schema & Lifecycle**
- **D-01:** Tabelle `helper_token`. Felder: `id` (BLOB UUID), `assembly_id` (FK → `assembly.id`, RESTRICT), `memo` (TEXT), `token_hash` (TEXT, SHA256(code) hex/base64), `created`, `used_at` (Option), `session_id` (Option<TEXT>, FK → `session.id` ON DELETE SET NULL), `revoked_at` (Option), `deleted` (Option), `version` (UUID).
- **D-02:** Status `Open`/`Used`/`Revoked` abgeleitet aus Spalten — keine Status-Spalte.
- **D-03:** Revoke eines bereits eingelösten Tokens → 409 Conflict. Cascade auf Session entfällt.
- **D-04:** `helper_token` führt `deleted` Slot ohne Delete-Pfad in Phase 2.
- **D-05:** Soft-Delete-Filter: alle Queries `WHERE deleted IS NULL`.
- **D-06:** `Auditable`-Trait: `entity_type() = "helper_token"`, `audit_fields()` ohne `token_hash` (Information-Leakage), inkl. `assembly_id`, `memo`, `revoked_at`.

**Audit-Strategie**
- **D-07:** Nur Token-Erzeugung auditiert; Process-Identifier `"helper_token.create"` mit `audited_create!`.
- **D-08:** Redeem und Revoke werden NICHT auditiert.

**Klartext-Code & QR**
- **D-09:** Fix 10 Zeichen Crockford Base32, Alphabet `0123456789ABCDEFGHJKMNPQRSTVWXYZ`.
- **D-10:** Cryptographically-secure RNG (`rand::rngs::OsRng` o.ä.). TokenGenerator-Service-Form ist Plan-Detail.
- **D-11:** Speicherung: SHA256(code) hex lowercase in `token_hash`. Klartext nur einmalig im Create-Response.
- **D-12:** QR-Inhalt: `${APP_URL}/helper?code=ABC1234567`. APP_URL aus Env, fail-fast-Strategie ist Plan-Detail.
- **D-13:** QR-Crate `qrcode` 0.14, EC-Level `EcLevel::Q`, SVG-String als `qr_svg`-Feld.

**AuthContext::Helper-Wiring & Session**
- **D-14:** `AuthContext::Helper { session_id: Arc<str>, assembly_id: uuid::Uuid }` — keine Feature-Gate.
- **D-15:** Reuse von Cookie `app_session`. Helfer-Session ist `session`-Eintrag mit `claims = JSON({"kind":"helper",...})`.
- **D-16:** Claims-Schema: `{ "kind": "helper", "assembly_id": "<uuid-string>" }`.
- **D-17:** `user_id` synthetisch: `helper:<token_id_uuid>`. Auto-Register via `permission_dao.ensure_user_exists`.
- **D-18:** Long-lived `expires` (24h ab Redeem) + Status-Check beim Verify (`assembly.status == Open`).
- **D-19:** Verdrahtung des Status-Checks (SessionService vs. neuer Wrapper vs. extract_auth_context) ist **Claude's Discretion**.
- **D-20:** `PermissionService::check_permission` mit `AuthContext::Helper` → `Err(PermissionDenied)` (Phase 2 Stub).

**REST-Endpoint-Vertrag**
- **D-21:** `POST/GET /api/assembly/{id}/helper-tokens`, `POST .../helper-tokens/{token_id}/revoke`. Alle erfordern `admin`.
- **D-22:** `POST /api/helper/redeem` öffentlich, ohne Auth-Middleware. Body `{code}`. Erfolg: Set-Cookie `app_session` + JSON.
- **D-23:** Revoke erlaubt in Status `Preparation` UND `Open`. Status `Closed`: 409.
- **D-24:** Redeem-Fehler-Codes: 404 (unknown), 410 (used), 403 (revoked oder Assembly !Open), 400 (Format), 200 (OK).
- **D-25:** Atomarer Redeem: `UPDATE helper_token SET used_at=?, session_id=? WHERE token_hash=? AND used_at IS NULL AND revoked_at IS NULL AND deleted IS NULL RETURNING id, assembly_id`. 0 Rows → differenzierter Status-Lookup.

**Naming**
- **D-26:** Englisch durchgängig: `HelperToken`, `HelperTokenEntity`, `HelperTokenDao`, `HelperTokenService`, `HelperTokenServiceImpl`, `HelperTokenTO`, `HelperTokenCreateResponseTO`. Tabelle `helper_token`.
- **D-27:** Migration-Filename `YYYYMMDDHHMMSS_create_helper_token_table.sql`.

### Claude's Discretion

- Verdrahtungsort des Assembly-Status-Checks (SessionService erweitern vs. Wrapper vs. extract_auth_context) — D-19
- TokenGenerator-Service vs. freie Funktion — D-10
- Hex vs. Base64 für `token_hash`-Encoding — D-11 (Empfehlung in dieser Research: Hex)
- APP_URL-Default-Verhalten (fail-fast Server-Start vs. fail-on-first-Create) — D-12
- Index-Strategie für `helper_token` — vermutlich `(assembly_id)` und UNIQUE auf `(token_hash)`
- `session_id`-FK-ON-DELETE-Verhalten (`SET NULL` vs. `RESTRICT`)
- Pro-IP-Rate-Limiting für `/api/helper/redeem`

### Deferred Ideas (OUT OF SCOPE)

**Phase 3:**
- Cascade-Invalidation Helfer-Sessions in `close_assembly`
- Positive PermissionService-Branch für `AuthContext::Helper`
- `AttendanceMemberTO`-Erzeugung
- Live-Stats-Endpoint `GET /api/assembly/{id}/stats`

**Phase 4:**
- Manual-Code-Eingabe-UI (HLPR-03)
- QR-Scanner-Integration

**Out of Scope für Phase 2:**
- Bulk-QR-Erzeugung (BULK-01/BULK-02 sind v2)
- Audit-Log für Redeem/Revoke (D-08)
- `tower-sessions` 0.14 → 0.15 Upgrade
- Differenzierte `manage_helper_tokens`-Permission

</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| HLPR-01 | Vorstand kann pro Helfer Token mit Memo erzeugen; Backend liefert QR-SVG + 8–12-Zeichen-Klartext-Code | Standard Stack: `qrcode` 0.14.1 + `rand` 0.8 (`OsRng`) + `sha2` 0.10 (bereits im Stack); Code-Beispiele unten zeigen QR-Render und Crockford-Generierung |
| HLPR-02 | Helfer kann Token via Redeem-Endpoint atomar einlösen (`UPDATE ... WHERE used_at IS NULL RETURNING ...`); Session bind an GV | Code-Beispiel `redeem_atomic_pattern` zeigt korrekte SQLx-0.8-Form mit `fetch_optional` (Workaround für RETURNING-NULL-Bug); Session-Erzeugung über bestehendes `ensure_user_and_create_session_with_claims` |
| HLPR-04 | E2E-Race-Test: zwei parallele Redeem-Requests → genau ein Erfolg, einer 410 | Code-Beispiel `race_test_pattern` zeigt `tokio::join!` über zwei `reqwest`-Aufrufe gegen den `start_test_server()`-Setup |
| HLPR-05 | Helfer-Session ungültig nach `close_assembly` | D-18-Empfehlung: Status-Check im erweiterten `verify_user_session` (Discretion D-19); Code-Beispiel `verify_with_assembly_status_check` |
| HLPR-06 | Vorstand sieht Token-Liste mit Memo + Status; offene Token revokebar | Status `Open`/`Used`/`Revoked` abgeleitet im `From<&HelperTokenEntity>` für `HelperTokenTO`; analog zu `AssemblyStatusTO`-Mapping |
| HLPR-07 | Token-Erzeugung in Audit-Hashchain mit Memo, Erzeuger, Timestamp, GV-Bezug | `audited_create!`-Macro vorhanden; Audit-Query-Pfad: `GET /api/audit?entity_type=helper_token` (kein `process`-Filter im `AuditQueryFilter`! — siehe Pitfall 4) |

</phase_requirements>

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| QR-Code-Generierung (SVG-Render) | Service Layer | — | Reine CPU-Operation, gehört in `HelperTokenServiceImpl` neben Klartext-RNG; nicht in REST-Layer (Test-Isolierbarkeit) und nicht in DAO (kein DB-Bezug) |
| Klartext-Code-Generierung (Crockford-Base32, OsRng) | Service Layer | — | Reine Logik; entweder freie Funktion in `genossi_service_impl/src/helper_token.rs` oder per Trait abstrahiert (Plan entscheidet, D-10) |
| SHA256(code) → token_hash | Service Layer | — | Hashing ist Domain-Logik (Token-Speicher-Schema); kommt in den Service-Pfad VOR DAO-Aufruf |
| Atomarer Redeem-UPDATE | DAO Layer | Service | DAO führt das `UPDATE ... RETURNING` via SQLx aus; Service interpretiert 0-row-Result und ruft Differential-Lookup für 404/410/403-Diskriminierung |
| Session-Erzeugung mit Claims | Service Layer | DAO | Reuse von `SessionServiceImpl::ensure_user_and_create_session_with_claims` (bereits vorhanden) |
| Cookie Set-Cookie-Header | REST Layer | — | Axum-Handler setzt `Set-Cookie: app_session=<id>; Path=/; HttpOnly; SameSite=Lax` |
| `AuthContext::Helper`-Rekonstruktion aus Claims | Service Layer (`SessionService::extract_auth_context`) | REST (auth-middleware ruft sie auf) | JSON-Parse der `claims`-Spalte mit `serde_json` und Discriminator `kind`-Feld |
| Permission-Check-Stub für Helper | Service Layer (`PermissionServiceImpl::check_permission`) | — | Match-Arm `AuthContext::Helper { .. }` → `Err(PermissionDenied)` (Phase 2 Stub, Phase 3 erweitert) |
| Redeem-Endpoint Rate-Limit | REST Layer (Tower middleware) | — | Pro-IP via `tower_governor::GovernorLayer` analog zu bestehenden `/authenticate` und `/api/public/join` |
| Migration-Schema | DAO Layer | — | `migrations/sqlite/<ts>_create_helper_token_table.sql` mit FK auf `assembly` und `session` |

## Standard Stack

### Core (bereits im Stack — KEINE neuen Deps)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `axum` | 0.8.3 | REST handlers, Set-Cookie, Json extraction | [VERIFIED: codebase Cargo.toml] — bereits Stack |
| `sqlx` | 0.8 | `query_as::<_, RedeemRow>(...).fetch_optional(...)` mit RETURNING | [VERIFIED: codebase Cargo.toml] |
| `tokio` | 1.35+ | async runtime; `tokio::join!` für Race-Test | [VERIFIED] |
| `uuid` | 1.6 | Token-IDs als BLOB | [VERIFIED] |
| `time` | 0.3 | `PrimitiveDateTime` für `created`/`used_at`/`revoked_at` | [VERIFIED] |
| `sha2` | 0.10 | SHA256(code) → token_hash | [VERIFIED: bereits in Stack für Audit-Hashchain] |
| `serde` / `serde_json` | 1.0 | Claims-JSON-Parsing für `kind: "helper"` | [VERIFIED] |
| `tower-sessions` | 0.14 | Session-Cookie-Layer (bestehend, Reuse) | [VERIFIED: codebase Cargo.toml:41] |
| `tower_governor` | 0.6 | Pro-IP-Rate-Limiting für Redeem-Endpoint | [VERIFIED: codebase genossi_rest/src/lib.rs:448] |
| `tracing` | 0.1 | `#[instrument]` auf Handlern | [VERIFIED] |
| `utoipa` | 5.0 | OpenAPI-Schemas für `HelperTokenTO` | [VERIFIED] |
| `mockall` | 0.13 | DAO-Mocks im Service-Test | [VERIFIED] |

### Supporting (NEUE Crates — müssen hinzugefügt werden)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `qrcode` | 0.14.1 | SVG-Render via `code.render::<svg::Color>().build()` | Genau einmal im `create_helper_token`-Service — `EcLevel::Q` für gedruckte Codes |
| `rand` | 0.8 (workspace already pulls 0.8 + 0.9 transitively) | `OsRng` für 10-Zeichen-Crockford-Klartext | Nur in Token-Erzeugung; **nicht** für UUIDs (dafür `UuidService`) |

**Hinweis zu `rand`:** Cargo.lock zeigt **rand 0.8.5 und 0.9.4 transitiv** [VERIFIED: Cargo.lock]. Die Version, die direkt deklariert wird, sollte 0.8 sein (Konsistenz mit den Crates, die sie schon nutzen — `ring`/`uuid`); sonst zieht Phase 2 unnötig 0.9-Duplikat in den direkten Workspace-Graph.

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `qrcode` 0.14 | `qrcode-generator` oder `qrcodegen` | `qrcode` ist [VERIFIED: crates.io] de-facto-Standard, hat dedizierten `render::svg`-Modul, MIT/Apache. Alternativen sind kleiner aber bringen weniger SVG-Komfort. **Bleibe bei D-13.** |
| `base32` 0.5.1 crate | Hand-rolled Crockford-Encoder | `base32`-Crate [CITED: crates.io/crates/base32] unterstützt Crockford-Alphabet. ABER: Für 10 zufällige Zeichen aus einem fixen 32-Zeichen-Alphabet ist eine Hand-Roll-Implementierung trivial (~10 Zeilen) und vermeidet eine zusätzliche Dep. **Empfehlung: hand-rolled** — siehe Code Examples unten. |
| `crockford` crate | base32 mit Crockford-Alphabet | `crockford` [CITED: crates.io/crates/crockford] zielt auf u64→String — für Random-Strings nicht ideal. |

**Installation:**
```bash
# In Cargo.toml [workspace.dependencies] hinzufügen:
qrcode = "0.14"
rand = { version = "0.8", default-features = false, features = ["std", "std_rng", "getrandom"] }
# Dann in genossi_service_impl/Cargo.toml:
qrcode = { workspace = true }
rand = { workspace = true }
```

**Version verification:**
```bash
# Manuell zu prüfen vor Plan-Schreibung:
cargo search qrcode --limit 1   # Erwartet: 0.14.1
cargo search base32 --limit 1   # Erwartet: 0.5.1 (falls doch verwendet)
cargo search rand --limit 1     # Erwartet: 0.8.5 (LTS) oder 0.9.x
```
[VERIFIED: webfetch docs.rs/qrcode/0.14.1 — current 0.14.1, MIT/Apache, `code.render::<svg::Color>().build()` API]
[VERIFIED: webfetch docs.rs/base32 — current 0.5.1]

## Architecture Patterns

### System Architecture Diagram

```text
┌──────────────────────────────────────────────────────────────────┐
│ Vorstand (Browser, OIDC-Session)                                │
│   ↓ POST /api/assembly/{aid}/helper-tokens {memo:"Anna"}        │
└──────────────────────────────────────────────────────────────────┘
         ↓
┌──────────────────────────────────────────────────────────────────┐
│ REST Layer (genossi_rest/src/helper_token.rs — NEU)              │
│   • require_admin middleware                                     │
│   • extract_auth_context(context)                                │
│   • Json<CreateHelperTokenRequest>                               │
└──────────────────────────────────────────────────────────────────┘
         ↓
┌──────────────────────────────────────────────────────────────────┐
│ Service Layer (genossi_service_impl/src/helper_token.rs — NEU)   │
│   create_helper_token(assembly_id, memo, ctx):                   │
│     • check_permission("admin", ctx)                             │
│     • assembly_dao.find_by_id(...)? must exist & not Closed     │
│     • generate_crockford_code(10) via OsRng       ─→ "ABC1234567"│
│     • token_hash = sha256_hex(code)                              │
│     • build HelperTokenEntity                                    │
│     • audited_create!(self, helper_token_dao, ...) [process=     │
│           "helper_token.create"]                                 │
│     • qr_svg = render_qr_svg(format!("{APP_URL}/helper?code={}", │
│                                       code))                     │
│     • return (entity, code, qr_svg)                              │
└──────────────────────────────────────────────────────────────────┘
         ↓ INSERT helper_token + audit_log entries (one tx)
┌──────────────────────────────────────────────────────────────────┐
│ DAO Layer (helper_token table, FK→assembly + FK→session)         │
└──────────────────────────────────────────────────────────────────┘

╔════════════ REDEEM PATH (öffentlich, kein Auth) ════════════════╗
┌──────────────────────────────────────────────────────────────────┐
│ Helfer (Browser nach QR-Scan oder Manual-Code)                   │
│   ↓ POST /api/helper/redeem  {code: "ABC1234567"}                │
└──────────────────────────────────────────────────────────────────┘
         ↓ tower_governor (per-IP rate-limit, Discretion)
┌──────────────────────────────────────────────────────────────────┐
│ REST Handler (öffentlich, ohne require_authentication)           │
└──────────────────────────────────────────────────────────────────┘
         ↓
┌──────────────────────────────────────────────────────────────────┐
│ HelperTokenServiceImpl::redeem(code)                             │
│   1. Format-Validation (10 chars Crockford) → 400 falls falsch   │
│   2. token_hash = sha256_hex(code)                               │
│   3. ATOMIC: UPDATE helper_token SET used_at=now, session_id=?   │
│       WHERE token_hash=? AND used_at IS NULL AND                 │
│             revoked_at IS NULL AND deleted IS NULL               │
│       RETURNING id, assembly_id                                  │
│      → 1 Row: Erfolg → goto 4                                    │
│      → 0 Rows: differenzierter Status-Lookup                     │
│         a) SELECT used_at,revoked_at FROM helper_token WHERE     │
│            token_hash=? AND deleted IS NULL                      │
│         b) Keine Row → 404                                       │
│         c) revoked_at IS NOT NULL → 403                          │
│         d) used_at IS NOT NULL → 410                             │
│   4. Lookup assembly_dao.find_by_id(assembly_id)                 │
│      → status != Open → 403 + revert UPDATE? → siehe Pitfall 6   │
│   5. user_id = format!("helper:{}", token_id)                    │
│   6. claims = json!({"kind":"helper","assembly_id":aid})         │
│   7. session = session_service                                   │
│        .ensure_user_and_create_session_with_claims(              │
│           &user_id, 86400, Some(claims))                         │
│   8. UPDATE helper_token SET session_id = session.id WHERE id=?  │
│      (kann auch in Step-3-RETURNING-Pfad reingezogen werden,     │
│       aber dann session_id NACH session.create — Plan entscheidet│
│       für eine Reihenfolge)                                      │
│   9. Set-Cookie + JSON-Body                                      │
└──────────────────────────────────────────────────────────────────┘
```

**Component Responsibilities:**

| Capability | File | Role |
|-----------|------|------|
| Helper-Token Entity + Auditable + DAO trait | `genossi_dao/src/helper_token.rs` (NEU) | Daten-Schema, Auditable-Impl |
| Helper-Token DAO SQLite-Impl | `genossi_dao_impl_sqlite/src/helper_token.rs` (NEU) | atomarer Redeem-UPDATE; Listing |
| Service Layer | `genossi_service_impl/src/helper_token.rs` (NEU) | Codegen, SHA256, QR-Render, Status-Diskriminierung |
| Service-Trait (interface) | `genossi_service/src/helper_token.rs` (NEU) | DI-Trait für REST-Layer |
| REST Handlers + Routing | `genossi_rest/src/helper_token.rs` (NEU) | 4 Endpoints (create, list, revoke, redeem) |
| Auth Extension | `genossi_service/src/auth_types.rs` (EDIT) | `AuthContext::Helper` Variante |
| Auth Extraction | `genossi_service_impl/src/session.rs` (EDIT `extract_auth_context`) | Claims-JSON-Parse, Status-Check |
| Permission Stub | `genossi_service_impl/src/permission.rs` (EDIT) | Match-Arm für Helper → PermissionDenied |
| Migration | `migrations/sqlite/<ts>_create_helper_token_table.sql` (NEU) | Schema mit FKs |
| DI-Wiring | `genossi_bin/src/lib.rs` (EDIT) | `HelperTokenServiceImpl`-Instanziierung |
| E2E-Tests | `genossi_bin/tests/e2e_tests.rs` (EDIT — append) | HLPR-04 Race + HLPR-05 Cascade + HLPR-07 Audit |

### Recommended Project Structure

```text
genossi_dao/src/
├── helper_token.rs                # NEU: Entity, Auditable, DAO trait
genossi_dao_impl_sqlite/src/
├── helper_token.rs                # NEU: SQLite impl mit atomic redeem
genossi_service/src/
├── helper_token.rs                # NEU: Service trait
├── auth_types.rs                  # EDIT: AuthContext::Helper variant
genossi_service_impl/src/
├── helper_token.rs                # NEU: ServiceImpl + token codegen + SHA256 + QR render
├── session.rs                     # EDIT: extract_auth_context + verify_user_session
├── permission.rs                  # EDIT: check_permission Helper match-arm
genossi_rest/src/
├── helper_token.rs                # NEU: handlers + routes + ApiDoc + RestState trait
├── lib.rs                         # EDIT: nest /api/helper/redeem (without auth) + nest /api/assembly/{aid}/helper-tokens
genossi_bin/src/lib.rs             # EDIT: HelperTokenServiceDeps + DI wiring
genossi_bin/tests/e2e_tests.rs     # EDIT: append HLPR-04, -05, -07 e2e tests
genossi_rest_types/src/lib.rs      # EDIT: HelperTokenTO, HelperTokenStatusTO, CreateHelperTokenRequest, HelperTokenCreateResponseTO, RedeemRequest, RedeemResponse
migrations/sqlite/
├── <ts>_create_helper_token_table.sql  # NEU
```

### Pattern 1: Atomic Redeem mit `UPDATE ... RETURNING` (HLPR-02, HLPR-04)

**What:** SQLite (>=3.35) unterstützt `RETURNING` auch in `UPDATE`-Statements. SQLx 0.8 kann das via `query_as::<_, Row>(...).fetch_optional(...)` lesen.

**When to use:** Genau einmal im `HelperTokenDaoImpl::atomic_redeem`. Garantiert race-frei dank SQLite-Row-Locking pro UPDATE.

**Example:**
```rust
// Source: [VERIFIED: existing pattern in genossi_dao_impl_sqlite/src/assembly.rs:172
//                  combined with sqlx 0.8 RETURNING semantics from
//                  github.com/launchbadge/sqlx/issues/1531 + #1923]
#[derive(Debug, sqlx::FromRow)]
struct RedeemRow {
    id: Vec<u8>,            // BLOB UUID
    assembly_id: Vec<u8>,   // BLOB UUID
}

async fn atomic_redeem(
    &self,
    token_hash: &str,
    used_at: PrimitiveDateTime,
    tx: TransactionImpl,
) -> Result<Option<(Uuid, Uuid)>, DaoError> {
    let used_at_str = format_dt(&used_at)?;
    let row: Option<RedeemRow> = sqlx::query_as::<_, RedeemRow>(
        "UPDATE helper_token \
         SET used_at = ? \
         WHERE token_hash = ? \
           AND used_at IS NULL \
           AND revoked_at IS NULL \
           AND deleted IS NULL \
         RETURNING id, assembly_id",
    )
    .bind(used_at_str)
    .bind(token_hash)
    .fetch_optional(tx.tx.lock().await.as_mut())
    .await
    .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

    Ok(match row {
        Some(r) => Some((Uuid::from_slice(&r.id)?, Uuid::from_slice(&r.assembly_id)?)),
        None => None,
    })
}
```

**Wichtig:** Die `session_id`-Spalte ist im RETURNING-UPDATE absichtlich NICHT gesetzt — die Session existiert zu diesem Zeitpunkt noch nicht. Ein zweiter `UPDATE helper_token SET session_id = ? WHERE id = ?` folgt nach `session_service.create_session_with_claims(...)`. Beide UPDATEs liegen in derselben Service-Transaktion (`tx.clone()`), so dass entweder beide committed oder beide rolled back werden — die Race-Atomarität bleibt durch die WHERE-Clause am ersten UPDATE gewahrt.

### Pattern 2: `AuthContext::Helper` Discriminator-Parsing (D-15/D-16)

**What:** `extract_auth_context` muss anhand der `claims`-Spalte zwischen User-Session und Helper-Session unterscheiden.

**When to use:** In `SessionServiceImpl::extract_auth_context` (oder `verify_user_session`, je nach Discretion D-19).

**Example:**
```rust
// Source: [CITED: genossi_service_impl/src/session.rs:141-159 + new claims-parsing logic]
async fn extract_auth_context(
    &self,
    session_id: Option<String>,
) -> Result<Option<AuthContext>, ServiceError> {
    let Some(sid) = session_id else { return Ok(None); };
    let Some(session) = self.verify_user_session(&sid).await? else {
        return Ok(None);
    };

    // Try to parse claims as JSON discriminator
    if let Some(claims_str) = session.claims.as_deref() {
        if let Ok(parsed) = serde_json::from_str::<HelperClaims>(claims_str) {
            if parsed.kind == "helper" {
                // D-18: Assembly-Status-Check
                let tx = self.transaction_dao.use_transaction(None).await?;
                let assembly = self.assembly_dao
                    .find_by_id(parsed.assembly_id, tx.clone()).await?;
                self.transaction_dao.commit(tx).await?;
                match assembly {
                    Some(a) if a.status == AssemblyStatus::Open => {
                        return Ok(Some(AuthContext::Helper {
                            session_id: session.session_id,
                            assembly_id: parsed.assembly_id,
                        }));
                    }
                    _ => {
                        // Assembly closed/missing → invalidate the session and reject
                        self.permission_dao.delete_session(&sid).await.ok();
                        return Ok(None);
                    }
                }
            }
        }
    }

    // Fallback: existing user-session path
    Ok(Some(AuthContext::Mock(MockContext { user_id: session.user_id })))
}

#[derive(serde::Deserialize)]
struct HelperClaims {
    kind: String,
    assembly_id: uuid::Uuid,
}
```

**Hinweis (Discretion D-19):** Diese Variante zieht `assembly_dao` und `transaction_dao` als neue Dependencies in den `SessionService`. **Alternative:** Die Logik im REST-Layer-`auth_middleware.rs` zwischen `verify_user_session` und Context-Setzen einbauen, so dass `SessionService` agnostisch bleibt. Plan-Recommendation: **In `SessionServiceImpl`** belassen — die Schicht-Trennung ist sauberer (Permission-Check ist Service-Logik) und der Test wird leichter mockable.

### Pattern 3: Crockford-Base32 Klartext-Generierung (D-09/D-10)

**What:** 10 zufällige Zeichen aus `0123456789ABCDEFGHJKMNPQRSTVWXYZ`.

**When to use:** Genau einmal pro `create_helper_token`-Aufruf.

**Example:**
```rust
// Source: [ASSUMED: hand-rolled — keine externen Verifikationsquellen,
//          aber trivial und in Rust-Standard-Idiomatik]
use rand::{RngCore, rngs::OsRng};

const CROCKFORD_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

pub fn generate_crockford_code(len: usize) -> String {
    let mut buf = vec![0u8; len];
    OsRng.fill_bytes(&mut buf);
    buf.iter()
        .map(|&b| CROCKFORD_ALPHABET[(b & 0x1f) as usize] as char)
        .collect()
}
```

**Bias-Analyse:** `b & 0x1f` mappt 256 mögliche Bytes auf 32 Buckets — **gleichverteilt** (jeder Bucket bekommt exakt 8 Source-Werte). 50 Bit Entropie bei 10 Zeichen → keine Brute-Force-Sorge bei Rate-Limiting.

### Pattern 4: QR-Code-SVG-Rendering (D-13)

**Example:**
```rust
// Source: [VERIFIED: docs.rs/qrcode/0.14.1/qrcode/render/svg/index.html]
use qrcode::{QrCode, EcLevel};
use qrcode::render::svg;

pub fn render_qr_svg(payload: &str) -> Result<String, ServiceError> {
    let code = QrCode::with_error_correction_level(payload.as_bytes(), EcLevel::Q)
        .map_err(|e| ServiceError::InternalError(Arc::from(format!("QR generate: {}", e))))?;
    Ok(code.render::<svg::Color>().build())
}
```

### Pattern 5: Race-Test mit `tokio::join!` (HLPR-04)

**Example:**
```rust
// Source: [CITED: existing pattern in genossi_bin/tests/e2e_tests.rs:setup() +
//          tokio::join macro for parallel async ops]
#[tokio::test]
async fn test_redeem_race_one_succeeds_one_fails() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // Setup: create assembly, open it, create helper token
    let assembly = create_and_open_assembly(&client, &server).await;
    let create_resp = client
        .post(server.url(&format!("/api/assembly/{}/helper-tokens", assembly.id.unwrap())))
        .json(&serde_json::json!({"memo": "Anna"}))
        .send().await.unwrap();
    let body: serde_json::Value = create_resp.json().await.unwrap();
    let code = body["code"].as_str().unwrap().to_string();

    // Race: two parallel redeem requests
    let url = server.url("/api/helper/redeem");
    let body_a = serde_json::json!({"code": code.clone()});
    let body_b = serde_json::json!({"code": code.clone()});
    let (resp_a, resp_b) = tokio::join!(
        client.post(&url).json(&body_a).send(),
        client.post(&url).json(&body_b).send(),
    );
    let (status_a, status_b) = (resp_a.unwrap().status(), resp_b.unwrap().status());

    // Exactly one 200 OK and one 410 Gone
    let mut statuses = [status_a, status_b];
    statuses.sort();
    assert_eq!(statuses, [StatusCode::OK, StatusCode::GONE]);
}
```

### Pattern 6: HLPR-07 E2E Audit-Verify

**Example:**
```rust
// Source: [VERIFIED: existing endpoint shape from genossi_rest/src/audit_log.rs:69-79]
#[tokio::test]
async fn test_helper_token_create_appears_in_audit_chain() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let assembly = create_assembly(&client, &server).await;

    // Create helper token
    client.post(server.url(&format!("/api/assembly/{}/helper-tokens", assembly.id.unwrap())))
        .json(&serde_json::json!({"memo": "Bernd"}))
        .send().await.unwrap();

    // PITFALL 4: AuditQueryFilter has no `process` field. Filter by entity_type and
    // verify the process string in returned entries.
    let resp = client
        .get(server.url("/api/audit?entity_type=helper_token"))
        .send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let paged: serde_json::Value = resp.json().await.unwrap();
    let entries = paged["entries"].as_array().unwrap();
    assert!(!entries.is_empty(), "expected at least one helper_token audit entry");
    assert!(entries.iter().any(|e| e["process"] == "helper_token.create"));

    // Verify hash chain still intact
    let verify = client.get(server.url("/api/audit/verify")).send().await.unwrap();
    assert_eq!(verify.status(), StatusCode::OK);
    let v: serde_json::Value = verify.json().await.unwrap();
    assert!(v["broken_links"].as_array().unwrap().is_empty(),
        "audit chain must be intact after helper_token create");
}
```

### Anti-Patterns to Avoid

- **Service-Layer eigene Transaktion erzeugen** — nutze `transaction_dao.use_transaction(None)` (Phase-1-Anti-Pattern wiederholt; siehe `genossi_service_impl/src/assembly.rs:72`).
- **Klartext speichern** — D-11: nur `SHA256(code)` ins DB; Klartext im JSON-Response 1× und nirgends loggen (auch nicht in `tracing::debug!`).
- **`token_hash` in `audit_fields()` aufnehmen** — auch SHA256 ist eine Pre-Image-Information, die in der Audit-Hashchain für Forensiker auftauchen würde. D-06 erlaubt es bewusst nicht.
- **Manueller `dao.update`-Call statt `audited_create!`** — D-07 erzwingt Audit-Macro-Pfad für Token-Create.
- **Cookie ohne `HttpOnly`/`SameSite=Lax`** — Cookie-Defaults von tower-sessions decken das ab, aber bei manuellem Set-Cookie unbedingt prüfen.
- **`fetch_one` statt `fetch_optional`** beim atomaren Redeem — `fetch_one` panicked bei 0 Rows; wir BRAUCHEN die 0-Row-Diskriminierung.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| QR-Code-Rendering | Eigener Bit-Matrix-Encoder | `qrcode` 0.14 mit `render::svg` | Reed-Solomon-EC ist nicht-trivial; falsche Implementierung produziert non-scannable Codes |
| SHA256-Hashing | Eigene Hash-Funktion | `sha2::Sha256` (bereits im Stack) | Konsistenz mit Audit-Hashchain, kein Crypto-Hand-Roll-Risk |
| Session-Cookie-Mechanik | Eigene Cookie-Parsing-Logik | Bestehendes `extract_session_from_cookie` aus `genossi_rest/src/auth_middleware.rs:137` | Reuse einer schon-getesteten Routine |
| User-Auto-Register | Direkte INSERT auf `user`-Tabelle | `permission_dao.ensure_user_exists(...)` (D-17) | Pattern existiert für „inventur token", behandelt UNIQUE-Race korrekt |
| Session-mit-Claims-Erzeugung | Direktes `INSERT INTO session` | `session_service.ensure_user_and_create_session_with_claims(...)` | Bestehende Routine handelt User-Auto-Register mit |
| RNG für Klartext | `time::now()` als Seed o.ä. | `rand::rngs::OsRng` | Crypto-secure required (D-10) — sonst Brute-Force-Risk |

**Key insight:** Die Versuchung in Phase 2 ist, beim QR oder beim atomaren Redeem etwas „selbst zu schreiben". Das lohnt sich nicht: das `qrcode`-Crate ist Standard, das `RETURNING`-Pattern ist eine 10-Zeilen-Konstruktion, und die Crockford-Generierung ist trivial genug, um ohne externe Crate auszukommen — aber komplex genug, dass die Wiederverwendung bestehender RNG (`OsRng`) und Hash (`Sha256`) Pflicht ist.

## Runtime State Inventory

> Phase 2 ist eine **Greenfield-Phase** für ein neues Aggregat — kein Rename, kein Refactor, kein migriertes Bestandsschema. Diese Sektion ist daher größtenteils N/A.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — `helper_token`-Tabelle existiert noch nicht; keine Bestandsdaten zu migrieren | None |
| Live service config | None — keine externen Workflows referenzieren `helper_token` | None |
| OS-registered state | None — keine Tasks oder pm2-Prozesse referenzieren das Aggregat | None |
| Secrets/env vars | `APP_URL` wird in Phase 2 erstmals serverseitig im **Mock-Auth-Build** gelesen. Bisher [VERIFIED: genossi_rest/src/lib.rs:276] nur OIDC-Build-Pfad. Plan muss `APP_URL`-Default-Strategie klären (Discretion D-12). | Plan-Detail: in Mock-Auth-Build entweder default auf `"http://localhost:3000/"` oder fail-fast |
| Build artifacts | None — keine vorberechneten Caches, keine Snapshot-Files | None |

## Common Pitfalls

### Pitfall 1: SQLx-RETURNING-Spalten-Nullability-Bug (BEKANNT)

**What goes wrong:** `query_as!`-Compile-Time-Macro inferiert in SQLite alle RETURNING-Spalten als nullable, selbst wenn sie NOT NULL sind. Compile-Fehler `non-primitive cast: Option<T> as T`.

**Why it happens:** [CITED: github.com/launchbadge/sqlx/issues/2939] — bekannter, ungefixter Bug seit 0.6.0.

**How to avoid:** Verwende **`query_as::<_, Row>` (kein `!`)** mit explizitem `#[derive(sqlx::FromRow)]` — das ist exakt das Pattern, das `genossi_dao_impl_sqlite/src/assembly.rs` schon nutzt. Vermeide `query_as!`-Macro für RETURNING-Statements.

**Warning signs:** Compile-Fehler mit `Option<Vec<u8>>` für Spalten, die `BLOB NOT NULL` deklariert sind.

### Pitfall 2: `extract_auth_context` ist async und im hot-path

**What goes wrong:** Wenn der D-18-Status-Check (`assembly_dao.find_by_id`) bei jedem authentifizierten Request läuft, kostet das einen DB-Roundtrip pro Request — auch für reine OIDC-User-Requests, die keine Helfer-Sessions sind.

**Why it happens:** Naive Implementierung würde immer JSON-Parsen und Assembly nachschlagen.

**How to avoid:** **Early-Return**, wenn `claims.is_none()` oder `kind != "helper"`. Dann fällt der OIDC-Pfad sofort durch ohne DB-Roundtrip. Siehe Code-Beispiel Pattern 2 — `if let Some(claims_str)` ist der Guard.

**Warning signs:** P99-Latenz auf `/api/members` steigt nach Phase 2 ohne ersichtlichen Grund.

### Pitfall 3: Doppel-Update auf `helper_token` für `session_id`

**What goes wrong:** Atomarer Redeem-UPDATE setzt `used_at` aber **nicht** `session_id` (weil die Session noch nicht existiert). Ein zweiter UPDATE folgt. Wenn dazwischen ein Crash passiert, bleibt `helper_token.used_at IS NOT NULL` aber `session_id IS NULL`.

**Why it happens:** Logischer 2-Schritt-Vorgang.

**How to avoid:** **Beide UPDATEs in derselben Transaktion** (`tx.clone()`). Die Service-Layer-`use_transaction(None)` wickelt das ein. Falls der zweite UPDATE fehlschlägt, rollback auch den ersten — der Token bleibt `Open`.

**Warning signs:** Token-Listing zeigt `Used`-Status mit `session_id IS NULL` — Inkonsistenz.

### Pitfall 4: Audit-Filter — kein `process`-Filter im `AuditQueryFilter`!

**What goes wrong:** [VERIFIED: genossi_dao/src/audit_log.rs:26-33] `AuditQueryFilter` hat **nur** Felder `entity_type, entity_id, user_id, action, from, to`. Der HLPR-07-E2E-Test kann nicht direkt auf `process="helper_token.create"` filtern — der `?process=`-Query-Parameter existiert nicht im REST-Endpoint.

**Why it happens:** Phase-1-D-11 etablierte zwar Punkt-Notation (`assembly.create`), aber der Filter wurde nicht gleichzeitig erweitert.

**How to avoid:** Filtere via `?entity_type=helper_token` und prüfe das `process`-Feld der returned `AuditLogEntryTO`-Einträge im Test-Code. **Tipp für Plan:** Ggf. einen kleinen Phase-2-Sub-Task für `process`-Filter im `AuditQueryFilter` (`AuditLogDao::query`) als Optimierung — aber YAGNI, das ist nicht von HLPR-07 gefordert.

**Warning signs:** Test schreibt `?process=helper_token.create` und bekommt 400 oder leere Resultate.

### Pitfall 5: `mock_auth`-Feature-Build kennt nur `MockSessionServiceImpl` mit hardcoded `MockContext`

**What goes wrong:** [VERIFIED: genossi_service_impl/src/session.rs:582-591] In `mock_auth`-Build gibt `extract_auth_context` immer `Some(AuthContext::Mock(MockContext::default()))` zurück. Der neue `AuthContext::Helper`-Pfad wird in Mock-Tests **nie geübt**.

**Why it happens:** Mock-Service ignoriert die Cookie-Inhalte komplett.

**How to avoid:** Für Phase-2-E2E-Tests müssen wir **entweder** (a) `MockSessionServiceImpl` erweitern, dass er Helper-Cookies erkennt, **oder** (b) E2E-Tests laufen mit `oidc`-Feature **nicht** im `e2e_tests.rs`-File. **Empfehlung:** `MockSessionServiceImpl::extract_auth_context` so erweitern, dass es bei `Some(sid)` mit Format `"helper:<assembly_uuid>:<token_id>"` einen `AuthContext::Helper` baut. Alternativ: `MockSessionServiceImpl` durch `SessionServiceImpl<MockDeps>` ersetzen, der echte Sessions in der In-Memory-DB lookupt — das ist **drastisch weniger Mock**, aber genauer für Phase-2-Tests.

**Warning signs:** E2E-Test für „Helfer-Request gegen Closed-Assembly → 401" schlägt fehl, weil der Mock-Session-Service unconditional `MockContext` returned.

### Pitfall 6: Race zwischen Redeem und Assembly-Close

**What goes wrong:** Helfer-A redeemt erfolgreich (atomarer UPDATE setzt `used_at`). Vor dem zweiten UPDATE für `session_id` ruft Vorstand `close_assembly` auf. D-18-Status-Check würde die Session sofort beim ersten `verify_user_session`-Aufruf invalidieren. **Aber:** der Token ist trotzdem `used_at IS NOT NULL`, also nicht mehr re-deemable.

**Why it happens:** Lifecycle-Window.

**How to avoid:** Dies ist **akzeptabel** — D-18 garantiert, dass die Session sofort ungültig wird; der „verbrannte" Token ist nicht regelwidrig (HLPR-04 sagt nur: Token ist One-Time-Use). Plan sollte dieses Verhalten explizit dokumentieren, nicht „fixen".

**Warning signs:** Vorstand fragt sich, warum eine Helfer-Liste „Used"-Tokens für eine geschlossene GV zeigt.

### Pitfall 7: `tower_governor` 0.6 Pro-Route-Konfiguration

**What goes wrong:** Die bestehenden GovernorLayer in `genossi_rest/src/lib.rs:448-485` sind global pro Path-Prefix angewendet (`auth_rate_layer` auf `/authenticate`, `api_rate_layer` auf `/api/*`, `join_rate_layer` auf `/api/public/join`). Ein zusätzlicher Per-IP-Schutz für `/api/helper/redeem` muss als **eigene Layer** auf der Subroute liegen.

**Why it happens:** GovernorLayer-Config gilt für die Route, an die sie attachiert wird.

**How to avoid:** Im `genossi_rest/src/lib.rs`-Router neuen `redeem_rate_config` (z.B. 10/min/IP) bauen und auf die `/api/helper/redeem`-Route layern, **bevor** sie ins Haupt-Routing-Setup gemerged wird. Plan-Detail.

**Warning signs:** Brute-Force-Skripts bekommen 60-req/min global statt 10-req/min spezifisch.

## Code Examples

Bereits oben in „Architecture Patterns" — Examples 1–6.

Zusätzlich für die Migration (D-27):

### Migration-Skeleton

```sql
-- migrations/sqlite/<YYYYMMDDHHMMSS>_create_helper_token_table.sql
-- Source: [VERIFIED: schema derived from D-01, applying same pattern as
--          migrations/sqlite/20260502000000_create_assembly_table.sql]

CREATE TABLE IF NOT EXISTS helper_token (
    id BLOB PRIMARY KEY NOT NULL,
    assembly_id BLOB NOT NULL,
    memo TEXT NOT NULL,
    token_hash TEXT NOT NULL,
    created TEXT NOT NULL,
    used_at TEXT,
    session_id TEXT,
    revoked_at TEXT,
    deleted TEXT,
    version BLOB NOT NULL,
    FOREIGN KEY (assembly_id) REFERENCES assembly(id) ON DELETE RESTRICT,
    FOREIGN KEY (session_id) REFERENCES session(id) ON DELETE SET NULL
);

-- UNIQUE on token_hash for atomic redeem WHERE-clause + brute-force hardening
CREATE UNIQUE INDEX IF NOT EXISTS idx_helper_token_token_hash ON helper_token(token_hash);

-- Listing query: GET /api/assembly/{id}/helper-tokens
CREATE INDEX IF NOT EXISTS idx_helper_token_assembly ON helper_token(assembly_id);

-- Soft-delete filter
CREATE INDEX IF NOT EXISTS idx_helper_token_deleted ON helper_token(deleted);
```

**Hinweis zum partiellen Index:** Ein `WHERE used_at IS NULL`-Index wäre für den atomaren Redeem-WHERE-Clause hilfreich, aber: SQLite supports partial indexes, **and** the UNIQUE-Index auf `token_hash` ist bereits sehr selektiv (Klartext ist Crockford-zufallsverteilt mit 50 bit Entropie → Index-Lookup ist O(1) auch bei 100k Token). **Empfehlung: kein partieller Index nötig.**

**FK-Hinweis (Discretion):** [VERIFIED: 20250129000000_create_auth_tables.sql:48] `session.id` ist `TEXT PRIMARY KEY`, FK-kompatibel. `ON DELETE SET NULL` funktioniert in SQLite voll. Cleanup-Job-Implikation: ein Token bleibt nach Session-Cleanup mit `session_id = NULL, used_at IS NOT NULL` sichtbar — exakt das gewünschte Listing-Verhalten (D-01 Begründung).

### `Auditable`-Impl für `HelperTokenEntity` (D-06)

```rust
// Source: [VERIFIED: pattern from genossi_dao/src/assembly.rs:58-94]
impl crate::auditable::Auditable for HelperTokenEntity {
    fn entity_type() -> &'static str { "helper_token" }
    fn entity_id(&self) -> Uuid { self.id }
    fn audit_fields(&self) -> Vec<(&'static str, Option<String>)> {
        let format_dt = |dt: &PrimitiveDateTime| {
            dt.assume_utc()
                .format(&Iso8601::DEFAULT)
                .unwrap_or_else(|err| {
                    tracing::error!(error = ?err, entity = "helper_token",
                        "Failed to format datetime for audit field");
                    "<invalid datetime>".to_string()
                })
        };
        // D-06: NO token_hash. Includes assembly_id, memo, used_at, session_id,
        // revoked_at — anything else useful for forensic review.
        vec![
            ("assembly_id", Some(self.assembly_id.to_string())),
            ("memo", Some(self.memo.to_string())),
            ("used_at", self.used_at.as_ref().map(format_dt)),
            ("session_id", self.session_id.as_ref().map(|s| s.to_string())),
            ("revoked_at", self.revoked_at.as_ref().map(format_dt)),
        ]
    }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Eigene UUID-basierte Token (z.B. `uuid::Uuid::new_v4().to_string()` als Klartext-Code) | Crockford-Base32 (10 Zeichen) | D-09 | Manual-Code-Eingabe per Hand wird realistisch (Phase 4 HLPR-03) |
| `query_as!`-Compile-Time-Macro | `query_as::<_, Row>(...)` mit `FromRow`-Derive | SQLx 0.8 RETURNING-Bug | Workaround für [CITED: github.com/launchbadge/sqlx/issues/2939] |
| Session ohne Claims | Session **mit JSON-Claims** als Discriminator (`kind: "helper"`) | D-15/D-16 | Erlaubt Future-Erweiterung (z.B. `kind: "vorstand-impersonation"`) ohne Schema-Change |

**Deprecated/outdated:**
- `tower-sessions 0.13` → `tower-sessions 0.14` ist im Stack [VERIFIED: Cargo.toml:41]; das STATE.md-TODO „0.14 → 0.15 Upgrade" ist **nicht Phase-2-blockend**: die für D-15/D-18 verwendeten APIs (`session.id`, `session.expires`, `session.claims`) sind seit 0.13 stabil. Verschoben auf eine spätere Phase.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Crockford-Base32 hand-rolled mit `OsRng.fill_bytes` und `b & 0x1f` ist gleichverteilt | Pattern 3 | **LOW** — Mathematisch verifiziert (256/32=8 Source-Werte pro Bucket), aber unter ASSUMED markiert weil keine externe Quelle das exakt so dokumentiert |
| A2 | `tower-sessions` 0.14 erlaubt per-Session unterschiedliche Expiry-Werte über `session.expires`-Spalte (im Genossi-eigenen Schema), nicht über Cookie-Layer-Config | Architecture | **LOW** — Genossi nutzt eine **eigene** `session`-Tabelle, nicht den `tower-sessions`-MemoryStore (siehe `genossi_dao/src/permission.rs:117-125` `SessionEntity`). Der Cookie-Layer ist nur für die Cookie-Übertragung zuständig; das `session.expires`-Feld ist unsere Wahrheit. Daher nutzt D-18 die Custom-Spalte direkt, ohne Cookie-Layer-Override. **Verifiziert** durch Code-Lese, aber als A2 gelistet weil die explizite Bestätigung außerhalb dieser Research nicht in einem Doc steht. |
| A3 | `MockSessionServiceImpl` muss in Phase 2 erweitert werden, damit Helper-E2E-Tests laufen | Pitfall 5 | **MEDIUM** — Wenn das übersehen wird, sind E2E-Tests (HLPR-05) silently nicht-aussagekräftig. Plan muss diesen Sub-Task explizit listen. |
| A4 | `APP_URL` ist in Phase 2 auch im `mock_auth`-Build erforderlich (für QR-Inhalt). Bestehender Read-Pfad ist nur im OIDC-Pfad | Stored data | **MEDIUM** — Plan muss klären, ob Mock-Auth-Tests `APP_URL` env-var setzen müssen oder ob ein Default akzeptabel ist (`"http://localhost:3000/"`). Discretion D-12. |
| A5 | `session_id` als String-Foreign-Key zu `session.id` (TEXT) ist FK-kompatibel | Migration | **LOW** — verified durch [VERIFIED: 20250129000000_create_auth_tables.sql:48] (`session.id TEXT PRIMARY KEY`). |

**Falls die Assumption-Tabelle nicht leer ist:** Plan-Phase und Discuss-Phase sollten die A2/A3/A4-Annahmen explizit bestätigen. A1 und A5 sind triviale Verification-Schritte beim Implementieren.

## Open Questions (RESOLVED)

1. **`MockSessionServiceImpl` Erweiterung — eigener Sub-Task oder einfach als ServiceImpl-Switch im e2e-Test-Setup?**
   - What we know: Bestehender `MockSessionServiceImpl` ignoriert Cookie-Inhalte komplett.
   - What's unclear: Sauberer wäre, `e2e_tests.rs` auf `SessionServiceImpl<RealDeps>` umzuschalten — aber das könnte bestehende Member/Application-E2E-Tests brechen.
   - Recommendation: Plan baut **kleine zusätzliche Routine** in `MockSessionServiceImpl::extract_auth_context`, die das `claims`-Feld aus einem custom Test-Cookie-Format liest (z.B. `app_session=helper:<assembly_uuid>`). Pragmatisch und änderungsarm.
   - **RESOLVED:** Plan 02-06 Task 2 implementiert die Helper-Cookie-Format-Erkennung `helper:<assembly_uuid>:<token_id>` in `MockSessionServiceImpl::extract_auth_context` und ergänzt einen optionalen `assembly_status_probe`, damit Plan 02-08 Task 2 die D-18-Cascade nach `close_assembly` exerzieren kann.

2. **Welche Reihenfolge für die zwei UPDATEs auf `helper_token` beim Redeem?**
   - What we know: Atomarer UPDATE setzt `used_at` (für Race-Sicherheit). Session existiert danach noch nicht.
   - What's unclear: Soll der zweite UPDATE (`session_id`) im selben Service-Method-Aufruf oder als separater DAO-Call stehen?
   - Recommendation: Einzelner Service-Method-Aufruf, beide UPDATEs in derselben TX. DAO bietet `atomic_redeem(...) -> (Uuid, Uuid)` und `set_session_id(token_id, session_id)` als separate Methoden — Service orchestriert.
   - **RESOLVED:** Plan 02-05 Task 2 orchestriert beide UPDATEs in derselben TX (`tx.clone()` von `atomic_redeem` zu `set_session_id`); Pitfall 3 ist im Threat-Register T-02-05-02 dokumentiert.

3. **Cookie-Lifetime beim Set-Cookie aus `/api/helper/redeem`-Response?**
   - What we know: D-18 sagt 24h `expires` in der Custom-Tabelle.
   - What's unclear: Sollte das `Max-Age=86400` auf dem Set-Cookie auch gesetzt werden, oder ein Session-Cookie ohne Expiry?
   - Recommendation: `Max-Age=86400` setzen, damit der Browser das Cookie nach Logout-Zeit verwirft. Für die Authoritative-Wahrheit zählt aber `session.expires` in der DB — D-18 wirkt früher (Assembly-Close-Zeit).
   - **RESOLVED:** Plan 02-07 setzt `Max-Age=86400` auf dem Set-Cookie (acceptance_criterion `grep -c "Max-Age=86400"`); D-18 bleibt Source-of-Truth, Cookie-Expiry ist Browser-Hint.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | Hauptbuild | ✓ | bestehend (1.70+) | — |
| SQLx CLI | Migration-Run | ✓ | bestehend | — |
| SQLite >= 3.35 | `RETURNING` in UPDATE | ✓ | sqlx 0.8 bundled libsqlite3 (modern) | — |
| `qrcode` 0.14 | QR-Render | ✗ | — | Muss zu Cargo.toml hinzugefügt werden |
| `rand` 0.8 (direkt) | Klartext-RNG | ✓ (transitiv 0.8.5 + 0.9.4) | [VERIFIED: Cargo.lock] | — |
| `sha2` 0.10 | Token-Hash | ✓ | bestehend | — |
| `serde_json` 1.0 | Claims-JSON-Parse | ✓ | bestehend | — |

**Missing dependencies with no fallback:**
- `qrcode` 0.14.1 — muss als neue Workspace-Dep in `Cargo.toml` und in `genossi_service_impl/Cargo.toml` hinzugefügt werden.

**Missing dependencies with fallback:** Keine.

## Project Constraints (from CLAUDE.md)

- **Tests pflicht für jede Änderung** — Phase 2 hat in CONTEXT.md keine explizite Anweisung gegen Unit-Tests; CLAUDE.md (`/home/neosam/.claude/CLAUDE.md`) sagt: „Always make sure you have tests for the changes". Plan muss Unit-Tests pro Service-Methode UND E2E-Tests für HLPR-04/-05/-07 listen.
- **Layered DAO/Service/REST** — pflicht; Phase 2 folgt dem Phase-1-Pattern.
- **Audit-Macros** für Member/MemberAction/MemberDocument/Application **plus** für `helper_token.create` (D-07).
- **Soft-Delete-Konvention** — `deleted: Option<PrimitiveDateTime>` in `helper_token`-Entity (D-04).
- **Optimistic Locking** — `version: Uuid` in `helper_token`-Entity (D-01).
- **ISO8601-Datetime** — `genossi_rest_types::iso8601_datetime`-Modul für `HelperTokenTO`.
- **Component-First Frontend** — N/A für Phase 2 (Backend-Only).
- **Audit-Pflicht für bestehende Entitäten** — bleibt bestehen; `helper_token` erweitert die Liste der auditierten Entitäten (CLAUDE.md §Audit Log System).
- **Auditable-Trait + AuditLogDao-Dep + Macros + Wiring** — alle vier Schritte aus CLAUDE.md §"Adding Audit to New Entities" sind erforderlich.

## Sources

### Primary (HIGH confidence)
- `/home/neosam/programming/rust/projects/genossi3/CLAUDE.md` — Audit-Log-Workflow für neue Entitäten
- `/home/neosam/programming/rust/projects/genossi3/genossi_service_impl/src/assembly.rs` — Phase-1-Service-Template (Lifecycle-Guard, Optional-TX, Audit-Macros)
- `/home/neosam/programming/rust/projects/genossi3/genossi_service_impl/src/session.rs:52-189` — `create_session_with_claims`, `verify_user_session`, `extract_auth_context`
- `/home/neosam/programming/rust/projects/genossi3/genossi_service_impl/src/audit_macros.rs` — `audited_create!`-Macro-Definition
- `/home/neosam/programming/rust/projects/genossi3/genossi_dao/src/permission.rs:88-125` — SessionEntity-Schema, `ensure_user_exists`-Default-Impl
- `/home/neosam/programming/rust/projects/genossi3/genossi_dao/src/auditable.rs` — Auditable-Trait
- `/home/neosam/programming/rust/projects/genossi3/genossi_dao/src/audit_log.rs:26-33` — `AuditQueryFilter`-Schema (kein process-Feld!)
- `/home/neosam/programming/rust/projects/genossi3/genossi_rest/src/auth_middleware.rs:101-156` — `extract_context_from_headers`, `extract_session_from_cookie`
- `/home/neosam/programming/rust/projects/genossi3/genossi_dao_impl_sqlite/src/assembly.rs:73-180` — SQLx-Pattern mit `query_as::<_, Row>` und `FromRow`-Derive
- `/home/neosam/programming/rust/projects/genossi3/migrations/sqlite/20250129000000_create_auth_tables.sql:48-65` — Bestehendes `session`-Tabellen-Schema (TEXT PRIMARY KEY)
- `/home/neosam/programming/rust/projects/genossi3/migrations/sqlite/20260502000000_create_assembly_table.sql` — Phase-1-Migration als Vorlage
- `/home/neosam/programming/rust/projects/genossi3/genossi_rest/src/lib.rs:448-485` — `tower_governor` Konfiguration
- `/home/neosam/programming/rust/projects/genossi3/Cargo.toml:41` — `tower-sessions = "0.14"` (verified)
- docs.rs/qrcode/0.14.1/qrcode/render/svg/index.html — `code.render::<svg::Color>().build()`-API verifiziert

### Secondary (MEDIUM confidence)
- crates.io/crates/qrcode — Version 0.14.1, MIT/Apache (web search)
- github.com/launchbadge/sqlx/issues/2939 — RETURNING-Nullability-Bug bekannt
- github.com/launchbadge/sqlx/issues/1531 — RETURNING-Pattern-Doku
- crates.io/crates/base32 — Version 0.5.1 (Crockford-Alphabet-fähig — bei Bedarf, aber nicht empfohlen)

### Tertiary (LOW confidence)
- Hand-rolled Crockford-Base32-Bias-Analyse (256/32=8 Bucket-Verteilung) — selbst-verifiziert mathematisch, ASSUMED-Status für Konformität mit Output-Format-Standard

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — alle Crates direkt im Code/Lockfile/docs.rs verifiziert
- Architecture: HIGH — Phase-1-Pattern als 1:1-Vorlage, kein neues architekturelles Risiko
- Pitfalls: HIGH — Pitfalls 1, 4, 5, 7 direkt aus Code-Inspektion identifiziert; 2/3/6 logisch deduziert

**Research date:** 2026-05-03
**Valid until:** 2026-06-03 (30 Tage — Stack ist stabil; SQLx 0.8 → 0.9 sollte beobachtet werden für RETURNING-Fix)
