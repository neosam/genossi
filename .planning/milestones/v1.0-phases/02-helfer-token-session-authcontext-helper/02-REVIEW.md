---
phase: 02-helfer-token-session-authcontext-helper
reviewed: 2026-05-03T00:00:00Z
depth: standard
files_reviewed: 19
files_reviewed_list:
  - Cargo.toml
  - genossi_bin/src/lib.rs
  - genossi_bin/tests/e2e_tests.rs
  - genossi_dao_impl_sqlite/src/helper_token.rs
  - genossi_dao_impl_sqlite/src/lib.rs
  - genossi_dao/src/helper_token.rs
  - genossi_dao/src/lib.rs
  - genossi_rest/src/helper_token.rs
  - genossi_rest/src/lib.rs
  - genossi_rest/src/test_server.rs
  - genossi_rest_types/src/lib.rs
  - genossi_service_impl/Cargo.toml
  - genossi_service_impl/src/helper_token.rs
  - genossi_service_impl/src/lib.rs
  - genossi_service_impl/src/session.rs
  - genossi_service/src/auth_types.rs
  - genossi_service/src/helper_token.rs
  - genossi_service/src/lib.rs
  - migrations/sqlite/20260503000000_create_helper_token_table.sql
findings:
  blocker: 4
  warning: 11
  total: 15
status: issues_found
---

# Phase 02: Code Review Report — Helfer-Token + Session/AuthContext::Helper

**Reviewed:** 2026-05-03
**Depth:** standard
**Files Reviewed:** 19
**Status:** issues_found

## Summary

Die Helfer-Token-Implementierung ist breit getestet (Unit-Tests im DAO/Service, E2E-Tests im Bin) und folgt dem etablierten Layered-Pattern. Die Hash-Chain-Audit-Integration, die Crockford-Codegen mit OsRng und der Atomic-Redeem-WHERE-Pattern sind sauber umgesetzt.

Allerdings gibt es **vier Blocker**: (1) der `AuthContext::Helper`-Pfad ist im Request-Lifecycle nirgendwo gewired — die Variante wird nur in `SessionService::extract_auth_context` konstruiert, das in Production aber nie aufgerufen wird, was D-15/D-16/D-18 effektiv neutralisiert; (2) die FK-Constraints (`assembly_id ON DELETE RESTRICT`, `session_id ON DELETE SET NULL`) feuern in Production nicht, weil die Codebase `PRAGMA foreign_keys = ON` explizit nicht setzt — die in der Migration und in den DAO-Tests behaupteten Semantiken sind Lügen; (3) der Public-Redeem-Endpoint (`POST /api/helper/redeem`) ist in der OIDC-Build-Konfiguration nicht zwingend ohne Auth erreichbar — die Routen-Reihenfolge und Layer-Anwendung muss überprüft werden; (4) `validate_code_format` cast `char as u8` und akzeptiert dadurch Unicode-Codepoints, deren tiefe 8 Bit zufällig im Crockford-Alphabet landen — das ist kein Sicherheits-Bug, aber ein Korrektheits-Bug, der die Format-Validation umgeht.

Daneben elf Warnings: u.a. `set_session_id` filtert nicht `deleted IS NULL`, der Default-`find_by_id` lädt N Rows in Memory, eine Closed-Assembly kann einen Helfer-Token „verbrennen" obwohl er nie eingelöst werden kann, und der unsafe-`Send/Sync`-Block in `genossi_bin/src/lib.rs` ist auf reinen Marker-Types redundant aber bleibt unkommentiert stehen.

## Blocker Issues

### BLOCKER-01: AuthContext::Helper wird im Request-Lifecycle nie konstruiert

**File:** `genossi_rest/src/session.rs:73-129`
**File:** `genossi_service_impl/src/session.rs:161-232`

**Issue:**
Phase 2 spezifiziert (D-15/D-16/D-18), dass `SessionServiceImpl::extract_auth_context` Helper-Sessions aus den Claims discriminiert, das Assembly-Status-Cascade-Check fährt und `AuthContext::Helper { session_id, assembly_id }` an die Handler weiterreicht. Die Methode existiert auch und ist gut getestet, aber: kein wired-in Middleware-Pfad ruft `extract_auth_context` auf.

