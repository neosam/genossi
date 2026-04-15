## Context

Die Public-Join-API ist serverseitig vollständig implementiert (`POST /api/config/generate-api-key`, `POST /api/public/join`). Admins müssen aktuell per Swagger UI oder curl den API-Key generieren und Config-Einträge setzen. Das WordPress-Plugin "Genossi Beitritt" erwartet in seinen Einstellungen (Settings > Genossi Beitritt) eine API-URL und einen API-Key.

Die Config-Seite (`config_page.rs`, ~1210 Zeilen) hat bereits dedizierte Sektionen für SMTP, IMAP, Mail-Footer und WebDAV-Backup. Jede Sektion folgt dem gleichen Pattern: Signals für Formularzustand, `reload()`-Funktion befüllt Signals aus Config-Entries, Speichern per `api::set_config_entry()`.

## Goals / Non-Goals

**Goals:**
- Dedizierte "WordPress-Integration"-Sektion auf der Config-Seite
- API-Key generieren und anzeigen (mit Copy-to-Clipboard)
- Alle WordPress-relevanten Config-Einträge bearbeitbar (Bankdaten, Anteilswert, Genossenschaftsname)
- Einrichtungsanleitung mit den URLs, die im WordPress-Plugin eingetragen werden müssen
- Statusanzeige: welche Config-Einträge fehlen noch

**Non-Goals:**
- Verwaltung der Beitrittserklärungen (eigener Change)
- Automatische WordPress-Plugin-Konfiguration
- Test-Endpoint zum Prüfen der Verbindung

## Decisions

### 1. Neue Sektion in bestehender Config-Seite statt eigener Seite

**Entscheidung:** Die WordPress-Integration wird als neue Sektion in `config_page.rs` eingefügt, zwischen WebDAV-Backup und dem Advanced-Bereich.

**Alternativen:**
- Eigene Route/Seite: Overhead für 6 Config-Einträge nicht gerechtfertigt
- Tab innerhalb der Config-Seite: Kein Tab-System vorhanden, einzuführen wäre too much

### 2. Neue API-Funktion für generate-api-key

**Entscheidung:** Neue Funktion `api::generate_api_key()` in `api.rs`, die `POST /api/config/generate-api-key` aufruft und den Key zurückgibt. Der Key wird temporär im State angezeigt, damit der Admin ihn kopieren kann. Da der Key als `secret` gespeichert wird, zeigt ein erneuter Reload nur `***`.

**Alternativen:**
- Key per `set_config_entry` manuell setzen: Unsicher, weil der Admin dann selbst einen UUID generieren müsste

### 3. Einrichtungsanleitung als statische Infobox

**Entscheidung:** Eine Infobox zeigt dem Admin die konkreten Schritte:
1. API-Key generieren (Button)
2. Im WordPress-Plugin unter "Settings > Genossi Beitritt":
   - API-URL eintragen: `<BASE_PATH>` (aus der aktuellen Config, z.B. `https://genossi.example.com`)
   - API-Key eintragen (der gerade generierte Key)
3. Shortcode `[genossi_beitritt]` auf einer Seite einbinden

Die API-URL wird dynamisch aus der `BASE_PATH`-Umgebungsvariable bzw. dem Config-Wert abgeleitet, sodass sie nicht geraten werden muss.

### 4. Vollständigkeits-Check

**Entscheidung:** Die Sektion zeigt einen einfachen Status pro Config-Eintrag (gesetzt / nicht gesetzt), damit der Admin sofort sieht, ob die Konfiguration vollständig ist. Pflichtfelder: `public_api_key`, `share_value_cents`, `bank_iban`, `bank_name`, `genossenschaft_name`. Optional: `bank_bic`.

### 5. Component-Architektur

**Entscheidung:** Die WordPress-Integration-Sektion wird als eigene Dioxus-Komponente `WordPressIntegrationSection` in `src/component/` implementiert, nicht direkt inline in `config_page.rs`. Dies folgt dem Component-First-Prinzip aus CLAUDE.md und hält die ohnehin lange Config-Seite übersichtlich. Die Komponente erhält die benötigten Signals als Props.

## Risks / Trade-offs

- **config_page.rs ist bereits groß (~1210 Zeilen)** → Mitigation: Neue Sektion als eigene Komponente auslagern, Config-Page ruft nur die Komponente auf
- **API-Key wird nur einmal sichtbar angezeigt** → Mitigation: Klare Hinweisbox "Jetzt kopieren, wird danach nur als *** angezeigt". Re-Generate ist jederzeit möglich.
- **BASE_PATH nicht immer korrekt** → Mitigation: URL wird als editierbares Feld angezeigt, vorausgefüllt mit aktuellem Wert
