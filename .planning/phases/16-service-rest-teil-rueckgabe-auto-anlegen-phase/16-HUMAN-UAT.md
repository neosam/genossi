---
status: resolved
phase: 16-service-rest-teil-rueckgabe-auto-anlegen-phase
source: [16-VERIFICATION.md, 16-REVIEW.md]
started: 2026-06-05T10:00:00Z
updated: 2026-06-05T13:40:00Z
decision: gap-closure (Option A — Status-Guard einbauen via Plan 16-05)
resolved_by: plan 16-05 (commits 87f97841, 5b334cc9, 4ec92404)
---

## Current Test

[resolved via Plan 16-05 — Closed-Phase-Status-Guard implementiert + 1 Unit-Test + 1 E2E-Test]

## Tests

### 1. Closed-Phase-Status-Guard (CR-01 aus 16-REVIEW.md)

expected: HTTP 409 Conflict — Phase ist geschlossen, kein neuer Entry erlaubt. Alternativ falls Preparation absichtlich erlaubt ist: nur Closed soll 409 geben.
result: resolved (Plan 16-05 — Closed → 409 mit Body 'closed' + fiscal_year; Preparation/Open passieren)
debug_session: [n/a — direct gap-closure implementation]

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
passed: 1
issues: 0
pending: 0
skipped: 0
blocked: 0

## Gaps

### 1. CR-01: Closed-Phase-Reuse in partial_repayment

status: resolved
source: 16-REVIEW.md CR-01 + 16-HUMAN-UAT.md Test 1
description: `partial_repayment` lookt die Zielphase ohne Status-Filter; eine geschlossene Phase wird wiederverwendet und bekommt einen neuen Entry, was D-11.1 umgeht.
proposed_fix: Status-Guard in `genossi_service_impl/src/membership_adjust.rs:344-348` einbauen — Closed-Phase → ServiceError::Conflict (HTTP 409). Plus 1 Unit-Test (mockall) + 1 E2E-Test gegen Closed-Phase-Reuse.
target_phase: 16 (Plan 16-05 gap-closure innerhalb Major-Phase)
resolved_by: Plan 16-05 (Commits 87f97841 feat + 5b334cc9 unit-test + 4ec92404 e2e-test); SUMMARY: `16-05-SUMMARY.md`