- Der OIDC-Build (`genossi_rest/src/session.rs:73`) ruft `verify_user_session(session_id)` und konstruiert `AuthenticatedContext` direkt — **kein** Aufruf von `extract_auth_context`, **keine** Helper-Discrimination, **keine** D-18 Cascade.
- Der mock_auth-Build (`genossi_rest/src/session.rs:122`) injiziert unconditional `MockContext` ohne überhaupt das Cookie zu lesen.
- `auth_middleware::extract_auth_context` (Z. 16-35) würde `SessionService::extract_auth_context` verwenden, ist aber nirgends in `lib.rs::create_app` verdrahtet — der Router benutzt ausschließlich `session::context_extractor`.

**Konsequenz:** Ein Helfer, der via `/api/helper/redeem` ein Cookie bekommt, wird in OIDC als regulärer Vorstand (`AuthenticatedContext`) und in mock_auth als `MockContext::DEVUSER` (= Admin) behandelt. Die in D-20 dokumentierte „PermissionDenied"-Schranke fired nie. Die D-18 Session-Invalidate-Cascade nach Closed-Assembly fired nie. Die Plan-02-08-Tests „cascade signals" beobachten nur das `revoke→409` aus dem ASSEMBLY-Lifecycle (D-23), nicht aus dem Helper-Cookie (siehe E2E-Test-Kommentar `genossi_bin/tests/e2e_tests.rs:9047-9068, 9157-9168`).

**Fix:**
Den Production-Pfad auf `SessionService::extract_auth_context` umstellen. Konkret in `genossi_rest/src/session.rs:73-129`:

```rust
#[cfg(feature = "oidc")]
pub async fn context_extractor<RestState: RestStateDef>(
    State(rest_state): State<RestState>,
    mut request: Request,
    next: Next,
) -> Response {
    let cookies = request
        .extensions()
        .get::<Cookies>()
        .expect("Cookies extension not set");
    let session_id = cookies.get("app_session").map(|c| c.value().to_string());

    match rest_state
        .session_service()
        .extract_auth_context(session_id)
        .await
    {
        Ok(Some(AuthContext::Helper { session_id, assembly_id })) => {
            // Insert AuthContext::Helper as a separate request extension so
            // handlers / permission checks can match on it. (Requires a
            // companion change in extract_auth_context() in lib.rs to map
            // Helper to PermissionDenied per D-20 until Phase 3.)
            request.extensions_mut().insert(Some(AuthContext::Helper {
                session_id, assembly_id,
            }));
        }
        Ok(Some(AuthContext::Mock(_))) | Ok(Some(AuthContext::Oidc(_))) => {
            // ...existing AuthenticatedContext handling
        }
        Ok(None) | Err(_) => {
            request.extensions_mut().insert(None::<AuthenticatedContext>);
        }
    }
    next.run(request).await
}
```

Im mock_auth-Build dasselbe Muster — falls der Helper-Cookie über Cookie-Jar reinkommt, soll `MockSessionServiceImpl::extract_auth_context` gerufen werden statt unbedingt `MockContext` einzuwerfen. Bis zu diesem Fix ist die ganze D-15/D-16/D-18-Story rein theoretisch.

---

### BLOCKER-02: FK-Constraints werden in Production nicht erzwungen, Migration suggeriert das Gegenteil

**File:** `migrations/sqlite/20260503000000_create_helper_token_table.sql:22-23`
**File:** `migrations/sqlite/20260502000001_create_assembly_member_snapshot_table.sql:2-10`

**Issue:**
Die neue Migration deklariert zwei FKs:
- `FOREIGN KEY (assembly_id) REFERENCES assembly(id) ON DELETE RESTRICT`
- `FOREIGN KEY (session_id) REFERENCES session(id) ON DELETE SET NULL`

Der Migration-Header begründet beide Semantik-Entscheidungen (D-23 / Cleanup-Jobs). Der vorhergehende Migrationskommentar (`20260502000001_create_assembly_member_snapshot_table.sql:2`) bestätigt aber wörtlich:

> This codebase does not enable `PRAGMA foreign_keys=ON` on the SqlitePool

In SQLite sind FKs per default OFF; die Migration-Deklarationen sind reine Doku-Kommentare ohne Runtime-Wirkung. `genossi_bin/src/lib.rs::RestStateImpl::new` setzt das PRAGMA auch nicht. Konsequenzen:

