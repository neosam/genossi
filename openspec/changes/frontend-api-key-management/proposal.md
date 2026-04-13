## Why

Die WordPress-Integration für Beitrittserklärungen erfordert einen API-Key und mehrere Config-Einträge (Bankdaten, Anteilswert, Genossenschaftsname), die aktuell nur per Swagger UI oder curl gesetzt werden können. Admins brauchen ein übersichtliches UI auf der Config-Seite, das alle benötigten Einstellungen für die WordPress-Anbindung zusammenfasst — inklusive API-Key-Generierung, der einzutragenden URLs und einer Anleitung.

## What Changes

- Neue Sektion "WordPress-Integration" auf der bestehenden Config-Seite (`config_page.rs`)
- Button zum Generieren/Regenerieren des API-Keys mit Anzeige und Copy-Button
- Formularfelder für die WordPress-relevanten Config-Einträge:
  - `share_value_cents` (Anteilswert in Cent)
  - `bank_iban` (IBAN)
  - `bank_name` (Bankname)
  - `bank_bic` (BIC, optional)
  - `genossenschaft_name` (Name der Genossenschaft)
- Infobox mit den URLs und Einstellungen, die im WordPress-Plugin eingetragen werden müssen:
  - API-URL (z.B. `https://genossi.example.com`)
  - API-Key (der generierte Key)
  - Hinweis auf das WordPress-Plugin "Genossi Beitritt" und dessen Settings-Seite
- Frontend-API-Funktion für `POST /api/config/generate-api-key`
- i18n-Keys für alle neuen UI-Texte (DE, EN, CS)

## Capabilities

### New Capabilities
- `wordpress-integration-config`: Admin-UI-Sektion auf der Config-Seite zur Verwaltung aller Einstellungen für die WordPress-Beitritts-Anbindung (API-Key, Bankdaten, Anteilswert, Genossenschaftsname, Einrichtungsanleitung)

### Modified Capabilities

## Impact

- **Frontend**: `genossi-frontend/src/page/config_page.rs` bekommt eine neue Sektion
- **Frontend API**: `genossi-frontend/src/api.rs` braucht eine neue Funktion für `generate-api-key`
- **i18n**: Neue Keys in `genossi-frontend/src/i18n/mod.rs`, `en.rs`, `de.rs`, `cs.rs`
- **Backend**: Keine Änderungen — der `generate-api-key`-Endpoint existiert bereits
