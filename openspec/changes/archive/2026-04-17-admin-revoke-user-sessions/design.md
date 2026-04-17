## Context

Der SessionService hat bereits `revoke_all_for_user(user_id: &str)`, der alle Sessions eines Users löscht. Der bestehende Endpoint `POST /api/session/revoke-all` nutzt diese Methode für den eigenen User. Es fehlt ein admin-geschützter Endpoint, der die Sessions eines beliebigen Users beendet.

Die Permissions-Seite (`page/permissions.rs`) zeigt eine User-Tabelle mit Username, Anzeigename und Admin-Checkbox. Beim Deaktivieren der Admin-Checkbox wird `api::remove_user_role()` aufgerufen — der ideale Punkt, um automatisch ein Session-Revoke nachzuschalten.

## Goals / Non-Goals

**Goals:**
- Admin kann Sessions eines bestimmten Users sofort beenden
- Rechteentzug (Admin-Checkbox deaktivieren) revoked automatisch die Sessions des betroffenen Users
- Manueller "Sessions beenden"-Button pro User als Fallback

**Non-Goals:**
- Session-Liste oder -Zähler pro User anzeigen
- Revoke für einzelne Sessions (nur "alle Sessions dieses Users")
- Revoke bei Änderung anderer Rollen (nur Admin-Rolle)

## Decisions

### 1. Neuer Endpoint: `POST /api/session/revoke/{user_id}`

**Entscheidung:** Separater admin-only Endpoint statt Erweiterung des bestehenden Self-Service-Endpoints.

**Alternativen:**
- *Query-Parameter am bestehenden Endpoint:* Vermischt Self-Service und Admin-Logik, unklare Autorisierung.
- *Separater Endpoint:* Klare Trennung, eigene Privilegprüfung, eigene Rate-Limits möglich.

**Begründung:** Saubere Autorisierungsgrenzen. Der Self-Service-Endpoint braucht nur eine Session, der Admin-Endpoint braucht das `admin`-Privileg.

### 2. Privilegprüfung: `admin`-Rolle

**Entscheidung:** Der Endpoint prüft, ob der aufrufende User das `admin`-Privileg hat (gleiche Prüfung wie auf der Permissions-Seite selbst).

**Begründung:** Nur Admins sehen die Permissions-Seite, nur Admins dürfen Sessions anderer User beenden.

### 3. Automatisches Revoke beim Admin-Rechteentzug

**Entscheidung:** Nach erfolgreichem `remove_user_role()` wird im selben `spawn`-Block `api::revoke_user_sessions()` aufgerufen. Fehlschlag des Revoke wird geloggt, blockiert aber nicht den Rollen-Entzug.

**Alternativen:**
- *Backend-seitig im Role-Service:* Koppelt Rollen- und Session-Logik eng, bricht Single-Responsibility.
- *Frontend-seitig nach Rollen-Entzug:* Lockere Kopplung, einfach zu implementieren, sichtbar im Code.

**Begründung:** Im Frontend ist die Kopplung explizit und leicht anpassbar. Der Rollenruf ist authoritative; das Revoke ist best-effort.

### 4. UI: Button in der User-Zeile

**Entscheidung:** Kleiner "Sessions beenden"-Button in jeder User-Zeile auf der Permissions-Seite, analog zum bestehenden "Speichern"-Button für den Anzeigenamen.

**Begründung:** Konsistent mit dem bestehenden Tabellen-Layout. Kein neuer Modal nötig — ein Klick + visuelles Feedback (Ladezustand + Erfolgsmeldung) reicht, da die Aktion nicht destruktiv für Daten ist (nur Sessions).

## Risks / Trade-offs

**[Race Condition bei Rollenänderung + Revoke]** → Der User könnte zwischen `remove_role` und `revoke_sessions` eine neue Session erstellen (über OIDC). In der Praxis vernachlässigbar, da die Zeitspanne Millisekunden beträgt.

**[Kein Confirmation-Dialog]** → Sessions beenden ist weniger destruktiv als Datenlöschung (User kann sich jederzeit neu anmelden). Ein Klick-Feedback mit Ladezustand reicht.

**[Revoke-Fehler nach erfolgreichem Rollenänderung]** → Wird im Frontend geloggt. Der Admin sieht keinen expliziten Fehler, aber die Rolle ist entzogen. Beim nächsten Request des betroffenen Users greift die neue Rollenkonfiguration.
