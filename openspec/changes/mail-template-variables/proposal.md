## Why

E-Mails werden aktuell mit identischem Text an alle Empfänger gesendet. Mitglieder können nicht persönlich angesprochen werden (z.B. "Sehr geehrter Herr Mustermann"). Für einen professionellen Vereinsauftritt ist personalisierte Ansprache mit Variablen und bedingter Logik notwendig.

## What Changes

- Template-Engine (MiniJinja) für Subject und Body in E-Mails
- Alle Felder des Member-Objekts als Template-Variablen verfügbar (`{{ first_name }}`, `{{ last_name }}`, `{{ salutation }}`, etc.)
- Bedingte Logik in Templates (`{% if salutation == "Frau" %}...{% endif %}`)
- Preview-Endpoint zum Testen eines Templates gegen ein konkretes Mitglied
- Template-Validierung vor dem Senden (Syntax-Check + Probe-Rendering gegen alle Empfänger)
- Variablen-Einfüge-Buttons im Frontend-Editor
- Worker löst Templates zur Sendezeit pro Empfänger auf (member_id pro Recipient bereits vorhanden)
- Recipients ohne member_id werden nicht unterstützt (nur Member-basierte E-Mails)

## Capabilities

### New Capabilities
- `mail-template-rendering`: Template-Engine-Integration mit MiniJinja, Variablen-Kontext aus Member-Daten, Template-Validierung und Preview-Endpoint
- `mail-template-ui`: Frontend-Variablen-Buttons zum Einfügen von Template-Variablen und Live-Preview gegen ausgewähltes Mitglied

### Modified Capabilities
- `mail-sending`: Worker muss Templates pro Empfänger auflösen statt identischen Body zu senden. Subject wird ebenfalls als Template behandelt. member_id ist jetzt Pflicht für alle Recipients.

## Impact

- **Backend**: `genossi_mail` Crate bekommt MiniJinja-Dependency. Worker braucht Zugriff auf MemberDao. Neuer REST-Endpoint `POST /api/mail/preview`.
- **Frontend**: Mail-Compose-Seite bekommt Variablen-Buttons und Preview-Panel.
- **Dependencies**: `minijinja` Crate wird zum Workspace hinzugefügt.
- **Datenbank**: Keine Schema-Änderungen nötig (job.body speichert Template-String, member_id pro Recipient existiert bereits).
- **API**: Neuer Endpoint `/api/mail/preview`. `POST /api/mail/send-bulk` akzeptiert weiterhin dasselbe Format, interpretiert body/subject aber als Templates. Validierung wird vor Job-Erstellung durchgeführt.
