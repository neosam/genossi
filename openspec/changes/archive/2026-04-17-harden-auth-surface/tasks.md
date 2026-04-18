## 1. Datenbank-Migration

- [x] 1.1 Neue Migration-Datei unter `migrations/sqlite/` erstellen (Namens-Präfix mit aktuellem Datum): `ALTER TABLE session ADD COLUMN last_used_at INTEGER NOT NULL DEFAULT 0;`
- [x] 1.2 In der gleichen Migration: `UPDATE session SET last_used_at = created WHERE last_used_at = 0;` für Bestandsdaten
- [x] 1.3 In der gleichen Migration: Index auf `last_used_at` anlegen für effizienten Cleanup: `CREATE INDEX idx_session_last_used ON session(last_used_at);`
- [x] 1.4 Migration lokal testen (`cargo run --bin genossi` auf Kopie der DB, Schema-Diff verifizieren)

## 2. DAO-Schicht erweitern

- [x] 2.1 `SessionEntity` in `genossi_dao/src/permission.rs` um Feld `last_used_at: i64` erweitern
- [x] 2.2 `PermissionDao`-Trait um Methode `touch_session(session_id: &str, now: i64) -> Result<(), DaoError>` erweitern
- [x] 2.3 `PermissionDao`-Trait um Methode `delete_sessions_for_user(user_id: &str) -> Result<u64, DaoError>` erweitern (gibt Anzahl gelöschter Sessions zurück)
- [x] 2.4 SQLite-Implementation in `genossi_dao_impl_sqlite/src/permission.rs`: `create_session` schreibt `last_used_at = created`
- [x] 2.5 SQLite-Implementation: `get_session` liest auch `last_used_at`
- [x] 2.6 SQLite-Implementation: `touch_session` als `UPDATE session SET last_used_at = ? WHERE id = ?`
- [x] 2.7 SQLite-Implementation: `delete_sessions_for_user` als `DELETE FROM session WHERE user_id = ?` mit Rückgabe von `rows_affected`
- [x] 2.8 Mock-Implementation von `PermissionDao` (falls vorhanden) entsprechend erweitern
- [x] 2.9 Unit-Tests für `touch_session` und `delete_sessions_for_user` in `genossi_dao_impl_sqlite`

## 3. Service-Schicht erweitern

- [x] 3.1 `SessionService`-Trait in `genossi_service/src/session.rs` um Methoden `touch_session` und `revoke_all_for_user` erweitern
- [x] 3.2 `UserSession`-Struct um Feld `last_used_at: i64` erweitern
- [x] 3.3 In `genossi_service_impl/src/session.rs`: `create_session` setzt `last_used_at = now` in `SessionEntity`
- [x] 3.4 `verify_user_session` erweitern: nach Expires-Check zusätzlich `now - last_used_at > 30 * 24 * 60 * 60` prüfen; bei überschrittenem Inaktivitäts-Timeout Session löschen und `Ok(None)` zurückgeben
- [x] 3.5 `verify_user_session` ruft nach erfolgreicher Verifikation `permission_dao.touch_session(session_id, now)` auf
- [x] 3.6 Neue Methode `revoke_all_for_user(user_id: &str) -> Result<u64, ServiceError>` implementieren, die auf `delete_sessions_for_user` durchgreift
- [x] 3.7 Mock-Implementation von `SessionService` (für Tests) entsprechend erweitern
- [x] 3.8 Unit-Tests: Session verifiziert → `last_used_at` wurde aktualisiert
- [x] 3.9 Unit-Tests: Session > 30d inaktiv → wird abgelehnt und gelöscht
- [x] 3.10 Unit-Tests: `revoke_all_for_user` löscht alle Sessions eines Users

## 4. REST-Schicht: Session-Lifetime-Konstante und Logging-Fix

