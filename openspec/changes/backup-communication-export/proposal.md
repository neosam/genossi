## Why

Die Genossenschaft muss auch ohne Genossi handlungsfähig bleiben. Aktuell werden Mitgliederdaten, Aktionen und Dokumente exportiert — aber die gesamte E-Mail-Kommunikation (ein-/ausgehend) fehlt im Backup. Damit gehen wichtige Geschäftsvorgänge verloren, wenn das System ausfällt.

## What Changes

- E-Mail-Kommunikation (inbound + outbound) wird als .txt-Dateien pro Mitglied in den Backup-Export aufgenommen
- Der bestehende `/backup/documents` REST-Endpoint liefert die Kommunikation im selben ZIP mit aus
- Der WebDAV-Worker synchronisiert einen neuen `kommunikation/` Ordner pro Mitglied (append-only, keine Hash-Prüfung)
- Nur Mails mit Mitglieds-Zuordnung werden exportiert (nicht zugeordnete werden ignoriert)
- Dateiformat: Plain-Text mit Header-Block (Richtung, Datum, Von, An, Betreff) + Body
- Dateiname-Pattern: `{YYYY-MM-DD}_{HHmm}_{richtung}_{betreff_sanitized}.txt`

## Capabilities

### New Capabilities
- `backup-communication`: Export der E-Mail-Kommunikation (inbound + outbound) als .txt-Dateien pro Mitglied, integriert in den bestehenden Backup-Mechanismus (ZIP-Download und WebDAV-Sync)

### Modified Capabilities

## Impact

- `genossi_backup/src/generator.rs`: Neue Funktion zur .txt-Generierung pro Mail
- `genossi_rest/src/backup.rs`: `export_documents` Endpoint erweitern um `kommunikation/` Ordner im ZIP
- `genossi_backup/src/worker.rs`: Backup-Zyklus um Kommunikations-Sync erweitern
- `genossi_backup/src/sync.rs`: Neue Sync-Logik für Kommunikation (append-only)
- `genossi_dao/src/backup.rs`: Neuer DAO-Trait/Methode für Kommunikations-Export-Daten
- `genossi_dao_impl_sqlite/src/backup.rs`: SQL-Queries für Mail-Daten mit Member-Info
- Neue Migration für Sync-Tracking-Tabelle (welche Mails schon gesynct)
