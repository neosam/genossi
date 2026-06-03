---
title: "Bulk-Action: alle no_repayment_letter-Briefe in einem Job auf einmal generieren + retry"
created: 2026-06-03
origin: quick-260603-evf
priority: low
---

# Bulk-Action für no_repayment_letter-Recovery

## Kontext

Quick 260603-evf hat den **per-Empfänger**-Action-Button "Brief generieren + Retry"
ausgeliefert (`NoRepaymentLetterAction` Component in beiden Empfänger-Tabellen).
Bewusst NICHT umgesetzt: ein Job-weiter **Bulk-Button**, der alle
no_repayment_letter-Empfänger in einem einzigen Klick abarbeitet.

## Warum als Follow-up

- Per-Empfänger-Recovery deckt 95% der Realität ab. Vorstände haben in einem
  Bulk-Mail-Job typischerweise 0-3 Empfänger mit fehlendem Brief.
- Bulk-Action bringt zusätzliche UX-Fragen mit: All-or-nothing vs. Best-Effort,
  Progress-Bar pro Empfänger, Teilfehler-Handling. Das gehört in einen
  eigenen Plan mit eigenen Acceptance-Criteria.
- Scope-Eindämmung schützt den Quick davor, in ein Mini-Phase-Refactor zu
  geraten.

## Vorschlag für die Umsetzung

- Neuer Button "Alle ({count}) Briefe generieren + Retry" oberhalb der
  Empfänger-Tabelle, sichtbar wenn `count(is_no_repayment_letter_failure) > 0`.
- Wiederverwendung der bestehenden `find_entry_for_member`-Logik aus
  `no_repayment_letter_action.rs`: alle betroffenen Member-IDs auf
  Entry-IDs auflösen, EIN `generate_repayment_letters`-Call mit ALLEN
  Entry-IDs, dann genau EIN `retry_mail_job`-Call.
- Progress als Toast-Sequenz (`Generating 1/3 letters...`, `Generated 3 letters,
  triggering retry...`, `Done — N recipients retried`).

## Acceptance Criteria

- [ ] Button erscheint nur wenn >= 1 Empfänger mit no_repayment_letter-Failure
- [ ] Klick generiert alle fehlenden Briefe in einem Endpoint-Call (kein N+1)
- [ ] Bei Teilfehler (z.B. ein Member hat keinen Entry in der Phase) wird der
      gesamte Flow abgebrochen und der Vorstand bekommt einen Error-Toast mit
      genauer Liste der nicht-resolvierbaren Member-Nummern.
- [ ] Single-Retry-Call am Ende, nicht ein Retry pro Member.
- [ ] Reload der Tabelle nach Abschluss.

## Nicht-Ziele

- Asynchroner Hintergrund-Job für Riesen-Empfänger-Listen (wäre Phase-Scope).
- Differenzierte Retry-Strategien je nach Failure-Mode.

## Referenzen

- Bestehende Per-Empfänger-Component: `genossi-frontend/src/component/no_repayment_letter_action.rs`
- Bestehende Endpoints: `api::list_repayment_entries`, `api::generate_repayment_letters`, `api::retry_mail_job`
- Per-Empfänger-Recovery: SUMMARY Quick 260603-evf
