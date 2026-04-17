## Why

Wenn einem User die Admin-Rolle entzogen wird, behält er seine bestehenden Sessions und damit weiterhin Zugriff — bis die Session natürlich abläuft. Ein Admin hat aktuell keine Möglichkeit, die Sessions eines bestimmten Users sofort zu beenden. Das ist ein Sicherheitsproblem: Rechteentzug wirkt nicht sofort.

## What Changes

- Neuer Backend-Endpoint `POST /api/session/revoke/{user_id}` (admin-only), der alle Sessions eines bestimmten Users beendet
- "Sessions beenden"-Button pro User auf der Permissions-Seite
- Automatisches Session-Revoke beim Entziehen der Admin-Rolle (beim Deaktivieren der Admin-Checkbox)
- Neue Frontend-API-Funktion `api::revoke_user_sessions(user_id)`

## Capabilities

### New Capabilities

- `admin-session-revoke`: Admin-Endpoint und UI zum Beenden der Sessions eines bestimmten Users

### Modified Capabilities

_(keine — die bestehende Self-Service-Revoke-Capability bleibt unberührt)_

## Impact

**Code:**
- `genossi_rest/src/session_management.rs` — neuer admin-only Endpoint `POST /revoke/{user_id}` mit Admin-Privilegprüfung
- `genossi_rest/src/lib.rs` — Route einbinden
- `genossi-frontend/src/api.rs` — neue Funktion `revoke_user_sessions(config, user_id)`
- `genossi-frontend/src/page/permissions.rs` — "Sessions beenden"-Button pro User-Zeile + automatisches Revoke beim Admin-Rechteentzug

**Backend:**
- `SessionService::revoke_all_for_user(user_id)` existiert bereits — der neue Endpoint nutzt ihn direkt
- Admin-Privilegprüfung über bestehenden `AuthContext`

**Dependencies:**
- Keine neuen Crates
- Setzt `harden-auth-surface` voraus (Session-Service + Revoke-Methode)
