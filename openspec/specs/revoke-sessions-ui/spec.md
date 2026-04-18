### Requirement: Revoke-Sessions-Button in der TopBar

Die TopBar SHALL einen "Sessions beenden"-Eintrag im Auth-Bereich anzeigen, wenn der Benutzer eingeloggt ist. Der Eintrag MUST neben dem bestehenden Logout-Link platziert sein.

#### Scenario: Button sichtbar bei eingeloggtem Benutzer
- **WHEN** ein Benutzer eingeloggt ist und die TopBar angezeigt wird
- **THEN** ist ein "Sessions beenden"-Eintrag im Auth-Bereich der TopBar sichtbar

#### Scenario: Button nicht sichtbar ohne Login
- **WHEN** kein Benutzer eingeloggt ist
- **THEN** wird kein "Sessions beenden"-Eintrag angezeigt

### Requirement: Bestätigungsdialog vor Session-Revoke

Ein Klick auf den "Sessions beenden"-Button MUST einen modalen Bestätigungsdialog öffnen. Der Dialog SHALL den Benutzer darauf hinweisen, dass alle Sessions beendet werden und ein erneutes Anmelden erforderlich ist.

#### Scenario: Dialog öffnet bei Klick
- **WHEN** der Benutzer auf "Sessions beenden" klickt
- **THEN** öffnet sich ein modaler Dialog mit Warntext, Bestätigungs-Button und Abbrechen-Button

#### Scenario: Abbrechen schließt Dialog
- **WHEN** der Bestätigungsdialog offen ist und der Benutzer "Abbrechen" klickt
- **THEN** schließt sich der Dialog und keine Aktion wird ausgeführt

### Requirement: Session-Revoke-Ausführung

Der Bestätigungs-Button im Dialog MUST `POST /api/session/revoke-all` über `api::revoke_all_sessions()` aufrufen. Während des API-Calls MUST der Bestätigungs-Button deaktiviert sein, um Doppelklicks zu verhindern.

#### Scenario: Erfolgreicher Revoke
- **WHEN** der Benutzer im Dialog auf "Bestätigen" klickt und der API-Call erfolgreich ist
- **THEN** wird der Benutzer zum Backend-Logout-Endpoint (`{backend_url}/logout`) weitergeleitet

#### Scenario: Doppelklick-Schutz
- **WHEN** der Benutzer auf "Bestätigen" klickt und der API-Call noch läuft
- **THEN** ist der Bestätigungs-Button deaktiviert und zeigt einen Ladezustand an

#### Scenario: API-Fehler
- **WHEN** der API-Call fehlschlägt (z.B. Netzwerkfehler)
- **THEN** bleibt der Dialog offen, eine Fehlermeldung wird über die `ErrorAlert`-Komponente im Dialog angezeigt, und der Benutzer kann es erneut versuchen oder abbrechen
