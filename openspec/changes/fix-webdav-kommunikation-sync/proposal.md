## Why

Beim WebDAV-Export der Kommunikationsdateien schlägt das Erstellen der Mitglieder-Verzeichnisse fehl (`Sabre\DAV\Exception\Conflict` / HTTP 409). Das Problem tritt bei jedem Mitglied auf und verhindert den kompletten Kommunikations-Export. Ursache: Das übergeordnete `kommunikation/`-Verzeichnis wird nie explizit angelegt, bevor die Mitglieder-Unterverzeichnisse erstellt werden.

## What Changes

- `sync_communications` in `genossi_backup/src/sync.rs` wird um einen `mkcol_recursive`-Aufruf für das `kommunikation/`-Basisverzeichnis ergänzt, analog zum bestehenden Pattern in `sync_documents`
- Kein Breaking Change, reine Bugfix-Korrektur

## Capabilities

### New Capabilities

(keine)

### Modified Capabilities

- `backup-communication`: Das `kommunikation/`-Elternverzeichnis muss vor der Mitglieder-Verzeichnis-Erstellung rekursiv angelegt werden

## Impact

- **Code**: `genossi_backup/src/sync.rs` — Funktion `sync_communications`
- **Verhalten**: Kommunikations-Export funktioniert wieder vollständig über WebDAV
- **APIs/Dependencies**: Keine Änderung
