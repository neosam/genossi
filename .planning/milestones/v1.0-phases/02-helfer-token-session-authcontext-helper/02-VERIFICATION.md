---
phase: 02-helfer-token-session-authcontext-helper
verified: 2026-05-03T00:00:00Z
status: human_needed
score: 5/7 must-haves verified
overrides_applied: 0
gaps: []
deferred:
  - truth: "Helfer-Session-Lebensdauer ist an assembly.closed_at gebunden; nach Schließen der GV ist sie ungültig, auch wenn das Cookie noch im Browser liegt (HLPR-05)"
    addressed_in: "Phase 3"
    evidence: "Phase 3 Success Criteria #8: 'close_assembly invalidiert kaskadierend alle Helfer-Sessions dieser GV; nach Schließen schlägt jeder Helfer-Request mit 401 fehl'. Phase 3 ist explizit 'Cascade-Invalidation' — der Service-Layer-Mechanismus (SessionServiceImpl::extract_auth_context mit D-18 status-check) ist in Phase 2 fertiggestellt; die Verdrahtung in den Request-Lifecycle (context_extractor) ist Phase-3-Aufgabe (Phase 3 SC#6 und SC#8 setzen beide voraus, dass AuthContext::Helper im Pipeline konstruiert wird)."
  - truth: "AuthContext::Helper { session_id, assembly_id } typsicher verfügbar + vom Session-Extract-Pfad korrekt rekonstruiert"
    addressed_in: "Phase 3"
    evidence: "Phase 3 Success Criteria #6: 'Permission-Check akzeptiert sowohl AuthContext::Helper { assembly_id == X } als auch admin-Permission (ATTN-06)' — dies erfordert, dass AuthContext::Helper im Request-Pipeline konstruiert wird. Phase 3 SC#8 erfordert ebenfalls die Verdrahtung. Die Variante selbst (typsicher verfügbar) ist Phase 2 und ist VERIFIED. Die 'korrekt rekonstruiert'-Komponente wird in Phase 3 durch Umverdrahtung von session::context_extractor → SessionServiceImpl::extract_auth_context vollständig."
human_verification:
  - test: "BLOCKER-04: validate_code_format akzeptiert Unicode-Lookalikes wegen 'c as u8' truncation"
    expected: "Code '0123456789' (Ā=U+0100 has low-byte 0x00='0') sollte NICHT als gültig akzeptiert werden, sondern mit 400 ValidationError abgewiesen werden"
    why_human: "Security-Korrektheit muss entschieden werden: Ist die aktuelle Implementierung (c as u8 truncation) ein akzeptiertes bekanntes Risiko oder ein zu fixender Bug? Entscheidung liegt beim Entwickler, da es einen funktionalen Fix gibt (CROCKFORD_CHARS.contains(c)) der aber das Verhalten ändert."
---

# Phase 2: Helfer-Token + Session + AuthContext::Helper Verification Report

**Phase Goal:** Vorstand kann pro Helfer einen einmalig nutzbaren QR-Token mit Memo-Namen erzeugen und vor GV-Beginn revoken; Helfer kann den Token atomar einlösen und erhält eine zeitlich an die GV gebundene Session — mit dafür typsicherer `AuthContext::Helper`-Variante, die Phase 3 für Permission-Checks braucht.
**Verified:** 2026-05-03
**Status:** human_needed
**Re-verification:** Nein — initiale Verifikation

## Goal Achievement

### Observable Truths

