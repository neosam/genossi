## Why

Mitglieder der Genossenschaft müssen in Briefen und E-Mails korrekt angesprochen werden können ("Sehr geehrter Herr Dr. Müller", "Sehr geehrte Frau Prof. Schmidt", "Sehr geehrte Damen und Herren"). Dafür fehlen aktuell zwei Felder: die Anredeform und ein akademischer Titel. Ohne diese Felder ist keine korrekte formelle Korrespondenz möglich.

## What Changes

- Neues Enum-Feld `salutation` (Anrede) auf dem Member mit den Werten: `Herr`, `Frau`, `Firma`, oder leer (optional)
- Neues Freitext-Feld `title` (Titel) auf dem Member für akademische Titel wie "Dr.", "Prof. Dr." (optional)
- Beide Felder durch alle Layer: Migration, DAO, Service, REST API, Frontend
- Frontend: Anzeige und Bearbeitung in der Mitgliederliste (Spalten) und auf der Detail-Seite
- Anrede als Dropdown/Select, Titel als Freitext-Input

## Capabilities

### New Capabilities
_(keine neuen Capabilities - die Felder erweitern die bestehende Member-Verwaltung)_

### Modified Capabilities
- `member-management`: Das Member-Datenmodell wird um die Felder `salutation` (Enum: Herr/Frau/Firma, optional) und `title` (String, optional) erweitert. Die Member-Liste, Detail-Seite und alle CRUD-Operationen müssen die neuen Felder unterstützen.

## Impact

- **Database**: Neue SQLite-Migration mit zwei Spalten (`salutation TEXT`, `title TEXT`)
- **Backend**: DAO, Service, REST-Types und REST-Handler müssen die Felder durchreichen
- **Frontend**: Spalten-Definition, Mitgliederliste (inline editing), Detail-Seite, REST-Types
- **API**: `GET/POST/PUT /api/members` Request/Response-Schemas erweitert
- **Excel-Import**: Muss ggf. angepasst werden falls Import-Dateien diese Felder enthalten
- **Bestehende Daten**: Beide Felder optional, keine Breaking Changes für existierende Mitglieder
