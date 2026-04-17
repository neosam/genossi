## Why

Der `POST /api/session/revoke-all` Endpoint existiert bereits (aus `harden-auth-surface`), aber es gibt keinen UI-Zugang dafür. Wenn ein Vorstandsmitglied sein Gerät verliert, muss es aktuell entweder die API direkt aufrufen oder einen Admin bitten. Ein Button im Frontend macht den Self-Service-Revoke für alle zugänglich.

## What Changes

- Neuer "Alle Sessions beenden"-Button im User-Profil-Bereich (z.B. Auth-Info-Dropdown oder Einstellungs-Seite)
- Button ruft `api::revoke_all_sessions()` auf (Funktion existiert bereits in `genossi-frontend/src/api.rs`)
- Nach erfolgreichem Revoke: Redirect zum Login (`/authenticate`)
- Bestätigungsdialog vor dem Aufruf ("Alle Sessions werden beendet. Sie werden ausgeloggt.")

## Capabilities

### New Capabilities

- `revoke-sessions-ui`: Button und Bestätigungsdialog für Session-Revoke im Frontend

### Modified Capabilities

_(keine)_

## Impact

**Code:**
- Neue UI-Komponente unter `genossi-frontend/src/component/` (Revoke-Button mit Confirmation)
- Integration in bestehende Profil-/Menü-Komponente
- Kein Backend-Change nötig — API und Types existieren bereits

**Benutzer:**
- Neuer Button sichtbar im eingeloggten Zustand
- Nach Klick + Bestätigung: sofortiger Logout, Re-Login über Nextcloud-OIDC nötig

**Dependencies:**
- Keine neuen Crates
- Setzt `harden-auth-surface` voraus (Revoke-Endpoint + `SessionRevokeResponse` Type)
