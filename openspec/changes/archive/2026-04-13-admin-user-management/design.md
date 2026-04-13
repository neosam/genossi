## Context

Das RBAC-System (User/Role/Privilege) ist vollständig im Backend implementiert mit REST-Endpoints unter `/api/permission/`. User-Preferences (Key-Value-Store pro User) existieren ebenfalls, aber die Preference-API erlaubt nur Zugriff auf die eigenen Preferences des eingeloggten Users. Es fehlt eine Admin-UI zur User-Verwaltung und ein Backend-Endpoint, um Preferences anderer User zu lesen/schreiben.

## Goals / Non-Goals

**Goals:**
- Admin kann alle User mit Anzeigename und Admin-Status sehen
- Admin kann Admin-Rolle für jeden User togglen
- Admin kann den `sender_name` jedes Users editieren
- Neue REST-Endpoints für Admin-Zugriff auf User-Preferences

**Non-Goals:**
- Neue User anlegen (werden automatisch via OIDC registriert)
- User löschen
- Andere Rollen/Privileges verwalten (nur Admin-Toggle)
- Andere Preferences als `sender_name` über die UI verwalten

## Decisions

### 1. Admin-Preference-Endpoints unter `/api/permission/user/{username}/preferences/{key}`

Die neuen Endpoints werden in den bestehenden Permission-Router integriert, da sie Admin-Privilege erfordern und logisch zur User-Verwaltung gehören.

**Alternative**: Eigene Route `/api/admin/user-preferences/{username}/{key}` — abgelehnt, da das Permission-Modul bereits alle Admin-User-Operationen bündelt.

### 2. Wiederverwendung des bestehenden `UserPreferenceDao`

Der DAO-Layer unterstützt bereits beliebige `user_id`-Werte. Es braucht nur eine neue Service-Methode, die statt `current_user_id` den übergebenen `username` nutzt.

**Ansatz**: Neue Methoden `get_user_preference_by_admin` und `set_user_preference_by_admin` im `PermissionService` (oder alternativ im `UserPreferenceService` mit explizitem `username`-Parameter). Da die Admin-Berechtigung geprüft werden muss und das logisch zum Permission-Bereich gehört, werden die Methoden direkt als REST-Handler im Permission-Modul implementiert, die intern den `UserPreferenceDao` nutzen.

### 3. Frontend: Neue Seite "Berechtigungen" mit Tabelle

Die Seite lädt alle User, deren Rollen und `sender_name` und zeigt sie in einer editierbaren Tabelle:
- Username (read-only, da PK)
- Anzeigename (editierbares Textfeld)
- Admin-Checkbox (togglet die admin-Rolle)

**Datenfluss Frontend:**
1. `GET /api/permission/user` → alle User
2. Pro User parallel: `GET /api/permission/user/{name}/roles` + `GET /api/permission/user/{name}/preferences/sender_name`
3. Änderungen einzeln speichern (kein globaler Save-Button)

### 4. REST-State muss `UserPreferenceDao` im Permission-Modul zugänglich machen

Der Permission-REST-Handler braucht Zugriff auf den `UserPreferenceDao`, um Preferences zu lesen/schreiben. Da `RestStateDef` bereits `user_preference_service()` exponiert, kann der neue Handler darauf zugreifen — allerdings umgeht er dann die UserPreferenceService-Logik. Besser: Der `UserPreferenceService` bekommt neue Methoden mit explizitem `username`-Parameter, die Admin-Privilege prüfen.

**Entscheidung**: `UserPreferenceService` um `get_by_key_for_user(username, key, context)` und `upsert_for_user(username, key, value, context)` erweitern. Diese prüfen Admin-Privilege. Die REST-Handler im Permission-Modul nutzen `rest_state.user_preference_service()`.

## Risks / Trade-offs

- **N+1 API-Calls im Frontend**: Pro User werden 2 zusätzliche Requests gemacht (Rollen + Preferences). Bei wenigen Usern (<20) akzeptabel. → Falls nötig, später Batch-Endpoint ergänzen.
- **Kein Schutz vor Selbst-Entadminnung**: Ein Admin könnte sich selbst die Admin-Rolle entziehen. → Akzeptiert, da über DB wiederherstellbar und im Scope dieses Changes nicht adressiert.
