---
title: Pre-Flight-Check: vor Send prüfen, ob alle Empfänger einen RepaymentLetter haben
date: 2026-06-03
priority: low
blocked_by: keiner — Backend-Feature ist fertig (Quick 260603-cz6)
---

# Pre-Flight-Check für attach_repayment_letter

## Was

Neuer REST-Endpoint (oder Erweiterung des bestehenden Mail-Compose-Flows) `POST /api/mail/check-repayment-letters`, der für eine Liste von `(member_id, repayment_phase_id)`-Paaren prüft, ob jeder Member einen passenden `RepaymentLetter`-MemberDocument hat. Antwort: Liste der Member ohne Letter, damit der Vorstand sie *vor* dem Send sieht.

```
POST /api/mail/check-repayment-letters
{
  "member_ids": [...],
  "repayment_phase_id": "<uuid>"
}
→ 200 OK
{
  "missing": [
    { "member_id": "...", "member_number": 1234, "name": "Hans Müller" }
  ],
  "total_checked": 25,
  "missing_count": 1
}
```

## Warum

Aktueller Failure-Modus (Worker markiert Empfänger als `failed` während des Send-Loops) ist reaktiv: der Vorstand sieht erst *nach* dem Klick auf „Senden", dass für N Empfänger kein Brief existiert. Pre-Flight verschiebt das Feedback nach vorne — Vorstand kann die fehlenden Briefe **zuerst** generieren und dann den Bulk-Send sauber durchlaufen lassen.

Außerdem: weniger „rote" Job-Detail-Seiten, weniger Retry-Klicks.

## Trade-off / Diskussion vor Implementierung

Diese Funktion überlappt mit [[frontend-uat-empfaenger-status-no-repayment-letter]] (UI-Anzeige post-send). Eine der beiden Optionen sollte gewählt werden — beide gleichzeitig wären Doppelarbeit:

- **Pre-Flight (dieses Todo)**: Validation **vor** Send, sauberer Job-Verlauf, aber extra Endpoint + extra Klick im UI-Flow
- **Post-Send-UI (das andere Todo)**: leitet sich aus existierenden Failure-Codes ab, kein neuer Endpoint, dafür „roter" Job mit Retry-Loop

Empfehlung: zuerst Post-Send-UI (kürzerer Pfad zu Wert), Pre-Flight nur wenn der Vorstand sich aktiv beklagt, dass die Retry-Schleife stört.

## Vorbedingung

- `MemberDocumentDao.find_by_member_id` + `RepaymentPhaseDao.find_by_id` existieren (✓)
- `find_repayment_letter_for_recipient` ist Helper im Worker — könnte in `genossi_service_impl` extrahiert und vom REST-Handler wiederverwendet werden

## Schritte (grob)

1. Helper `find_repayment_letter_for_recipient` aus `genossi_mail/src/worker.rs` in `genossi_service_impl/src/repayment_letter.rs` (oder neues Modul) extrahieren, mit `&[MemberDocumentEntity] + fiscal_year` Signatur
2. Neuer REST-Handler in `genossi_rest/src/repayment_letter.rs` (oder `mail.rs`)
3. Service-Layer: `RepaymentLetterService::find_missing_letters(member_ids, phase_id) -> Vec<MissingLetter>` oder ähnlich
4. Tests: 0/N missing, mixed-bag (manche da, manche fehlen), nicht-existierende Phase → 404
5. Frontend-Integration: vor `Send` Button-Click den Pre-Flight aufrufen, missing-Liste rendern, „Briefe jetzt generieren"-Button → bestehender Generate-Endpoint

## Akzeptanz

- Endpoint liefert deterministisch missing-Liste in derselben Reihenfolge wie input
- Performance: < 200ms für 50 Member (in-memory Filter, sollte trivial sein)
- Wird im Frontend vor jedem Bulk-Send mit `attach_repayment_letter=true` aufgerufen
- Workspace-Tests grün, clippy clean

## Routing

`/gsd-quick --discuss` wenn jemand Zeit hat — discuss-Flag empfohlen, weil die Trade-off-Entscheidung mit dem UI-Todo geklärt werden sollte.

## Cross-Refs

- Backend-Commit: `62e62b7`
- Konkurrent: [[frontend-uat-empfaenger-status-no-repayment-letter]] — wahrscheinlich Either-Or
- Verwandt: [[frontend-checkbox-attach-repayment-letter]] — das Feature, das diesen Check überhaupt nötig macht