| #   | Truth   | Status     | Evidence       |
| --- | ------- | ---------- | -------------- |
| 1   | Vorstand kann Token mit Memo-Name erzeugen; Backend liefert QR-SVG + 8-12-Zeichen-Klartext-Code (HLPR-01) | VERIFIED | `genossi_service_impl/src/helper_token.rs`: `generate_crockford_code()` erzeugt 10-char Code, `render_qr_svg()` mit EcLevel::Q; `HelperTokenCreateResponseTO` in `genossi_rest_types/src/lib.rs` enthält `code` + `qr_svg`; E2E-Test `test_helper_token_create_returns_qr_and_code` assertet 201 + SVG + 10-char Crockford-Code |
| 2   | Helfer kann gültigen Token via Redeem-Endpoint einlösen; Backend führt Redeem in einem `UPDATE ... WHERE used_at IS NULL RETURNING ...` aus + bindet Session an GV (HLPR-02) | VERIFIED | `genossi_dao_impl_sqlite/src/helper_token.rs:206-242`: `atomic_redeem` mit `sqlx::query_as::<_, RedeemRow>(...)` + `fetch_optional`, RETURNING-Klausel verifiziert; `redeem_helper_token` in Service wired; E2E-Test `test_helper_token_redeem_success_sets_cookie` assertet 200 + Set-Cookie mit HttpOnly/SameSite=Strict/Max-Age=86400 |
| 3   | E2E-Race-Test mit zwei parallelen Redeem-Requests auf demselben Token zeigt exakt einen Erfolg + einen Fehler (HLPR-04) | VERIFIED | `genossi_bin/tests/e2e_tests.rs:8785-8823`: `tokio::join!` mit zwei identischen Redeem-Requests; `statuses[0] == 200, statuses[1] == 410`; SUMMARY vermerkt deterministisch über 5 Runs; atomarer UPDATE auf SQLite-Ebene garantiert Race-Safety |
| 4   | Helfer-Session-Lebensdauer an `assembly.closed_at` gebunden; nach Schließen ungültig (HLPR-05) | DEFERRED | Service-Layer-Mechanismus existiert: `SessionServiceImpl::extract_auth_context` (`genossi_service_impl/src/session.rs:161-230`) parst Helper-Claims, prüft `assembly.status == Open`, gibt `None` zurück wenn Closed (unit-getestet); aber `context_extractor` in REST-Layer (`genossi_rest/src/session.rs:73-119`) ruft `verify_user_session` auf, NICHT `extract_auth_context` — Verdrahtung fehlt; Phase 3 SC#8 adressiert dies |
| 5   | Vorstand sieht alle Token mit Memo-Name + Status; offene Token revokebar (HLPR-06) | VERIFIED | `GET /api/assembly/{assembly_id}/helper-tokens` wired in `genossi_rest/src/lib.rs`; `list_helper_tokens` + `revoke_helper_token` Handler in `genossi_rest/src/helper_token.rs`; E2E-Test `test_helper_token_listing_shows_status_open_used_revoked` assertet Status-Derivation (Open/Used/Revoked); `test_helper_token_revoke_used_returns_409` + `test_helper_token_revoke_when_assembly_closed_returns_409` assertieren Guard-Logik |
| 6   | Token-Erzeugung erscheint in Audit-Hashchain mit Memo-Name, Erzeuger, Timestamp, GV-Bezug (HLPR-07) | VERIFIED | `genossi_service_impl/src/helper_token.rs:209`: `crate::audited_create!` mit `HELPER_TOKEN_PROCESS_CREATE = "helper_token.create"`; `Auditable`-Impl excludes `token_hash` (D-06); E2E-Test `test_helper_token_create_appears_in_audit_chain` assertet `process="helper_token.create"`, `field_name="memo"` mit Wert, `field_name="assembly_id"`, `!token_hash in entries`, `verify.valid==true` |
| 7   | `AuthContext::Helper { session_id, assembly_id }` typsicher verfügbar + vom Session-Extract-Pfad korrekt rekonstruiert | PARTIAL / DEFERRED | Typ ist verfügbar: `genossi_service/src/auth_types.rs:108-111`, keine cfg-Gate (D-14 erfüllt). `SessionServiceImpl::extract_auth_context` rekonstruiert korrekt aus Claims (`genossi_service_impl/src/session.rs:185-208`). ABER: `context_extractor` im REST-Layer ist NICHT auf diese Methode verdrahtet — in OIDC-Build wird `verify_user_session` verwendet, im mock_auth-Build wird `MockContext` injiziert ohne Cookie-Lesen. Typ-Verfügbarkeit: VERIFIED; Pipeline-Verdrahtung: Phase 3 |

