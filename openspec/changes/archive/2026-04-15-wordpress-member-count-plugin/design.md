## Context

Die Genossenschaft betreibt eine WordPress-Website und nutzt bereits das `genossi-beitritt`-Plugin für Beitrittserklärungen. Die Genossi-API bietet unter `GET /api/public/member-count` einen öffentlichen Endpunkt, der die Anzahl aktiver Mitglieder als JSON zurückgibt. Dieses Plugin soll diese Zahl per Shortcode auf der WordPress-Seite anzeigbar machen.

Das bestehende `genossi-beitritt`-Plugin speichert die API-URL als WordPress-Option `genossi_api_url`. Das neue Plugin soll diese Option mitnutzen, wenn vorhanden.

## Goals / Non-Goals

**Goals:**
- Shortcode `[genossi_member_count]` zur Anzeige der Mitgliederzahl
- WordPress Transient Caching zur Vermeidung unnötiger API-Requests
- Automatische Übernahme der API-URL vom `genossi-beitritt`-Plugin, mit Fallback auf eigene Einstellung
- Einfache Settings-Seite für standalone-Betrieb und Cache-Konfiguration

**Non-Goals:**
- Styling/Design des angezeigten Werts (wird als einfaches `<span>` ausgegeben, Theme übernimmt Styling)
- Authentifizierung (der Endpunkt ist öffentlich)
- Widget oder Gutenberg-Block (Shortcode reicht)

## Decisions

### Plugin-Struktur: Flach wie genossi-beitritt
Die gleiche Struktur wie beim bestehenden `genossi-beitritt`-Plugin: Hauptdatei + `includes/`-Verzeichnis mit Klassen. Das hält die Plugins konsistent.

**Dateien:**
```
wordpress-plugin/genossi-member-count/
├── genossi-member-count.php          # Hauptdatei, Shortcode-Registrierung
├── includes/
│   └── class-settings.php            # Settings-Seite und Option-Handling
└── readme.txt                        # WordPress-Plugin-Beschreibung
```

### URL-Auflösung: Option-Fallback-Kette
Die API-URL wird über folgende Kette aufgelöst:
1. `get_option('genossi_api_url')` — vom Beitritt-Plugin geschrieben
2. `get_option('genossi_mc_api_url')` — eigene Option als Fallback

Die Settings-Seite zeigt den aktuellen Zustand: Wenn die URL vom Beitritt-Plugin kommt, wird das angezeigt und das Eingabefeld deaktiviert. Wenn nicht, wird ein normales Eingabefeld gezeigt.

**Alternative erwogen:** Gemeinsames Basis-Plugin für geteilte Einstellungen. Verworfen — zu viel Overhead für eine einzelne Option.

### Caching: WordPress Transients mit konfigurierbarem TTL
- Transient-Key: `genossi_member_count`
- Standard-TTL: 900 Sekunden (15 Minuten)
- Konfigurierbar über Settings-Seite
- Zusammen mit dem serverseitigen Cache (5 Min) ergibt sich ein maximales Verzögerung von ~20 Minuten — akzeptabel für eine Mitgliederzahl

**Alternative erwogen:** Kein Caching, bei jedem Seitenaufruf API anfragen. Verworfen — unnötige Last, besonders bei stark besuchten Seiten.

### Shortcode-Output: Einfaches `<span>`
```html
<span class="genossi-member-count">42</span>
```

Kein zusätzliches Styling, kein Wrapper-`div`. So kann es überall inline eingesetzt werden: "Unsere Genossenschaft hat aktuell `[genossi_member_count]` Mitglieder."

Bei Fehlern (API nicht erreichbar, Feature deaktiviert) wird nichts angezeigt — kein Fehler auf der öffentlichen Seite. Admins sehen einen Hinweis.

### API-Aufruf: `wp_remote_get`
WordPress-native HTTP-Funktion. Kein cURL direkt, keine externe Library. Der Endpunkt wird als `{api_url}/api/public/member-count` aufgebaut.

## Risks / Trade-offs

- **[Doppeltes Caching]** → Akzeptiert. WP-Cache (15 Min) + API-Cache (5 Min) = max ~20 Min Verzögerung. Für eine Mitgliederzahl ist das unproblematisch.
- **[Beitritt-Plugin ändert Option-Name]** → Unwahrscheinlich, aber der Fallback auf eigene Option fängt das ab. Geringe Kopplung.
- **[API nicht erreichbar]** → Shortcode gibt leeren String zurück. Kein Fehler für Besucher sichtbar. Cached Wert bleibt bis zum Ablauf gültig.
- **[public_stats_enabled = false]** → API gibt 403 zurück, Shortcode zeigt nichts an. Admin sieht Konfigurationshinweis.
