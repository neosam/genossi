## Context

Genossi hat aktuell keine Möglichkeit, mit Mitgliedern zu kommunizieren. E-Mail-Versand über ein dediziertes SMTP-Postfach soll als erster Kommunikationskanal eingeführt werden. Gleichzeitig fehlt ein allgemeiner Konfigurationsmechanismus — SMTP-Credentials und andere Einstellungen sollen über eine Config-Tabelle in der Datenbank gespeichert und im Frontend verwaltet werden.

Das System nutzt eine layered Architecture mit DAO → Service → REST. Neue Capabilities folgen diesem Muster. Alle Entities verwenden UUID-IDs, `created`/`deleted` Timestamps und `version` für Optimistic Locking.

## Goals / Non-Goals

**Goals:**
- Generischer Config-Store als Key-Value-Tabelle mit Typ-Information
- SMTP-Mailversand für Plain-Text-Mails via `lettre`
- Persistenz gesendeter Mails mit Status-Tracking (sent/failed) und Fehlermeldungen
- Config und Mail über REST-API und Frontend bedienbar

**Non-Goals:**
- HTML-Mails (kommt als separater Change)
- Mail-Templates oder automatisierte Mails (z.B. bei Member-Events)
- Mail-Queue oder Retry-Logik
- Verschlüsselung von Config-Werten im Backend (Klartext in DB ist akzeptiert)
- Rollen/Rechte-Einschränkung (aktuell ist jeder User Admin)

## Decisions

### 1. Config-Store als schlankes DAO-Pattern (kein UUID, kein Soft-Delete)

Die `config_entries`-Tabelle nutzt `key` als Primary Key mit `value` (TEXT) und `value_type` (TEXT). Kein UUID-ID, kein `created`/`deleted`/`version`. Hard-Delete statt Soft-Delete. Das ist bewusst anders als die Domain-Entities, weil Config technische, nicht fachliche Daten sind.

```
config_entries
├── key        TEXT  PK
├── value      TEXT
└── value_type TEXT  (string, int, bool, secret)
```

Das DAO hat: `get(key)`, `set(entry)` (upsert), `all()`, `delete(key)`.

**Alternative**: Config als volles Entity-Pattern mit UUID/Version. Verworfen — Config-Einträge sind keine fachlichen Entities, der Overhead ist unnötig.

**Alternative**: Config weiterhin über Environment Variables. Verworfen — macht Frontend-Konfiguration unmöglich.

### 2. Eigenes Crate `genossi_config` für Config-Store

Umfasst DAO-Trait, SQLite-Implementierung, Service und REST-Endpunkte. Hält den Config-Store von der Domain-Logik getrennt.

**Alternative**: Config in bestehende DAO/Service-Crates integrieren. Verworfen — Config ist cross-cutting und hat ein eigenes Pattern (kein UUID/Soft-Delete). Ein eigenes Crate hält die Trennung sauber.

### 3. Sent Mails als volles Entity-Pattern

Gesendete Mails sind fachliche Daten und folgen dem normalen Entity-Pattern mit `id` (UUID), `created`, `deleted`, `version`. Zusätzliche Felder:

```
sent_mails
├── id           BLOB (UUID)
├── created      TEXT (Timestamp)
├── deleted      TEXT (Timestamp, nullable)
├── version      BLOB (UUID)
├── to_address   TEXT
├── subject      TEXT
├── body         TEXT
├── status       TEXT (sent/failed)
├── error        TEXT (nullable)
└── sent_at      TEXT (Timestamp, nullable)
```

Das DAO folgt dem Standard-Pattern: `dump_all()`, `create()`, `update()` mit Default-Implementierungen für `all()` und `find_by_id()`.

### 4. Eigenes Crate `genossi_mail` für Mailversand

Enthält:
- SentMail-DAO-Trait und SQLite-Implementierung
- Mail-Service: liest SMTP-Config aus dem Config-Service, versendet via `lettre`, speichert Ergebnis in SentMail-DAO
- REST-Endpunkte: `POST /api/mail/send`, `GET /api/mail/sent` (Historie)

Der Mail-Service hängt vom Config-Service ab (für SMTP-Credentials).

### 5. SMTP-Config über definierte Keys

Folgende Config-Keys werden für SMTP verwendet:

| Key | value_type | Beschreibung |
|-----|-----------|--------------|
| `smtp_host` | string | SMTP-Server Hostname |
| `smtp_port` | int | SMTP-Port (Standard: 587) |
| `smtp_user` | string | SMTP-Benutzername |
| `smtp_pass` | secret | SMTP-Passwort |
| `smtp_from` | string | Absender-Adresse |
| `smtp_tls` | string | TLS-Modus: `none`, `starttls`, `tls` |

Der Mail-Service liest diese zur Sendezeit. Keine Caching — bei jedem Send werden die aktuellen Werte gelesen.

### 6. Synchroner Versand ohne Queue

`POST /api/mail/send` versendet die Mail synchron im Request-Handler:
1. Mail-Daten validieren
2. SMTP-Config aus Config-Service lesen
3. `lettre` Transport erstellen und Mail senden
4. SentMail-Entity mit Status `sent` oder `failed` + Fehlermeldung speichern
5. Response mit SentMail-Entity zurückgeben

**Alternative**: Async Queue mit Background-Worker. Verworfen — für den aktuellen Anwendungsfall (einzelne manuelle Mails) ist synchroner Versand einfacher und ausreichend.

### 7. `value_type` als Frontend-Hint und Service-Validierung

Der Config-Service validiert beim `set()`:
- `int`: Wert muss als Integer parsebar sein
- `bool`: Wert muss `true` oder `false` sein
- `string`: Keine Einschränkung
- `secret`: Wie string, aber Frontend zeigt Passwort-Input und maskiert Anzeige. `GET /api/config` liefert für `secret`-Werte nur `***` statt des echten Werts.

### 8. REST-Endpunkte

**Config:**
- `GET /api/config` — Alle Config-Einträge (secrets maskiert)
- `PUT /api/config/{key}` — Config-Wert setzen (upsert)
- `DELETE /api/config/{key}` — Config-Wert löschen

**Mail:**
- `POST /api/mail/send` — Mail versenden
- `GET /api/mail/sent` — Gesendete Mails auflisten

## Risks / Trade-offs

- **SMTP-Timeout blockiert Request**: Synchroner Versand kann bei langsamen SMTP-Servern den HTTP-Response verzögern. → Mitigation: `lettre` hat konfigurierbare Timeouts. Für den Anfang akzeptabel, Queue kann später ergänzt werden.
- **Klartext-Passwort in DB**: SMTP-Passwort liegt unverschlüsselt in `config_entries`. → Mitigation: Akzeptiert für den Anwendungsfall (eigener Server, nur Admins). GET-Endpoint maskiert den Wert.
- **Keine Retry-Logik**: Fehlgeschlagene Mails werden nur protokolliert, nicht erneut versendet. → Mitigation: User sieht Fehlerstatus in der Historie und kann manuell erneut senden.
- **Config-Keys nicht vordefiniert**: Jeder beliebige Key kann gesetzt werden. → Mitigation: Mail-Service prüft ob alle benötigten SMTP-Keys vorhanden sind und gibt verständliche Fehlermeldung.
