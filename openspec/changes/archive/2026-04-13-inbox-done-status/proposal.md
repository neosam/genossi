## Why

Eingehende Mails enthalten häufig Handlungsanweisungen (z.B. "Adresse geändert", "Kündigung"). Das aktuelle Statusmodell bildet dies nicht ab — es gibt keinen Weg, eine Mail als "erledigt" zu markieren oder nach offenen Aufgaben zu filtern. Außerdem sind `replied`, `assigned` und `done` konzeptionell unabhängig voneinander, werden aber aktuell in einem einzigen `status`-Feld vermischt.

## What Changes

- **BREAKING**: Das `status`-Feld in `inbound_mails` wird aufgelöst in drei unabhängige Felder:
  - `replied` (bool) — wurde auf diese Mail geantwortet?
  - `done` (bool) — ist die Aufgabe hinter der Mail erledigt?
  - `archived` (bool) — wurde die Mail im IMAP archiviert?
- `assigned_member_id` bleibt unverändert als eigenständiges Feld
- **BREAKING**: Der `ignored`-Status und der zugehörige REST-Endpoint `/api/inbox/{id}/ignore` werden entfernt. Bestehende `ignored`-Mails werden zu `done=true` migriert.
- Neuer REST-Endpoint `/api/inbox/{id}/done` zum Markieren als erledigt
- Die Inbox-Standardansicht zeigt nur offene Mails (`done == false`)
- Filter in der UI: Offen / Erledigt / Alle

## Capabilities

### New Capabilities
- `inbox-task-tracking`: Ermöglicht das Markieren von Inbox-Mails als "erledigt" und das Filtern nach offenem/erledigtem Status

### Modified Capabilities
_(keine bestehenden Specs betroffen)_

## Impact

- **Datenbank**: Migration von `status`-Feld zu `replied`, `done`, `archived` Feldern in `inbound_mails`
- **DAO**: `InboundMail`-Struct und SQLite-Queries anpassen
- **Service**: Status-Transitions-Logik durch unabhängige Feld-Updates ersetzen
- **REST**: Endpoint `/api/inbox/{id}/ignore` entfernen, `/api/inbox/{id}/done` hinzufügen, Response-Typ anpassen
- **Frontend**: Status-Badge durch individuelle Indikatoren ersetzen, Filter-UI hinzufügen
