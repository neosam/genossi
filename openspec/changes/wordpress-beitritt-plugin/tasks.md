## 1. Plugin-Grundstruktur

- [x] 1.1 Erstelle Verzeichnisstruktur `wordpress-plugin/genossi-beitritt/` mit Unterverzeichnissen `includes/` und `assets/`
- [x] 1.2 Erstelle `genossi-beitritt.php` mit Plugin-Header, Aktivierungs-Hooks und Autoloading der Include-Dateien

## 2. Settings-Seite

- [x] 2.1 Erstelle `includes/class-settings.php` mit WP Settings API: Registriere Settings-Seite unter "Settings > Genossi Beitritt"
- [x] 2.2 Implementiere Eingabefelder für `genossi_api_url` und `genossi_api_key` mit Validierung (Pflichtfelder)
- [x] 2.3 Registriere Settings-Menü in `admin_menu`-Hook und Settings in `admin_init`-Hook

## 3. Formular-Rendering

- [x] 3.1 Erstelle `includes/class-form-renderer.php` mit Methode zum Rendern des HTML-Formulars
- [x] 3.2 Implementiere alle Formularfelder (Anrede-Select, Textfelder, E-Mail, Anteile-Number, Checkboxen) mit HTML5-Validierung
- [x] 3.3 Registriere Shortcode `[genossi_beitritt]` in der Plugin-Hauptdatei
- [x] 3.4 Implementiere Logik: Wenn Settings nicht konfiguriert, zeige Admin-Hinweis / nichts für Besucher
- [x] 3.5 Implementiere Wiederherstellung der Eingabewerte bei Fehlern (Pre-Fill)

## 4. Formular-Verarbeitung

- [x] 4.1 Erstelle `includes/class-form-handler.php` mit POST-Verarbeitungslogik
- [x] 4.2 Implementiere Nonce-Prüfung (`wp_verify_nonce`)
- [x] 4.3 Implementiere serverseitige Pflichtfeld-Validierung
- [x] 4.4 Implementiere `wp_remote_post()`-Call an Genossi API mit JSON-Body und `X-Api-Key`-Header
- [x] 4.5 Implementiere Fehlerbehandlung: API 422 (Validierungsfehler anzeigen), 401 (generische Meldung + Logging), Timeout (generische Meldung + Logging)
- [x] 4.6 Implementiere Erfolgsanzeige nach erfolgreichem Submit

## 5. Styling

- [x] 5.1 Erstelle `assets/style.css` mit minimalem Formular-Layout (Labels, Inputs, Buttons, Fehlermeldungen)
- [x] 5.2 Enqueue CSS nur auf Seiten mit dem Shortcode (`wp_enqueue_scripts` mit Shortcode-Detection)

## 6. Dokumentation & Packaging

- [x] 6.1 Erstelle `readme.txt` mit Plugin-Beschreibung, Installationsanleitung und Konfigurationshinweisen
- [x] 6.2 Dokumentiere empfohlene Captcha-Plugins und deren Integration
