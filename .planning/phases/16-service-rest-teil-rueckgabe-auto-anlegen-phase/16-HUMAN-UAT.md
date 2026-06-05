---
status: partial
phase: 16-service-rest-teil-rueckgabe-auto-anlegen-phase
source: [16-VERIFICATION.md, 16-REVIEW.md]
started: 2026-06-05T10:00:00Z
updated: 2026-06-05T10:00:00Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. Closed-Phase-Status-Guard (CR-01 aus 16-REVIEW.md)

expected: HTTP 409 Conflict — Phase ist geschlossen, kein neuer Entry erlaubt. Alternativ falls Preparation absichtlich erlaubt ist: nur Closed soll 409 geben.
result: [pending]

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
issues: 0
pending: 1
skipped: 0
blocked: 0

## Gaps
