## MODIFIED Requirements

### Requirement: Member-Detail-Seite Löschfunktion
Die Member-Detail-Seite SHALL einen Löschen-Button anbieten, der vor der Ausführung der Soft-Delete-Operation ein Bestätigungs-Modal anzeigt. Der Delete-API-Call SHALL erst nach expliziter Bestätigung im Modal ausgeführt werden.

#### Scenario: Löschen mit Bestätigung
- **WHEN** der Benutzer auf der Member-Detail-Seite den Löschen-Button klickt
- **THEN** wird ein Bestätigungs-Modal angezeigt
- **WHEN** der Benutzer im Modal bestätigt
- **THEN** wird `DELETE /api/members/{id}` aufgerufen
- **THEN** wird zur Member-Liste navigiert

#### Scenario: Löschen abgebrochen
- **WHEN** der Benutzer auf der Member-Detail-Seite den Löschen-Button klickt
- **THEN** wird ein Bestätigungs-Modal angezeigt
- **WHEN** der Benutzer im Modal abbricht
- **THEN** wird keine API-Anfrage gesendet
- **THEN** bleibt der Benutzer auf der Detail-Seite
