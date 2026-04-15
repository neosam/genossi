## Why

Seit dem 01.01.2025 erlaubt § 15 GenG (geändert durch BEG IV) Beitrittserklärungen in Textform statt Schriftform. Damit können Genossenschaften erstmals rechtssicher digitale Beitrittsformulare anbieten. Genossi soll einen öffentlichen API-Endpunkt bereitstellen, über den Beitrittserklärungen eingereicht werden können – gedacht für die Anbindung an die WordPress-Seite der Genossenschaft via serverseitigem PHP-Call.

## What Changes

- **Neue `Application`-Entität**: Separate Tabelle für Beitrittserklärungen, unabhängig von der Mitgliederliste. Speichert Antragsdaten (Name, Adresse, E-Mail, gewünschte Anteile) mit Status-Tracking (Offen, Bestätigt, Abgelehnt).
- **Öffentlicher API-Endpunkt** `POST /api/public/join`: Nimmt Beitrittserklärungen entgegen, gesichert durch API-Key (kein User-Login). Validiert Pflichtfelder und legt Application an.
- **Bestätigungs-Mail**: Nach Eingang einer Beitrittserklärung wird automatisch eine E-Mail an den Antragsteller gesendet mit Überweisungsdaten (IBAN, Betrag basierend auf Anteile × Anteilswert).
- **Bestätigungs-/Ablehnungs-Workflow**: Admin-Endpunkte zum Bestätigen (legt vollwertiges Mitglied an mit Mitgliedsnummer, Eintritt- und Aufstockung-Aktionen) und Ablehnen von Anträgen.
- **Config-Store-Einträge**: Neue Konfigurationswerte für API-Key (auto-generierbar), Anteilswert, Bankdaten und Genossenschaftsname.
- Mitgliedsnummer wird erst bei Bestätigung vergeben, nicht bei Antragseingang.

## Capabilities

### New Capabilities
- `membership-application`: Verwaltung von Beitrittserklärungen als eigene Entität mit öffentlichem Einreichungs-Endpunkt, Bestätigungs-Mail, und Admin-Workflow zur Bestätigung/Ablehnung.

### Modified Capabilities
<!-- Keine bestehenden Specs werden auf Requirements-Ebene geändert. Der Config-Store wird genutzt (neue Keys), aber sein Verhalten ändert sich nicht. Die Mitglieder-Erstellung wird intern aufgerufen, aber das Member-Management-Spec bleibt unverändert. -->

## Impact

- **Neue Datenbank-Tabelle**: `applications` mit Migration
- **Neue DAO/Service/REST-Schichten**: Für Application-Entität, analog zu bestehenden Entitäten
- **Neuer öffentlicher Endpunkt**: `POST /api/public/join` – erster Endpunkt ohne User-Authentifizierung (API-Key stattdessen)
- **Mail-System**: Nutzt bestehende Mail-Infrastruktur für Bestätigungs-Mails an Nicht-Mitglieder (neues Pattern)
- **Config-Store**: Neue Einträge (`public_api_key`, `share_value_cents`, `bank_iban`, `bank_name`, `bank_bic`, `genossenschaft_name`)
- **Admin-Endpunkte**: `GET /api/applications`, `POST /api/applications/{id}/confirm`, `POST /api/applications/{id}/reject`
