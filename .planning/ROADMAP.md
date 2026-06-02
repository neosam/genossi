# Roadmap: Genossi

**Project core value:** Genossenschaften verwalten ihre Mitglieder ohne Excel — verbandskonform, nachvollziehbar (Audit-Hashchain), mit weniger manueller Arbeit.

## Milestones

- ✅ **v1.0 GV-Anwesenheits-Erfassung** — Phasen 1-6 (Phase 5 SKIPPED) — shipped 2026-05-29
- ✅ **v1.1 Anteile-Rückzahlungsphase** — Phasen 7-13 — shipped 2026-06-02

Run `/gsd-new-milestone` to plan the next milestone.

## Phases

<details>
<summary>✅ v1.0 GV-Anwesenheits-Erfassung (Phases 1-6, Phase 5 SKIPPED) — SHIPPED 2026-05-29</summary>

- [x] Phase 1: Assembly-Aggregat + Audit-Hardening (5/5 plans) — completed 2026-05-03
- [x] Phase 2: Helfer-Token + Session + AuthContext::Helper (8/8 plans) — completed 2026-05-04
- [x] Phase 3: Attendance-Aggregat + Cascade-Invalidation (6/6 plans) — completed 2026-05-04
- [x] Phase 4: Frontend (Component-First) mit QR-Scanner und Manual-Code-Fallback (11/11 plans) — completed 2026-05-06
- [~] Phase 5: Pre-GV-Generalprobe und Operations-Plan — SKIPPED (echte GV bereits durchgeführt; obsolet)
- [x] Phase 6: Teilnehmerlisten-Export für Generalversammlungen (4/4 plans) — completed 2026-05-17

**Full milestone details:** `milestones/v1.0-ROADMAP.md`
**Archived phases:** `milestones/v1.0-phases/`
**Requirements archive:** `milestones/v1.0-REQUIREMENTS.md`
**Audit:** `milestones/v1.0-MILESTONE-AUDIT.md` (status: tech_debt, 22/22 requirements satisfied)

</details>

<details>
<summary>✅ v1.1 Anteile-Rückzahlungsphase (Phases 7-13) — SHIPPED 2026-06-02</summary>

- [x] Phase 7: RepaymentPhase Backend (Foundation) (5/5 plans) — completed 2026-06-01
- [x] Phase 8: RepaymentEntry + Auto-Befüllung (10/10 plans) — completed 2026-05-31
- [x] Phase 9: Auszahlungs-Buchung (atomisch + auditiert) (5/5 plans) — completed 2026-06-01
- [x] Phase 10: Massenmail-Anbindung + Template-Variablen (8/8 plans) — completed 2026-05-31
- [x] Phase 11: Export (PDF) (6/6 plans) — completed 2026-06-01
- [x] Phase 12: Frontend (Component-First) (15/15 plans) — completed 2026-06-01
- [x] Phase 13: RepaymentLetter-Bulk-Anschreiben für Nicht-Email-Mitglieder (7/7 plans) — completed 2026-06-02

**Full milestone details:** `milestones/v1.1-ROADMAP.md`
**Archived phases:** `milestones/v1.1-phases/`
**Requirements archive:** `milestones/v1.1-REQUIREMENTS.md`
**Audit:** `milestones/v1.1-MILESTONE-AUDIT.md` (status: tech_debt, 33/34 requirements satisfied — UI-06 partial, 7 dokumentierte Tech-Debt-Items)

</details>

### 📋 Next Milestone (TBD)

Run `/gsd-new-milestone` to start planning the next milestone cycle.

## Progress

| Phase                                                           | Milestone | Plans Complete | Status                  | Completed  |
| --------------------------------------------------------------- | --------- | -------------- | ----------------------- | ---------- |
| 1. Assembly-Aggregat + Audit-Hardening                          | v1.0      | 5/5            | Complete                | 2026-05-03 |
| 2. Helfer-Token + Session + AuthContext::Helper                 | v1.0      | 8/8            | Complete                | 2026-05-04 |
| 3. Attendance-Aggregat + Cascade-Invalidation                   | v1.0      | 6/6            | Complete                | 2026-05-04 |
| 4. Frontend (Component-First) + QR + Manual-Code-Fallback       | v1.0      | 11/11          | Complete                | 2026-05-06 |
| 5. Pre-GV-Generalprobe und Operations-Plan                      | v1.0      | 0/0            | SKIPPED (GV produktiv)  | 2026-05-17 |
| 6. Teilnehmerlisten-Export für Generalversammlungen             | v1.0      | 4/4            | Complete                | 2026-05-17 |
| 7. RepaymentPhase Backend (Foundation)                          | v1.1      | 5/5            | Complete                | 2026-06-01 |
| 8. RepaymentEntry + Auto-Befüllung                              | v1.1      | 10/10          | Complete                | 2026-05-31 |
| 9. Auszahlungs-Buchung (atomisch + auditiert)                   | v1.1      | 5/5            | Complete                | 2026-06-01 |
| 10. Massenmail-Anbindung + Template-Variablen                   | v1.1      | 8/8            | Complete                | 2026-05-31 |
| 11. Export (PDF)                                                | v1.1      | 6/6            | Complete                | 2026-06-01 |
| 12. Frontend (Component-First)                                  | v1.1      | 15/15          | Complete                | 2026-06-01 |
| 13. RepaymentLetter-Bulk-Anschreiben für Nicht-Email-Mitglieder | v1.1      | 7/7            | Complete                | 2026-06-02 |

---

*Roadmap created: 2026-05-02*
*Last updated: 2026-06-02 after v1.1 Anteile-Rückzahlungsphase shipped (56/56 plans, 33/34 requirements satisfied — UI-06 partial deferred)*
