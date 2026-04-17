## Why

Bei Mitgliederversammlungen muss die Anwesenheit der Mitglieder verifiziert und dokumentiert werden. Diese Aufgabe wird typischerweise von Personen durchgeführt, die keinen regulären Zugang zum System haben. Aktuell gibt es keine Möglichkeit, temporäre, eingeschränkte Zugänge für diesen Zweck bereitzustellen, und keine Check-in-Funktionalität.

## What Changes

- Neues Konzept "Assembly Session": Ein Admin kann eine zeitlich begrenzte Session für eine Mitgliederversammlung erstellen und beenden
- Spezieller Zugangstoken für die Assembly Session, der ohne reguläre Authentifizierung funktioniert
- Neuer REST-Endpoint liefert eine reduzierte Mitgliederliste (nur aktive Mitglieder mit: Mitgliedsnummer, Anrede, Titel, Vorname, Nachname)
- Check-in-Funktionalität: Mitglieder können als anwesend markiert werden
- Anzeige des Check-in-Status (ob jemand bereits eingecheckt ist)
- Admin-UI zum Erstellen, Einsehen und Beenden von Assembly Sessions
- Check-in-UI für Versammlungspersonal mit reduzierter Mitgliederliste

## Capabilities

### New Capabilities
- `assembly-session`: Admin-verwaltete, zeitlich begrenzte Sessions für Mitgliederversammlungen. Umfasst Erstellen, Aktivieren und Beenden von Sessions sowie Token-basierte Authentifizierung für Nicht-Nutzer.
- `assembly-checkin`: Check-in-Funktionalität innerhalb einer aktiven Assembly Session. Reduzierte Mitgliederliste (nur aktive Mitglieder), Abhaken von Anwesenheit und Status-Anzeige.

### Modified Capabilities
_(keine bestehenden Capabilities betroffen)_

## Impact

- **Neue DB-Tabellen**: `assembly_session` (Session-Verwaltung), `assembly_checkin` (Anwesenheitsprotokoll)
- **Neue DAO-Entities**: AssemblySessionEntity, AssemblyCheckinEntity
- **Neue Services**: AssemblySessionService, AssemblyCheckinService
- **Neue REST-Endpoints**: `/api/assembly-sessions/`, `/api/assembly-sessions/{id}/checkins/`
- **Auth-Erweiterung**: Token-basierte Authentifizierung für Assembly Sessions neben dem bestehenden OIDC/Session-System
- **Frontend**: Neue Seiten/Komponenten für Admin-Verwaltung und Check-in-UI
- **Bestehender Code**: Kein Breaking Change, rein additiv
