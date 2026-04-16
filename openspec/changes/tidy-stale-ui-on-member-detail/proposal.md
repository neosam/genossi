## Why

Auf der Mitglieds-Detailseite (`member_details.rs`) zeigen zwei UI-Elemente Lärm an, der den Blick auf relevante Information verstellt:

1. Der Knopf „Eintrittsbestätigung generieren" wird auch dann angezeigt, wenn das Dokument bereits existiert. Der Code prüft das eigentlich (`member_details.rs:1005`), aber der Vergleich gegen den Hardcode-String `"join_confirmation"` greift offensichtlich nicht — der Button bleibt sichtbar.
2. Der grüne „Migriert"-Badge erscheint bei jedem Mitglied im Normalfall (über 99 % aller Mitglieder sind migriert) und enthält keine zusätzliche Information. Der gegenteilige Zustand (`pending`) zeigt dagegen wertvolle Daten und einen Bestätigungs-Button.

## What Changes

- Vergleich für „Dokument existiert schon" gegen `DocumentTypeTO::JoinConfirmation.as_str()` führen statt gegen Hardcode-String — eine Quelle der Wahrheit.
- Den `if status == "migrated"`-Zweig im Migration-Status-Block entfernen. Bei `migrated` wird gar nichts angezeigt; nur der `pending`-Block bleibt erhalten.

## Capabilities

### New Capabilities
- `member-detail-ui-tidy`: UI-Anforderungen an die Mitglieds-Detailseite, die sicherstellen, dass irrelevante oder bereits erledigte Elemente nicht angezeigt werden.

### Modified Capabilities
<!-- keine - existierende Capabilities bleiben in ihren Anforderungen unverändert; das Verhalten wird nur präzisiert/aufgeräumt -->

## Impact

- **Frontend**:
  - `genossi-frontend/src/page/member_details.rs:1003-1031` — Vergleich gegen Enum-Wert statt Hardcode.
  - `genossi-frontend/src/page/member_details.rs:322-371` — Den `if migrated`-Zweig entfernen.
- **Backend**: Keine Änderung.
- **Berechtigungen**: Keine Änderung.
