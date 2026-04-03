## Why

Der Mail-Worker verarbeitet fehlgeschlagene Empfänger wiederholt in einer Endlosschleife, obwohl sie bereits als `failed` markiert wurden. Der Job wechselt von `failed` zurück auf `running` ohne expliziten Benutzer-Retry. Die genaue Ursache ist noch unklar — möglicherweise ein Race Condition, ein fehlgeschlagenes Job-Update, oder ein unbeabsichtigter Retry-Aufruf. Logging und robustere Absicherung sind nötig, um das Problem zu diagnostizieren und zu verhindern.

## What Changes

- Logging am `retry_job` REST-Endpoint hinzufügen, um zu erfassen wann und von wem Retries ausgelöst werden
- Worker robuster machen: Job-Completion-Update absichern (nicht stillschweigend ignorieren wenn es fehlschlägt)
- Worker soll nur explizit freigegebene Recipients verarbeiten — keine automatische Wiederholung von `failed` Recipients

## Capabilities

### New Capabilities

- `mail-worker-resilience`: Absicherung des Mail-Workers gegen unbeabsichtigte Retry-Loops und besseres Logging für Diagnose

### Modified Capabilities

- `mail-sending`: Worker darf fehlgeschlagene Recipients nicht automatisch erneut verarbeiten; Job-Update-Fehler müssen behandelt werden

## Impact

- `genossi_mail/src/worker.rs`: Worker-Loop Logik und Fehlerbehandlung
- `genossi_mail/src/rest.rs`: Logging am retry_job Endpoint
- `genossi_mail/src/service.rs`: Keine funktionale Änderung, aber ggf. zusätzliche Absicherung
