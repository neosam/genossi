## 1. H3 — `eprintln!` im Auth-Pfad ersetzen

- [x] 1.1 In `genossi_rest/src/auth_middleware.rs:26` `eprintln!("Auth context extraction error: {:?}", err)` durch `tracing::warn!(error = ?err, "auth context extraction failed")` ersetzen. `use tracing;` am Dateikopf ergänzen (aktuell kein `tracing`-Import in dieser Datei)
- [x] 1.2 `cargo test -p genossi_rest` — 28 Tests grün inkl. `auth_middleware::tests::test_extract_session_from_cookie` und `test_extract_bearer_token`

## 2. M1 — CORS-Whitelist für Methods und Headers

- [x] 2.1 In `genossi_rest/src/lib.rs:366-367` `AllowMethods::any()` durch explizite Liste `[Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS]` ersetzen
- [x] 2.2 `AllowHeaders::any()` durch explizite Liste `[header::CONTENT_TYPE, header::AUTHORIZATION, header::COOKIE]` ersetzen
- [x] 2.3 `use http::{Method, header};` am Dateikopf ergänzen (bisher kein nicht-feature-gated `http::`-Import in `lib.rs`; `http::HeaderValue::from_str` wird qualifiziert aufgerufen)
- [x] 2.4 Nicht mehr benötigte Imports `AllowMethods`/`AllowHeaders` aus `tower_http::cors` entfernen
- [x] 2.5 E2E-Test `test_cors_preflight_allowed_method_post` in `genossi_bin/tests/e2e_tests.rs`: bestätigt Methods-Whitelist (GET/POST/PUT/DELETE/OPTIONS), kein `*`
- [x] 2.6 E2E-Test `test_cors_preflight_disallowed_method_patch`: `PATCH` nicht in `Access-Control-Allow-Methods`
- [x] 2.7 E2E-Test `test_cors_preflight_allowed_headers`: `Content-Type`/`Authorization`/`Cookie` in `Access-Control-Allow-Headers`, kein `*`

## 3. N2 — Fehlende Error-Varianten ergänzen

- [x] 3.1 In `genossi_mail/src/service.rs` `MailServiceError`-Enum um `BadRequest(Arc<str>)` erweitern
- [x] 3.2 In `genossi_mail/src/rest.rs:184` den `error_handler` um einen `Err(MailServiceError::BadRequest(msg))`-Match-Arm erweitern, der HTTP 400 zurückgibt (analog zur existierenden `TemplateValidation`-Behandlung)
- [x] 3.3 In `genossi_mail/src/mail_template_service.rs` `MailTemplateError`-Enum um `BadRequest(Arc<str>)` erweitern
- [x] 3.4 In `genossi_rest/src/lib.rs` den `impl From<MailServiceError> for RestError` (Z. 100-118) um einen `MailServiceError::BadRequest(msg) => RestError::BadRequest(msg.to_string())`-Match-Arm erweitern (der bestehende Match ist exhaustive ohne `_`-Arm und bricht nach 3.1 sonst beim Kompilieren)
- [x] 3.5 `cargo check` — sicherstellen, dass weitere existierende `match`-Ausdrücke über `MailServiceError`/`MailTemplateError` die neue Variante behandeln (auch `genossi_mail/src/inbox_rest.rs:127` `map_error` erweitert)

## 4. N2 — `From<serde_json::Error>`-Impls pro Error-Type

