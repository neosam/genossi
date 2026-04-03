## Why

Genossi braucht die Möglichkeit, E-Mails direkt aus dem Frontend zu versenden — z.B. um Mitglieder zu kontaktieren oder Benachrichtigungen zu verschicken. Aktuell gibt es keinen Kommunikationskanal im System. Ein dediziertes SMTP-Postfach wird eingerichtet, über das Mails versendet werden können.

## What Changes

- Neues Crate `genossi_mail` für SMTP-Mailversand via `lettre`
- Neue generische Config-Tabelle (Key-Value-Store mit Typ-Information) für SMTP-Einstellungen und zukünftige Konfiguration
- REST-Endpunkt zum Versenden von Plain-Text-Mails
- REST-Endpunkte für Config-CRUD (lesen, setzen, löschen)
- Gesendete Mails werden in einer `sent_mails`-Tabelle mit Statustracking (sent/failed) und Fehlermeldungen gespeichert
- Frontend: Config-Seite für SMTP-Einstellungen und Mail-Compose-Formular

## Capabilities

### New Capabilities
- `config-store`: Generischer Key-Value-Konfigurationsspeicher mit Typ-Information (string, int, bool, secret). Schlankes DAO-Pattern ohne UUID/Soft-Delete. Konfigurierbar über REST-API und Frontend.
- `mail-sending`: SMTP-Mailversand mit Plain-Text-Mails. Liest SMTP-Konfiguration aus dem Config-Store. Speichert gesendete Mails mit Status und Fehlermeldungen in der Datenbank (volles Entity-Pattern).

### Modified Capabilities

_(keine)_

## Impact

- **Neue Crates**: `genossi_config` (DAO + Service + REST), `genossi_mail` (DAO + Service + REST)
- **Neue Dependency**: `lettre` für SMTP
- **Datenbank**: Zwei neue Tabellen (`config_entries`, `sent_mails`), neue Migrationen nötig
- **Frontend**: Neue Seiten für Config und Mail-Compose
- **Workspace**: `Cargo.toml` erweitern um neue Workspace-Members
