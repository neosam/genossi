## 1. Config-Store: Crate-Setup und DAO

- [x] 1.1 Crate `genossi_config` anlegen mit Cargo.toml, als Workspace-Member registrieren
- [x] 1.2 ConfigEntry-Struct definieren (key, value, value_type) und ConfigDao-Trait mit `get`, `set`, `all`, `delete`
- [x] 1.3 SQLite-Migration für `config_entries`-Tabelle erstellen (key TEXT PK, value TEXT, value_type TEXT)
- [x] 1.4 SQLite-Implementierung des ConfigDao-Traits (upsert via INSERT OR REPLACE)
- [x] 1.5 Unit-Tests für ConfigDao SQLite-Implementierung

## 2. Config-Store: Service und REST

- [x] 2.1 ConfigService-Trait definieren mit CRUD-Methoden und value_type-Validierung (int parsebar, bool true/false)
- [x] 2.2 ConfigService-Implementierung mit Validierung
- [x] 2.3 Unit-Tests für ConfigService (Validierung: gültige/ungültige int/bool-Werte)
- [x] 2.4 REST-Endpunkte: GET /api/config (mit Secret-Maskierung), PUT /api/config/{key}, DELETE /api/config/{key}
- [x] 2.5 REST-Types für Config (Request/Response-Structs mit Utoipa-Schema)
- [x] 2.6 Config-Endpunkte in den Axum-Router integrieren und in main.rs verdrahten
- [x] 2.7 E2E-Tests für Config-REST-Endpunkte (CRUD + Secret-Maskierung)

## 3. Mail: Crate-Setup und DAO

- [x] 3.1 Crate `genossi_mail` anlegen mit Cargo.toml (Dependency auf genossi_config, lettre), als Workspace-Member registrieren
- [x] 3.2 SentMail-Entity definieren (id, created, deleted, version, to_address, subject, body, status, error, sent_at)
- [x] 3.3 SentMailDao-Trait nach Standard-Pattern (dump_all, create, update) mit Default-Implementierungen
- [x] 3.4 SQLite-Migration für `sent_mails`-Tabelle erstellen
- [x] 3.5 SQLite-Implementierung des SentMailDao-Traits
- [x] 3.6 Unit-Tests für SentMailDao SQLite-Implementierung

## 4. Mail: Service

- [x] 4.1 MailService-Trait definieren: `send_mail(to, subject, body)` → Result<SentMail>
- [x] 4.2 MailService-Implementierung: SMTP-Config aus ConfigService lesen, lettre-Transport erstellen, Mail senden, Ergebnis in SentMailDao speichern
- [x] 4.3 TLS-Modus-Auswahl implementieren (none/starttls/tls basierend auf smtp_tls Config)
- [x] 4.4 SMTP-Config-Validierung: alle erforderlichen Keys prüfen vor Sendeversuch
- [x] 4.5 Unit-Tests für MailService (Mock ConfigDao + Mock SentMailDao, Fehlerszenarien)

## 5. Mail: REST

- [x] 5.1 REST-Types für Mail (SendMailRequest, SentMailResponse mit Utoipa-Schema)
- [x] 5.2 REST-Endpunkt POST /api/mail/send
- [x] 5.3 REST-Endpunkt GET /api/mail/sent (Liste gesendeter Mails, absteigend nach created)
- [x] 5.4 Mail-Endpunkte in den Axum-Router integrieren und in main.rs verdrahten
- [x] 5.5 E2E-Tests für Mail-REST-Endpunkte (Validierungsfehler, fehlende Config)

## 6. Frontend

- [x] 6.1 Config-Seite: Alle Config-Einträge auflisten, bearbeiten, löschen. Secret-Felder als Passwort-Input.
- [x] 6.2 Mail-Compose-Seite: Formular mit An, Betreff, Text und Senden-Button
- [x] 6.3 Mail-Historie-Anzeige: Liste gesendeter Mails mit Status (sent/failed) und Fehlermeldungen
- [x] 6.4 Navigation: Links zu Config und Mail in der Seitenleiste/Navigation ergänzen
