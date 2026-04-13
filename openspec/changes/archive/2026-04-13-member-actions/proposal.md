## Why

Die Genossenschaft verwaltet aktuell Mitgliederdaten per Excel. In der Excel-Liste gibt es nur Snapshot-Werte (Anteile bei Beitritt, aktuelle Anteile, Anzahl Aktionen), aber keine vollständige Historie der Anteils-Veränderungen. Genossi soll diese Historie als erste Quelle der Wahrheit abbilden. Gleichzeitig muss der Migrationspfad von Excel zu Genossi unterstützt werden: ~600 Mitglieder müssen überführt und validiert werden.

## What Changes

- Neues Entity **MemberAction** mit Aktionstypen für Status-Ereignisse (Eintritt, Austritt, Todesfall) und Anteils-Veränderungen (Aufstockung, Verkauf, Übertragung Empfang, Übertragung Abgabe)
- Vollständiger CRUD-Lifecycle für MemberActions (DAO, Service, REST, Frontend)
- Neues Feld `action_count` am Member (aus Excel-Import, Spalte "Anzahl Aktionen")
- Automatische Erzeugung der Eintritts-Aktion beim Import für Mitglieder mit `action_count == 0` und `shares_at_joining == current_shares`
- Migrations-Validierungslogik: Vergleich von Σ Anteils-Aktionen gegen `current_shares` und Anzahl Aktionen gegen `action_count`

## Capabilities

### New Capabilities
- `member-actions`: Aktionen auf Mitgliedern (Eintritt, Austritt, Todesfall, Aufstockung, Verkauf, Übertragung) mit vollständiger Historie und Migrations-Validierung

### Modified Capabilities
- `member-management`: Neues Feld `action_count` am Member-Entity; Import liest "Anzahl Aktionen" aus Excel und erzeugt automatisch Eintritts-Aktionen für auto-migrierbare Mitglieder

## Impact

- **Datenbank**: Neue Tabelle `member_actions`, neue Spalte `action_count` auf `members`
- **DAO-Layer**: Neues `MemberActionDao` Trait + SQLite-Implementierung
- **Service-Layer**: Neuer `MemberActionService`, Validierungslogik für Migration
- **REST-Layer**: Neue Endpoints unter `/api/members/{id}/actions`
- **REST-Types**: Neues `MemberActionTO`
- **Frontend**: Aktionen-Anzeige und -Erfassung auf der Member-Detail-Seite
- **Excel-Import**: Erweiterung um `action_count`-Feld und automatische Eintritts-Aktion-Erzeugung
