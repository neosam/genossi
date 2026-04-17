## 1. Backend-Endpoint

- [x] 1.1 Neuen Handler `revoke_user_sessions` in `genossi_rest/src/session_management.rs` hinzufügen: `POST /revoke/{user_id}`, Admin-Privilegprüfung, ruft `session_service.revoke_all_for_user(user_id)` auf
- [x] 1.2 Route `/revoke/{user_id}` in `generate_route()` in `session_management.rs` einbinden
- [x] 1.3 OpenAPI-Annotation für den neuen Endpoint (200, 401, 403)
- [x] 1.4 E2E-Test: Admin kann Sessions eines Users beenden (HTTP 200 + revoked_count)
- [x] 1.5 E2E-Test: Nicht-Admin bekommt HTTP 403 (mock_auth-Mode hat immer Admin-Rechte — 403-Test nicht möglich, Admin-Prüfung im Handler via `check_permission` verifiziert)

## 2. Frontend-API

- [x] 2.1 Neue Funktion `pub async fn revoke_user_sessions(config: &Config, user_id: &str) -> Result<SessionRevokeResponse, AppError>` in `genossi-frontend/src/api.rs`

## 3. Permissions-Seite: Manueller Button

- [x] 3.1 "Sessions beenden"-Button in `UserRowComponent` in `permissions.rs` hinzufügen (neue Tabellenspalte)
- [x] 3.2 Tabellen-Header um "Sessions"-Spalte erweitern
- [x] 3.3 Loading-Signal und Erfolgs-/Fehlerfeedback für den Button

## 4. Automatisches Revoke bei Rechteentzug

- [x] 4.1 Im `onchange`-Handler der Admin-Checkbox: nach erfolgreichem `remove_user_role()` zusätzlich `api::revoke_user_sessions()` aufrufen
- [x] 4.2 Revoke-Fehler loggen, aber Rollenänderung nicht blockieren

## 5. i18n

- [x] 5.1 Neue i18n-Keys: `RevokeSessions` ("Sessions beenden" / "Revoke sessions"), `SessionsRevoked` ("Sessions beendet" / "Sessions revoked"), `Sessions` (Spalten-Header)
