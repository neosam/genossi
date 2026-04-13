## Why

Die Genossenschaft hat bereits einen öffentlichen API-Endpunkt (`GET /api/public/member-count`), der die Anzahl aktiver Mitglieder zurückgibt. Auf der WordPress-Website soll diese Zahl angezeigt werden können, z.B. auf der Startseite oder einer Info-Seite. Ein natives WordPress-Plugin mit Shortcode ist die einfachste Lösung, die sich nahtlos ins bestehende Theme einfügt.

## What Changes

- **WordPress-Plugin `genossi-member-count`**: Neues PHP-Plugin mit Shortcode `[genossi_member_count]`, das die Mitgliederzahl aus der Genossi-API abruft und anzeigt.
- **Settings-Seite**: Admin-Bereich zur Konfiguration der API-URL und Cache-Dauer. Wenn das `genossi-beitritt`-Plugin installiert ist, wird die API-URL automatisch von dort übernommen.
- **WordPress Transient Caching**: Gecachter API-Aufruf mit konfigurierbarem TTL (Standard: 15 Minuten), um unnötige HTTP-Requests zu vermeiden.
- **Kein API-Key nötig**: Der Endpunkt ist öffentlich und erfordert keine Authentifizierung.

## Capabilities

### New Capabilities
- `wp-member-count-plugin`: WordPress-Plugin mit Shortcode, Settings-Seite mit Fallback auf genossi-beitritt URL, und Transient-basiertem Caching

### Modified Capabilities
<!-- Keine bestehenden Capabilities werden geändert -->

## Impact

- **Neues Verzeichnis**: `wordpress-plugin/genossi-member-count/` im Repository
- **Technologie**: PHP (WordPress-Plugin-API), kein Rust
- **Abhängigkeit**: Setzt voraus, dass `public_stats_enabled` in der Genossi-Config auf `true` steht
- **Optionale Abhängigkeit**: Kann API-URL vom `genossi-beitritt`-Plugin übernehmen (Option `genossi_api_url`)
- **Deployment**: Installation als ZIP oder direkt in WordPress `wp-content/plugins/`
