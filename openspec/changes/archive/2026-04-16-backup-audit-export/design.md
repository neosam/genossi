## Context

Der Backup-Worker (`genossi_backup::worker`) exportiert periodisch Mitgliederlisten, Aktionen, Dokumente und Kommunikation auf einen WebDAV-Server. Das Audit-Log und die qualifizierten Zeitstempel (.tsr-Tokens) werden aktuell nur in SQLite gespeichert. Der Timestamp-Worker enthält WebDAV-Upload-Code, der besser im Backup-Worker aufgehoben ist.

## Goals / Non-Goals

**Goals:**
- Audit-Log als `audit-log.csv` im Backup-Verzeichnis exportieren (kompletter Dump pro Zyklus)
- .tsr-Timestamp-Tokens inkrementell in `audit-timestamps/` Unterverzeichnis hochladen
- `webdav_path` im `audit_timestamp`-Record nach Upload aktualisieren
- WebDAV-Code aus dem Timestamp-Worker entfernen

**Non-Goals:**
- Inkrementeller Audit-Log-Export (zu komplex, vollständiger Dump ist bei erwarteter Größe unproblematisch)
- Verifikation der .tsr-Tokens beim Upload
- Änderungen am Backup-Intervall oder der WebDAV-Konfiguration

## Decisions

### 1. Audit-Log als CSV im Root-Verzeichnis

**Entscheidung**: `audit-log.csv` wird direkt im `backup_webdav_directory` abgelegt, neben den bestehenden CSV-Dateien.

**Rationale**: Konsistent mit `mitgliederliste-*.csv` und `aktionen.csv`. CSV ist menschenlesbar und kann in Excel/LibreOffice geöffnet werden. Spalten: `id,timestamp,user_id,process,transaction_id,entity_type,entity_id,action,field_name,old_value,new_value,prev_hash,entry_hash`.

### 2. .tsr-Tokens in Unterverzeichnis

**Entscheidung**: .tsr-Dateien werden in `audit-timestamps/` Unterverzeichnis abgelegt. Dateiname: `audit-checkpoint-{ISO8601-Timestamp}.tsr`.

**Rationale**: Separates Verzeichnis hält das Root-Verzeichnis übersichtlich, da sich über Zeit viele .tsr-Dateien ansammeln.

### 3. Inkrementeller .tsr-Upload über webdav_path

**Entscheidung**: Nur `audit_timestamp`-Einträge mit `webdav_path = NULL` und `status = "success"` werden hochgeladen. Nach erfolgreichem Upload wird `webdav_path` aktualisiert.

**Rationale**: Vermeidet doppelte Uploads. Braucht keine separate Sync-Tabelle — das bestehende `webdav_path`-Feld reicht als Marker.

### 4. Neue DAO-Methode statt Sync-Tabelle

**Entscheidung**: `AuditTimestampDao` bekommt zwei neue Methoden: `get_pending_upload()` (alle mit `webdav_path IS NULL AND status = 'success'`) und `update_webdav_path(id, path)`.

**Rationale**: Einfacher als eine separate Sync-Tabelle. Das bestehende Datenmodell hat das Feld bereits.

### 5. Backup-Worker bekommt AuditLogDao und AuditTimestampDao als Dependencies

**Entscheidung**: Der `run_backup_cycle` und `start_backup_worker` bekommen die beiden DAOs als zusätzliche Parameter.

**Rationale**: Folgt dem bestehenden Pattern — der Worker hat bereits `BackupDao`, `BackupDocumentSyncDao`, etc. als Parameter.

## Risks / Trade-offs

**[Große Audit-Logs]** → Bei sehr vielen Audit-Einträgen könnte der CSV-Export langsam werden. Mitigation: Für Genossenschaften mit ein paar hundert Mitgliedern erwarten wir ~10.000-100.000 Einträge — ein vollständiger Dump bleibt performant.

**[WebDAV-Upload schlägt fehl]** → Wenn der Upload einer .tsr-Datei fehlschlägt, bleibt `webdav_path = NULL` und der nächste Zyklus versucht es erneut. Kein Datenverlust, da das Token lokal in der DB bleibt.
