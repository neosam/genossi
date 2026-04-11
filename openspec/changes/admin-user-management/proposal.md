## Why

Admins können aktuell keine User verwalten — weder Admin-Rechte vergeben/entziehen noch Anzeigenamen anderer User ändern. Die Backend-Endpoints für RBAC existieren bereits, aber es fehlt eine Frontend-Seite und ein Admin-Endpoint zum Lesen/Schreiben von User-Preferences anderer User.

## What Changes

- Neuer REST-Endpoint: Admin kann Preferences beliebiger User lesen und schreiben (`GET/PUT /api/permission/user/{username}/preferences/{key}`)
- Neue Frontend-Seite "Berechtigungen" (Admin-only): Listet alle User mit Anzeigename und Admin-Status
- Pro User: Admin-Rolle togglen (bestehende `user-role` Endpoints nutzen)
- Pro User: `sender_name` Preference inline editieren (neuer Admin-Preference-Endpoint)

## Capabilities

### New Capabilities
- `admin-user-management`: Admin-Seite zur Verwaltung von Usern — Admin-Rolle togglen und Anzeigenamen anderer User ändern

### Modified Capabilities

(keine bestehenden Specs betroffen)

## Impact

- **Backend**: Neue Endpoints unter `/api/permission/user/{username}/preferences/{key}` im `genossi_rest` Crate
- **Service Layer**: `PermissionService` oder `UserPreferenceService` um Admin-Preference-Zugriff erweitern
- **DAO Layer**: Bestehende `UserPreferenceDao` kann wiederverwendet werden (unterstützt bereits beliebige user_ids)
- **Frontend**: Neue Seite + Navigation-Eintrag, nutzt bestehende Permission-API-Calls + neue Preference-Calls
- **Bestehende Endpoints**: Keine Änderungen, keine Breaking Changes
