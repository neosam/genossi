# Phase 2: Helfer-Token + Session + AuthContext::Helper - Context

**Gathered:** 2026-05-03
**Status:** Ready for planning

<domain>
## Phase Boundary

Backend-Aggregat für Helfer-Authentifizierung. Vorstand erzeugt pro Helfer ein `helper_token`-Aggregat mit Memo-Name und kriegt Klartext-Code (8–12 alphanumerisch, in Phase 2 fix 10 Zeichen Crockford Base32) plus QR-SVG zurück. Helfer löst den Token atomar via `UPDATE ... WHERE used_at IS NULL RETURNING`-SQL ein und erhält eine Session, die an die Generalversammlung gebunden ist. `AuthContext::Helper { session_id, assembly_id }` wird als typsichere Enum-Variante eingeführt — Phase 2 verdrahtet sie in `extract_auth_context`, gibt aber im `PermissionService` ausschließlich `PermissionDenied` zurück; die Logik dahinter (Attendance-Endpoints) kommt in Phase 3.

**Phase 2 liefert NICHT:**
- Cascade-Invalidation der Helfer-Sessions in `close_assembly()` (Phase 3 — Hook wird dort gebaut, sobald `HelperSessionService`-Cascade-Path existiert; HLPR-05 in Phase 2 wird über Status-Check beim Verify abgedeckt, siehe D-09)
- Manual-Code-Eingabe-UI (Phase 4 / HLPR-03 — Phase 2 muss aber sicherstellen, dass derselbe Redeem-Endpoint Klartext-Code akzeptiert, sodass Phase-4 nur die UI baut)
- Frontend für QR-Generierung, -Druck oder Listing (Phase 4)
- Attendance-Endpoints, reduzierter Member-View (Phase 3)
- Live-Stats-Endpoint `/api/assembly/{id}/stats` (Phase 3, ASSY-04)
- Permission-Logik für `AuthContext::Helper` jenseits eines `PermissionDenied`-Stubs (Phase 3)

</domain>

<decisions>
## Implementation Decisions

