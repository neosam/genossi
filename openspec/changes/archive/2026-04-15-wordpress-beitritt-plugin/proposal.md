## Why

Die Genossenschaft betreibt eine WordPress-Website und möchte dort ein Beitrittsformular anbieten. Anstatt ein iFrame einzubetten, soll ein natives WordPress-Plugin das Formular rendern und serverseitig (PHP) die Genossi-API aufrufen. So fügt sich das Formular nahtlos ins bestehende Theme ein, Captcha-Plugins können genutzt werden, und die Genossi-API-URL bleibt nicht öffentlich exponiert.

## What Changes

- **WordPress-Plugin `genossi-beitritt`**: Neues PHP-Plugin mit Shortcode `[genossi_beitritt]`, das ein Beitrittsformular rendert.
- **Settings-Seite**: Admin-Bereich in WordPress zur Konfiguration von Genossi-API-URL und API-Key.
- **Serverseitiger API-Call**: Bei Formular-Submit macht der WordPress-Server einen cURL-POST an `POST /api/public/join` der Genossi-API (API-Key im Header, nie im Browser).
- **Formularfelder**: Vorname, Nachname, Anrede (optional), E-Mail, Straße, Hausnummer, PLZ, Ort, Anzahl Geschäftsanteile.
- **Validierung**: Client-seitige Pflichtfeld-Validierung + Server-seitige Validierung. Fehler der Genossi-API werden benutzerfreundlich angezeigt.
- **Erfolgsseite**: Nach erfolgreichem Absenden wird eine Bestätigungsnachricht angezeigt ("Vielen Dank, bitte überweisen Sie...").
- **Captcha-Kompatibilität**: Das Plugin soll mit gängigen WordPress-Captcha-Plugins kompatibel sein (z.B. reCAPTCHA via Form-Hooks).

## Capabilities

### New Capabilities
- `wp-beitritt-plugin`: WordPress-Plugin mit Shortcode, Settings-Seite, Formular-Rendering und serverseitigem Genossi-API-Call

### Modified Capabilities
<!-- Keine bestehenden Capabilities werden geändert -->

## Impact

- **Neues Verzeichnis**: `wordpress-plugin/genossi-beitritt/` im Repository
- **Technologie**: PHP (WordPress-Plugin-API), kein Rust
- **Abhängigkeit**: Setzt den `public-join-api`-Change voraus (Genossi-Endpunkt muss existieren)
- **Deployment**: Muss als ZIP oder direkt in WordPress `wp-content/plugins/` installiert werden
