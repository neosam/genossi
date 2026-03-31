## Why

Genossi verwaltet Genossenschaftsmitglieder, aber es gibt bisher keine Möglichkeit, Dokumente zu Mitgliedern zu speichern. In der Praxis fallen pro Mitglied mindestens eine Beitrittserklärung und eine Beitrittsbestätigung an, bei Aufstockungen weitere Dokumente. Aktuell müssen diese Unterlagen außerhalb des Systems verwaltet werden, was fehleranfällig und umständlich ist.

## What Changes

- Neues Dokumenten-System für Mitglieder mit Upload, Liste, Download und Soft-Delete
- Typisierte Dokumente: Beitrittserklärung, Beitrittsbestätigung, Aufstockung, Sonstige (mit Beschreibung)
- Singleton-Typen (Beitrittserklärung, Beitrittsbestätigung) erlauben max. 1 aktives Dokument pro Mitglied; erneuter Upload soft-deleted das vorherige
- Multi-Typen (Aufstockung, Sonstige) erlauben beliebig viele Dokumente
- Dateispeicherung im Dateisystem mit konfigurierbarem Basispfad (`DOCUMENT_STORAGE_PATH`)
- Opake Dateinamen (UUID-basiert) im Dateisystem, Original-Dateiname in DB
- Max. Dateigröße: 50 MB, alle MIME-Types erlaubt
- Zugriff nur für Vorstände
- REST-API: CRUD-Endpunkte unter `/members/{id}/documents`
- Frontend: Dokumente-Sektion in der Member-Details-Seite (Upload-Formular, Dokumentenliste mit Download/Delete)

## Capabilities

### New Capabilities
- `member-documents`: Dokumentenverwaltung für Mitglieder - Upload, Speicherung, Download und Soft-Delete von typisierten Dokumenten mit Dateisystem-Storage

### Modified Capabilities

_Keine bestehenden Capabilities werden verändert._

## Impact

- **Neue DB-Tabelle**: `member_document` mit Migration
- **Neuer Storage-Layer**: `DocumentStorage` Trait + Filesystem-Implementierung
- **Neue Env-Variable**: `DOCUMENT_STORAGE_PATH` für den Basispfad der Dokumentenspeicherung
- **Alle Architektur-Layer betroffen**: DAO, Service, REST, Frontend
- **API-Erweiterung**: 4 neue Endpunkte unter `/members/{member_id}/documents`
- **Frontend**: Erweiterung der Member-Details-Seite um Dokumente-Sektion