### Token-Schema & Lifecycle
- **D-01:** Tabelle: `helper_token`. Felder: `id` (BLOB UUID), `assembly_id` (FK auf `assembly.id`, RESTRICT — Soft-Delete-Konvention), `memo` (TEXT, Freitext-Memo-Name), `token_hash` (TEXT, SHA256(code) hex/base64), `created` (PrimitiveDateTime), `used_at` (Option<PrimitiveDateTime>), `session_id` (Option<TEXT>, FK zu `session.id` ON DELETE SET NULL — bleibt sichtbar im Token-Listing als „eingelöst" auch nach Session-Cleanup), `revoked_at` (Option<PrimitiveDateTime>), `deleted` (Option<PrimitiveDateTime>, reserviert wie bei `assembly`), `version` (UUID, optimistic locking).
- **D-02:** Status `Open`/`Used`/`Revoked` wird **abgeleitet** aus den Spalten — keine eigene Status-Spalte. Begründung: hält den atomaren Redeem-UPDATE minimal (`WHERE used_at IS NULL AND revoked_at IS NULL`); kein Sync-Risiko zwischen Status-Spalte und State-Spalten.
- **D-03:** Revoke eines bereits eingelösten Tokens ist verboten → 409 Conflict. Service-Layer prüft `used_at IS NULL` vor dem Revoke-UPDATE; Cascade auf die zugehörige Session entfällt damit (sie wird nie zusammen mit Revoke „herrenlos" — Phase 3 invalidiert sie über `close_assembly`-Cascade ODER über Long-Lived-Expires-Timeout).
- **D-04:** `helper_token` führt einen `deleted: Option<PrimitiveDateTime>` (Soft-Delete-Slot) ohne Delete-Pfad in Phase 2 — analog zu Phase-1-Konvention für `assembly` (siehe `genossi_service_impl/src/assembly.rs:11-23`). Keine `audited_delete!`-Aufrufe, kein DELETE-Endpoint. Slot reserviert für eine spätere Cleanup-Phase ohne erneute Migration.
- **D-05:** Soft-Delete-Filter — alle DAO-Queries filtern `deleted IS NULL` analog zu bestehenden Aggregaten. `find_by_token_hash` (für Redeem) und `all_for_assembly` (für Listing) müssen das einhalten.
- **D-06:** `helper_token` implementiert den `Auditable`-Trait (`genossi_dao/src/auditable.rs`). `entity_type() = "helper_token"`. `audit_fields()` enthält **nicht** `token_hash` (würde den Klartext-Hash sichtbar machen — minimale Informationsleckage), wohl aber `assembly_id`, `memo`, `revoked_at`. id/version/created/deleted ausgeschlossen wie bei Member/Application.

### Audit-Strategie
- **D-07:** Nur die **Token-Erzeugung** wird auditiert (HLPR-07-konform). Process-Identifier: `"helper_token.create"` (Punkt-Notation analog zu `assembly.create`/`assembly.open` aus Phase 1, D-11). Aufruf via `audited_create!`-Macro.
- **D-08:** Redeem und Revoke werden **nicht** in die Audit-Hashchain geschrieben. Redeem ist eine Helfer-Aktion ohne Vorstands-Initiative; Revoke wird nicht explizit von HLPR gefordert. Falls in Phase-3-Audit-Review nachträglich gefordert, wird die Architektur kein Hindernis sein (`audited_update!` wäre ein One-Liner).

### Klartext-Code & QR
- **D-09:** Klartext-Code-Format: **fix 10 Zeichen, Crockford Base32** (Alphabet `0123456789ABCDEFGHJKMNPQRSTVWXYZ`, ohne Verwechslungs-Zeichen `0/O`, `1/I/L`, `U`). Entropie ≈ 50 bit, weit über Brute-Force-Schwelle bei Rate-Limiting via `tower_governor` (in Genossi-Stack vorhanden). Konstante Länge erlaubt einfache Frontend-Validation in Phase 4 (Manual-Code-Fallback) und einen statischen Backend-Length-Check.
- **D-10:** Klartext-Generierung: Cryptographically-secure RNG (`rand::rngs::OsRng` oder Äquivalent). Liegt nicht im `UuidService`; ein dedizierter `TokenGenerator`-Trait/Service ist optional (Plan entscheidet — könnte auch eine freie Funktion in `genossi_service_impl/src/helper_token.rs` sein). Rückgabe-Type: `Arc<str>` oder `String`.
- **D-11:** Token-Speicherung: `SHA256(code)` als Hex-String (lowercase) in `token_hash`. Klartext **nirgends** persistiert. Klartext wird **einmalig** im Create-Response zurückgegeben (Field `code: String` in `HelperTokenCreateResponseTO`). Salting nicht nötig: 50-bit-Entropie auf einer Crockford-Base32-Pre-Image macht Rainbow-Tables irrelevant. Plan/Researcher entscheidet finale Hex-vs-Base64-Encoding-Detail.
- **D-12:** QR-Inhalt: **URL mit Code als Query-Parameter** — `${APP_URL}/helper?code=ABC1234567`. APP_URL kommt aus der Env (existiert bereits in OIDC-Setup, siehe `genossi_rest/src/lib.rs`). Wenn `APP_URL` nicht gesetzt ist, fail fast beim Server-Start oder beim ersten Token-Create — die Konvention für Default ist Plan-Detail.
- **D-13:** QR-Crate: **`qrcode` 0.14** (de-facto-Standard im Rust-Ökosystem, MIT/Apache, pure-Rust). Output: SVG-String. EC-Level: `EcLevel::Q` (gut für gedruckte QR-Codes). Rückgabe als String-Feld `qr_svg: String` in `HelperTokenCreateResponseTO`.

### AuthContext::Helper-Wiring & Session
- **D-14:** Neue `AuthContext`-Variante: `AuthContext::Helper { session_id: Arc<str>, assembly_id: uuid::Uuid }`. Wird in `genossi_service/src/auth_types.rs` neben `Mock` und `Oidc` ergänzt. Variante existiert in beiden Feature-Builds (`mock_auth`, `oidc`) — keine Feature-Gate.
- **D-15:** Cookie-Pfad: **Reuse von `app_session`** (bestehender Cookie aus `genossi_rest/src/auth_middleware.rs:142`). Helfer-Session ist ein Eintrag in der existierenden `session`-Tabelle mit `claims = JSON({"helper": true, "assembly_id": "<uuid>"})`. `extract_auth_context` parst die Claims und gibt entweder `AuthContext::Helper {...}` oder die bisherige Mock/Oidc-Variante zurück. Keine Cookie-Doppel-Pfade.
- **D-16:** Claims-Payload-Schema (JSON, wird im DB-Feld `session.claims` gespeichert):
  ```json
  { "kind": "helper", "assembly_id": "<uuid-string>" }
  ```
  Das `kind`-Discriminator-Feld erlaubt Future-Erweiterung (z.B. später `kind: "vorstand-impersonation"`). `extract_auth_context` schaltet auf `kind` um — fehlt das Feld, fällt es auf den bisherigen User-Session-Pfad zurück.
- **D-17:** `user_id` in der `session`-Tabelle: **synthetischer User pro Token** — `helper:<token_id_uuid>`. Auto-Registrierung via bestehendem `permission_dao.ensure_user_exists("helper:<token_id>", "helper-token-redeem")` (Pattern aus `SessionServiceImpl::ensure_user_and_create_session_with_claims`, das schon für „inventur token" existiert). Vorteile: `revoke_all_for_user` funktioniert pro-Helfer; spätere Audit-Spuren in Phase-3-Attendance-Aktionen sind eindeutig zuzuordnen.
- **D-18:** Session-Lebensdauer: **Long-lived `expires` (24h ab Redeem) + zusätzlicher Status-Check beim Verify**. Bei jedem `verify_user_session`-Aufruf, der einen Helfer-Session-Eintrag findet, wird zusätzlich geprüft, dass `assembly.status == Open` ist (DAO-Lookup gegen `assembly_dao.find_by_id(assembly_id)`). Wenn nicht: Session wird invalidiert (DELETE) und `verify_user_session` gibt `None` zurück. Dies erfüllt HLPR-05 (SC#4) **schon in Phase 2** — testbar via E2E-Test, der nach `close_assembly` einen Helfer-Request macht und 401 erwartet. Phase 3 kann als Optimierung den Cascade-Invalidate-Hook in `close_assembly` ergänzen (proaktive Cleanup), aber Phase 2 ist nicht darauf angewiesen.
- **D-19:** Wo der Status-Check verdrahtet ist (im `SessionService::verify_user_session` direkt vs. in einem neuen `HelperSessionService`-Wrapper vs. im `extract_auth_context` der Auth-Middleware), ist **Claude's Discretion** im Plan. Kriterien: minimaler Eingriff in den bestehenden generischen `SessionService`, klare Schicht-Trennung, Test-Isolierbarkeit.
- **D-20:** `PermissionService::check_permission` mit `AuthContext::Helper` gibt in Phase 2 **explizit `Err(ServiceError::PermissionDenied)`** zurück. Phase 3 ergänzt die positive Branch (Helfer darf Attendance-Endpoints aufrufen, wenn `assembly_id` matched). Begründung: kein versehentlicher Helfer-Zugriff auf Member-CRUD oder Assembly-Lifecycle.

### REST-Endpoint-Vertrag
- **D-21:** Routen für Token-Erzeugung und -Listing **nested unter Assembly** (konsistent mit Phase-1-Pattern):
  - `POST /api/assembly/{assembly_id}/helper-tokens` — Token erzeugen, Body `{memo: "Anna"}`, Response `{token: HelperTokenTO, code: "ABC1234567", qr_svg: "<svg>...</svg>"}`. Nur einmal im Response, nirgends gespeichert.
  - `GET /api/assembly/{assembly_id}/helper-tokens` — Listing aller Tokens dieser Assembly (für Vorstand-UI), `Vec<HelperTokenTO>` mit Status-Feld (`Open`/`Used`/`Revoked`, abgeleitet im TO-Mapping aus den Spalten).
  - `POST /api/assembly/{assembly_id}/helper-tokens/{token_id}/revoke` — Revoke offener Token; nur erlaubt wenn `used_at IS NULL`; in Status `Preparation` UND `Open` der Assembly erlaubt (siehe D-23).
  - Alle drei: erfordern `admin`-Permission via bestehendem `PermissionService`-Pattern.
- **D-22:** Redeem-Endpoint: **`POST /api/helper/redeem`** — öffentlich, ohne Auth-Middleware-Anforderung (Helfer hat noch keine Session). Body `{code: "ABC1234567"}`. Auf Erfolg: Set-Cookie `app_session=<session_id>` (Lifetime aus D-18) + JSON-Body `{assembly_id: "<uuid>", expires_at: "<iso8601>"}`. Backend rechnet Klartext-Code → SHA256-Hash → DB-Lookup → atomarer Redeem-UPDATE (siehe D-25).
- **D-23:** Revoke-Erlaubnis: **in Status `Preparation` UND `Open` erlaubt**. Real-Welt-Begründung: Helfer-Tablet kann während laufender GV verloren gehen — Vorstand muss ohne Re-Open der GV reagieren können. Da Revoke nur für `used_at IS NULL`-Token gilt (D-03), gibt es keine Cascade-Komplexität. In Status `Closed`: nicht erlaubt → 409 (Token-Liste ist eingefroren mit GV-Schluss).
- **D-24:** HTTP-Status-Codes für Redeem-Fehler:
  - **404 Not Found** — `token_hash` nicht gefunden (Code unbekannt oder typo)
  - **410 Gone** — Token gefunden, aber `used_at IS NOT NULL` (one-time-use bereits eingelöst)
  - **403 Forbidden** — Token gefunden, aber `revoked_at IS NOT NULL` ODER `assembly.status != Open` (revoked oder GV nicht im richtigen Lifecycle-State)
  - **400 Bad Request** — Code-Format ungültig (Länge ≠ 10 oder ungültiges Crockford-Base32-Zeichen)
  - **200 OK** mit Set-Cookie + Body — Erfolg
- **D-25:** Atomarer Redeem-Pfad (Hard Constraint Phase 2 erfüllt): SQL `UPDATE helper_token SET used_at = ?, session_id = ? WHERE token_hash = ? AND used_at IS NULL AND revoked_at IS NULL AND deleted IS NULL RETURNING id, assembly_id`. Falls 0 Zeilen returned: differenzierter Lookup für richtigen Status-Code (D-24). Race-Test: zwei parallele Redeem-Requests auf demselben Token → exakt eine 200, andere 410 (HLPR-04).

### Naming
- **D-26:** Code-Identifier durchgängig englisch, konsistent mit Phase-1-Konvention (D-16 dort): `HelperToken`, `HelperTokenEntity`, `HelperTokenDao`, `HelperTokenService`, `HelperTokenServiceImpl`, `HelperTokenTO`, `HelperTokenCreateResponseTO`. Tabelle `helper_token`. Endpoints englisch (siehe D-21/D-22). Memo-Feld speichert die Freitext-Bezeichnung des Vorstands (z.B. „Anna", „Bernd") — bleibt unverändert UTF-8.
- **D-27:** Migration-Filenames: `YYYYMMDDHHMMSS_create_helper_token_table.sql` (englisch, konsistent mit `20260413000000_create_application_table.sql`-Vorlage und Phase-1-D-15).

### Claude's Discretion
- **Wo der Assembly-Status-Check beim Verify verdrahtet wird** (siehe D-19): SessionService erweitern, neuer HelperSessionService-Wrapper, oder im `extract_auth_context`. Plan entscheidet basierend auf minimalem Eingriff in den generischen SessionService.
- **`TokenGenerator`-Service vs. freie Funktion** (siehe D-10): trait-basierter Service für Mockability oder einfache Funktion mit `OsRng` direkt. Plan/Test-Bedarf entscheidet.
- **Hex vs. Base64 für `token_hash`-Encoding** (siehe D-11): Plan wählt; Hex ist konsistenter mit SHA256-Konvention im Audit-Hashchain.
- **`APP_URL`-Default-Verhalten** beim Token-Create, falls Env nicht gesetzt (siehe D-12): fail fast beim Server-Start vs. fail beim ersten Create. Plan entscheidet — fail-fast am Server-Start ist defensiv besser, aber bricht ggf. bestehende Mock-Auth-Test-Setups.
- **Index-Strategie für `helper_token`**: vermutlich `(assembly_id)` für Listing und UNIQUE auf `(token_hash)` für Lookup-Performance + Race-Condition-Hardening. Plan finalisiert.
- **`session_id`-FK-ON-DELETE-Verhalten**: `SET NULL` (D-01 vorgeschlagen) vs. `RESTRICT` — Plan wägt mit Cleanup-Job-Implikationen ab.
- **Rate-Limiting für `/api/helper/redeem`**: bestehende `tower_governor`-Konfiguration vermutlich okay, aber spezifische Pro-IP-Limits für Brute-Force-Schutz auf Redeem-Endpoint sind Claude's Discretion (z.B. 10 Versuche/Minute/IP).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project-Level Locking-Dokumente
- `.planning/PROJECT.md` — Core Value, Active Requirements (10 Punkte; Helfer-Token mit Memo, Manual-Code-Fallback, Helfer-Sessions invalidiert beim Schließen), Constraints (Audit-Pflicht für bestehende Entitäten — neue GV-Entitäten **nicht** außer Token-Erzeugung), Key Decisions (One-Time-Use, atomarer Redeem, Manual-Code-Fallback).
- `.planning/REQUIREMENTS.md` §Helfer-Token & Session — HLPR-01..HLPR-07 als Abnahme-Kriterien dieser Phase. **Achtung:** HLPR-03 (Manual-Code-UI) ist **Phase 4**, nicht Phase 2 — Phase 2 muss aber den Redeem-Endpoint so bauen, dass er Klartext-Code akzeptiert.
- `.planning/ROADMAP.md` §Phase 2 — Goal, Success Criteria (7 Items inkl. SC#7 für `AuthContext::Helper`-Variante), Hard Constraints Phase 2: Atomarer Redeem-SQL, SHA256-Hash mit Klartext nur einmalig ausgegeben, `AuthContext::Helper`-Enum-Variante.
- `.planning/STATE.md` §Accumulated Context — Key-Decisions-Tabelle und Open-TODO „tower-sessions 0.14 → 0.15 Upgrade jetzt oder später?" — relevant für diese Phase.
- `.planning/phases/01-assembly-aggregat-audit-hardening/01-CONTEXT.md` — Phase-1-Decisions (D-05..D-17), insbesondere `Assembly`-Tabelle als FK-Ziel (D-05), Status-Werte englisch (D-06/D-17), Process-String-Punkt-Notation (D-11), Soft-Delete-Slot ohne Delete-Pfad (Service-Datei-Header).

### Codebase-Maps (Bestands-Architektur)
- `.planning/codebase/ARCHITECTURE.md` — Schicht-Struktur, Audit-Datenfluss, Anti-Patterns (Service Creating Own Transaction, Hard Delete Without Audit Trail).
- `.planning/codebase/STACK.md` — Versionierungen (`tower-sessions` 0.14, `tower_governor` 0.6, `axum-oidc` 0.6, `sha2` 0.10).
- `.planning/codebase/INTEGRATIONS.md` — `axum-oidc`-Auth-Pfad, Session-Cookie-Pattern, Auth-Middleware-Chain.
- `.planning/codebase/CONVENTIONS.md` — Naming (snake_case files, PascalCase types, `*Impl`-Suffix), Error-Handling-Konvention für `RestError`-Codes.
- `.planning/codebase/TESTING.md` — E2E-Pattern mit `start_test_server()`, Mockall-Pattern.

### Bestehende Patterns als Vorlage
- `genossi_dao/src/assembly.rs` — Vorlage für neuen Aggregat-DAO mit FK-Constraint und `Auditable`-Trait-Impl.
- `genossi_dao/src/auditable.rs` — `Auditable`-Trait-Definition (D-06).
- `genossi_dao/src/permission.rs:88-93,118-125` — `SessionEntity`-Struktur mit `claims: Option<Arc<str>>` (D-15/D-16) und PermissionDao-Session-Methoden.
- `genossi_dao/src/permission.rs:23-40` — `ensure_user_exists`-Default-Impl (D-17).
- `genossi_service/src/auth_types.rs:94-100` — `AuthContext`-Enum (D-14 erweitert ihn um `Helper`-Variante).
- `genossi_service/src/auth_types.rs:82-90` — `UserSession`-Struktur mit `claims`-Feld.
- `genossi_service_impl/src/session.rs:52-82,175-189` — `create_session_with_claims` und `ensure_user_and_create_session_with_claims` (D-17 — Pattern vorhanden, „inventur token"-Auto-Register als Vorbild).
- `genossi_service_impl/src/session.rs:84-120` — `verify_user_session` mit Inactivity-Timeout-Logik (D-18 erweitert ihn um Assembly-Status-Check).
- `genossi_service_impl/src/audit_macros.rs` — `audited_create!` (D-07).
- `genossi_service_impl/src/assembly.rs` — Vorlage für Service-Implementation mit Lifecycle-Guards, Optional-Transaction-Pattern, Audit-Macros mit Process-Strings (D-07-Format).
- `genossi_service_impl/src/macros.rs` — `gen_service_impl!`-Macro für DI-Skeleton.
- `genossi_rest/src/auth_middleware.rs:101-156` — `extract_context_from_headers` (D-14/D-15/D-16: muss um Claims-Parse erweitert werden).
- `genossi_rest/src/auth_middleware.rs:137-147` — `extract_session_from_cookie` (`app_session`-Cookie-Name; D-15 reused).
- `genossi_rest/src/assembly.rs` — Vorlage für Axum-Handler unter Phase-1-Konvention.
- `genossi_bin/src/lib.rs` — Vorlage für DI-Wiring (`RestStateImpl`); neue `HelperTokenServiceImpl` landet hier.
- `genossi_bin/tests/e2e_tests.rs` — E2E-Test-Pattern für CI-Härtung (Race-Test HLPR-04, Cascade-Test HLPR-05).
- `migrations/sqlite/20260413000000_create_application_table.sql` — Vorlage für Migration-Struktur eines neuen Aggregats (D-27).

### CLAUDE.md (Projekt-Konventionen)
- `/home/neosam/programming/rust/projects/genossi3/CLAUDE.md` §Audit Log System — Schritte zum Hinzufügen von Audit für neue Entitäten (Auditable → AuditLogDao-Dep → Audit-Macros → Wiring).
- `/home/neosam/programming/rust/projects/genossi3/CLAUDE.md` §Architecture Overview — Layer-Struktur, ISO8601-Datetime-Handling, Soft-Delete-Pattern.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`gen_service_impl!`-Macro** (`genossi_service_impl/src/macros.rs`) — generiert `*Impl`-Struct mit DI-Feldern; `HelperTokenServiceImpl` folgt diesem Pattern.
- **`audited_create!`** (`genossi_service_impl/src/audit_macros.rs`) — D-07 ruft es mit Process `"helper_token.create"` auf.
- **`SessionService::create_session_with_claims`** und **`ensure_user_and_create_session_with_claims`** (`genossi_service_impl/src/session.rs:52,175`) — die Auto-Registrierung-Pattern für Token-basierte Sessions sind bereits vorhanden („inventur token"). D-15/D-17 reusen sie.
- **`SessionEntity.claims: Option<Arc<str>>`** (`genossi_dao/src/permission.rs:123`) — JSON-Claims-Slot für `kind=helper`-Marker (D-16).
- **`extract_context_from_headers`** (`genossi_rest/src/auth_middleware.rs:101`) — wird um Claims-Parse erweitert für `AuthContext::Helper`-Rekonstruktion (D-15).
- **`tower_governor` 0.6** (`Cargo.toml`) — Rate-Limiting-Middleware bereits im Stack; Brute-Force-Schutz für `/api/helper/redeem` ist Konfigurations-Detail (Claude's Discretion).
- **`sha2` 0.10** (`Cargo.toml`) — bereits für Audit-Hashchain im Stack; D-11 nutzt SHA256 für Token-Hash ohne neue Dep.
- **E2E-Test-Pattern** in `genossi_bin/tests/e2e_tests.rs` — `start_test_server()`, In-Memory-SQLite, `reqwest::Client` für parallele Race-Test-Requests (HLPR-04).

### Established Patterns
- **Trait-basierte DI** — `HelperTokenDao` als Trait, `HelperTokenDaoImpl` als SQLite-Impl; Service generic über `HelperTokenServiceDeps`.
- **Optional-Transaction-Argument** im Service-Layer; Caller (REST-Handler) entscheidet Transaktions-Scope.
- **Soft-Delete-Konvention** — `deleted: Option<PrimitiveDateTime>` für `helper_token` (D-04); Queries filtern `WHERE deleted IS NULL`.
- **Optimistic Locking** — `version: Uuid` für Vorstand-Listing-Anzeige (Etag/If-Match auf Revoke optional, Plan wählt).
- **TO/Entity-Trennung** — `HelperTokenTO` für REST mit ISO8601-serde, `HelperTokenEntity` für DAO; `HelperTokenCreateResponseTO` enthält zusätzlich `code` und `qr_svg` (nur einmalige Rückgabe).
- **Mock-Auth/OIDC-Feature-Gate** — beide Builds müssen `AuthContext::Helper` kennen (D-14 — keine Feature-Gate auf der Variante).

### Integration Points
- `genossi_bin/src/lib.rs` `RestStateImpl::new()` — neue `HelperTokenServiceImpl` wird hier instanziiert; benötigt `HelperTokenDao`, `AssemblyDao` (für D-18-Status-Check), `AuditLogDao`, `PermissionService`, `PermissionDao` (für D-17 `ensure_user_exists`), `SessionService` (für D-17 Session-Erzeugung) und `UuidService`.
- `genossi_rest/src/lib.rs` Router — neue Route-Gruppen `/api/assembly/{id}/helper-tokens/*` (mit Auth-Middleware) und `/api/helper/redeem` (**ohne** Auth-Middleware-Erfordernis). Utoipa-Schemas für `HelperTokenTO`, `HelperTokenCreateResponseTO`, `RedeemRequestTO`, `RedeemResponseTO` registrieren.
- `migrations/sqlite/` — eine neue Migration-File (`helper_token`-Tabelle); auto-Run beim Server-Start via SQLx.
- `genossi_service/src/auth_types.rs` — `AuthContext`-Enum erweitert; betroffene Match-Arms in allen Permission-Service-Aufruf-Stellen müssen die neue Variante (mindestens als `_ => Err(PermissionDenied)`-Stub) handhaben.

</code_context>

<specifics>
## Specific Ideas

- **Phase-1-Process-String-Konvention exakt übernehmen:** `"helper_token.create"` mit Punkt-Notation, damit das Audit-Log-Endpoint via `?process_prefix=helper_token.` filtern kann (parallel zur `assembly.`-Konvention aus Phase-1-D-11).
- **`session_id` im `helper_token`-Eintrag bleibt sichtbar nach Cleanup:** FK auf `session.id` mit `ON DELETE SET NULL` (D-01) — wenn die Session per Cleanup-Job (oder per Phase-3-Cascade) gelöscht wird, bleibt der Token-Eintrag mit `used_at IS NOT NULL, session_id IS NULL` erhalten und das Listing zeigt korrekt „eingelöst" an, selbst wenn die ursprüngliche Session weg ist.
- **`AuthContext::Helper`-Variante existiert in beiden Feature-Builds** (mock_auth und oidc), nicht feature-gated (D-14). Phase 3 wird die positive Permission-Branch in beiden Builds gleich verdrahten.
- **HLPR-05-Test in Phase 2 vollständig E2E-machbar:** dank D-18-Status-Check brauchen wir den Cascade-Hook in `close_assembly` nicht — der Test öffnet Assembly, erzeugt Token, redeemed, ruft Helfer-Endpoint (selbst wenn nur Stub-403 in Phase 2 kommt) zur Verify, schließt Assembly, ruft erneut → 401. Phase 3 verfeinert den Cascade-Pfad als Optimierung.
- **CI-E2E-Test-Sequenz für HLPR-04 (Race):** zwei parallele `reqwest::post`-Aufrufe (`tokio::join!`) mit demselben Klartext-Code; Assert: exakt einer erhält 200+Cookie, der andere 410.
- **CI-E2E-Test-Sequenz für HLPR-07 (Audit):** Token erzeugen, dann `GET /api/audit?entity_type=helper_token` → mind. ein Eintrag mit `process="helper_token.create"`, `entity_id=<token_id>`, Memo-Feld im Audit-Diff sichtbar; Hash-Chain-Verify intakt (`GET /api/audit/verify` → leere Mismatch-Liste).

</specifics>

<deferred>
## Deferred Ideas

### Phase 3
- **Cascade-Invalidation der Helfer-Sessions in `close_assembly`** — proaktives DELETE aller Sessions mit `claims.assembly_id == X`. Phase 2 deckt HLPR-05 über D-18 ab; Phase 3 ergänzt die Cascade als Optimierung (vermeidet stale Session-Einträge in der DB).
- **Positive PermissionService-Branch für `AuthContext::Helper`** — Helfer dürfen Attendance-Endpoints aufrufen, wenn `claims.assembly_id == endpoint.assembly_id`. Phase 2 stubbt mit `PermissionDenied` (D-20).
- **`AttendanceMemberTO`-Erzeugung** — reduzierter Member-View (4 Felder), den Helfer abrufen können.
- **Live-Stats-Endpoint** `GET /api/assembly/{id}/stats` — ASSY-04, Phase 3.

### Phase 4
- **Manual-Code-Eingabe-UI** (HLPR-03) — Frontend-Pfad `/helper`, Code-Validierung (Crockford-Base32, fix 10 Zeichen), POST gegen `/api/helper/redeem`. Phase 4 baut nur die UI; Backend-Vertrag (D-22, D-24) steht in Phase 2.
- **QR-Scanner-Integration** (BarcodeDetector + Polyfill) — Phase 4 baut die Camera-Permission-Pfade und greift dann auf den gleichen Redeem-Endpoint zu wie der Manual-Code-Pfad.

### Spätere Phasen / Out of Scope
- **Bulk-QR-Erzeugung** (BULK-01/BULK-02) — explizit v2 in REQUIREMENTS.md. Phase 2 baut nur Single-Token-Create.
- **Audit-Log für Redeem/Revoke** — explizit ausgeschlossen (D-08); falls Vorstand später nachfragt, ist die Erweiterung ein One-Liner in den jeweiligen Service-Methoden.
- **`tower-sessions` 0.14 → 0.15 Upgrade** — STATE.md-TODO; nicht in Phase 2 erforderlich, da bestehende API-Surface ausreichend ist. Falls Phase-2-Plan-Researcher Sicherheits-Issues findet, eskalieren.
- **Differenzierte `manage_helper_tokens`-Permission** — Phase 2 nutzt `admin` (analog zu Phase 1 D-14). Falls Vorstand-Rolle sich später aufspaltet (z.B. Schriftführer), nachziehen.
- **Pro-IP-Rate-Limiting auf `/api/helper/redeem`** — Plan-Detail (Claude's Discretion); aktuelle `tower_governor`-Konfiguration prüfen und ggf. spezifischer machen.

### Reviewed Todos (not folded)
None — discussion stayed within phase scope.

</deferred>

---

*Phase: 2-Helfer-Token + Session + AuthContext::Helper*
*Context gathered: 2026-05-03*
