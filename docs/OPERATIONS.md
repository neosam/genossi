# Operations Runbook

Betreiber-Runbooks für Genossi. Enthält Verify-in-Prod-Schritte, die aus der Dev-Umgebung nicht automatisierbar sind, weil sie auf produktiv erreichbare Ressourcen (SMTP-Relays, Backup-Ziele o. ä.) angewiesen sind.

## SMTP-Encoding umschalten (MAIL-04)

Der Default für den Text-Teil ausgehender Mails ist `quoted-printable`. Das ist der sichere Fallback und funktioniert mit jedem SMTP-Relay. `8bit` ist ein Opt-in und erfordert, dass der Produktivrelay das `8BITMIME`-Feature (RFC 6152) per EHLO-Antwort ankündigt. Aus der Dev-Umgebung ist der Prod-Relay nicht erreichbar (Netz-Isolation), deshalb muss der Betreiber diesen Check ONE-SHOT im Prod-Netz durchführen — die Aktivierung von `8bit` gegen einen Relay ohne `8BITMIME` kann `550`-Bounces oder stille Byte-Verstümmelung nach sich ziehen.

**Reihenfolge ist verbindlich: erst Schritt 1 verifizieren, dann Schritt 2 setzen.** Umgekehrt niemals.

### Schritt 1 — 8BITMIME am Relay verifizieren

Auf einem Host im Prod-Netz die TLS-STARTTLS-Verbindung zum Relay öffnen:

```bash
openssl s_client -starttls smtp -connect <relay-host>:<port> -crlf
```

`<relay-host>:<port>` durch die tatsächlichen Werte aus der aktuellen Genossi-Config (`smtp_host`, `smtp_port`) ersetzen.

Sobald die STARTTLS-Handshake-Ausgabe endet, im interaktiven Prompt der Sitzung ein EHLO an den Relay senden — der Hostname darf frei gewählt werden, für Genossi-Betrieb passt z. B.:

```
EHLO genossi.local
```

Erwartete Antwort ist eine mehrzeilige EHLO-Antwort. In dieser Antwort muss die folgende Zeile vorhanden sein:

```
250-8BITMIME
```

Wenn diese Zeile FEHLT, ist Schritt 2 nicht erlaubt — der Relay unterstützt kein `8BITMIME` und würde 8-bit-Bytes im Text-Teil möglicherweise verstümmeln. In diesem Fall auf dem Default `quoted-printable` bleiben.

### Schritt 2 — Config-Toggle setzen

Nur wenn Schritt 1 grün ist (Zeile `250-8BITMIME` in der EHLO-Antwort gesehen):

Den Config-Key `smtp_encoding` auf den Wert `8bit` setzen. Der Key wird in der bestehenden Genossi-Config-UI gepflegt (er landet in der `config_entries`-KV-Tabelle — kein separater Admin-Flow, keine Migration).

- Key: `smtp_encoding`
- Wert: `8bit`

Ein Neustart des Backends ist nicht erforderlich: `SmtpConfig` wird pro Sendevorgang frisch aus der KV-Tabelle geladen, die Umstellung greift ab der nächsten ausgehenden Mail.

### Rollback

Um auf den sicheren Default zurückzugehen, den Key `smtp_encoding` entweder explizit auf `quoted-printable` setzen oder den Key ganz löschen — beides ist gleichwertig: `load_smtp_config` fällt bei fehlendem oder leerem Key automatisch auf `MailEncoding::QuotedPrintable` zurück (tolerantes Fallback-Muster analog zu `smtp_tls`). Auch hier ist kein Neustart nötig.

---

Dieser Abschnitt deckt MAIL-04 aus Phase 22 ab. Weitere Betreiber-Runbooks (Phase 23 und später) landen in dieser Datei unter neuen `##`-Abschnitten.
