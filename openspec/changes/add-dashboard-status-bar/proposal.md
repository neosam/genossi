## Why

Mitgliedsanträge und unbearbeitete Mails sind heute hinter zwei Klicks im Dropdown-Menü „Mitglieder" bzw. „Kommunikation" versteckt. Auf dem Handy ist das Menü zusätzlich hinter dem Hamburger-Icon weg. Damit fehlt dem Admin auf einen Blick die Information, ob etwas anliegt — und beim Öffnen der Anwendung erwartet er ohnehin sofort die Mitgliederliste, nicht die aktuelle Splash-Seite.

## What Changes

- Route `/` leitet automatisch auf `/members` um. Die bestehende Splash-Seite (`home.rs`) entfällt.
- Über der Mitgliederliste erscheint ein kompakter, einzeiliger Statusbalken mit aktuell zwei Items:
  - „N offene Anträge" bzw. „Keine offenen Anträge" — verlinkt auf `/applications`
  - „N offene Mails" bzw. „Keine offenen Mails" — verlinkt auf `/inbox`
- Beide Items sind immer sichtbar und immer klickbar (auch bei Anzahl 0), damit der Sprung in die jeweilige Liste ein konsistenter Reflex bleibt.
- Counts werden beim Aufruf der Mitgliederliste einmalig geladen. Aktualisierung erfolgt implizit beim erneuten Besuch der Seite — kein Polling.

## Capabilities

### New Capabilities
- `dashboard-status-bar`: Startseiten-Verhalten und Statusbalken über der Mitgliederliste mit Counts für offene Anträge und offene Mails.

### Modified Capabilities
<!-- keine - bestehende Capabilities (member-management, application-management-ui, inbox-task-tracking) bleiben in ihren Anforderungen unverändert -->

## Impact

- **Frontend**:
  - `genossi-frontend/src/page/home.rs`: Inhalt entfällt, Route `/` wird Redirect auf `/members`.
  - `genossi-frontend/src/page/members.rs`: Statusbalken oberhalb der bestehenden Toolbar/Filter einfügen.
  - Neue wiederverwendbare Komponente `genossi-frontend/src/component/status_bar.rs` (gemäß Component-First-Prinzip).
  - i18n-Keys für „N offene Anträge", „Keine offenen Anträge", „N offene Mails", „Keine offenen Mails" in allen drei Sprachen (de, en, cs).
- **Backend**: Keine Änderung. Bestehende Endpoints werden genutzt:
  - `GET /api/applications?status=Offen` (Counter via `.len()`)
  - `GET /api/inbox?status=open` (Counter via `.len()`)
- **Berechtigungen**: Aktuell sind alle aktiven Nutzer Admins; der Statusbalken wird ohne Sichtbarkeits-Gate ausgespielt. Eine spätere Verfeinerung der Rechte ist ein eigener Change.
