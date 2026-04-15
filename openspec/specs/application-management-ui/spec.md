## ADDED Requirements

### Requirement: Anträge auflisten mit Status-Filter
Die Seite `/applications` SHALL alle Beitrittserklärungen als Liste anzeigen. Die Liste SHALL per Tabs nach Status filterbar sein: "Alle", "Offen", "Bestätigt", "Abgelehnt". Der Default-Tab SHALL "Offen" sein. Jeder Eintrag SHALL Name, E-Mail, Anteile, Status und Einreichungsdatum anzeigen.

#### Scenario: Offene Anträge anzeigen
- **WHEN** ein Admin die Seite `/applications` öffnet
- **THEN** werden alle Beitrittserklärungen mit Status "Offen" angezeigt

#### Scenario: Nach Status filtern
- **WHEN** der Admin auf den Tab "Bestätigt" klickt
- **THEN** werden nur Beitrittserklärungen mit Status "Bestätigt" angezeigt

#### Scenario: Keine Anträge vorhanden
- **WHEN** keine Beitrittserklärungen mit dem gewählten Status existieren
- **THEN** wird ein Hinweis "Keine Beitrittserklärungen vorhanden" angezeigt

### Requirement: Antragsdetails anzeigen
Beim Klick auf einen Antrag in der Liste SHALL ein Modal mit allen Details erscheinen: Anrede, Vorname, Nachname, E-Mail, Straße, Hausnummer, PLZ, Ort, Anzahl Anteile, Status und Einreichungsdatum.

#### Scenario: Detailansicht öffnen
- **WHEN** der Admin auf einen Antrag in der Liste klickt
- **THEN** öffnet sich ein Modal mit allen Antragsdaten

### Requirement: Antrag bestätigen
Für Anträge mit Status "Offen" SHALL ein "Bestätigen"-Button angezeigt werden. Beim Klick SHALL ein Bestätigungsdialog erscheinen, der darauf hinweist, dass ein neues Mitglied angelegt wird. Nach Bestätigung SHALL `POST /api/applications/{id}/confirm` aufgerufen und die Liste aktualisiert werden.

#### Scenario: Antrag bestätigen
- **WHEN** der Admin den "Bestätigen"-Button klickt und den Dialog bestätigt
- **THEN** wird der Antrag als bestätigt markiert, ein Mitglied wird angelegt, und die Liste aktualisiert sich

#### Scenario: Bestätigung abbrechen
- **WHEN** der Admin den Bestätigungsdialog abbricht
- **THEN** bleibt der Antrag unverändert

#### Scenario: Bereits bearbeiteter Antrag
- **WHEN** ein Antrag den Status "Bestätigt" oder "Abgelehnt" hat
- **THEN** werden die Aktions-Buttons nicht angezeigt

### Requirement: Antrag ablehnen
Für Anträge mit Status "Offen" SHALL ein "Ablehnen"-Button angezeigt werden. Beim Klick SHALL ein Bestätigungsdialog erscheinen. Nach Bestätigung SHALL `POST /api/applications/{id}/reject` aufgerufen und die Liste aktualisiert werden.

#### Scenario: Antrag ablehnen
- **WHEN** der Admin den "Ablehnen"-Button klickt und den Dialog bestätigt
- **THEN** wird der Antrag als abgelehnt markiert und die Liste aktualisiert sich

### Requirement: Navigation
Die TopBar SHALL für Admin-Benutzer einen neuen Link "Beitrittserklärungen" enthalten, der zur Seite `/applications` führt.

#### Scenario: Admin sieht Navigation
- **WHEN** ein eingeloggter Admin-Benutzer die TopBar sieht
- **THEN** ist der Link "Beitrittserklärungen" sichtbar

#### Scenario: Nicht-Admin sieht keinen Link
- **WHEN** ein Benutzer ohne Admin-Rechte die TopBar sieht
- **THEN** ist der Link "Beitrittserklärungen" nicht sichtbar

### Requirement: Frontend-API-Funktionen
Das Frontend SHALL API-Funktionen bereitstellen für:
- `get_applications(status_filter)` → `GET /api/applications?status=...`
- `get_application(id)` → `GET /api/applications/{id}`
- `confirm_application(id)` → `POST /api/applications/{id}/confirm`
- `reject_application(id)` → `POST /api/applications/{id}/reject`

#### Scenario: Anträge laden
- **WHEN** die Seite geladen wird
- **THEN** werden die Anträge per `GET /api/applications?status=Offen` geladen

### Requirement: i18n-Unterstützung
Alle UI-Texte SHALL über das i18n-System lokalisiert sein (DE, EN).

#### Scenario: Deutsche Oberfläche
- **WHEN** die Sprache auf Deutsch eingestellt ist
- **THEN** werden alle Labels und Texte auf Deutsch angezeigt
