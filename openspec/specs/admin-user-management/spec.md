# Admin User Management

## Purpose

Allow administrators to manage user preferences, roles, and display names via API endpoints and a dedicated frontend page.

## Requirements

### Requirement: Admin kann Preferences beliebiger User lesen
Das System SHALL einem Admin erlauben, die Preferences eines beliebigen Users über die REST-API abzurufen. Der Endpoint ist `GET /api/permission/user/{username}/preferences/{key}`.

#### Scenario: Admin liest sender_name eines anderen Users
- **WHEN** ein Admin `GET /api/permission/user/maria.k/preferences/sender_name` aufruft
- **THEN** gibt das System die Preference mit Key `sender_name` und dem gespeicherten Wert für User `maria.k` zurück (HTTP 200)

#### Scenario: Admin liest nicht-existierende Preference
- **WHEN** ein Admin `GET /api/permission/user/maria.k/preferences/nonexistent` aufruft und keine solche Preference existiert
- **THEN** gibt das System HTTP 404 zurück

#### Scenario: Nicht-Admin versucht Preference eines anderen Users zu lesen
- **WHEN** ein User ohne Admin-Privilege `GET /api/permission/user/maria.k/preferences/sender_name` aufruft
- **THEN** gibt das System HTTP 401/403 zurück

### Requirement: Admin kann Preferences beliebiger User schreiben
Das System SHALL einem Admin erlauben, Preferences eines beliebigen Users über die REST-API zu erstellen oder zu aktualisieren. Der Endpoint ist `PUT /api/permission/user/{username}/preferences/{key}`.

#### Scenario: Admin setzt sender_name eines anderen Users
- **WHEN** ein Admin `PUT /api/permission/user/maria.k/preferences/sender_name` mit Body `{"value": "Maria Koch"}` aufruft
- **THEN** speichert das System die Preference und gibt HTTP 200 mit der aktualisierten Preference zurück

#### Scenario: Admin aktualisiert bestehende Preference
- **WHEN** ein Admin eine existierende Preference überschreibt
- **THEN** wird der Wert aktualisiert (Upsert-Semantik)

#### Scenario: Nicht-Admin versucht Preference eines anderen Users zu schreiben
- **WHEN** ein User ohne Admin-Privilege `PUT /api/permission/user/maria.k/preferences/sender_name` aufruft
- **THEN** gibt das System HTTP 401/403 zurück

### Requirement: Frontend-Seite Berechtigungen zeigt alle User
Das System SHALL eine Admin-only Seite "Berechtigungen" bereitstellen, die alle registrierten User mit Anzeigename und Admin-Status auflistet.

#### Scenario: Admin öffnet Berechtigungen-Seite
- **WHEN** ein Admin die Berechtigungen-Seite öffnet
- **THEN** werden alle User mit Username, Anzeigename (sender_name) und Admin-Status (Checkbox) angezeigt

#### Scenario: Nicht-Admin sieht Seite nicht
- **WHEN** ein User ohne Admin-Privilege die Berechtigungen-Seite aufruft
- **THEN** wird die Seite nicht angezeigt oder der Zugriff verweigert

### Requirement: Admin kann Admin-Rolle über UI togglen
Das System SHALL einem Admin erlauben, die Admin-Rolle eines Users über eine Checkbox in der Berechtigungen-Seite zuzuweisen oder zu entziehen.

#### Scenario: Admin gibt User die Admin-Rolle
- **WHEN** ein Admin die Admin-Checkbox eines Users aktiviert
- **THEN** wird `POST /api/permission/user-role` mit `{"user_name": "<username>", "role_name": "admin"}` aufgerufen und der User hat die Admin-Rolle

#### Scenario: Admin entzieht User die Admin-Rolle
- **WHEN** ein Admin die Admin-Checkbox eines Users deaktiviert
- **THEN** wird `DELETE /api/permission/user-role` mit `{"user_name": "<username>", "role_name": "admin"}` aufgerufen und der User hat keine Admin-Rolle mehr

### Requirement: Admin kann Anzeigename über UI editieren
Das System SHALL einem Admin erlauben, den Anzeigenamen (sender_name) eines beliebigen Users über die Berechtigungen-Seite zu ändern.

#### Scenario: Admin ändert Anzeigename
- **WHEN** ein Admin den Anzeigename eines Users ändert und speichert
- **THEN** wird `PUT /api/permission/user/{username}/preferences/sender_name` aufgerufen und der neue Name ist gespeichert
