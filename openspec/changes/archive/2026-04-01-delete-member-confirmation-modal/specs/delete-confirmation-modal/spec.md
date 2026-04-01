## ADDED Requirements

### Requirement: Bestätigungs-Modal vor Member-Löschung
Das System SHALL ein Bestätigungs-Modal anzeigen, wenn der Benutzer auf der Member-Detail-Seite den Löschen-Button klickt. Die Löschung SHALL erst nach expliziter Bestätigung im Modal ausgeführt werden.

#### Scenario: Benutzer klickt Löschen-Button
- **WHEN** der Benutzer auf der Member-Detail-Seite den Löschen-Button klickt
- **THEN** wird ein Modal mit einer Bestätigungsfrage angezeigt
- **THEN** enthält das Modal den Namen des Mitglieds
- **THEN** enthält das Modal einen Bestätigen-Button und einen Abbrechen-Button

#### Scenario: Benutzer bestätigt die Löschung
- **WHEN** das Bestätigungs-Modal angezeigt wird
- **WHEN** der Benutzer auf den Bestätigen-Button klickt
- **THEN** wird der Delete-API-Call ausgeführt
- **THEN** wird der Benutzer zur Member-Liste navigiert

#### Scenario: Benutzer bricht die Löschung ab
- **WHEN** das Bestätigungs-Modal angezeigt wird
- **WHEN** der Benutzer auf den Abbrechen-Button klickt
- **THEN** wird das Modal geschlossen
- **THEN** wird keine Löschung durchgeführt
- **THEN** bleibt der Benutzer auf der Member-Detail-Seite

### Requirement: i18n-Unterstützung für Modal-Texte
Alle Texte im Bestätigungs-Modal SHALL in allen unterstützten Sprachen (Englisch, Deutsch, Tschechisch) verfügbar sein.

#### Scenario: Modal-Texte in allen Sprachen
- **WHEN** das Bestätigungs-Modal angezeigt wird
- **THEN** werden Titel, Nachricht und Button-Beschriftungen in der aktuell eingestellten Sprache angezeigt