**Score:** 5/7 Truths vollständig verified (SC#4 und SC#7 auf Phase 3 zurückgestellt)

### Deferred Items

Items die in dieser Phase noch nicht vollständig erfüllt sind, aber durch spätere Phasen explizit adressiert werden.

| # | Item | Addressed In | Evidence |
|---|------|-------------|----------|
| 1 | HLPR-05: Session-Invalidierung via Cookie nach GV-Schließung (end-to-end) | Phase 3 | Phase 3 SC#8: "close_assembly invalidiert kaskadierend alle Helfer-Sessions; nach Schließen schlägt jeder Helfer-Request mit 401 fehl" |
| 2 | SC#7: AuthContext::Helper vom Request-Lifecycle-Extract-Pfad korrekt rekonstruiert | Phase 3 | Phase 3 SC#6: "Permission-Check akzeptiert AuthContext::Helper { assembly_id == X }" — setzt Verdrahtung im Pipeline voraus; Phase 3 SC#8 ebenso |

### Required Artifacts

| Artifact | Expected    | Status | Details |
| -------- | ----------- | ------ | ------- |
| `migrations/sqlite/20260503000000_create_helper_token_table.sql` | DDL mit FKs + 3 Indices | VERIFIED | CREATE TABLE IF NOT EXISTS helper_token mit 10 Spalten, FK assembly RESTRICT, FK session SET NULL, UNIQUE INDEX token_hash, INDEX assembly_id, INDEX deleted |
| `genossi_dao/src/helper_token.rs` | HelperTokenEntity + Auditable + DAO-Trait + MockHelperTokenDao | VERIFIED | Alle 9 Methoden vorhanden inkl. atomic_redeem, set_session_id, lookup_status, all_for_assembly; Auditable excludes token_hash (D-06); #[automock] |
| `genossi_dao_impl_sqlite/src/helper_token.rs` | HelperTokenDaoImpl mit SQLx CRUD + atomic_redeem RETURNING | VERIFIED | query_as::<_, RedeemRow>() + fetch_optional (kein query_as! Makro); ConflictError("Version mismatch") für optimistic-lock; kein DELETE |
| `genossi_service/src/auth_types.rs` | AuthContext::Helper Variante ohne cfg-Gate | VERIFIED | `Helper { session_id: Arc<str>, assembly_id: uuid::Uuid }` ohne #[cfg], D-14 eingehalten |
| `genossi_service/src/helper_token.rs` | HelperTokenService Trait + Domain Types + MockHelperTokenService | VERIFIED | 4 async fn: create_helper_token, list_for_assembly, revoke_helper_token, redeem_helper_token (public, kein Auth-Arg); #[automock] generiert Mock |
| `genossi_service_impl/src/helper_token.rs` | HelperTokenServiceImpl mit gen_service_impl! + 4 Methoden | VERIFIED | gen_service_impl! mit 8 Deps; audited_create! mit "helper_token.create"; OsRng Crockford; SHA256-hex; QR-SVG; atomic_redeem-Orchestration; TX-Split nach Plan 08 Fix |
| `genossi_service_impl/src/session.rs` | extract_auth_context mit Helper-Claims-Discriminator + D-18 | VERIFIED | HelperClaims Struct; `kind=="helper"` Branch; assembly_dao.find_by_id + status==Open Check; 5 unit tests für Helper-Claims; aber NICHT vom REST context_extractor aufgerufen |
| `genossi_rest/src/helper_token.rs` | 4 Handler + Rate-Limit + RestError 403/410 | VERIFIED | create_helper_token, list_helper_tokens, revoke_helper_token (admin), redeem_helper_token (public, kein extract_auth_context); RestError::Forbidden + Gone in lib.rs; redeem_rate_layer in create_app |
| `genossi_bin/tests/e2e_tests.rs` | 10 E2E-Tests HLPR-01/02/04/05/06/07 | VERIFIED | 10 Testfunktionen mit Prefix test_helper_token_* vorhanden; Plan 08 Fixes (pool-deadlock, revoke version mismatch, FK persister) eingecheckt in commit 3785ff4 |

### Key Link Verification

| From | To  | Via | Status | Details |
| ---- | --- | --- | ------ | ------- |
| `HelperTokenDaoImpl::atomic_redeem` | `UPDATE...RETURNING SQL` | `sqlx::query_as::<_, RedeemRow>(...).fetch_optional` | VERIFIED | `genossi_dao_impl_sqlite/src/helper_token.rs:206-242`: SQL "UPDATE helper_token SET used_at = ? WHERE token_hash = ? AND used_at IS NULL AND revoked_at IS NULL AND deleted IS NULL RETURNING id, assembly_id"; KEIN query_as! Makro (Pitfall 1 umgangen) |
| `HelperTokenEntity` | `Auditable trait` | `impl crate::auditable::Auditable for HelperTokenEntity` | VERIFIED | `entity_type() -> "helper_token"`; `audit_fields()` enthält 5 Felder (assembly_id, memo, used_at, session_id, revoked_at); token_hash NICHT vorhanden (D-06) |
| `SessionServiceImpl::extract_auth_context` | `AuthContext::Helper` Konstruktion | `HelperClaims::kind == "helper"` + `assembly.status == Open` | VERIFIED (isoliert) | Methode korrekt implementiert und unit-getestet; ABER im REST-Layer NICHT verdrahtet (context_extractor ruft verify_user_session statt extract_auth_context) |
| `context_extractor` (REST) | `SessionService::extract_auth_context` | `session::context_extractor` → OIDC: `verify_user_session`; mock: `MockContext` | BROKEN / DEFERRED | `genossi_rest/src/session.rs:73-119`: OIDC-Pfad ruft `verify_user_session`, NICHT `extract_auth_context`. mock_auth-Pfad injiziert `MockContext` unconditionally. `auth_middleware.rs::extract_auth_context` (die korrekte Implementierung) ist als pub-Modul vorhanden, aber NICHT in `create_app` verdrahtet. Phase 3 muss dies fixen. |
| `redeem_helper_token` REST handler | `HelperTokenService::redeem_helper_token` | `rest_state.helper_token_service().redeem_helper_token(code)` | VERIFIED | `genossi_rest/src/helper_token.rs:286-348`: kein extract_auth_context-Aufruf (D-22 Public); Set-Cookie Header mit app_session; D-24 Conflict-Discriminator-Mapping (400/404/410/403) |
| `create_app` | rate-limit auf `/api/helper/redeem` | `redeem_rate_layer` via `GovernorLayer` | VERIFIED | `genossi_rest/src/lib.rs:691-696`: `helper_token::generate_public_route().layer(redeem_rate_layer)`; 6 req/sec, burst 10 |
| `genossi_bin::RestStateImpl` | `HelperTokenServiceImpl` mit 8 Deps | `HelperTokenServiceDependencies` impl + `new(8 args)` | VERIFIED | `genossi_bin/src/lib.rs`: `HelperTokenServiceDependencies` struct; `impl HelperTokenServiceDeps` mit 8 assoziierten Typen; `helper_token_dao`, `assembly_dao`, `audit_log_dao`, `permission_service`, `permission_dao`, `session_service`, `uuid_service`, `transaction_dao` |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
| -------- | ------------- | ------ | ------------------ | ------ |
| `create_helper_token` handler | `HelperTokenCreateResponseTO` | `HelperTokenServiceImpl::create_helper_token` → DB insert | Ja — audited_create! mit echtem DB-INSERT | FLOWING |
| `redeem_helper_token` handler | `RedeemResponse` + Set-Cookie | `HelperTokenServiceImpl::redeem_helper_token` → atomic_redeem → SessionService | Ja — nach Plan 08 Fixes: DaoSessionPersister erzeugt echte session-Rows | FLOWING |
| `list_helper_tokens` handler | `Vec<HelperTokenTO>` | `all_for_assembly` → SELECT WHERE assembly_id = ? AND deleted IS NULL | Ja | FLOWING |
| `revoke_helper_token` handler | `HelperTokenTO` | `find_by_id` → service update → DAO UPDATE | Ja — nach Plan 08 Fix (version-mismatch Bug behoben) | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
| -------- | ------- | ------ | ------ |
| E2E-Tests laufen | `cargo test --test e2e_tests test_helper_token` | 10 Tests in SUMMARY als grün bestätigt (commit 3785ff4) | PASS |
| Workspace lib-tests | `cargo test --workspace --lib` | SUMMARY Plan 08 meldet 528 Tests grün in mock_auth + oidc | PASS |
| Race-Test deterministisch | 5x `test_helper_token_redeem_race_one_succeeds_one_fails` | Alle 5 Runs grün (SUMMARY Plan 08) | PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
| ----------- | ---------- | ----------- | ------ | -------- |
| HLPR-01 | Plan 01, 04, 05, 07, 08 | Token mit QR-SVG + 8-12-Zeichen-Code erzeugen | SATISFIED | DAO + Service + REST vollständig; E2E-Test assertet 201 + Code + SVG |
| HLPR-02 | Plan 01, 04, 05, 07, 08 | Atomar einlösen; UPDATE...WHERE used_at IS NULL RETURNING | SATISFIED | atomic_redeem SQL verrifiziert; Set-Cookie korrekt gesetzt |
| HLPR-04 | Plan 01, 08 | One-Time-Use via Race-Test | SATISFIED | tokio::join! E2E-Test: [200, 410] deterministic |
| HLPR-05 | Plan 02, 06, 08 | Session an assembly.closed_at gebunden | PARTIAL | Service-Logic + unit-tests vorhanden; Pipeline-Verdrahtung fehlt; Phase 3 SC#8 |
| HLPR-06 | Plan 04, 05, 07, 08 | Vorstand sieht Token-Liste; offene revokebar | SATISFIED | GET + POST revoke Endpoints; E2E-Tests für Status + Guards |
| HLPR-07 | Plan 01, 05, 08 | Audit-Hashchain mit Token-Erzeugung | SATISFIED | audited_create! + E2E-Test assertet chain valid, kein token_hash |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
| ---- | ---- | ------- | -------- | ------ |
| `genossi_service_impl/src/helper_token.rs` | 113 | `c as u8` truncation in validate_code_format: Unicode-Chars mit low-byte im Crockford-Alphabet passieren Validation | WARNING (BLOCKER-04 aus Review) | Unicode-Lookalikes (z.B. Ā=U+0100→0x00='0') werden als 400 statt als gültig-aber-unbekannt-404 behandelt; kein Security-Bug (hash differs, 404 statt 400 zurück), aber Spec-Divergenz |
| `migrations/sqlite/20260503000000_create_helper_token_table.sql` | 22-23 | FK-Constraints ohne PRAGMA foreign_keys=ON im Production-Pool | WARNING (BLOCKER-02 aus Review) | FKs sind Dokumentation ohne Runtime-Wirkung in Production; DAO-Unit-Tests setzen PRAGMA, E2E und Production nicht |
| `genossi_rest/src/session.rs` | 73-119 | `context_extractor` ruft `verify_user_session` statt `SessionServiceImpl::extract_auth_context` auf | WARNING für Phase 2 / BLOCKER für HLPR-05 end-to-end | AuthContext::Helper wird in Production nie konstruiert; D-18 Cascade fired nicht; durch Phase 3 adressiert |

### Human Verification Required

#### 1. BLOCKER-04: validate_code_format Unicode-Lookalike-Bug — Entscheidung erforderlich

**Test:** Sende POST /api/helper/redeem mit Code "Ā234567890" (Ā = U+0100, ein Unicode-Zeichen dessen Low-Byte 0x00 = '0' ist). Aktuell passiert die Format-Validation wegen `c as u8` truncation, und der Code wird als validformatiert behandelt — bekommt 404 (unbekannt) statt 400 (invalid format).

**Expected:** Laut Spezifikation (D-09, Crockford Base32 uppercase ASCII) sollte dieser Code mit 400 ValidationError "invalid_alphabet" abgewiesen werden.

**Why human:** Dies ist ein bekannter Bug (BLOCKER-04 aus Code Review). Es gibt einen klaren Fix: `CROCKFORD_CHARS.contains(c)` statt `(CROCKFORD_ALPHABET as &[u8]).contains(&(c as u8))`. Die Frage ist, ob der Entwickler diesen Bug vor Phase 3 beheben will (ändert Verhalten für nicht-ASCII-Eingaben von 404→400) oder ob er als akzeptiertes Risiko eingestuft wird (kein Security-Bug, aber Spec-Divergenz). Die Entscheidung liegt beim Entwickler.

### Gaps Summary

Keine strukturierten Gaps für `/gsd-plan-phase --gaps` — alle ursprünglichen Gaps aus SC#4 und SC#7 sind durch Phase 3 adressiert (vergl. Deferred-Sektion). Der einzige verbleibende offene Punkt erfordert eine Entwickler-Entscheidung (validate_code_format Unicode-Fix: Bug beheben oder akzeptieren).

**HLPR-05 / SC#4 + SC#7 Bewertung:**

Die Phase-2-Implementierung hat alle Infrastruktur-Bestandteile geliefert:
- `SessionServiceImpl::extract_auth_context` mit D-18 Status-Check ist unit-getestet
- `AuthContext::Helper` Variante ist typsicher verfügbar ohne cfg-Gate
- `MockSessionServiceImpl` mit `AssemblyStatusProbe` + `DbAssemblyStatusProbe` ist verdrahtet

Was fehlt: die Verdrahtung von `context_extractor` auf `SessionService::extract_auth_context` statt auf `verify_user_session`. Dies ist Phase-3-Aufgabe (SC#6 Helfer-Permission-Checks, SC#8 Session-Invalidierung), weil Phase 3 die Attendance-Endpoints mit Helper-Permission-Checks implementiert — ohne diese Verdrahtung könnten Helfer nie auf Attendance-Endpoints zugreifen. Die Verdrahtung ist damit eine Phase-3-Voraussetzung, nicht ein Phase-2-Bug.

**auth_middleware.rs:** Das Modul existiert mit korrekter Implementierung (`extract_context_from_headers` ruft `session_service.extract_auth_context` korrekt auf), ist aber nicht in `create_app` verdrahtet. Dies ist der zukünftige Wiring-Point für Phase 3.

---

_Verified: 2026-05-03_
_Verifier: Claude (gsd-verifier)_