1. Hard-Delete einer Assembly löscht assoziierte Helper-Tokens NICHT, lässt aber dangling `assembly_id`-Referenzen zurück — `find_by_id` würde einen Helper-Token ohne Assembly liefern, das `revoke_helper_token`-Lifecycle-Guard `assembly_dao.find_by_id(...).await?.ok_or(EntityNotFound)?` wirft dann fälschlich `EntityNotFound(token_id)` (Z. 261), obwohl der Token existiert.
2. `ON DELETE SET NULL` für `session_id` fired nicht — Cleanup-Jobs, die alte Sessions löschen (`SessionService::cleanup_expired_sessions`), lassen veraltete `session_id`-Strings im helper_token zurück.
3. Der DAO-Unit-Test (`helper_token.rs:330-333`) setzt `PRAGMA foreign_keys = ON` explizit — Tests laufen unter anderen Constraints als Production. Falsche Sicherheit.

**Fix:**
Entweder PRAGMA global aktivieren (Phase-Plan/Memory hat das als Phase-3-Task gelassen) ODER die Migration anpassen und die FK-Klauseln entweder weglassen oder mit deutlichem Kommentar als „documentation-only" kennzeichnen. Empfehlung: PRAGMA in `genossi_bin/src/main.rs` und in `setup()` der E2E-Tests aktivieren, sobald die Codebase reif dafür ist:

```rust
// in genossi_bin (main): direkt nach pool-setup
sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await?;

// in genossi_bin/tests/e2e_tests.rs::setup():
sqlx::query("PRAGMA foreign_keys = ON").execute(&*pool).await.unwrap();
```

Der DAO-Unit-Test setzt PRAGMA bereits korrekt (`helper_token.rs:330`), also ist die FK-Semantik auf DB-Schicht lokal korrekt — das Problem ist ausschließlich die nicht-aktivierte Pragma-Konfiguration im Pool für Production und E2E-Tests.

---

### BLOCKER-03: `redeem_helper_token` läuft in der OIDC-Build evtl. durch die OIDC-Auth-Layer

**File:** `genossi_rest/src/lib.rs:546-697`

**Issue:**
Die Router-Konstruktion in `create_app` schichtet so:

1. Z. 548-613: `app` wird mit allen `/api/...`-Routes aufgebaut, dann `with_state` und Middleware-Layer (`forbid_unauthenticated`, `context_extractor`, `cors_layer`).
2. Z. 625-684 (cfg `oidc`): `OidcAuthLayer` wird auf das gesamte `app` geschichtet.
3. Z. 689-697: Public-Routen (`/api/helper`, `/api/public`) werden NACH der OIDC-Layer per `nest` angehängt.

In Axum gilt: `.layer()` wirkt nur auf bereits im Router vorhandene Routen, NICHT auf später per `.nest` hinzugefügte. Das ist also korrekt — Public-Routes umgehen die OIDC-Layer.

ABER: Z. 614-617 hängt vorher `forbid_unauthenticated` per `from_fn_with_state` an `app`. In OIDC-Build verlangt `forbid_unauthenticated` ein gültiges `Context` (Z. 152-153 in `genossi_rest/src/session.rs`), was in der Public-Pfad-Lane fehlt. Da die Layer-Reihenfolge: `context_extractor → forbid_unauthenticated → … → oidc_auth_service` für die geschachtelten `/api/*`-Routen gilt — und die `/api/helper`-Routen werden NACH all dem geschachtelt — sind diese tatsächlich frei. Das ist die richtige Reihenfolge.

**Aber** der `axum_oidc::OidcAuthLayer` ist eine Tower-Layer; deren genaue Side-Effects auf den Cookie-Jar und die Session-State sind unklar. Wenn die Session-Layer (Tower-Sessions) globalt einen Cookie verlangen, oder Memory-Store-State vor jeder Request initialisiert, könnte die Public-Route bei einer leeren Session auf einen Fehler-Pfad gehen. Das ist ohne Build/E2E-Test im OIDC-Modus nicht verifizierbar.

Ein zweites konkretes Risiko: die `CookieManagerLayer` wird zweimal angewendet (`#[cfg(not(feature = "oidc"))] app.layer(CookieManagerLayer::new())` Z. 686-687, und implizit über `oidc_auth_service.layer(session_layer).layer(CookieManagerLayer::new())` Z. 683-684). Doppelt geschachtelte Cookie-Layer können Cookie-Set-Konflikte verursachen.