- [x] 4.1 In `genossi_rest/src/lib.rs` (neben den existierenden `From`-Impls ab Zeile 82) `impl From<serde_json::Error> for RestError` ergänzen: mappt nach `RestError::InternalError(format!("serialize failed: {}", e))`
- [x] 4.2 In `genossi_mail/src/service.rs` `impl From<serde_json::Error> for MailServiceError` ergänzen: mappt nach `MailServiceError::DataAccess(Arc::from(format!("serialize failed: {}", e)))`. **Hinweis:** `DataAccess` nimmt `Arc<str>`, nicht `String`
- [x] 4.3 In `genossi_config/src/service.rs` `impl From<serde_json::Error> for ConfigServiceError` ergänzen: mappt nach `ConfigServiceError::DataAccess(Arc::from(format!("serialize failed: {}", e)))`. **Hinweis:** `DataAccess` nimmt `Arc<str>`, nicht `String`
- [x] 4.4 In `genossi_mail/src/mail_template_service.rs` `impl From<serde_json::Error> for MailTemplateError` ergänzen: mappt nach `MailTemplateError::DataAccess(Arc::from(format!("serialize failed: {}", e)))` (`DataAccess` existiert bereits als Variante mit `Arc<str>`)
- [x] 4.5 Unit-Tests pro `From`-Impl in den jeweiligen Test-Modulen (`genossi_rest/src/lib.rs`, `genossi_mail/src/service.rs`, `genossi_config/src/service.rs`, `genossi_mail/src/mail_template_service.rs`) — alle grün
- [x] 4.6 `cargo build --all-features` — From-Impls kompilieren ohne Regression (via `cargo check --workspace --all-features` verifiziert, Stand siehe oben)

## 5. N2 — Muster-B-Handler auf Muster A umbauen

- [x] 5.1 `genossi_mail/src/rest_templates.rs`: `fn error_response(err: MailTemplateError) -> Response` in `fn error_handler(result: Result<Response, MailTemplateError>) -> Response` umbauen (analog zu `genossi_mail/src/rest.rs:184`). Die bestehenden 4 Match-Arme (NotFound→404, DuplicateName→409, VersionConflict→409, DataAccess→500) bleiben, plus Ok-Pass-through und **plus neuer Arm für `BadRequest(msg)→400`** (wegen der in Task 3.3 ergänzten Variante)
- [x] 5.2 `rest_templates.rs::list_templates` (Z. 124): Body in `error_handler((async { ... Result<Response, MailTemplateError> }).await)` wickeln, `match`-Block durch `?` auf Service-Call ersetzen, `.unwrap()` nach `serde_json::to_string` durch `?` ersetzen
- [x] 5.3 `rest_templates.rs::create_template` (Z. 149): analog zu 5.2
- [x] 5.4 `rest_templates.rs::get_template` (Z. 181): analog, UUID-Parse-Fehler nach `MailTemplateError::BadRequest(Arc::from("Invalid UUID"))` mappen statt early-return
- [x] 5.5 `rest_templates.rs::update_template` (Z. 224): analog 5.4 mit UUID + Version-Parse (beide mappen nach `MailTemplateError::BadRequest(...)`)
- [x] 5.6 `rest_templates.rs::delete_template` (Z. 283): muss mit umgebaut werden, weil Task 5.1 die Signatur von `error_response` entfernt. UUID-Parse-Fehler (Z. 287-298) nach `MailTemplateError::BadRequest(Arc::from("Invalid UUID"))` mappen. Body in `error_handler((async { ... Result<Response, MailTemplateError> }).await)` wickeln
- [x] 5.7 `genossi_mail/src/rest.rs::preview_mail` (Z. 406): Body in `error_handler((async { ... Result<Response, MailServiceError> }).await)` wickeln. UUID-Parse-Fehler nach `MailServiceError::BadRequest(Arc::from("Invalid member_id"))`, Member-Not-Found nach `MailServiceError::NotFound` mappen
- [x] 5.8 `genossi_rest/src/mail_footer.rs::get_footer` (Z. 29): Body in `error_handler((async { ... Result<Response, RestError> }).await)` wickeln. Drei Pfade konkret:
  - Leeres Template (Z. 38-47): **kein Fehler** — als `return Ok(Response::builder()...)` im async-Block belassen
  - Auth-Error (Z. 49-57): `crate::extract_auth_context(Some(context))?` reicht, weil die Funktion bereits `Err(RestError::Unauthorized)` zurückgibt — kein explizites Mapping nötig
  - `render_footer`-Fehler (Z. 77-83): `render_footer(...).map_err(|e| RestError::BadRequest(e.message))?` — alternativ `impl From<genossi_mail::template::TemplateError> for RestError` ergänzen, aber einmalige Verwendung → `map_err` vorziehen
- [x] 5.9 `cargo build --all-features` — alle umgebauten Handler kompilieren

## 6. N2 — `.unwrap()` → `?` in allen 52 Call-Sites

### 6.a `genossi_rest` (39 Sites, 11 Dateien)

