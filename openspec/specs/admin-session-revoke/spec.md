### Requirement: Admin-Endpoint zum Beenden von User-Sessions

Das System SHALL einen Endpoint `POST /api/session/revoke/{user_id}` bereitstellen, der alle aktiven Sessions des angegebenen Users beendet. Der Endpoint MUST nur für Benutzer mit dem `admin`-Privileg zugänglich sein.

#### Scenario: Erfolgreicher Revoke durch Admin
- **WHEN** ein Admin `POST /api/session/revoke/alice` aufruft
- **THEN** werden alle Sessions von `alice` gelöscht und die Antwort enthält `{"message": "...", "revoked_count": N}` mit HTTP 200

#### Scenario: Nicht-Admin wird abgelehnt
- **WHEN** ein Benutzer ohne `admin`-Privileg `POST /api/session/revoke/alice` aufruft
- **THEN** antwortet das System mit HTTP 403

#### Scenario: Nicht authentifiziert
- **WHEN** ein nicht-authentifizierter Request `POST /api/session/revoke/alice` aufruft
- **THEN** antwortet das System mit HTTP 401

### Requirement: Sessions-beenden-Button auf der Permissions-Seite

Die Permissions-Seite SHALL pro User-Zeile einen "Sessions beenden"-Button anzeigen. Der Button MUST den Admin-Revoke-Endpoint für den jeweiligen User aufrufen.

#### Scenario: Button sichtbar für jeden User
- **WHEN** die Permissions-Seite geladen ist
- **THEN** hat jede User-Zeile einen "Sessions beenden"-Button

#### Scenario: Klick beendet Sessions
- **WHEN** ein Admin auf "Sessions beenden" für User `alice` klickt
- **THEN** wird `POST /api/session/revoke/alice` aufgerufen und visuelles Feedback (Ladezustand, Erfolgsmeldung) angezeigt

#### Scenario: API-Fehler zeigt Fehlermeldung
- **WHEN** der Revoke-API-Call fehlschlägt
- **THEN** wird eine Fehlermeldung angezeigt und der Button ist wieder klickbar

### Requirement: Automatisches Session-Revoke bei Admin-Rechteentzug

Beim Entziehen der Admin-Rolle über die Permissions-Seite MUST das System automatisch alle Sessions des betroffenen Users beenden.

#### Scenario: Admin-Checkbox deaktivieren löst Revoke aus
- **WHEN** ein Admin die Admin-Checkbox für User `alice` deaktiviert und der Rollenänderung erfolgreich ist
- **THEN** wird zusätzlich `POST /api/session/revoke/alice` aufgerufen

#### Scenario: Revoke-Fehler blockiert Rollenänderung nicht
- **WHEN** die Rollenänderung erfolgreich ist, aber der Revoke-Call fehlschlägt
- **THEN** bleibt die Rollenänderung bestehen und der Fehler wird geloggt
