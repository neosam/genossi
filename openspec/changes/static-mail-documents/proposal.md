## Why

Für wiederkehrende Versandaktionen (z.B. Satzung, Infoflyer, Formulare) müssen Vorstandsmitglieder dasselbe PDF heute außerhalb des Systems beilegen. Die bestehenden `mail-attachments` sind an `member-documents` gekoppelt und nur für einen einzelnen Empfänger nutzbar — globale, wiederverwendbare Anhänge sind damit nicht möglich.

## What Changes

- Neue Verwaltung globaler, hochladbarer Dokumente ("statische Dokumente"), unabhängig von Mitgliedern
- Upload, Liste, Download und (Soft-)Delete über REST-API und Admin-UI
- Speicherung der Dateien auf dem Filesystem; Metadaten in der Datenbank
- Größen- und Content-Type-Begrenzung beim Upload
- Beim Bulk-Mail-Versand können optional mehrere statische Dokumente ausgewählt werden; jeder Empfänger erhält dieselben Dateien
- Mail-Worker hängt statische Dokumente als Multipart-Anhänge an jede ausgehende Nachricht an
- Neue Admin-Seite „Dokumente" im Frontend mit Upload- und Löschfunktion
- Mail-Compose-Seite erhält Multiselect für statische Dokumente (zusätzlich zu bestehenden member-gebundenen Anhängen)

## Capabilities

### New Capabilities
- `static-documents`: Verwaltung (Upload, Liste, Download, Soft-Delete) globaler Dokumente, die unabhängig von Mitgliedern existieren und für Mail-Versand wiederverwendbar sind. Speicherung auf Filesystem, Metadaten in DB.

### Modified Capabilities
- `mail-sending`: Der Bulk-Send-Endpoint akzeptiert optional eine Liste von `static_document_ids`. Der Mail-Worker baut Multipart-Nachrichten, in denen jeder Empfänger dieselben statischen Dokumente als Anhang erhält — zusätzlich zu bereits bestehenden member-gebundenen Anhängen.

## Impact

- **Datenbank**: Neue Tabellen `static_documents` (Metadaten) und `mail_static_attachments` (Join Mail ↔ Dokument). Neue SQLite-Migration.
- **Filesystem**: Neuer Ordner für Dokumente, Pfad per ENV-Variable `STATIC_DOCUMENTS_PATH` konfigurierbar (Default z.B. `./data/static_documents`). Dateien liegen unter `<uuid>`.
- **Backend**:
  - Neuer DAO + Impl für `static_documents` und `mail_static_attachments`
  - Neuer Service für Upload/Download/Delete inkl. Validierung (Größe, Content-Type)
  - `genossi_rest` erhält neue Routen unter `/api/static-documents` (GET list, POST upload, GET/{id} download, DELETE/{id})
  - `genossi_mail` Worker erweitert um Einbindung statischer Dokumente in Multipart-MIME
  - Bulk-Mail-Endpoint und entsprechende Request-Typen in `genossi_rest_types` erweitert
- **Frontend**:
  - Neue Seite „Dokumente" mit Liste, Upload-Button und Löschen
  - Mail-Compose-Seite um Multiselect für statische Dokumente ergänzen
  - `api.rs` bekommt neue Client-Funktionen
- **Config**: Neue ENV-Variable `STATIC_DOCUMENTS_PATH`; optionales Größenlimit (z.B. `STATIC_DOCUMENTS_MAX_BYTES`)
- **Dependencies**: Keine neuen Crates — `lettre` unterstützt Multipart bereits, Axum-Multipart für Upload ist vorhanden bzw. trivial hinzufügbar
- **Auth**: Upload/Delete erfordert Vorstandsrechte (analog zu Template-Verwaltung)
