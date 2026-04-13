## Why

Die Backend-Endpoints für die Verwaltung von Beitrittserklärungen (`GET /api/applications`, `GET /api/applications/{id}`, `POST /api/applications/{id}/confirm`, `POST /api/applications/{id}/reject`) existieren bereits, aber es gibt kein Admin-UI dafür. Admins können eingegangene Beitrittserklärungen nicht einsehen, bestätigen oder ablehnen — das geht aktuell nur per Swagger UI.

## What Changes

- Neue Seite "Beitrittserklärungen" (`/applications`) im Frontend
- Listenansicht aller Beitrittserklärungen mit Status-Filter (Offen, Bestätigt, Abgelehnt)
- Detailansicht einer einzelnen Beitrittserklärung mit allen Daten (Name, Adresse, Anteile, Status, Datum)
- Bestätigungs- und Ablehnungs-Buttons für offene Anträge (mit Bestätigungsdialog)
- Navigation: neuer Link in der TopBar für Admins
- Frontend-API-Funktionen für die Application-Endpoints
- i18n-Keys für alle neuen UI-Texte (DE, EN)

## Capabilities

### New Capabilities
- `application-management-ui`: Admin-Seite zur Verwaltung von Beitrittserklärungen — Liste mit Status-Filter, Detailansicht, Bestätigen/Ablehnen

### Modified Capabilities

## Impact

- **Frontend**: Neue Seite `applications_page.rs`, neue Komponenten, neue Route `/applications`
- **Frontend API**: Neue Funktionen für `GET /api/applications`, `GET /api/applications/{id}`, `POST /api/applications/{id}/confirm`, `POST /api/applications/{id}/reject`
- **Router**: Neue Route + TopBar-Link
- **i18n**: Neue Keys in `mod.rs`, `de.rs`, `en.rs`
- **Backend**: Keine Änderungen — alle Endpoints existieren bereits
