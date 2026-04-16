## ADDED Requirements

### Requirement: Session Lifetime Caps

Das System SHALL jeder aktiven Server-Session sowohl einen absoluten Lebensdauer-Deckel von 14 Tagen (ab Erstellung) als auch einen Inaktivitäts-Timeout von 24 Stunden (ab letztem authentifiziertem Request) zuweisen. Eine Session SHALL ungültig sein, sobald einer der beiden Werte überschritten ist.

#### Scenario: Session ist jünger als beide Timeouts

- **WHEN** ein Request mit einer Session-ID eintrifft, deren `created` weniger als 14 Tage zurückliegt UND deren `last_used_at` weniger als 24 Stunden zurückliegt
- **THEN** das System akzeptiert die Session als gültig und aktualisiert `last_used_at` auf den aktuellen Zeitpunkt

#### Scenario: Session hat absolutes Timeout überschritten

- **WHEN** ein Request mit einer Session-ID eintrifft, deren `created` mehr als 14 Tage zurückliegt
- **THEN** das System lehnt die Session ab, löscht sie aus der DB und antwortet mit HTTP 401

#### Scenario: Session hat Inaktivitäts-Timeout überschritten

- **WHEN** ein Request mit einer Session-ID eintrifft, deren `last_used_at` mehr als 24 Stunden zurückliegt (auch wenn `created` noch innerhalb der 14 Tage liegt)
- **THEN** das System lehnt die Session ab, löscht sie aus der DB und antwortet mit HTTP 401

### Requirement: Session-ID darf nicht in Logs erscheinen

Das System SHALL niemals Session-IDs, Session-Cookies oder Session-Entities als Strukturen (z.B. via `{:?}`) in Log-Ausgaben oder Error-Messages schreiben. Erlaubt ist die Ausgabe der User-ID und des Login-Ereignisses auf `debug`-Level.

#### Scenario: Authentifizierter Request

- **WHEN** ein Request mit gültiger Session verifiziert wird
- **THEN** die Logs enthalten höchstens die User-ID und einen generischen Status-Hinweis, aber keinerlei Session-ID, Cookie-Wert oder Session-Struktur-Debug-Output

#### Scenario: Session-Verifikation schlägt fehl

- **WHEN** eine Session nicht gefunden oder abgelaufen ist
- **THEN** die Logs verzeichnen den Fehl-Status ohne die Session-ID des versuchten Zugriffs auszugeben

### Requirement: Self-Service Session Revocation

Das System SHALL authentifizierten Usern einen Endpoint `POST /api/session/revoke-all` bereitstellen, der alle aktiven Sessions des aktuellen Users in der DB löscht — einschließlich der Session, mit der der Request ausgeführt wurde.

#### Scenario: Authentifizierter User revoked alle Sessions

- **WHEN** ein User mit gültiger Session `POST /api/session/revoke-all` aufruft
- **THEN** das System löscht alle `session`-Einträge mit `user_id` des Aufrufers und antwortet mit HTTP 200 und einer Bestätigungsnachricht

#### Scenario: Nachfolgender Request mit gleicher Session

- **WHEN** nach einem erfolgreichen `revoke-all` ein Request mit der zuvor gültigen Session-ID eintrifft
- **THEN** das System lehnt den Request mit HTTP 401 ab, weil die Session nicht mehr existiert

#### Scenario: Unauthentifizierter Aufruf

- **WHEN** `POST /api/session/revoke-all` ohne gültige Session aufgerufen wird
- **THEN** das System antwortet mit HTTP 401 und führt keine DB-Änderung durch

### Requirement: Kein Panic im Auth-Path

Das System SHALL in Middleware und Handlern des Auth-Pfads (OIDC-Callback, Session-Erstellung, Session-Verifikation) keine Panics auslösen. Fehler aus der DB-Schicht SHALL als HTTP 500 Response zurückgegeben werden, ohne interne Details an den Client zu exponieren. Der Server-seitige Log SHALL den Fehler mit genug Kontext für Debugging enthalten (aber ohne Session-IDs, s.o.).

#### Scenario: DB-Fehler beim Session-Erstellen

- **WHEN** während `register_session` ein DB-Fehler auftritt
- **THEN** der Server antwortet mit HTTP 500, die Response-Body enthält keine internen Fehlerdetails, und der Log enthält User-ID und Fehler-Typ

### Requirement: Session-Inaktivitäts-Tracking via `last_used_at`

Das System SHALL eine Spalte `last_used_at` (Unix-Timestamp, Sekunden) in der `session`-Tabelle führen. Bei jeder erfolgreichen Session-Verifikation SHALL der Wert auf den aktuellen Zeitpunkt aktualisiert werden. Bei Session-Erstellung SHALL der Wert gleich `created` gesetzt werden. Für bereits existierende Sessions beim Einspielen der Migration SHALL der Wert mit `created` initialisiert werden.

#### Scenario: Session-Erstellung

- **WHEN** eine neue Session erstellt wird
- **THEN** `last_used_at` ist gleich `created` und wird als Unix-Timestamp in Sekunden gespeichert

#### Scenario: Session-Verifikation aktualisiert Timestamp

- **WHEN** eine Session erfolgreich verifiziert wird
- **THEN** `last_used_at` wird auf den aktuellen Unix-Timestamp aktualisiert

#### Scenario: Bestandsdaten bei Migration

- **WHEN** die Migration `last_used_at` zur `session`-Tabelle hinzufügt
- **THEN** alle existierenden Zeilen bekommen `last_used_at = created` als Initialwert
