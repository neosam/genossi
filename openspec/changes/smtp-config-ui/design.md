# Design: SMTP Config UI

## Frontend-Änderungen

### Config-Seite Aufbau

Die Config-Seite (`config_page.rs`) wird umstrukturiert in zwei Sektionen:

1. **SMTP-Einstellungen** (oben) — typisiertes Formular
2. **Erweiterte Konfiguration** (unten) — bestehende generische Key-Value-Tabelle

### SMTP-Formular

Das Formular zeigt beschriftete Felder und mapped sie intern auf die Config-Keys:

| Feld-Label         | Config-Key  | Input-Typ       | value_type |
|--------------------|-------------|-----------------|------------|
| SMTP-Server        | `smtp_host` | text            | `string`   |
| Port               | `smtp_port` | number          | `int`      |
| Verschlüsselung    | `smtp_tls`  | radio (3 Werte) | `string`   |
| Benutzername       | `smtp_user` | text            | `string`   |
| Passwort           | `smtp_pass` | password        | `secret`   |
| Absender-Adresse   | `smtp_from` | email           | `string`   |

**Verschlüsselungs-Radio-Buttons**: `none` / `starttls` (default) / `tls`

**Verhalten**:
- Beim Laden werden die bestehenden Config-Einträge gelesen und in die Formularfelder eingetragen (Secrets zeigen Platzhalter wenn belegt)
- "Speichern" setzt alle 6 Keys per `PUT /api/config/{key}` (sequentiell oder parallel)
- Es werden nur geänderte/neue Werte gesendet, oder alle auf einmal — je nachdem was einfacher ist. Da es nur 6 Keys sind, können wir alle senden.

### Test-E-Mail

- Textfeld für die Ziel-E-Mail-Adresse
- Button "Test-E-Mail senden"
- Ruft `POST /api/mail/test` auf mit `{ "to_address": "..." }`
- Zeigt Erfolgs- oder Fehlermeldung an

## Backend-Änderungen

### Neuer Endpoint: `POST /api/mail/test`

Im `genossi_mail`-Modul:

**Request**:
```json
{ "to_address": "admin@example.com" }
```

**Verhalten**:
- Sendet eine kurze Test-E-Mail mit festem Betreff ("Genossi Test-E-Mail") und Body ("Diese E-Mail bestätigt, dass die SMTP-Konfiguration korrekt ist.")
- Nutzt den bestehenden `MailService::send_mail()`
- Speichert das Ergebnis in `sent_mails` (wie reguläre Mails)

**Response**: Wie `POST /api/mail/send` — `SentMailTO` mit Status

### MailService-Erweiterung

Neue Methode im `MailService`-Trait:
```rust
async fn send_test_mail(&self, to: &str) -> Result<SentMail, MailServiceError>;
```

Diese ruft intern `send_mail()` mit festem Betreff/Body auf.

## i18n

Neue Keys:
- `SmtpSettings` — "SMTP-Einstellungen" / "SMTP Settings"
- `SmtpHost` — "SMTP-Server" / "SMTP Server"
- `SmtpPort` — "Port" / "Port"
- `SmtpEncryption` — "Verschlüsselung" / "Encryption"
- `SmtpEncryptionNone` — "Keine" / "None"
- `SmtpEncryptionStarttls` — "STARTTLS" / "STARTTLS"
- `SmtpEncryptionTls` — "TLS" / "TLS"
- `SmtpUser` — "Benutzername" / "Username"
- `SmtpPassword` — "Passwort" / "Password"
- `SmtpFrom` — "Absender-Adresse" / "From Address"
- `SmtpTestMail` — "Test-E-Mail senden" / "Send Test Email"
- `SmtpTestMailTo` — "Test-Adresse" / "Test Address"
- `SmtpTestSuccess` — "Test-E-Mail erfolgreich gesendet" / "Test email sent successfully"
- `SmtpTestFailed` — "Test-E-Mail fehlgeschlagen" / "Test email failed"
- `SmtpSaving` — "Speichere..." / "Saving..."
- `AdvancedConfig` — "Erweiterte Konfiguration" / "Advanced Configuration"