**Fix:**
1. E2E-Test für die `oidc`-Feature-Konfiguration hinzufügen, der `POST /api/helper/redeem` ohne Cookie/Bearer aufruft und 200 erwartet (Cargo-Feature `oidc` aktivieren).
2. Wenn der Test fehlschlägt, Public-Routen VOR der OIDC-Layer registrieren (mit eigenem `with_state`-Branch), oder die Public-Path-Whitelist in `forbid_unauthenticated` und in `axum_oidc` setzen.
3. Doppelte `CookieManagerLayer`-Anwendung im OIDC-Pfad durch `apply_unless_oidc` o.ä. faktorisieren, um ungewollte Cookie-Side-Effects zu vermeiden.

---

### BLOCKER-04: `validate_code_format` cast `char as u8` umgeht die Alphabet-Prüfung

**File:** `genossi_service_impl/src/helper_token.rs:111-119`

**Issue:**
```rust
} else if !code
    .chars()
    .all(|c| (CROCKFORD_ALPHABET as &[u8]).contains(&(c as u8)))
{
```

`c as u8` truncate eine 32-Bit-`char` auf das Low-Byte. Konsequenz: Unicode-Codepoints, deren tiefe 8 Bit zufällig auf einen Crockford-Alphabet-Byte mappen, passieren die Validation. Beispiele:

- `'Ā'` (U+0100, Decimal 256) → `256 as u8 = 0` → matcht `'0'` ✓ (im Alphabet)
- `'ı'` (U+0131, Decimal 305) → `305 as u8 = 49` → matcht `'1'` ✓ (im Alphabet)
- `'Ġ'` (U+0120, Decimal 288) → `288 as u8 = 32` → matcht space (NICHT im Alphabet, korrekt rejected)
- `'Ā₂₃₄₅₆₇₈₉Ā'` würde alle Validation passieren (10 Codepoints, alle „im Alphabet" nach `as u8`)

**Konsequenz:** Ein REST-Client kann mit Unicode-Codes einen 200-Response-Pfad erreichen (Format-Validation passiert), und der nachfolgende `sha256_hex(code)` (Z. 314) hashen die UTF-8-Repräsentation des Strings — die wird natürlich nicht in der DB sein, also Routing zu 404. **Kein Security-Bug**, aber: 

1. **Brute-Force-Protection wird unterlaufen:** Die Rate-Limit-Layer (`redeem_rate_layer`, `lib.rs:509-518`) zählt 10/min. Mit 32 Crockford-Chars + ~256 Unicode-Codepoints, die auf Alphabet-Bytes mappen, expandiert der mögliche Eingaberaum drastisch (~256^10 statt 32^10 = 50 bit Entropie), aber relevant nur, wenn die Validation nicht echte Unicode-Filter ist. Da der Hash sich aber unterscheidet (UTF-8-Bytes), sind die zusätzlichen Codes alle „unbekannt" — die effektive Brute-Force-Resistance bleibt bei 50 Bit. Trotzdem: Doku/Spec lügt über die akzeptierten Eingaben.
2. **Fehler-Pfad:** Statt 400 BadRequest (Format-Fehler) bekommt der Client 404 NotFound — divergiert von D-24-Spec.
3. **Korrektheit:** Die Test-Suite testet exakt diese Edge-Cases nicht (`test_validate_code_format_rejects_invalid_alphabet` Z. 511-532 testet nur ASCII-Variants).

**Fix:**
Char-by-char gegen die Alphabet-Chars vergleichen, nicht gegen Bytes:

```rust
const CROCKFORD_CHARS: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";
// ...
} else if !code.chars().all(|c| CROCKFORD_CHARS.contains(c)) {
    errors.push(ValidationFailureItem {
        field: Arc::from("code"),
        message: Arc::from("invalid_alphabet (use Crockford base32 uppercase)"),
    });
}
```

Plus Test:
```rust
#[test]
fn test_validate_code_format_rejects_unicode_lookalikes() {
    // U+0131 'ı' (Latin small letter dotless i, 0x0131 → low byte 0x31 = '1')
    assert!(matches!(
        validate_code_format("ı234567890"),
        Err(ServiceError::ValidationError(_))
    ));
    // U+0100 'Ā' (Latin capital letter A with macron, 0x0100 → low byte 0x00 = '0')
    assert!(matches!(
        validate_code_format("Ā234567890"),
        Err(ServiceError::ValidationError(_))
    ));
}
```

---

## Warnings

### WARNING-01: `set_session_id` filtert nicht `deleted IS NULL`

**File:** `genossi_dao_impl_sqlite/src/helper_token.rs:245-256`

**Issue:**
`UPDATE helper_token SET session_id = ? WHERE id = ?` filtert weder `deleted IS NULL` noch `session_id IS NULL`. Wenn ein Token zwischen `atomic_redeem` und `set_session_id` soft-gelöscht wird (kein realistisches Szenario in Phase 2, aber möglich), wird die deleted-Row trotzdem upgedatet. Außerdem ist `set_session_id` nicht idempotent: ein erneutes Aufrufen würde `session_id` überschreiben.

**Fix:**
```sql
UPDATE helper_token SET session_id = ?
 WHERE id = ? AND deleted IS NULL AND session_id IS NULL
```
Plus `if rows_affected == 0` → `Err(DaoError::ConflictError("session_id already set or row deleted"))`.

---

### WARNING-02: Default `find_by_id` lädt alle Rows in Memory

**File:** `genossi_dao/src/helper_token.rs:108-118`

**Issue:**
```rust
async fn find_by_id(&self, id: Uuid, tx: ...) -> Result<Option<HelperTokenEntity>, DaoError> {
    let all_entities = self.dump_all(tx).await?;
    Ok(all_entities.iter().find(|e| e.id == id && e.deleted.is_none()).cloned())
}
```

Das ist konsistent mit dem Codebase-Pattern (`genossi_dao/src/lib.rs` notiert „Only 3 required methods"), aber: `revoke_helper_token` ruft `find_by_id` auf, das pro Token-Lookup ALLE Helper-Tokens in der DB lädt, deserialisiert und filtert. Bei wachsendem Volumen (mehrere GVs, viele Helfer) wird das einen O(N) Cost pro Revoke erzeugen.

**Fix:**
Ein dedizierter `find_by_id` im SQLite-Impl mit gezieltem `SELECT ... WHERE id = ?` schreiben — analog zum Pattern, das Z. 167-177 (Update-Existence-Check) bereits zeigt. Da Performance laut Review-Scope „out of scope for v1" ist, primär als WARNING dokumentiert.

---

### WARNING-03: Geburnter Token bei Closed/Preparation-Assembly bleibt für immer ungültig

**File:** `genossi_service_impl/src/helper_token.rs:331-371`

**Issue:**
`atomic_redeem` (Z. 331-334) setzt `used_at = NOW()` bevor `assembly.status` geprüft wird (Z. 361-371). Wenn die Assembly nicht `Open` ist, geht der Token in den `Conflict("assembly_not_open")`-Pfad, wird aber NIE rückgängig gemacht (kein UPDATE auf `used_at = NULL`). Konsequenz: ein Helfer, der um 17:59 redeemt während die GV noch in Preparation ist (Vorstand drückt um 18:00 „open"), hat einen tot-eingelösten Token. Vorstand muss den Helfer-Token neu erzeugen.

Der Inline-Kommentar Z. 367-369 dokumentiert das als akzeptabel („D-18 garantiert Session-Invalidate; verbrannter Token ist OK"), aber das gilt nur für den geschlossenen-State, nicht für den Preparation-State. Plan-02-CONTEXT.md sollte diese UX-Konsequenz dokumentieren.

**Fix:**
Entweder:
1. Status-Check VOR `atomic_redeem` durchführen (zwei DB-Roundtrips, race-window vergrößert sich): `SELECT status FROM assembly WHERE id = (SELECT assembly_id FROM helper_token WHERE token_hash = ?)`.
2. Oder im Conflict-Pfad ein UPDATE-Reset durchführen: `UPDATE helper_token SET used_at = NULL WHERE id = ? AND session_id IS NULL` — riskant wegen Race mit konkurrentem Redeem.
3. Oder dokumentieren als „Preparation-Phase Helfer-Tokens sind für 17:59-Redeems gefährdet; Vorstand soll Tokens erst NACH `open` erzeugen".

---

### WARNING-04: Doppelte `unsafe impl Send/Sync` ohne Begründung

**File:** `genossi_bin/src/lib.rs:69-70, 92-95, 110-111, 133-134, 155-156, 177-178, 234-235, 266-267, 285-286, 307-308, 326-327`

**Issue:**
Jede `*ServiceDependencies`-Marker-Struct hat:
```rust
unsafe impl Send for PermissionServiceDependencies {}
unsafe impl Sync for PermissionServiceDependencies {}
```

Das sind Marker-Structs ohne Felder. Marker-Structs sind automatisch `Send + Sync` ohne `unsafe`. Die `unsafe impl`-Statements sind unnötig und verbergen, dass es keine echte unsafe-Logik gibt. Wenn jemand später ein non-Send-Feld hinzufügt, würde der unsafe-impl die Compile-Time-Sicherheit umgehen.

**Fix:**
Alle `unsafe impl Send/Sync`-Zeilen entfernen. Wenn der Compiler sich beklagt, ist das ein Symptom — ein `PhantomData`-Feld o.ä. wäre die Ursache, nicht das `unsafe impl` als Pflaster.

---

### WARNING-05: `app_url()` mit silent default in OIDC-Build

**File:** `genossi_service_impl/src/helper_token.rs:130-132`

**Issue:**
```rust
fn app_url() -> String {
    std::env::var("APP_URL").unwrap_or_else(|_| "http://localhost:3000/".to_string())
}
```

Der Kommentar (Z. 127-129) sagt „In OIDC build APP_URL is required at server start (Plan 07 wires fail-fast)", aber der Code selbst macht KEIN fail-fast. Wenn ein OIDC-Production-Deployment vergisst APP_URL zu setzen, embeddet das QR-Code-Payload einen `localhost`-Link. Die Helfer scannen ihre QRs und landen auf einer 404 oder fehlschlagenden Verbindung.

`genossi_rest/src/lib.rs::oidc_config()` (Z. 290-308) liest APP_URL via `expect("APP_URL env variable")`, also fail-fast IST gesetzt. Der Code in `app_url()` weiß das aber nicht und produziert weiterhin den default.

**Fix:**
```rust
#[cfg(feature = "oidc")]
fn app_url() -> String {
    std::env::var("APP_URL").expect("APP_URL must be set in oidc build")
}
#[cfg(not(feature = "oidc"))]
fn app_url() -> String {
    std::env::var("APP_URL").unwrap_or_else(|_| "http://localhost:3000/".to_string())
}
```

Oder den OIDC-Build mit dem fail-fast-`oidc_config`-Wert befüttern — Inject statt Env-Lookup pro Request.

---

### WARNING-06: `DbAssemblyStatusProbe` swallowt jeden DB-Fehler als „closed"

**File:** `genossi_bin/src/lib.rs:212-230`

**Issue:**
```rust
async fn is_open(&self, assembly_id: uuid::Uuid) -> bool {
    let Ok(tx) = self.transaction_dao.use_transaction(None).await else {
        return false;
    };
    let result = self.assembly_dao.find_by_id(assembly_id, tx).await;
    matches!(result, Ok(Some(a)) if a.status == AssemblyStatus::Open)
}
```

Der Inline-Kommentar (Z. 200-204) dokumentiert das als „D-18 cascade-safe". Allerdings: `find_by_id` mit dem default-trait-impl ruft `dump_all`, das ALLE Assemblies als `Arc<[…]>` zurückliefert. Bei einem temporären DB-Lock-Error (Pool-busy, IO-Glitch) reportet der Probe falschen `false` — die Helfer-Cookies werden invalidiert, obwohl die GV läuft. Fail-Open vs. Fail-Close ist eine bewusste Sicherheitsentscheidung, aber:

1. Der Kommentar im Code lügt: er sagt die TX wird in `find_by_id` consumed — das ist korrekt fürs Default-Impl, aber dann gibt es kein commit/rollback. Tower-Sessions kann dann pool-Connections leaken.
2. „Best-effort" + „swallow errors" ist exakt das Anti-Pattern, das CLAUDE.md unter „Quality" warnt: unsichtbare Failures.

**Fix:**
Tracing-Log auf Error-Pfad hinzufügen:
```rust
async fn is_open(&self, assembly_id: uuid::Uuid) -> bool {
    let tx = match self.transaction_dao.use_transaction(None).await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::warn!(error = ?e, ?assembly_id, "is_open: transaction acquire failed; treating as closed");
            return false;
        }
    };
    match self.assembly_dao.find_by_id(assembly_id, tx).await {
        Ok(Some(a)) => a.status == AssemblyStatus::Open,
        Ok(None) => false,
        Err(e) => {
            tracing::warn!(error = ?e, ?assembly_id, "is_open: assembly lookup failed; treating as closed");
            false
        }
    }
}
```

---

### WARNING-07: `MockSessionServiceImpl::extract_auth_context` parsed Cookie-Format das Production nie setzt

**File:** `genossi_service_impl/src/session.rs:1066-1105`

**Issue:**
Der mock erkennt `helper:<assembly_uuid>:<token_id>` und wandelt das in `AuthContext::Helper` um. Aber der Production-Redeem-Flow (`genossi_rest/src/helper_token.rs:316-322`) setzt im Cookie `app_session=<session_id>` mit `session_id` = einer realen UUID, nicht im `helper:`-Prefix-Format.

Konsequenz: Die mock-Cookie-Erkennung ist toter Code in der echten Pipeline. Sie wird nur in den Plan-02-08-E2E-Tests aktiviert, die die Cookies künstlich konstruieren (`e2e_tests.rs:9100`: `format!("app_session=helper:{}:{}", ...)`) und nie tatsächlich an den Server senden (Z. 9100 setzt eine `_helper_cookie`-Variable mit `_`-Prefix, ungenutzt).

**Fix:**
Entweder:
1. Den Production-Redeem-Handler den `helper:`-Prefix-Cookie setzen lassen (DTO-Schema-Changes nötig — die OIDC-Build-`SessionServiceImpl::extract_auth_context` discriminiert via JSON-Claims, nicht via Cookie-Prefix; das Pattern wäre dann inkonsistent zwischen Builds).
2. Den `MockSessionServiceImpl` analog zur `SessionServiceImpl` machen: Helper-Discrimination via Claims-JSON statt Cookie-Prefix. Die mock-Persister (Plan 02-08) setzt die Claims schon korrekt (Z. 1010-1019).

Empfehlung: (2). Dann kann die Cookie-Prefix-Sniff-Logik entfernt werden und beide Builds verwenden denselben Discriminator-Pfad.

---

### WARNING-08: REST-Layer dupliziert Status-Mapping-Logik

**File:** `genossi_rest/src/helper_token.rs:172-180, 234-240`
**File:** `genossi_rest_types/src/lib.rs:1192-1199`

**Issue:**
Die D-02-Status-Derivation (`if revoked_at.is_some() { Revoked } else if used_at.is_some() { Used } else { Open }`) ist dreimal implementiert:
- `genossi_rest/src/helper_token.rs:173-179` (list_helper_tokens)
- `genossi_rest/src/helper_token.rs:234-240` (revoke_helper_token)
- `genossi_rest_types/src/lib.rs:1192-1199` (impl From<HelperTokenEntity>)

Der Listing-Handler arbeitet auf `genossi_service::helper_token::HelperToken` (Domain), kann nicht den `From<&HelperTokenEntity>`-Helper benutzen — daher die Dupplikation. Wenn jemand eines Tages D-02 ändert (z.B. „erst used dominiert revoked"), bleiben die Drei-Stellen-Implementations divergieren.

**Fix:**
```rust
// In genossi_service/src/helper_token.rs (Domain-Type)
impl HelperToken {
    pub fn derived_status(&self) -> HelperTokenStatus {
        if self.revoked_at.is_some() { Revoked }
        else if self.used_at.is_some() { Used }
        else { Open }
    }
}
```
…und alle 3 REST-Stellen rufen das auf.

---

### WARNING-09: `ensure_user_exists` läuft außerhalb der Redeem-TX

**File:** `genossi_service_impl/src/helper_token.rs:402-406`

**Issue:**
```rust
self.permission_dao
    .ensure_user_exists(&helper_user_id, HELPER_USER_PROCESS)
    .await?;
```

`PermissionDao::ensure_user_exists` hat keinen `tx`-Parameter (`genossi_dao/src/permission.rs:23`). Es nutzt eine eigene Pool-Connection. Wenn dieser Call zwischen Phase-1-commit (Z. 380) und `set_session_id` (Z. 423) fehlschlägt (Crash, Pool-Exhausted, Network-Glitch in Phase-3-Migration zu Postgres), bleibt der Token mit `used_at IS NOT NULL` aber `session_id IS NULL`. 

Der Inline-Kommentar (Z. 376-379) erklärt dieses 2-Step-Commit-Window als akzeptabel ("functionally identical to a token whose session was immediately invalidated"). Aber: ohne Helper-User existiert die Session-Row gar nicht, also auch keine Invalidate-Cascade. Der Helper sieht den `app_session=<UUID>`-Cookie im Browser, aber server-side gibt es keine Session und keinen User. Verifying-Request schlägt fehl mit „session not found" — kein recovery.

**Fix:**
Wenn `permission_dao.ensure_user_exists` und/oder `session_service.create_session_with_claims` fehlschlagen, sollte der Token-State (`used_at`) revertiert werden. Idiomatisch: alles in einer einzigen TX, und der `permission_dao.create_session` muss die transaction akzeptieren (API-Erweiterung). Alternative: kompensierende UPDATE-Logik in einem `Drop`-Guard.

Da der vorherige Refactor explizit das 2-Phase-Pattern eingeführt hat (Inline-Kommentar verweist auf RESEARCH Pitfall 3 und sqlx-sqlite-Deadlock-Risk bei nested-pool-acquire), ist die saubere Lösung: `PermissionDao::create_session_with_tx(...)` erweitern, sodass das TX-Sharing möglich wird.

---

### WARNING-10: Public-Helper-Redeem-Endpoint ohne CSRF-Schutz

**File:** `genossi_rest/src/helper_token.rs:280-348`
**File:** `genossi_rest/src/lib.rs:691-692`

**Issue:**
`POST /api/helper/redeem` setzt einen Cookie via Set-Cookie. Ein Browser-Client von einer fremden Origin könnte einen Helfer veranlassen, ein gestohlenes/gefangenes Code-JSON-Body via `<form action="https://genossi.example/api/helper/redeem" method="POST">` (mit JSON statt form-data, was 415-mäßig schwierig ist, aber technisch nicht unmöglich) zu submitten. Der CORS-Layer (`build_cors_layer`) filtert nach Origin, aber `cors::AllowOrigin::list(...)` blockt nur das Browser-Reading der Response — das Cookie-Setting passiert clientseitig auch bei blockierter Response.

Praktisch: SameSite=Strict im Cookie (Z. 318) schützt vor Cookie-Mitschicken, aber NICHT vor Cookie-Setzen durch eine cross-origin-Antwort.

**Fix:**
1. `Content-Type: application/json` verlangen (`axum::Json` macht das implizit per `application/json`-Header-Match — gut). Verifizieren, dass nicht-JSON-Body 415 zurückgibt, was dann auch von dem Form-CSRF-Vector schützt.
2. Optional: einen short-lived „redeem-challenge"-Token im UI-Flow einführen (außerhalb Phase 2 Scope).

Aktuell ist Phase 2 mit dem Rate-Limit (10/min/IP, `lib.rs:509`) und SameSite=Strict ausreichend hart, aber als Warning dokumentieren weil zukünftige Änderungen am Endpoint diese Properties nicht versehentlich brechen sollten.

---

### WARNING-11: `validate_create_helper_token_request` testet nicht die `chars().count() > 256`-Edge

**File:** `genossi_rest/src/helper_token.rs:49-69`
**File:** `genossi_rest/src/helper_token.rs:418-425` (Tests)

**Issue:**
`memo.chars().count() > 256` prüft Unicode-Codepoints, nicht Bytes oder Grapheme-Cluster. Ein Memo mit 256 Codepoints, aber 1024 Bytes (z.B. CJK), passiert die Validation. Die DB-Spalte `memo TEXT NOT NULL` (Migration Z. 14) hat keine Längenbegrenzung, aber Audit-Log und Vorstand-UI könnten überschreiten.

Der Test `test_validate_create_helper_token_request_too_long_memo` (Z. 428) testet `"a".repeat(257)` — also 257 ASCII-Bytes. Es testet KEIN Unicode (z.B. 256 ✓-Bytes, oder 257 ASCII-Zeichen mit 1 Whitespace am Ende, was nach trim wieder 256 wäre).

**Fix:**
Test-Suite erweitern:
```rust
#[test]
fn test_validate_memo_with_trim_at_boundary() {
    let body = CreateHelperTokenRequest { memo: format!("{}  ", "a".repeat(256)) };
    // After trim: 256 chars → valid
    assert!(validate_create_helper_token_request(&body).is_ok());
}

#[test]
fn test_validate_memo_unicode_bytes_vs_chars() {
    // 256 4-byte CJK chars = 1024 bytes
    let body = CreateHelperTokenRequest { memo: "宇".repeat(256) };
    assert!(validate_create_helper_token_request(&body).is_ok());
    // 257 chars must fail
    let body = CreateHelperTokenRequest { memo: "宇".repeat(257) };
    assert!(validate_create_helper_token_request(&body).is_err());
}
```

Optional: byte-length cap zusätzlich („max 1024 bytes") falls DB/UI-Constraints existieren.

---

_Reviewed: 2026-05-03_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
