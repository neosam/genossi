## 1. Backend: Test-E-Mail Endpoint

- [x] 1.1 `send_test_mail(&self, to: &str)` Methode zum `MailService`-Trait hinzufügen
- [x] 1.2 Implementierung in `MailServiceImpl`: ruft `send_mail()` mit festem Betreff/Body auf
- [x] 1.3 `TestMailRequest`-Struct und `POST /test` Route in `genossi_mail/src/rest.rs` hinzufügen
- [x] 1.4 Route in `generate_route()` registrieren und OpenAPI-Docs aktualisieren
- [x] 1.5 E2E-Test für `POST /api/mail/test` (Validierungsfehler bei fehlender Config)

## 2. Frontend: API und i18n

- [x] 2.1 `send_test_mail()` Funktion in `api.rs` hinzufügen
- [x] 2.2 Neue i18n-Keys in `mod.rs`, `de.rs` und `en.rs` hinzufügen (SmtpSettings, SmtpHost, SmtpPort, SmtpEncryption, SmtpUser, SmtpPassword, SmtpFrom, SmtpTestMail, SmtpTestMailTo, SmtpTestSuccess, SmtpTestFailed, SmtpSaving, AdvancedConfig)

## 3. Frontend: SMTP-Formular-Komponente

- [x] 3.1 SMTP-Einstellungen-Formular als eigene Komponente oder Sektion in `config_page.rs` erstellen
- [x] 3.2 Beim Laden: bestehende Config-Einträge in Formularfelder mappen (`smtp_host` → Server-Feld, etc.)
- [x] 3.3 Speichern: alle 6 Config-Keys per `set_config_entry()` setzen
- [x] 3.4 Radio-Buttons für Verschlüsselung (none/starttls/tls)
- [x] 3.5 Password-Feld mit Platzhalter wenn Secret bereits gesetzt
- [x] 3.6 Test-E-Mail-Sektion: Adressfeld + Button + Erfolgs-/Fehlermeldung

## 4. Frontend: Config-Seite umstrukturieren

- [x] 4.1 SMTP-Formular oben einbauen
- [x] 4.2 Generische Key-Value-Tabelle unten beibehalten (mit "Erweiterte Konfiguration" Überschrift, einklappbar)
