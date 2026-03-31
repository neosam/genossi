## Why

Das Backend unterstützt bereits vollständige Member-Actions (Eintritt, Austritt, Todesfall, Aufstockung, Verkauf, Übertragung) mit CRUD-Endpoints und Migrations-Validierung. Das Frontend zeigt aber bisher nur Stammdaten an — Aktionen können weder angezeigt noch erfasst werden. Für die Migration von ~600 Mitgliedern aus dem Excel muss das Frontend die Aktions-Historie darstellen und das Nachtragen von Aktionen ermöglichen.

## What Changes

- `MemberActionTO` und `ActionTypeTO` im Frontend rest-types hinzufügen
- API-Funktionen für Member-Actions CRUD und Migration-Status
- Aktionen-Sektion auf der Member-Detail-Seite: Liste der bestehenden Aktionen und Formular zum Erstellen/Bearbeiten
- Migrations-Status-Anzeige pro Mitglied (migriert/ausstehend mit Soll-Ist-Vergleich)
- i18n-Keys für alle Aktionstypen und UI-Elemente (DE, EN)

## Capabilities

### New Capabilities
- `frontend-member-actions`: Anzeige, Erfassung und Validierung von Member-Actions im Dioxus-Frontend

### Modified Capabilities
- `member-management`: Member-Detail-Seite wird um Aktionen-Sektion erweitert

## Impact

- **Frontend rest-types**: Neue Transfer-Objekte `MemberActionTO`, `ActionTypeTO`, `MigrationStatusTO`
- **Frontend API**: Neue Funktionen für Actions-Endpoints und Migration-Status
- **Frontend Pages**: `member_details.rs` wird um Aktionen-Bereich erweitert
- **Frontend i18n**: Neue Übersetzungsschlüssel für Aktionstypen und UI
- **Backend**: Keine Änderungen nötig — alle Endpoints existieren bereits
