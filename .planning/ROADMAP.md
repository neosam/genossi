# Roadmap: Genossi

Mitgliederverwaltungs-Software für Genossenschaften. Aktiver Stand: drei ausgelieferte Milestones — v1.0 (GV-Anwesenheits-Erfassung), v1.1 (Anteile-Rückzahlungsphase), v1.2 (Mitgliedschaft-Anpassungen während des Geschäftsjahres). Nächstes Milestone steht noch zur Definition; Start via `/gsd-new-milestone`.

## Milestones

- ✅ **v1.0 GV-Anwesenheits-Erfassung** — Phases 1-6 (Phase 5 SKIPPED — echte GV bereits durchgeführt) (shipped 2026-05-29)
- ✅ **v1.1 Anteile-Rückzahlungsphase** — Phases 7-13 (shipped 2026-06-02)
- ✅ **v1.2 Mitgliedschaft-Anpassungen** — Phases 14-18 (shipped 2026-06-07)
- 📋 **v1.3 (TBD)** — keine Phasen definiert; nächster Milestone wird mit `/gsd-new-milestone` gestartet

## Phases

<details>
<summary>✅ v1.0 GV-Anwesenheits-Erfassung (Phases 1-6) — SHIPPED 2026-05-29</summary>

- [x] Phase 1: Assembly-Aggregat + Audit-Hardening — completed
- [x] Phase 2: Helfer-Token + Session + AuthContext::Helper — completed
- [x] Phase 3: Attendance-Aggregat + Cascade-Invalidation — completed
- [x] Phase 4: Frontend (Component-First) mit QR-Scanner und Manual-Code-Fallback — completed
- [—] Phase 5: Pre-GV-Generalprobe — SKIPPED (echte GV bereits durchgeführt; Hotfixes lieferten echte Erkenntnisse zurück)
- [x] Phase 6: Teilnehmerlisten-Export für Generalversammlungen (PDF/CSV/XLSX) — completed

Archive: `.planning/milestones/v1.0-ROADMAP.md` · `v1.0-REQUIREMENTS.md` · `v1.0-MILESTONE-AUDIT.md`

</details>

<details>
<summary>✅ v1.1 Anteile-Rückzahlungsphase (Phases 7-13, 56 plans) — SHIPPED 2026-06-02</summary>

- [x] Phase 7: RepaymentPhase Backend Foundation (Aggregat + Lifecycle + 5 Audit-Prozesse) — completed
- [x] Phase 8: RepaymentEntry + Auto-Befüllung (10 plans) — completed
- [x] Phase 9: Atomare Auszahlungs-Buchung (12-Schritt-Cascade) — completed
- [x] Phase 10: Massenmail + Template-Variablen (`{{ payout_amount }}`, `{{ share_count }}`, `{{ fiscal_year }}`) — completed
- [x] Phase 11: Export (PDF Auszahlungsliste mit 6-Spalten-Tabelle) — completed
- [x] Phase 12: Frontend Component-First (15 plans, 3-Tab-Detail-Page, Shared-Components) — completed
- [x] Phase 13: RepaymentLetter-Bulk-Anschreiben für Nicht-Email-Mitglieder — completed

Archive: `.planning/milestones/v1.1-ROADMAP.md` · `v1.1-REQUIREMENTS.md` · `v1.1-MILESTONE-AUDIT.md`

</details>

<details>
<summary>✅ v1.2 Mitgliedschaft-Anpassungen während des Geschäftsjahres (Phases 14-18, 24 plans) — SHIPPED 2026-06-07</summary>

- [x] Phase 14: DAO/Domain Foundation (4 plans) — `compute_effective_date` Pure-Function + `RepaymentEntryDao::find_by_member_and_phase` + `/transfer-recipients`-Endpoint mit `MemberSlimTO` — completed 2026-06-04
- [x] Phase 15: Service+REST: Kündigung + Aufstockung (4 plans) — `MembershipAdjustService`-Trait + `cancel_membership` + `increase_shares` + `recalc_dates`-Free-Function-Refactor + 11 E2E-Tests — completed 2026-06-04
- [x] Phase 16: Service+REST: Teil-Rückgabe + Auto-Anlegen-Phase (5 plans, inkl. Gap-Closure 16-05) — `partial_repayment` mit 14-Schritt-Pipeline + Closed-Phase-Status-Guard + Auto-Fill-Skip-Pattern — completed 2026-06-05
- [x] Phase 17: Service+REST: Übertrag (4 plans) — `transfer_shares` 15-Schritt-Single-Tx-Cascade + Voll-Übertrag-Austritts-Cascade + 8 E2E + 2 Race-Patterns — completed 2026-06-06
- [x] Phase 18: Frontend Component-First (7 plans in 3 Wellen) — `MembershipAdjustModal` (1078 LOC) + 4 Sub-Views + `FiscalYearDateInput` + ToastVariant Success + Vorstand-UAT-Sign-Off — completed 2026-06-07

Archive: `.planning/milestones/v1.2-ROADMAP.md` · `v1.2-REQUIREMENTS.md` · `v1.2-MILESTONE-AUDIT.md`

</details>

### 📋 v1.3 (Not started — define via `/gsd-new-milestone`)

Kein Inhalt definiert. Wahrscheinlich-Themen für die nächste Iteration aus dokumentiertem Tech-Debt:

