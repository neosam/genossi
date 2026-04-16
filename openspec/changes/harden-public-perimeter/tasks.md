## 1. Dependencies

- [ ] 1.1 `tower-governor` zu `genossi_rest/Cargo.toml` hinzufügen
- [ ] 1.2 `constant_time_eq` zu `genossi_rest/Cargo.toml` hinzufügen
- [ ] 1.3 Prüfen ob `lettre` bereits als Dep in `genossi_rest` oder `genossi_mail` verfügbar ist (für Email-Validierung); falls nicht, Entscheidung ob simple Eigenimpl oder `lettre` in `genossi_rest` einziehen
- [ ] 1.4 `cargo build` zur Verifikation, dass keine Konflikte

## 2. CORS-Allowlist

- [ ] 2.1 Neue Funktion `build_cors_layer(state: &RestState) -> CorsLayer` in `genossi_rest/src/lib.rs`: Liest `BASE_PATH` aus Env und `cors_allowed_origins` aus Config-Store
- [ ] 2.2 `CorsLayer::new()` statt `CorsLayer::permissive()`; `allow_origin`, `allow_methods`, `allow_headers`, `allow_credentials` explizit setzen
- [ ] 2.3 Die Origins in der Allowlist via `HeaderValue::from_str` parsen; invalide Origins werden beim Start geloggt, aber verhindern nicht den Boot
- [ ] 2.4 In `genossi_rest/src/lib.rs:415`: alte `CorsLayer::permissive()` ersetzen durch den neuen Layer
- [ ] 2.5 Unit-Test: Build-Funktion mit `BASE_PATH="https://example.org"` und leerem Config → Allowlist enthält nur `https://example.org`
- [ ] 2.6 Unit-Test: mit `cors_allowed_origins="https://a.example,https://b.example"` → Allowlist enthält beide plus BASE_PATH
- [ ] 2.7 E2E-Test: Request mit `Origin: https://evil.example` → `Access-Control-Allow-Origin` ist nicht gesetzt auf diesen Origin

## 3. Security-Header

- [ ] 3.1 Neue Funktion `security_headers_layer()` in `genossi_rest/src/lib.rs` (oder neues File `genossi_rest/src/security_headers.rs`)
- [ ] 3.2 Stack mehrerer `SetResponseHeaderLayer::if_not_present` für jeden Header: HSTS, X-Content-Type-Options, X-Frame-Options, Referrer-Policy, Permissions-Policy
- [ ] 3.3 Layer in den Router einhängen — gilt für alle Routes
- [ ] 3.4 E2E-Test: ein beliebiger erfolgreicher API-Call → alle fünf Header vorhanden
- [ ] 3.5 E2E-Test: ein 404-Response → alle fünf Header vorhanden

## 4. Rate-Limiting

- [ ] 4.1 Helper-Funktion `make_rate_limit_layer(per_minute: u32)` mit `tower-governor`
- [ ] 4.2 Globaler Limit-Layer (60/min) auf den `/api/*`-Subrouter (aber nicht auf `/api/public/member-count` — separater Router ohne Limit)
- [ ] 4.3 Strikter Limit-Layer (10/min) auf `/authenticate`
- [ ] 4.4 Strikter Limit-Layer (5/min) auf `/join` (public application submission)
- [ ] 4.5 Statische Frontend-Assets sind NICHT gelimited (verifizieren dass Layer nur auf API-Router sitzt)
- [ ] 4.6 Rate-Limit-Response konfigurieren: HTTP 429 + `Retry-After`-Header
- [ ] 4.7 E2E-Test: 11 Requests in schneller Folge auf `/authenticate` → ab dem 11. kommt 429
- [ ] 4.8 E2E-Test: 6 Requests auf `/join` mit gültigem Key in <60s → ab dem 6. kommt 429
- [ ] 4.9 E2E-Test: 20 parallele `/api/members` Requests → alle innerhalb 60/min, alle gehen durch

## 5. /join Hardening: Constant-Time-Compare

