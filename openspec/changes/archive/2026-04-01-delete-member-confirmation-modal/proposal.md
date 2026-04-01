## Why

Aktuell wird beim Klick auf den Löschen-Button auf der Member-Detail-Seite das Mitglied sofort (soft-)gelöscht, ohne Rückfrage. Das birgt das Risiko versehentlicher Löschungen, insbesondere bei Fehlklicks. Ein Bestätigungs-Modal ist eine bewährte UX-Praxis, um destruktive Aktionen abzusichern.

## What Changes

- Beim Klick auf den Löschen-Button auf der Member-Detail-Seite wird ein Bestätigungs-Modal angezeigt, anstatt die Löschung sofort auszuführen
- Das Modal zeigt den Namen des Mitglieds und fragt explizit nach Bestätigung
- Erst nach Bestätigung im Modal wird der Delete-API-Call ausgeführt
- Das bereits existierende `Modal`-Component wird dafür nutzbar gemacht (aktuell definiert aber nicht exportiert)

## Capabilities

### New Capabilities
- `delete-confirmation-modal`: Bestätigungs-Dialog vor dem Löschen eines Members auf der Detail-Seite

### Modified Capabilities
- `member-management`: Die Member-Detail-Seite erhält einen Bestätigungs-Schritt vor der Löschung

## Impact

- **Frontend**: `genossi-frontend/src/page/member_details.rs` - Delete-Button-Logik wird angepasst
- **Frontend**: `genossi-frontend/src/component/modal.rs` - Modal-Component wird exportiert und ggf. erweitert
- **Frontend**: `genossi-frontend/src/component/mod.rs` - Modal-Export hinzufügen
- **Backend**: Keine Änderungen nötig
