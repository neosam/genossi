---
title: Phase-10-Mail-Worker auf RepaymentContextResolver migrieren
date: 2026-06-01
priority: low
blocked_by: RepaymentContextResolver-Service muss zuerst existieren (Bestandteil von [[repayment-letter-bulk-versand]])
---

# Phase-10-Mail-Worker auf RepaymentContextResolver migrieren

## Was

Die inline-Aggregations-Logik im Phase-10-Mail-Worker
(`genossi_mail/src/worker.rs` — pro Recipient `RepaymentEntry` laden mit
`status IN ('Open', 'Contacted')` + soft-delete-Filter, `share_count`
summieren, `payout_amount` per `share_count × phase.share_value` als
deutschen Euro-String "X,YZ" formatieren) auf den neuen
`RepaymentContextResolver`-Service umstellen.

## Warum

DRY: Mit dem Letter-Service (siehe [[repayment-letter-bulk-versand]]) gibt
es einen zweiten Caller derselben Aggregation. User-Decision: Resolver
zuerst für Letter bauen, dann Worker per separatem Refactor darauf
migrieren. Reihenfolge minimiert Risiko am stabilen Phase-10-Code.

Aggregations-Regel und Format-Konvention sind zentrale Domain-Logik
(Phase-10 D-04, D-06) und sollten einmal gepflegt werden.

## Vorbedingung

- `RepaymentContextResolver` existiert und ist getestet
  (entsteht im Zuge der Letter-Phase)
- Resolver-Signatur ist stabil — Worker und Letter-Service haben dieselben
  Anforderungen (input: phase_id + member_id; output: payout_amount-String +
  share_count + fiscal_year)

## Schritte (grob)

1. Im Mail-Worker (`genossi_mail/src/worker.rs`) die Inline-Aggregation
   identifizieren (vermutlich im Render-Pfad direkt vor `merge_repayment_context`)
2. Dependency `Arc<RepaymentContextResolver>` (oder direkter Service-Trait)
   in `start_mail_worker(...)`-Signatur ergänzen — analog zu den 4 anderen
   Dependencies aus Phase 10 D-11
3. DI-Wiring in `genossi_bin/src/lib.rs::RestStateImpl::new()` erweitern
4. Inline-Logik durch Resolver-Call ersetzen; `merge_repayment_context`-
   Aufruf bleibt identisch (nimmt die drei Werte)
5. Bestehende Phase-10-E2E-Tests müssen unverändert grün bleiben (Verhalten
   ist behavior-equivalent — reines Refactor)
6. `cargo clippy --workspace --all-targets` clean

## Akzeptanz

- Inline-Aggregations-Code im Worker entfernt
- Single Source of Truth für Repayment-Kontext-Resolution
- Keine Phase-10-Regressions (bestehende E2E-Tests grün)
- Audit-Hashchain bleibt nach Bulk-Mail-Run valide

## Routing

Mit `/gsd-quick` ausführen, wenn die Letter-Phase abgeschlossen ist und der
Resolver-Service stabil ist. Quick statt eigene Phase, weil reines Refactor
ohne neue User-Funktionalität.
