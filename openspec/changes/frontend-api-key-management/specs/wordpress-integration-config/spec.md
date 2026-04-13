## ADDED Requirements

### Requirement: API-Key generieren und anzeigen
Die Config-Seite SHALL einen Button "API-Key generieren" bereitstellen, der `POST /api/config/generate-api-key` aufruft. Nach erfolgreicher Generierung SHALL der Key in einem Textfeld angezeigt werden mit einem Copy-to-Clipboard-Button. Der Key SHALL nach einem Seiten-Reload als `***` angezeigt werden (da Typ `secret`). Es SHALL ein Hinweis erscheinen, dass der Key jetzt kopiert werden muss.

#### Scenario: API-Key erstmalig generieren
- **WHEN** kein `public_api_key` konfiguriert ist und der Admin auf "API-Key generieren" klickt
- **THEN** wird ein neuer UUID-Key generiert, gespeichert und im Klartext angezeigt mit Copy-Button

#### Scenario: API-Key regenerieren
- **WHEN** bereits ein `public_api_key` existiert und der Admin auf "API-Key neu generieren" klickt
- **THEN** wird ein neuer Key generiert, der alte überschrieben, und der neue Key im Klartext angezeigt

#### Scenario: Key nach Reload maskiert
- **WHEN** die Config-Seite nach Key-Generierung neu geladen wird
- **THEN** wird der API-Key als `***` angezeigt und der Status zeigt "API-Key konfiguriert"

### Requirement: WordPress-relevante Config-Einträge bearbeiten
Die Config-Seite SHALL Formularfelder für folgende Config-Einträge bereitstellen:
- `share_value_cents` (Anteilswert in Cent, Typ int, Pflicht)
- `bank_iban` (IBAN, Typ string, Pflicht)
- `bank_name` (Bankname, Typ string, Pflicht)
- `bank_bic` (BIC, Typ string, optional)
- `genossenschaft_name` (Name der Genossenschaft, Typ string, Pflicht)

Alle Felder SHALL per "Speichern"-Button über `api::set_config_entry()` gespeichert werden.

#### Scenario: Config-Einträge speichern
- **WHEN** der Admin Bankdaten und Anteilswert eingibt und "Speichern" klickt
- **THEN** werden alle Felder als Config-Einträge mit korrektem Typ gespeichert und eine Erfolgsmeldung angezeigt

#### Scenario: Bestehende Werte laden
- **WHEN** die Config-Seite geladen wird und Config-Einträge existieren
- **THEN** werden die Formularfelder mit den bestehenden Werten vorausgefüllt

### Requirement: Vollständigkeits-Statusanzeige
Die Sektion SHALL für jeden Pflicht-Config-Eintrag einen Status anzeigen (konfiguriert / nicht konfiguriert). Die Pflichtfelder sind: `public_api_key`, `share_value_cents`, `bank_iban`, `bank_name`, `genossenschaft_name`.

#### Scenario: Alle Pflichtfelder konfiguriert
- **WHEN** alle Pflichtfelder gesetzt sind
- **THEN** zeigt die Statusanzeige an, dass die WordPress-Integration vollständig konfiguriert ist

#### Scenario: Pflichtfelder fehlen
- **WHEN** ein oder mehrere Pflichtfelder nicht gesetzt sind
- **THEN** zeigt die Statusanzeige an, welche Felder noch fehlen

### Requirement: Einrichtungsanleitung mit URLs
Die Sektion SHALL eine Infobox mit Einrichtungsschritten anzeigen:
1. API-Key generieren
2. Im WordPress-Plugin (Settings > Genossi Beitritt):
   - API-URL: dynamisch vorausgefüllt aus Backend-URL (z.B. `https://genossi.example.com`)
   - API-Key: der generierte Key
3. Shortcode `[genossi_beitritt]` auf einer WordPress-Seite einbinden

Die API-URL SHALL als kopierbares Textfeld angezeigt werden.

#### Scenario: API-URL anzeigen
- **WHEN** die Config-Seite geladen wird
- **THEN** wird die API-URL basierend auf der aktuellen Backend-Konfiguration angezeigt und ist kopierbar

#### Scenario: Vollständige Anleitung sichtbar
- **WHEN** der Admin die WordPress-Integration-Sektion öffnet
- **THEN** sind alle drei Einrichtungsschritte mit konkreten Werten sichtbar

### Requirement: Frontend-API-Funktion für Key-Generierung
Das Frontend SHALL eine neue API-Funktion `generate_api_key()` in `api.rs` bereitstellen, die `POST /api/config/generate-api-key` aufruft und den generierten Key als String zurückgibt.

#### Scenario: API-Funktion aufrufen
- **WHEN** die Funktion `generate_api_key()` aufgerufen wird
- **THEN** wird ein POST-Request an `/api/config/generate-api-key` gesendet und der Key aus der Response extrahiert

### Requirement: i18n-Unterstützung
Alle UI-Texte der WordPress-Integration-Sektion SHALL über das i18n-System lokalisiert sein (DE, EN, CS).

#### Scenario: Deutsche Übersetzung
- **WHEN** die Sprache auf Deutsch eingestellt ist
- **THEN** werden alle Labels und Texte auf Deutsch angezeigt

#### Scenario: Englische Übersetzung
- **WHEN** die Sprache auf Englisch eingestellt ist
- **THEN** werden alle Labels und Texte auf Englisch angezeigt
