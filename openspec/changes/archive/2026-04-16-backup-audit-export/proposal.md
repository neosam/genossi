## Why

Das Audit-Log und die qualifizierten Zeitstempel (.tsr-Tokens) werden aktuell nur lokal in der SQLite-Datenbank gespeichert. Der Backup-Worker exportiert bereits Mitgliederlisten, Aktionen, Dokumente und Kommunikation auf WebDAV — das Audit-Log und die Zeitstempel fehlen dort aber. Für ein vollständiges Beweispaket auf dem externen Speicher müssen beide mit exportiert werden. Gleichzeitig enthält der Timestamp-Worker aktuell WebDAV-Upload-Code, der dort nicht hingehört — der Backup-Worker ist der richtige Ort für alle WebDAV-Uploads.

## What Changes

- Export des gesamten Audit-Logs als `audit-log.csv` im Backup-Verzeichnis auf WebDAV (vollständiger Dump bei jedem Backup-Zyklus)
- Export aller .tsr-Timestamp-Tokens in ein `audit-timestamps/` Unterverzeichnis auf WebDAV (inkrementell: nur Dateien ohne `webdav_path`)
- Update des `webdav_path`-Felds im `audit_timestamp`-Record nach erfolgreichem Upload
- Entfernung des WebDAV-Upload-Codes aus dem Timestamp-Worker (Trennung der Verantwortlichkeiten)

## Capabilities

### New Capabilities
- `backup-audit-log`: Export des Audit-Logs als CSV-Datei im Backup-Zyklus

### Modified Capabilities
- `webdav-backup`: Backup-Worker erhält zwei zusätzliche Schritte: Audit-Log CSV Export und .tsr-Token Upload
- `qualified-timestamping`: Entfernung des WebDAV-Upload-Codes aus dem Timestamp-Worker; `AuditTimestampDao` erhält eine `update_webdav_path`-Methode

## Impact

- **genossi_backup**: Neuer Schritt im Backup-Worker für Audit-Log CSV und .tsr Upload
- **genossi_dao**: `AuditTimestampDao` braucht eine `update_webdav_path`-Methode
- **genossi_dao_impl_sqlite**: Implementierung der neuen DAO-Methode
- **genossi_service_impl/timestamp_worker.rs**: WebDAV-Code entfernen
- **WebDAV-Struktur**: Neue Datei `audit-log.csv` und Verzeichnis `audit-timestamps/` im Backup-Verzeichnis
