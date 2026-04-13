## 1. Fix

- [x] 1.1 In `genossi_backup/src/sync.rs`, Funktion `sync_communications`: Vor der for-Schleife `mkcol_recursive` für `{base_dir}/kommunikation` aufrufen (analog zu `sync_documents` Zeile 28-29)

## 2. Tests

- [x] 2.1 Bestehenden Test `test_sync_new_communication` erweitern: MKCOL-Mock soll prüfen, dass sowohl das `kommunikation`-Basisverzeichnis als auch das Mitglieder-Verzeichnis angelegt werden
- [x] 2.2 Neuen Test hinzufügen: Verifikation, dass `mkcol_recursive` für den kommunikation-Pfad aufgerufen wird, bevor Mitglieder-Verzeichnisse erstellt werden