- [x] 6.1 `genossi_rest/src/application.rs` — **8 Sites** (Z. 163, 194, 253, 309, 344, 380, 416, 474; teils mehrzeilig)
- [x] 6.2 `genossi_rest/src/member.rs` — 6 Sites
- [x] 6.3 `genossi_rest/src/member_document.rs` — 4 Sites
- [x] 6.4 `genossi_rest/src/member_action.rs` — 5 Sites
- [x] 6.5 `genossi_rest/src/audit_log.rs` — 3 Sites
- [x] 6.6 `genossi_rest/src/audit_timestamp.rs` — 5 Sites
- [x] 6.7 `genossi_rest/src/user_preference.rs` — 2 Sites
- [x] 6.8 `genossi_rest/src/static_document.rs` — 2 Sites
- [x] 6.9 `genossi_rest/src/mail_footer.rs` — 2 Sites (im umgebauten Handler aus 5.8)
- [x] 6.10 `genossi_rest/src/template.rs` — 1 Site
- [x] 6.11 `genossi_rest/src/validation.rs` — 1 Site

### 6.b `genossi_mail` (10 Sites, 2 Dateien)

- [x] 6.12 `genossi_mail/src/rest.rs` — 6 Sites (davon 1 im umgebauten `preview_mail` aus 5.7)
- [x] 6.13 `genossi_mail/src/rest_templates.rs` — 4 Sites (in den umgebauten Handlern aus 5.2-5.5)

### 6.c `genossi_config` (3 Sites, 1 Datei)

- [x] 6.14 `genossi_config/src/rest.rs` — 3 Sites

### 6.d Regression- und Vollständigkeits-Check

- [x] 6.15 Projektweiter Multiline-Check: der ursprüngliche Regex `serde_json::to_string[^;]*?\.unwrap\(\)` ist hier ungeeignet (matcht die übriggebliebenen äußeren `Response::builder().unwrap()`). Stattdessen: `rg -U --multiline-dotall 'serde_json::to_string\([^()]*(\([^()]*\))?\)\?'` → 51 Treffer (bestätigt Konvertierung; der 52. Site ist ein mehrzeiliger Struct-Literal-Aufruf in `application.rs:194`, manuell als `?` verifiziert). Compile des Workspaces bestätigt, dass kein panikendes `.unwrap()` auf `serde_json::to_string` mehr existiert
- [x] 6.16 E2E-Regression für `/api/config`: durch bestehende Tests im `e2e_tests.rs` bereits abgedeckt (5+ Tests ab Zeile 2941 nutzen `/api/config`) — alle grün nach Umbau
- [x] 6.17 E2E-Regression für `/api/members` und `/api/mail/templates`: durch bestehende E2E-Tests abgedeckt (inkl. `test_template_crud`, `test_template_list`) — alle 215 E2E-Tests grün
- [x] 6.18 `cargo test --workspace` — 616 Tests grün (215 E2E + 401 Unit-Tests). **Hinweis:** Variante `--all-features` panikt bei Setup wegen `APP_URL`-Env-Variable im OIDC-Pfad — das ist eine pre-existierende Test-Infrastruktur-Einschränkung, unabhängig von diesem Change. Default-Features nutzen `mock_auth`, wie in CLAUDE.md dokumentiert

## 7. Abschluss

- [x] 7.1 `cargo fmt` — alle veränderten Dateien formatiert (via rustfmt 1.90 aus `/nix/store`)
- [~] 7.2 `cargo clippy` — kann lokal nicht ausgeführt werden: in `/nix/store` steht nur clippy 1.90 und 1.93, projekt-cargo ist 1.89.0; clippy meldet `E0514: incompatible version of rustc` für alle crates. Zu adressieren, sobald ein passendes clippy-Binary verfügbar ist (oder im CI, wo Toolchain konsistent sein sollte). Kein funktionales Problem, nur Lint-Check offen
- [ ] 7.3 Manuelle Rauchprobe: `cargo run --bin genossi`, Swagger-UI öffnen, einen GET-Endpoint aufrufen, einen umgebauten Handler (`GET /api/mail/templates` oder `GET /api/mail-footer`) testen, CORS-Preflight via Browser-DevTools checken — **muss vom User gemacht werden** (Browser-Test)
