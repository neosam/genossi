## Why

Die Genossenschaft möchte auf ihrer WordPress-Website die aktuelle Mitgliederanzahl anzeigen. Dafür wird ein öffentlicher (unauthentifizierter) API-Endpunkt benötigt, der ausschließlich die Anzahl aktiver Mitglieder zurückgibt. Aus Datenschutzgründen soll dieser Endpunkt standardmäßig deaktiviert sein und nur bei expliziter Konfiguration aktiviert werden.

## What Changes

- Neuer öffentlicher REST-Endpunkt `GET /api/public/member-count` ohne Authentifizierung
- Endpunkt gibt nur `{ "count": <n> }` zurück — keine weiteren Mitgliederdaten
- Feature ist standardmäßig deaktiviert; wird über Config-Key `public_stats_enabled` (bool) aktiviert
- Gibt 403 zurück wenn Config nicht gesetzt oder `false`
- Zählt nur aktive Mitglieder: kein `deleted` und kein `exit_date` in der Vergangenheit
- Caching mit 5 Minuten TTL für sowohl den Count als auch den Config-Wert
- Dedizierte `SELECT COUNT(*)` Query in der DAO-Schicht statt alle Mitglieder zu laden

## Capabilities

### New Capabilities
- `public-member-count`: Öffentlicher, gecachter Endpunkt zur Abfrage der aktiven Mitgliederanzahl, gesteuert über Config-Flag

### Modified Capabilities

## Impact

- **REST-Layer**: Neue Route außerhalb des Auth-Middleware-Stacks (nach den Auth-Layern registriert)
- **DAO-Layer**: Neue `count_active()` Methode im Member-DAO
- **Config-System**: Neuer Config-Key `public_stats_enabled`
- **Neues Modul**: Cache-Logik mit `tokio::sync::RwLock` und TTL — keine neue externe Dependency
- **API**: Neuer öffentlicher Endpunkt, keine Änderungen an bestehenden Endpunkten
