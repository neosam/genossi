## Why

Für die Migration müssen ca. 600 Beitrittserklärungen hochgeladen werden. Aktuell geht der Upload nur über die Mitglied-Detailseite — man müsste also 600 Mal in ein Mitglied reinklicken. Es braucht eine Möglichkeit, Dokumente direkt aus der Mitgliederliste hochzuladen.

## What Changes

- **Upload-Spalte in der Mitgliederliste**: Eine optionale Spezialspalte "Dokument" im Spalten-Picker, die pro Zeile einen Datei-Upload ermöglicht
- **Globale Upload-Einstellungen**: Über der Tabelle erscheinen (wenn die Spalte aktiv ist) ein Dokumenttyp-Dropdown und ein optionales Beschreibungsfeld — gelten für alle Uploads
- **Bulk Document Count Endpunkt**: Neuer Backend-Endpunkt, der pro Mitglied die Anzahl vorhandener Dokumente eines bestimmten Typs liefert (ein Request statt 600)
- **Upload-Status pro Zeile**: Zeigt an ob ein Dokument bereits vorhanden ist (Upload blockiert), gerade hochgeladen wird, erfolgreich war oder fehlgeschlagen ist
- Die Upload-Spalte funktioniert unabhängig vom Bearbeitungsmodus

## Dependencies

- `generate-and-store-documents`: Singleton-Upload-Verhalten muss zuerst umgestellt sein (Upload blockieren statt auto-replace), da die Bulk-Upload-Spalte sich auf dieses Verhalten verlässt

## Capabilities

### New Capabilities
- `member-list-document-upload`: Upload-Spalte in der Mitgliederliste mit globalen Einstellungen und Status-Anzeige
- `member-document-counts`: Bulk-Endpunkt für Dokumenten-Counts pro Typ

### Modified Capabilities
- `member-search`: Spalten-Picker wird um die Upload-Spezialspalte erweitert

## Impact

- `genossi_rest/src/member_document.rs`: Neuer Endpunkt `GET /api/member-documents/counts?type={type}` für Bulk-Counts
- `genossi_service/src/member_document.rs`: Neue Service-Methode für Counts nach Typ
- `genossi_dao/src/member_document.rs`: Neue DAO-Methode für Count-Query
- `genossi_dao_impl_sqlite/src/member_document.rs`: SQLite-Implementierung der Count-Query
- `genossi-frontend/src/page/members.rs`: Upload-Spalte mit Sonder-Rendering, globale Einstellungen UI, Upload-Status-Tracking
- `genossi-frontend/src/api.rs`: Neuer API-Call für Bulk-Counts
- `genossi-frontend/rest-types/src/lib.rs`: Neue Typen für Count-Response