- [ ] 5.1 In `genossi_rest/src/application.rs` den `if api_key != stored_key` Block durch `constant_time_eq::constant_time_eq(api_key.as_bytes(), stored_key.as_bytes())` ersetzen
- [ ] 5.2 Unit-Test: korrekt übereinstimmende Keys geben `Ok`, abweichende `Err(Unauthorized)`
- [ ] 5.3 Code-Review-Hinweis im Commit: "compare via constant_time_eq to prevent timing side-channel"

## 6. /join Hardening: Input-Validierung

- [ ] 6.1 Neue Funktion `validate_join_request(body: &PublicJoinRequest) -> Result<(), Vec<ValidationFailureItem>>` in `genossi_rest/src/application.rs`
- [ ] 6.2 Validierungsregeln implementieren laut Spec-Tabelle (Längen + Required + shares>=1 + email enthält '@')
- [ ] 6.3 Email-Validierung: entweder `lettre::Address::from_str` (wenn verfügbar) oder minimale Eigenimpl (`contains('@') && len >= 3`)
- [ ] 6.4 Alle Validierungsfehler sammeln (nicht short-circuit)
- [ ] 6.5 Fehler-Response-Struktur `{"errors": [{"field", "message"}, ...]}` — ggf. Type `ValidationErrorResponse` in `genossi_rest_types`
- [ ] 6.6 In `public_join`-Handler: `validate_join_request` VOR dem `submit`-Call aufrufen, bei Fehlern HTTP 422 mit der Error-Struktur zurückgeben
- [ ] 6.7 Unit-Test: `first_name=""` → Fehler "missing"
- [ ] 6.8 Unit-Test: `first_name` mit 200 Zeichen → Fehler "too long"
- [ ] 6.9 Unit-Test: `email="foo"` → Fehler "invalid email format"
- [ ] 6.10 Unit-Test: `shares=0` → Fehler "shares must be >= 1"
- [ ] 6.11 Unit-Test: mehrere Fehler gleichzeitig → alle in der Response
- [ ] 6.12 Unit-Test: gültiger Request mit allen Pflichtfeldern → `Ok(())`

## 7. OpenAPI / Swagger

- [ ] 7.1 `ValidationErrorResponse` in `utoipa`-Schema aufnehmen
- [ ] 7.2 `/join` OpenAPI-Annotation: 422-Response mit `ValidationErrorResponse` als Body-Type
- [ ] 7.3 Swagger-UI manuell prüfen: die neuen Error-Codes werden angezeigt

## 8. Config-Store-Integration

- [ ] 8.1 Config-Key `cors_allowed_origins` (Type `text`, optional) in der Config-Page-UI dokumentieren (Frontend-Änderung optional, kann Follow-up sein — in diesem Change reicht Backend)
- [ ] 8.2 `get_config_value`-Aufruf beim Build des CORS-Layers (beim Server-Start, sync)
- [ ] 8.3 Dokumentation: Änderungen an `cors_allowed_origins` erfordern Server-Restart — in `doc/` vermerken

## 9. WordPress-Plugin Smoke-Test

- [ ] 9.1 Nach Deploy: Manueller Test durch WordPress-Formular → erfolgreicher Antrag
- [ ] 9.2 WordPress-Form mit ungültiger Email → Plugin zeigt 422-Fehler sinnvoll an (kein "500", sondern der Field-spezifische Text)
- [ ] 9.3 Bei schlechter Anzeige: Follow-up-Task auf das WordPress-Plugin, keine Code-Änderung hier

## 10. Dokumentation & Release

- [ ] 10.1 `doc/` neuer Abschnitt "HTTP-Perimeter" mit CORS-, Rate-Limit- und Header-Politik
- [ ] 10.2 Release-Notes: "CORS ist jetzt strikt; bei eigener Origin-Konfiguration ggf. `cors_allowed_origins` im Config-Store setzen"
- [ ] 10.3 Smoke-Test nach Deploy: Browser-DevTools zeigen Security-Header; 429 bei Brute-Force-Simulation
