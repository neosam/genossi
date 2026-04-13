## Context

Der Mail-Worker (`genossi_mail/src/worker.rs`) verarbeitet Recipients in einer Endlosschleife. Die `next_pending()` Query filtert auf `r.status = 'pending' AND j.status = 'running'`. Wenn alle Recipients `failed` sind und `sent_count + failed_count >= total_count`, wird der Job auf `failed` gesetzt.

In der Praxis wird beobachtet, dass der Job von `failed` zurück auf `running` wechselt und fehlgeschlagene Recipients erneut verarbeitet werden. Die einzige Codestelle, die Recipients zurücksetzt, ist `retry_job()`. Es ist unklar, ob dieser Endpoint unbeabsichtigt aufgerufen wird oder ob ein anderer Mechanismus den Zustand ändert.

Zwei Schwachstellen im aktuellen Code:
1. Das Job-Update nach Completion-Check wird bei Fehlern nur geloggt, nicht retried
2. Es gibt kein Logging am `retry_job` Endpoint — Aufrufe sind unsichtbar

## Goals / Non-Goals

**Goals:**
- Diagnose ermöglichen: Logging am retry_job Endpoint, damit sichtbar wird ob/wann er aufgerufen wird
- Worker robuster machen: Job-Completion-Update darf nicht stillschweigend fehlschlagen
- Schutz gegen unbeabsichtigte Retry-Loops

**Non-Goals:**
- Automatische Retry-Logik (z.B. "versuche 3x bevor failed") — nicht gewünscht
- Änderung der `retry_job` Funktionalität — der explizite Retry soll weiter funktionieren
- Frontend-Änderungen

## Decisions

### 1. Logging am retry_job Endpoint

**Entscheidung:** `tracing::info!` mit Job-ID am Anfang des `retry_job` REST-Handlers hinzufügen.

**Warum:** Damit in den Server-Logs sichtbar wird, ob und wann der Retry-Endpoint aufgerufen wird. Das ist die einfachste Diagnose-Maßnahme.

### 2. Job-Update bei Completion absichern

**Entscheidung:** Wenn `job_dao.update()` nach der Completion-Prüfung fehlschlägt, soll der Worker es erneut versuchen (max 3 Versuche) statt den Fehler nur zu loggen.

**Warum:** Wenn das Job-Update fehlschlägt, bleibt der Job `running` in der DB obwohl alle Recipients verarbeitet sind. Beim nächsten `next_pending()` kommt zwar `None` zurück (weil alle Recipients `failed`/`sent` sind), aber der Job-Status in der DB ist inkonsistent. Falls ein externer Prozess den Job-Status liest und darauf reagiert, könnte das zu Problemen führen.

**Alternative verworfen:** Job-Status vor dem Recipient-Update setzen → Risiko dass Recipients als "pending" bleiben wenn der Worker crasht.

### 3. Kein automatischer Retry von failed Recipients im Worker

**Entscheidung:** Der Worker-Code bleibt wie er ist — `next_pending()` filtert korrekt auf `status = 'pending'`. Es wird keine zusätzliche Schutzlogik im Worker eingebaut.

**Warum:** Das Problem liegt nicht im Worker-Query-Filter, sondern darin, dass irgendwo der Zustand zurückgesetzt wird. Zusätzliche Filter wären Defense-in-Depth, aber lösen nicht die Ursache.

## Risks / Trade-offs

- [Logging reicht nicht zur Diagnose] → Falls der Retry nicht über den REST-Endpoint kommt, sondern direkt in der DB, brauchen wir zusätzlich DB-Monitoring. Erster Schritt ist aber das Endpoint-Logging.
- [Retry des Job-Updates kann bei persistenten DB-Problemen blockieren] → Max 3 Versuche mit kurzem Delay, danach weiter mit Fehler-Log wie bisher.