- Projektweite Cleanup-Phase: CR-02 Permission-Check-Ordering refactor (alle 4 v1.2-MembershipAdjustService-Methoden + alle 5 v1.1-RepaymentPhaseService-Methoden) — extrahierbar in `gen_auth_admin!`-Helper
- Phase-18-UX-Polish: CR-01 (`date_signal`-leak across Sub-Choice), CR-02 (`Signal::set` im Render-Pfad refactoren), unused i18n-Keys aktivieren oder entfernen
- Mail-Subsystem-Triage: pre-existing failure `test_mail_preview_repayment_no_entries_does_not_default_to_one`
- 16 deferred v1.1-Quick-Tasks reviewen + ggf. closen

## Progress

| Phase                                              | Milestone | Plans Complete | Status      | Completed  |
| -------------------------------------------------- | --------- | -------------- | ----------- | ---------- |
| 1. Assembly-Aggregat                               | v1.0      | -              | Complete    | 2026-05    |
| 2. Helfer-Token + Session                          | v1.0      | -              | Complete    | 2026-05    |
| 3. Attendance-Aggregat                             | v1.0      | -              | Complete    | 2026-05    |
| 4. Frontend Component-First                        | v1.0      | -              | Complete    | 2026-05    |
| 5. Pre-GV-Generalprobe                             | v1.0      | -              | SKIPPED     | -          |
| 6. Teilnehmerlisten-Export                         | v1.0      | -              | Complete    | 2026-05-29 |
| 7. RepaymentPhase Foundation                       | v1.1      | -              | Complete    | 2026-04+   |
| 8. RepaymentEntry + Auto-Befüllung                 | v1.1      | 10/10          | Complete    | 2026-05    |
| 9. Atomare Auszahlungs-Buchung                     | v1.1      | -              | Complete    | 2026-05    |
| 10. Massenmail + Template-Variablen                | v1.1      | -              | Complete    | 2026-05    |
| 11. Export (PDF)                                   | v1.1      | -              | Complete    | 2026-05    |
| 12. Frontend Component-First (v1.1)                | v1.1      | 15/15          | Complete    | 2026-05    |
| 13. RepaymentLetter-Bulk-Anschreiben               | v1.1      | -              | Complete    | 2026-06-02 |
| 14. DAO/Domain Foundation                          | v1.2      | 4/4            | Complete    | 2026-06-04 |
| 15. Service+REST: Kündigung + Aufstockung          | v1.2      | 4/4            | Complete    | 2026-06-04 |
| 16. Service+REST: Teil-Rückgabe + Auto-Anlegen     | v1.2      | 5/5            | Complete    | 2026-06-05 |
| 17. Service+REST: Übertrag (Atomare 2-Action)      | v1.2      | 4/4            | Complete    | 2026-06-06 |
| 18. Frontend Component-First (v1.2)                | v1.2      | 7/7            | Complete    | 2026-06-07 |
| 19. E-Mail-Anhänge anzeigen                        | v1.3 (TBD)| 2/6 | In Progress|  |

### Phase 19: E-Mail-Anhänge anzeigen — Backend-Endpoint zum Abrufen von Anhängen aus eingehenden Mails (mail-parser nutzt async-imap bereits) plus Dioxus-Frontend-UI zum Anzeigen/Herunterladen der Attachments in der Mail-Ansicht.

**Goal:** Eingehende E-Mail-Anhänge im Vorstands-Inbox persistent speichern (10-MB-Cap, Filesystem via DocumentStorage), per Vorstand-only-Endpoint ausliefern (Download + optional Inline-Preview), und im Dioxus-Frontend per Component-First-Liste mit Image-Thumbnail / PDF-Vorschau / Download-Action sichtbar machen — inkl. einmaligem Backfill-Worker für Bestandsmails.
**Requirements**: TBD (v1.3 noch nicht definiert; Scope = CONTEXT.md D-01..D-14 + UI-SPEC)
**Depends on:** Phase 18
**Plans:** 2/6 plans executed

Plans:
- [x] 19-01-PLAN.md — DAO + Migration (InboundMailAttachment-Entity, Trait, SQLite-Impl, Migration `20260608000000`)
- [ ] 19-02-PLAN.md — Service + IMAP (parse_raw_mail-Erweiterung, persist_attachment mit 10-MB-Cap + Rollback, fetch_one_by_uid mit UIDVALIDITY-Check, Poll-Worker-Persistenz)
- [ ] 19-03-PLAN.md — REST Endpoints (DetailTO-Extension, /api/inbox/{mail_id}/attachments/{attachment_id} mit ?disposition=inline, content_disposition_inline-Helper, 5 E2E-Tests)
- [ ] 19-04-PLAN.md — Backfill Worker (run_attachment_backfill für Bestandsmails, einmaliger tokio::spawn, best-effort silent-skip)
- [ ] 19-05-PLAN.md — Frontend Components (InboxAttachmentList + InboxAttachmentListItem, format_size Util, 7 i18n-Keys in De+En)
- [ ] 19-06-PLAN.md — Frontend Page Wiring (MVP-Hinweis löschen, ein Component-Aufruf in inbox_page.rs + manueller Sicht-Check)

---

_Last updated: 2026-06-07 after Phase 19 planning._
