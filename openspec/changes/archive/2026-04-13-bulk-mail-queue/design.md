## Context

Aktuell versendet der `MailService` E-Mails synchron im HTTP-Request. Das `SentMail`-Modell ist flach — eine Zeile pro Empfänger, ohne Gruppierung. Bei 600 Empfängern und IONOS Basic Mail (max ~100/Stunde im sicheren Betrieb) dauert der Versand ~6 Stunden. Der HTTP-Request kann das nicht halten, und ein Server-Neustart verliert den Fortschritt.

Die bestehende Architektur nutzt: async DAO-Traits mit SQLite-Impl, Service-Traits mit generischer DI, REST-State-Traits mit associated Types, und `gen_service_impl!` Macros.

## Goals / Non-Goals

**Goals:**
- Asynchroner, restart-sicherer Bulk-Mail-Versand über DB-basierte Queue
- Gruppierte Darstellung: ein Mail-Job mit N Empfängern
- Konfigurierbares Sende-Intervall (Default: 36s ≈ 100/Stunde)
- Fortschritts-Tracking pro Job (sent/failed/pending Counts)
- Einzelne Empfänger-Mails nach Fehler erneut sendbar

**Non-Goals:**
- HTML-Mails (bleibt Plain-Text)
- Personalisierung/Templates im Mail-Body (z.B. `{name}` ersetzen)
- Scheduled/Zeitgesteuerte Mails
- Abbrechen laufender Jobs (kann später ergänzt werden)
- Multi-Worker / verteiltes Senden

## Decisions

### 1. Zwei neue Tabellen statt `sent_mails`

**Entscheidung:** `mail_jobs` + `mail_recipients` ersetzen `sent_mails` komplett.

**Struktur:**
```
mail_jobs:
  id          BLOB PRIMARY KEY     -- UUID
  created     TEXT NOT NULL
  deleted     TEXT                  -- Soft delete
  version     BLOB NOT NULL        -- Optimistic locking
  subject     TEXT NOT NULL
  body        TEXT NOT NULL
  status      TEXT NOT NULL         -- pending, running, done, failed
  total_count INTEGER NOT NULL
  sent_count  INTEGER NOT NULL DEFAULT 0
  failed_count INTEGER NOT NULL DEFAULT 0

mail_recipients:
  id          BLOB PRIMARY KEY     -- UUID
  created     TEXT NOT NULL
  deleted     TEXT
  version     BLOB NOT NULL
  mail_job_id BLOB NOT NULL        -- FK → mail_jobs.id
  to_address  TEXT NOT NULL
  member_id   BLOB                 -- Optional FK → persons.id
  status      TEXT NOT NULL         -- pending, sent, failed
  error       TEXT
  sent_at     TEXT
```

**Alternativen:**
- *Flaches Modell beibehalten + Status-Feld:* Keine Gruppierung, Frontend bleibt unübersichtlich.
- *Separate Queue-Tabelle + sent_mails:* Unnötige Redundanz, zwei Systeme zu pflegen.

### 2. Background-Worker als Tokio-Task

**Entscheidung:** Ein `tokio::spawn`-Task wird beim Server-Start gestartet, der die DB pollt.

**Worker-Logik:**
```
loop {
    1. Finde nächsten mail_recipient mit status = "pending"
       dessen mail_job status = "running"
       ORDER BY mail_jobs.created ASC, mail_recipients.created ASC
    2. Falls gefunden:
       - Mail senden (bestehende SMTP-Logik)
       - Recipient-Status updaten (sent/failed)
       - Job-Counter updaten (sent_count/failed_count)
       - Prüfen ob Job fertig (sent_count + failed_count == total_count → status = done)
       - sleep(mail_send_interval_seconds)
    3. Falls nicht gefunden:
       - sleep(5 Sekunden)  -- idle polling
}
```

