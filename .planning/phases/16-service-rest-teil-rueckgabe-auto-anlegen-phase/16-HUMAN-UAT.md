---
status: failed
phase: 16-service-rest-teil-rueckgabe-auto-anlegen-phase
source: [16-VERIFICATION.md, 16-REVIEW.md]
started: 2026-06-05T10:00:00Z
updated: 2026-06-05T10:30:00Z
decision: gap-closure (Option A — Status-Guard einbauen via Phase 16.1)
---

## Current Test

[user decision recorded: Option A — Gap-Closure via Phase 16.1]

## Tests

### 1. Closed-Phase-Status-Guard (CR-01 aus 16-REVIEW.md)

expected: HTTP 409 Conflict — Phase ist geschlossen, kein neuer Entry erlaubt. Alternativ falls Preparation absichtlich erlaubt ist: nur Closed soll 409 geben.
result: failed (user confirmed missing guard is a bug; routed to gap closure)
debug_session: [pending — gap plan will define implementation]

**Test-Setup (HTTP gegen lokalen Server):**

1. `POST /api/repayment-phase` — Phase für FY 2026 anlegen (Preparation)
2. `POST /api/repayment-phase/{phase_id}/open` — Phase öffnen
3. `POST /api/repayment-phase/{phase_id}/close` — Phase schließen
4. Mitglied anlegen mit `current_shares=3`
5. `POST /api/members/{member_id}/partial-repayment` mit `willensbekundung_date="2026-03-15"` (H1 → FY 2026), `shares=1`

**Warum Human:** Service-Impl in `membership_adjust.rs:344-348` filtert beim Phase-Lookup nicht nach Status. CR-01 identifiziert das als Blocker, aber die Entscheidung (Preparation erlaubt, Closed verboten — oder beides verboten?) ist eine Domänen-Frage. Kein automatisierter Test deckt den Closed-Pfad ab.

**Optionen:**
- **A:** Guard einbauen (status != Closed; oder status == Open) + 1 Unit-Test + 1 E2E → wird Phase 16.1 Gap-Closure
- **B:** Verhalten ist absichtlich (Preparation darf vorab befüllt werden) → Test ergänzen, der genau das verifiziert

## Summary

total: 1
passed: 0
issues: 1
pending: 0
skipped: 0
blocked: 0

## Gaps

### 1. CR-01: Closed-Phase-Reuse in partial_repayment

status: failed
source: 16-REVIEW.md CR-01 + 16-HUMAN-UAT.md Test 1
description: `partial_repayment` lookt die Zielphase ohne Status-Filter; eine geschlossene Phase wird wiederverwendet und bekommt einen neuen Entry, was D-11.1 umgeht.
proposed_fix: Status-Guard in `genossi_service_impl/src/membership_adjust.rs:344-348` einbauen — Closed-Phase → ServiceError::Conflict (HTTP 409). Plus 1 Unit-Test (mockall) + 1 E2E-Test gegen Closed-Phase-Reuse.
target_phase: 16.1 (gap closure)
