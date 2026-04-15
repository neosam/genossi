## 1. Plugin-Grundgerüst

- [x] 1.1 Verzeichnis `wordpress-plugin/genossi-member-count/` und `includes/` anlegen
- [x] 1.2 Hauptdatei `genossi-member-count.php` mit Plugin-Header, Konstanten und Includes erstellen
- [x] 1.3 `readme.txt` mit Plugin-Beschreibung erstellen

## 2. Settings-Seite

- [x] 2.1 `includes/class-settings.php` erstellen mit Settings-Registrierung, Menü-Eintrag und Render-Funktionen
- [x] 2.2 URL-Fallback-Logik implementieren: `genossi_api_url` prüfen, bei leer auf `genossi_mc_api_url` zurückfallen
- [x] 2.3 Settings-Seite zeigt URL als read-only wenn von Beitritt-Plugin übernommen, sonst editierbares Feld
- [x] 2.4 Cache-TTL-Einstellung als numerisches Feld mit Standard 900 Sekunden

## 3. API-Aufruf und Caching

- [x] 3.1 Funktion zum Abrufen der Mitgliederzahl: Transient prüfen, bei Cache-Miss `wp_remote_get` an `{url}/api/public/member-count`, JSON parsen, Transient setzen
- [x] 3.2 Fehlerbehandlung: Bei API-Fehler (nicht-200, Netzwerkfehler) leeren Wert zurückgeben, nichts cachen

## 4. Shortcode

- [x] 4.1 Shortcode `[genossi_member_count]` registrieren
- [x] 4.2 Shortcode-Handler: URL auflösen, Mitgliederzahl holen, als `<span class="genossi-member-count">{count}</span>` ausgeben
- [x] 4.3 Admin-Hinweis bei fehlender Konfiguration (nur für Nutzer mit `manage_options`)
- [x] 4.4 Leerer String bei Fehlern für nicht-Admin-Besucher