- [x] 4.1 In `genossi_rest/src/session.rs`: Konstante `SESSION_ABSOLUTE_LIFETIME_SECS: i64 = 365 * 24 * 60 * 60` einführen (bestehender Wert beibehalten, da vor Implementierung entschieden wurde, die 365-Tage-Lifetime nicht zu reduzieren)
- [x] 4.2 Cookie-`expires` auf denselben 365-Tage-Wert anpassen
- [x] 4.3 Alle `tracing::info!("All cookies: ...")` / `"app_session cookie found: ..."` / `"Session ID: ..."` / `"Session found: ..."` in `context_extractor` entfernen
- [x] 4.4 Stattdessen: `tracing::debug!(user_id = %session.user_id, "session verified")` nach erfolgreicher Verifikation; `tracing::debug!("no session cookie")` / `tracing::debug!("session invalid or expired")` für die anderen Pfade
- [x] 4.5 Verifizieren: keine weiteren `{:?}`-Ausgaben von Cookie, SessionEntity oder Session-ID in `genossi_rest/src/session.rs` oder angrenzenden Files via grep
- [x] 4.6 `RUST_LOG`-Default in `genossi_bin/src/main.rs:16` lockern (`genossi=info` statt `genossi=debug`) — optional in diesem Change, aber konsistent mit der Logging-Reduktion

## 5. REST-Schicht: Panic-Entfernung

- [x] 5.1 In `register_session` das `.expect("Failed to create session for OIDC user")` durch `match` mit `Err(e)` → `tracing::error!(error = %e, user_id = %username, "failed to create session")` und `return Response::builder().status(500)…` ersetzen
- [x] 5.2 Sicherstellen, dass keine internen Fehlerdetails im Response-Body landen (nur generische Message "Internal Server Error")
- [x] 5.3 Unit-Test oder Integrations-Test: DB-Mock gibt Fehler zurück → Handler liefert 500, keine Panic

## 6. REST-Schicht: Self-Service Revoke-Endpoint

- [x] 6.1 Neue Datei `genossi_rest/src/session_management.rs` mit Handler `revoke_all_sessions` anlegen
- [x] 6.2 Handler: extrahiert User-ID aus AuthContext, ruft `session_service.revoke_all_for_user(user_id)` auf, antwortet mit `{"message": "Alle Sessions beendet.", "revoked_count": N}`
- [x] 6.3 Route `POST /api/session/revoke-all` in `genossi_rest/src/lib.rs` hinter `forbid_unauthenticated`-Middleware einhängen
- [x] 6.4 Utoipa-OpenAPI-Annotation mit 200/401-Responses
- [x] 6.5 In `genossi_bin/tests/e2e_tests.rs`: E2E-Test für Revoke-Flow (Login → Revoke → nächster Request → 401)
- [x] 6.6 E2E-Test: Revoke ohne Auth → 401

## 7. REST-Types und Frontend-API (optional)

- [x] 7.1 Response-Type `SessionRevokeResponse { message: String, revoked_count: u64 }` in `genossi_rest_types/src/lib.rs` mit `utoipa::ToSchema` definieren (falls Frontend den Count anzeigen soll)
- [x] 7.2 Frontend-API-Funktion in `genossi-frontend/src/api.rs` ergänzen (nur Backend-Call, kein UI-Button in diesem Change)

## 8. Tests gegen Specs

- [x] 8.1 Test: Session-Lifetime — Session bei `created + 365d - 1s` noch gültig, bei `created + 365d + 1s` ungültig
- [x] 8.2 Test: Inactivity — Session bei `last_used_at + 30d - 1s` noch gültig, bei `last_used_at + 30d + 1s` ungültig
- [x] 8.3 Test: `last_used_at` wird bei jedem erfolgreichen Verify aktualisiert
- [x] 8.4 Test: Log-Output bei Session-Verify enthält weder Session-ID noch Cookie-Struktur (z.B. via `tracing-test` oder Logger-Hook in e2e)
- [x] 8.5 Test: Revoke-All löscht alle Sessions des aufrufenden Users, nicht die anderer User
- [x] 8.6 Test: Nach Revoke-All Response → nächster Request mit alter Session-ID → 401

## 9. Dokumentation & Release

- [x] 9.1 `doc/AUTHENTICATION.md` aktualisieren: Session-Lifetime-Policy und Revoke-Endpoint dokumentieren
- [x] 9.2 Changelog-Eintrag / Release-Notes mit Hinweis auf BREAKING (alle User müssen sich neu einloggen)
- [ ] 9.3 Vorstand in internem Kommunikationskanal informieren: "Nach dem nächsten Deploy einmal neu einloggen"
- [ ] 9.4 Smoke-Test nach Deploy: Neuer Login, nach 30d ohne Traffic wird Session ungültig, Revoke-Endpoint funktioniert, Logs enthalten keine Session-IDs mehr
