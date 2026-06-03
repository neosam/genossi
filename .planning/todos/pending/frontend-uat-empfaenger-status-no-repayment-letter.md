---
title: UI-Anzeige „no_repayment_letter"-Status pro Empfänger im Job-Detail
date: 2026-06-03
priority: medium
blocked_by: keiner — Backend setzt das Error-Feld bereits (Quick 260603-cz6, Commit 62e62b7)
---

# UI-Anzeige „no_repayment_letter"-Status

## Was

Im Bulk-Mail-Job-Detail (Frontend) Empfänger, die mit `error="no_repayment_letter"` markiert wurden, deutlich kennzeichnen — idealerweise mit direktem Action-Link zum Letter-Generieren für den betroffenen Member.

## Warum

Der Worker markiert Empfänger mit `status=failed` und `error="no_repayment_letter"`, wenn der Vorstand `attach_repayment_letter=true` aktiviert hat, aber für einen Member noch kein Brief generiert ist. Aktuell sieht der Vorstand im UI nur „failed" mit einem Error-String. Bessere UX:

- Visueller Marker (Icon/Farbe) statt nur Text
- Action: „Brief jetzt generieren" → Pop-up oder direkter Call gegen `/api/repayment-phase/{phase_id}/letters/generate` mit `entry_ids=[<member-entry-id>]`
- Anschließend „Retry failed recipients" (Endpoint existiert schon: `MailService::retry_job`)

## Vorbedingung

- `MailRecipient.error` wird vom Worker korrekt gefüllt (✓ Quick 260603-cz6)
- Frontend hat bereits eine Job-Detail-Page mit Empfänger-Tabelle
- Repayment-Letter-Generate-Endpoint funktioniert (✓ Phase 13)
- `retry_job`-Endpoint existiert (✓ Phase 10)

## Schritte (grob)

1. Job-Detail-Page identifizieren (vermutlich `genossi-frontend/src/page/mail_job_details.rs` oder ähnlich; ggf. existiert sie noch nicht und muss gebaut werden)
2. Empfänger-Tabelle: für `status=failed` mit `error.starts_with("no_repayment_letter")` einen eigenen Marker rendern (z.B. orange-Badge mit Icon)
3. Aktion-Spalte/Button: „Brief generieren + erneut versuchen"
4. Component-First (siehe [[component-first]]): eigene Component `FailedRecipientActionRow` oder ähnlich, falls wiederverwendbar
5. i18n-Keys in `de.rs` + `en.rs`

## Akzeptanz

- Failed-Empfänger mit `no_repayment_letter` sind visuell vom generischen failed-Status unterschieden
- Ein-Klick-Pfad „Brief generieren + Retry" funktioniert end-to-end
- Workspace-Tests grün, Frontend kompiliert clean

## Routing

`/gsd-quick` — UI-Quick, ~1-2h je nach Component-Wiederverwendbarkeit.

## Cross-Refs

- Backend-Commit: `62e62b7`
- Verwandt: [[frontend-checkbox-attach-repayment-letter]] — die Checkbox, die diesen Failure-Pfad überhaupt aktiviert
- Verwandt: [[backend-pre-flight-check-attach-repayment-letter]] — alternative Ansatz, der diese UI teilweise überflüssig machen würde
