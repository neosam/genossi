## Context

Die Genossenschaft betreibt eine WordPress-Website. Mit dem `public-join-api`-Change stellt Genossi einen öffentlichen Endpunkt `POST /api/public/join` bereit, der durch einen API-Key geschützt ist. Dieses WordPress-Plugin bildet die Brücke zwischen dem Website-Besucher und dieser API.

Der WordPress-Server übernimmt die Rolle des Vermittlers: Das Formular wird als native WordPress-Seite gerendert, der API-Call geschieht serverseitig (PHP cURL), sodass weder die Genossi-URL noch der API-Key dem Browser exponiert werden.

## Goals / Non-Goals

**Goals:**
- Natives WordPress-Plugin mit Shortcode `[genossi_beitritt]`
- Settings-Seite im WP-Admin für API-URL und API-Key
- Serverseitiger API-Call (PHP → Genossi), kein Browser-zu-Genossi-Call
- Pflichtfeld-Validierung client- und serverseitig
- Benutzerfreundliche Fehleranzeige bei API-Fehlern
- Kompatibilität mit gängigen Captcha-Plugins

**Non-Goals:**
- Eigenes Captcha implementieren (delegiert an WP-Ökosystem)
- Styling über Basis-CSS hinaus (WordPress-Theme übernimmt Großteil)
- Multi-Language/i18n (erstmal nur Deutsch)
- Gutenberg-Block (Shortcode reicht)
- Automatisches Update über WordPress Plugin-Directory

## Decisions

### 1. Shortcode statt Gutenberg-Block

**Entscheidung:** Das Plugin registriert einen Shortcode `[genossi_beitritt]`, keinen Gutenberg-Block.

**Alternativen:**
- *Gutenberg-Block*: Modernerer Ansatz, aber deutlich mehr Code (JSX, Build-Pipeline) für ein einfaches Formular
- *Shortcode*: Funktioniert in Classic Editor und Gutenberg, kein Build-Step nötig, bewährtes Pattern

**Begründung:** Ein Beitrittsformular ist ein einfaches, statisches Formular. Der Aufwand für einen Gutenberg-Block steht in keinem Verhältnis zum Nutzen.

### 2. Serverseitiger API-Call via wp_remote_post

**Entscheidung:** Das Plugin nutzt `wp_remote_post()` (WordPress HTTP API) für den API-Call, nicht direkten cURL.

**Alternativen:**
- *cURL direkt*: Funktioniert, aber umgeht WordPress-Abstraktion
- *wp_remote_post()*: WordPress-native Funktion, respektiert Proxy-Settings, SSL-Konfiguration, Timeouts

**Begründung:** `wp_remote_post()` ist der WordPress-Standard und wird von Hosting-Providern erwartet.

### 3. Plugin-Dateistruktur

```
wordpress-plugin/genossi-beitritt/
├── genossi-beitritt.php          # Plugin-Header, Hooks, Shortcode
├── includes/
│   ├── class-settings.php        # WP-Admin Settings-Seite
│   ├── class-form-handler.php    # POST-Verarbeitung, API-Call
│   └── class-form-renderer.php   # HTML-Formular rendern
├── assets/
│   └── style.css                 # Minimales Formular-CSS
└── readme.txt                    # WordPress-Plugin-Readme
```

**Begründung:** Standardmäßiges WordPress-Plugin-Layout. Klassen-basiert für Testbarkeit und Übersichtlichkeit.

### 4. Formular-Verarbeitung via POST an dieselbe Seite

**Entscheidung:** Das Formular postet an die aktuelle WordPress-Seite (`action=""`). Das Plugin fängt den POST in einem `init`-Hook ab, ruft die API auf, und rendert dann Erfolg oder Fehler.

**Alternativen:**
- *AJAX/Fetch*: Bessere UX, aber komplexer (JS nötig, wp_ajax-Hooks, Nonce-Handling)
- *Standard POST*: Einfach, funktioniert ohne JavaScript, Captcha-Plugins arbeiten besser damit

**Begründung:** Ein Beitrittsformular wird selten genutzt. Die Einfachheit eines Standard-POSTs überwiegt die UX-Vorteile von AJAX.

### 5. Nonce-Schutz gegen CSRF

**Entscheidung:** Das Formular enthält ein WordPress-Nonce-Feld (`wp_nonce_field`), das bei Verarbeitung geprüft wird.

**Begründung:** Standard-WordPress-Sicherheitsmuster gegen Cross-Site Request Forgery.

## Risks / Trade-offs

- **[PHP-Version]** → Plugin setzt PHP 7.4+ voraus (WordPress-Mindestanforderung seit WP 5.2). Kein Risiko bei aktuellem WordPress.
- **[API nicht erreichbar]** → Wenn der Genossi-Server nicht erreichbar ist, zeigt das Plugin eine benutzerfreundliche Fehlermeldung. Der Besucher kann es später erneut versuchen.
- **[Captcha-Inkompatibilität]** → Nicht jedes Captcha-Plugin unterstützt Shortcode-Formulare. Empfehlung: Dokumentation welche Captcha-Plugins getestet wurden.
- **[Settings nicht konfiguriert]** → Wenn API-URL oder API-Key fehlen, zeigt das Formular eine Admin-Hinweismeldung statt des Formulars.