**Alternativen:**
- *In-Memory-Queue (tokio::mpsc):* Nicht restart-sicher, verliert State bei Crash.
- *Externe Queue (Redis, RabbitMQ):* Overkill für einen Verein mit monatlichem Newsletter.
- *Task pro Job spawnen:* Mehrere Jobs könnten parallel laufen und SMTP-Limits überschreiten.

### 3. Einzelner Worker, sequentieller Versand

**Entscheidung:** Genau ein Worker-Task, sendet eine Mail nach der anderen mit Pause.

**Begründung:** Bei IONOS Basic Mail mit 500/Stunde Limit und unserem Ziel von 100/Stunde ist Parallelität kontraproduktiv. Ein Worker garantiert, dass das Intervall eingehalten wird, ohne Koordination zwischen Tasks.

### 4. Job-Erstellung: Sofort pending, sofort zurück

**Entscheidung:** `POST /api/mail/send-bulk` erstellt einen Job mit Status `running` und alle Recipients mit Status `pending`, gibt HTTP 202 mit dem Job zurück. Der Worker übernimmt den Versand.

**Einzelversand:** `POST /api/mail/send` erstellt ebenfalls einen Job (mit 1 Recipient). Einheitlicher Code-Pfad.

**Test-Mail:** `POST /api/mail/test` bleibt synchron und direkt — kein Job. Dient nur zur SMTP-Config-Prüfung.

### 5. Sende-Intervall als Config-Store-Eintrag

**Entscheidung:** `mail_send_interval_seconds` (Typ: `int`, Default: `36`) im bestehenden Config-Store.

**Begründung:** Kein Code-Change nötig wenn sich der IONOS-Tarif ändert. Worker liest den Wert bei jeder Iteration.

### 6. Retry: Manuell über neuen Endpoint

**Entscheidung:** `POST /api/mail/jobs/{id}/retry` setzt alle `failed`-Recipients eines Jobs auf `pending` zurück und setzt Job-Status auf `running`.

**Alternativen:**
- *Automatisches Retry mit Backoff:* Komplex, IONOS könnte bei wiederholten Fehlern sperren.
- *Kein Retry:* Zu umständlich bei transient Errors.

### 7. Code-Struktur innerhalb `genossi_mail`

**Entscheidung:** Neue Dateien innerhalb des bestehenden `genossi_mail`-Crates:

- `dao.rs` → DAO-Trait für `MailJobDao` und `MailRecipientDao` (ersetzt `SentMailDao`)
- `dao_sqlite.rs` → SQLite-Implementierung (ersetzt bestehende)
- `service.rs` → `MailService`-Trait bekommt neue Methoden für Jobs
- `worker.rs` → Neues Modul: Background-Worker-Logik
- `rest.rs` → Angepasste Endpoints + neue Job-Status-Endpoints

**Begründung:** Alles Mail-bezogene bleibt im `genossi_mail`-Crate. Kein neues Crate nötig.

## Risks / Trade-offs

- **DB-Polling statt Event-driven** → Worker pollt alle 5s im Idle. Bei einem kleinen Verein ist das vernachlässigbar. Mitigation: Kurzer Sleep, minimale Query.
- **Einzelner Worker** → Nur ein Job kann gleichzeitig versendet werden. Wenn zwei Jobs erstellt werden, wird der zweite erst nach dem ersten bearbeitet. Mitigation: Für den Anwendungsfall (monatlicher Newsletter) ausreichend.
- **SMTP-Config wird pro Mail geladen** → Keine Änderung zum Status quo. Bei Interval von 36s ist das nicht performance-relevant.
- **Migration löscht sent_mails** → Bestehende Mail-Historie geht verloren. Mitigation: Tabelle ist nicht produktiv, keine relevanten Daten.
- **Config-Wert wird pro Iteration gelesen** → Ermöglicht Änderung zur Laufzeit, aber minimal mehr DB-Last. Trade-off akzeptabel.
