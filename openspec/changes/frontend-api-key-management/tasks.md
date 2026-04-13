## 1. Frontend-API

- [x] 1.1 Neue Funktion `generate_api_key()` in `genossi-frontend/src/api.rs` implementieren: `POST /api/config/generate-api-key`, gibt den Key als String zurück

## 2. i18n-Keys

- [x] 2.1 Neue i18n-Keys in `genossi-frontend/src/i18n/mod.rs` definieren (WordPressIntegration, WordPressIntegrationDesc, GenerateApiKey, RegenerateApiKey, ApiKeyGenerated, ApiKeyCopyHint, ApiKeyConfigured, ShareValueCents, BankIban, BankName, BankBic, GenossenschaftName, SetupInstructions, ApiUrl, ConfigComplete, ConfigIncomplete, MissingFields, WordPressShortcodeHint, CopyToClipboard, Copied)
- [x] 2.2 Deutsche Übersetzungen in `de.rs` hinzufügen
- [x] 2.3 Englische Übersetzungen in `en.rs` hinzufügen
- [x] 2.4 Tschechische Übersetzungen in `cs.rs` hinzufügen (entfällt: CS-Locale ist im Genossi3-Frontend nicht aktiv)

## 3. WordPress-Integration-Komponente

- [x] 3.1 Neue Komponente `WordPressIntegrationSection` in `genossi-frontend/src/component/` erstellen mit: API-Key-Generierung (Button + Anzeige + Copy), Formularfelder für Bankdaten/Anteilswert/Genossenschaftsname, Vollständigkeits-Statusanzeige, Einrichtungsanleitung mit API-URL und WordPress-Schritten
- [x] 3.2 Komponente in `genossi-frontend/src/component/mod.rs` exportieren

## 4. Config-Seite Integration

- [x] 4.1 WordPress-Integration-Sektion in `config_page.rs` einbinden: Signals für neue Felder anlegen, in `reload()` aus Config-Entries befüllen, `WordPressIntegrationSection`-Komponente zwischen WebDAV-Backup und Advanced-Bereich einfügen

## 5. Tests

- [x] 5.1 E2E-Test: API-Key generieren über UI-Flow (generate-api-key Endpoint aufrufen, prüfen dass Key zurückkommt)
- [x] 5.2 E2E-Test: Config-Einträge für WordPress-Integration speichern und laden (share_value_cents, bank_iban, bank_name, bank_bic, genossenschaft_name)
