# Roadmap: Genossi

**Project core value:** Genossenschaften verwalten ihre Mitglieder ohne Excel — verbandskonform, nachvollziehbar (Audit-Hashchain), mit weniger manueller Arbeit.

## Milestones

- ✅ **v1.0 GV-Anwesenheits-Erfassung** — Phasen 1-6 (Phase 5 SKIPPED) — shipped 2026-05-29
- 📋 **v1.1 Anteile-Rückzahlungsphase** — Phasen 7-12 — in planning (started 2026-05-29)

Use `/gsd-plan-phase 7` to start execution of the first phase.

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

### 📋 v1.1 Anteile-Rückzahlungsphase (Phases 7-12)

**Milestone goal:** Ersetzt die Excel-Liste für Anteils-Auszahlungen — Vorstand verwaltet Rückzahlungsphasen direkt in Genossi, schreibt Mitglieder per Massenmail an und exportiert auszahlbare Beträge als PDF zur Online-Banking-Übernahme.

**Build order:** Backend-First → Service-Logik → Integrationen (Mail, Export) → Frontend. Folgt Genossi-Konvention.

#### Phase 7: RepaymentPhase Backend (Foundation)

**Goal:** RepaymentPhase als auditpflichtiges Aggregat mit Lifecycle `Vorbereitung → Offen → Abgeschlossen` ohne Auto-Befüllung (kommt in Phase 8).

**Requirements:** PHAS-01, PHAS-04, PHAS-05 (vollständig), PHAS-02 + PHAS-03 (Status-Übergänge ohne Auto-Befüllung/Close-Validation — werden in Phase 8 vollständig).

**Success criteria:**
1. Migration legt `repayment_phase`-Tabelle an (BLOB-UUID, `fiscal_year INTEGER NOT NULL`, `share_value INTEGER NOT NULL` in Cent, `status TEXT NOT NULL`, `created`, `deleted`, `version`)
2. DAO + SQLite-Impl + Service-Trait + Impl mit `Auditable`-Implementierung; `audited_create!` und `audited_update!` greifen
3. REST-Handler für create/get/list/update/open/close registriert in OpenAPI (`/api/repayment-phase`)
4. E2E-Test: create → open → close-Lifecycle erfolgreich; Audit-Chain via `/api/audit/verify` bleibt valide
5. `share_value`-Korrektur in `Offen`-Status erzeugt genau einen Audit-Eintrag pro Feld-Änderung; `fiscal_year` ist nach `Offen` read-only

**Plans:** 4/5 plans executed

Plans:
- [x] 07-01-PLAN.md — Migration + DAO-Trait + Entity + Auditable (Wave 1)
- [x] 07-02-PLAN.md — SQLite-DAO-Impl mit Optimistic Locking (Wave 2)
- [x] 07-03-PLAN.md — Service-Trait + Impl mit Edit-Matrix, Validation, Audit-Macros (Wave 3)
- [x] 07-04-PLAN.md — REST-Handler + OpenAPI + TOs + DI-Wiring (Wave 4)
- [x] 07-05-PLAN.md — E2E-Tests: Lifecycle + Audit-Chain + 6 Negative-Paths (Wave 5)

#### Phase 8: RepaymentEntry + Auto-Befüllung

**Goal:** RepaymentEntry-Aggregat mit Auto-Befüllung beim Phase-Öffnen, manueller Ergänzung, und Status-Toggle `offen ↔ angeschrieben` (ohne `ausbezahlt` — kommt in Phase 9).

**Requirements:** ENTR-01, ENTR-02, ENTR-03, ENTR-04, ENTR-05, ENTR-06 + Vervollständigung von PHAS-02 (Auto-Befüllung) und PHAS-03 (Close-Validation gegen pending Entries).

**Success criteria:**
1. Migration legt `repayment_entry`-Tabelle an (kein Composite-PK, eigene UUID; `member_id`, `phase_id`, `share_count_to_pay_out INTEGER`, `status TEXT`, `created`, `deleted`, `version`)
2. Phase-Öffnen (`open_phase`) befüllt atomar Einträge für alle Mitglieder mit `exit_date BETWEEN ? AND ?` (Geschäftsjahres-Range) — `share_count_to_pay_out = Member.current_shares`-Snapshot
3. Manuelles `create_entry` über REST funktioniert; mehrere Einträge pro Mitglied+Phase im selben State verifiziert durch Integration-Test
4. Status-Toggle `offen ↔ angeschrieben` ist multi-select-fähig (Batch-Endpoint); Audit-Eintrag pro Toggle
5. `close_phase` (PHAS-03) blockt mit 409 Conflict wenn mindestens ein Eintrag nicht `ausbezahlt` ODER `deleted IS NULL` ist — E2E-Test deckt Negative-Path

**Plans:** 10/10 plans complete

Plans:
**Wave 1**
- [x] 08-01-PLAN.md — Migration + DAO-Trait + Entity + Auditable (Wave 1)

**Wave 2** *(blocked on Wave 1 completion)*
- [x] 08-02-PLAN.md — SQLite-DAO-Impl mit Optimistic Locking + Pre-Exists-Check (Wave 2)
- [x] 08-03-PLAN.md — Service-Trait + Impl mit Validation, Edit-Matrix, Batch-Tx (Wave 2)

**Wave 3** *(blocked on Wave 2 completion)*
- [x] 08-04-PLAN.md — RepaymentPhase-Service-Erweiterung: Auto-Fill + Pending-Validation (Wave 3)

**Wave 4** *(blocked on Wave 3 completion)*
- [x] 08-05-PLAN.md — REST-Handler + TOs + Router + DI-Wiring (Wave 4)

**Wave 5** *(blocked on Wave 4 completion)*
- [x] 08-06-PLAN.md — E2E-Tests: Auto-Fill + manueller CRUD + Batch + Close-Validation + Audit-Chain (Wave 5)

**Gap-Closure (post-verification 2026-05-31)** — fixt CR-01 (stale version response) + CR-02 (404 vs 409 Batch-Toggle) + IN-04 (E2E-Coverage-Lücke); Quellen: `08-VERIFICATION.md`, `08-REVIEW.md`
- [x] 08-07-PLAN.md — Gap CR-01: Re-Read-Pattern in `RepaymentEntryServiceImpl::update_repayment_entry` + `batch_toggle_status` (Wave 1, parallel zu 08-08)
- [x] 08-08-PLAN.md — Gap CR-01: Re-Read-Pattern in `RepaymentPhaseServiceImpl::create_repayment_phase` + `update_repayment_phase` + `open_repayment_phase` + `close_repayment_phase` (Wave 1, parallel zu 08-07; Phase-7-Erbe)
- [x] 08-09-PLAN.md — Gap CR-02: `batch_toggle_status` NotFound → 404 statt 409 + OpenAPI-Doku + `BatchFailureResponse`-Schema-Klarstellung (Wave 2, depends_on 08-07 — gleiche Datei)
- [x] 08-10-PLAN.md — Gap IN-04: 5 E2E-Regressionstests für CR-01-Folge-PUTs + CR-02-NotFound-Mapping (Wave 3, depends_on 08-07/08/09)

#### Phase 9: Auszahlungs-Buchung (atomisch + auditiert)

**Goal:** `ausbezahlt`-Toggle erzeugt atomar `MemberAction::Verkauf` und reduziert `Member.current_shares`; ist final und audit-konsistent.

**Requirements:** PAYO-01, PAYO-02, PAYO-03, PAYO-04.

**Success criteria:**
1. `mark_paid_out`-Service-Methode führt in einer Transaktion: `audited_create!` für `MemberAction::Verkauf` mit `shares_change=-N` + `audited_update!` für `Member.current_shares -= N` + `RepaymentEntry.status = ausbezahlt`
2. Validation: `current_shares < share_count_to_pay_out` blockt mit `ServiceError::ValidationError` (E2E-Test deckt Negative-Path)
3. Audit-Chain-Verification über `/api/audit/verify` zeigt MemberAction- und RepaymentEntry-Audit-Einträge in gleicher `transaction_id` gegroupt
4. Status `ausbezahlt` ist final — Toggle-Back-Versuch über REST liefert 409 Conflict
5. Race-Test mit `tokio::join!` auf zwei parallele `mark_paid_out`-Calls auf dem gleichen Eintrag: genau einer geht durch, der andere `Conflict`

**Plans:** 4/5 plans executed

Plans:
**Wave 1**
- [x] 09-01-PLAN.md — Backend-Cascade-Foundation: mark_paid_out-Trait + Impl + 6 Unit-Tests + TestMemberActionDao-Mock + compute_migration_status pub

**Wave 2** *(blocked on 09-01 completion)*
- [x] 09-02-PLAN.md — REST-Handler POST /api/repayment-entry/{id}/mark-paid-out + OpenAPI (5 Status-Codes)

**Wave 3** *(blocked on 09-02 completion)*
- [x] 09-03-PLAN.md — DI-Wiring: MemberActionDao an RepaymentEntryServiceImpl in genossi_bin/src/lib.rs

**Wave 4** *(blocked on 09-03 completion)*
- [x] 09-04-PLAN.md — 4 E2E-Tests: Happy-Path + PAYO-03 + PAYO-04 + Race (tokio::join!) + Audit-Chain-Verify

**Wave 5** *(blocked on 09-04 completion; NOT autonomous — human checkpoint)*
- [x] 09-05-PLAN.md — Requirements-Sign-off: PAYO-01..04 in REQUIREMENTS.md als [x] markieren

#### Phase 10: Massenmail-Anbindung + Template-Variablen

**Goal:** Vorstand kann mehrere RepaymentEntries gleichzeitig anschreiben; Mail-Templates haben Zugriff auf Auszahlungs-Wert (`{{ payout_amount }}`, `{{ share_count }}`, `{{ fiscal_year }}`). Wiederverwendet den existierenden `POST /api/mail/send-bulk`-Endpoint (D-01), kein neuer Endpoint.

**Requirements:** MAIL-01, MAIL-02, MAIL-03, MAIL-04.

**Success criteria:**
1. Bulk-Mail-Endpoint `POST /api/mail/send-bulk` akzeptiert optional `template_id` + `repayment_phase_id` im Body (D-01: kein eigener Endpoint, Wiederverwendung des Mitgliederlisten-Mail-Patterns); per OIDC auf Vorstand limitiert
2. minijinja-Template-Engine löst `{{ payout_amount }}` (= `SUM(share_count_to_pay_out) × phase.share_value`, deutsche Lokalisierung mit Komma als Dezimaltrennzeichen z.B. `"60,00"`), `{{ share_count }}` (i32-Aggregat), `{{ fiscal_year }}` (i32) pro Empfänger korrekt auf (Worker aggregiert offene+contacted Entries pro Member; Strict-`{% if %}`-Pattern als Opt-in)
3. Pro versendeter Mail wird ein `MemberDocument` (`document_type=repayment_mail`) mit `template_id`, `mail_recipient_id`, `status=sent|failed` erzeugt via inlined audit-helper im Worker; ein Bulk-Versand an N Empfänger erzeugt N MemberDocuments; Audit-Hashchain bleibt valide
4. SMTP-Fehler bei einzelnem Empfänger → MemberDocument-Status `failed` (description enthält `[FAILED: ...]`-Suffix; max 200 Zeichen; KEINE PII), übrige Empfänger werden weiterhin verarbeitet (kein All-or-Nothing); E2E-Test mit MockSmtp/Stub-SMTP

**Plans:** 8/8 plans complete

Plans:
**Wave 1** *(schema/foundation; parallel)*
- [x] 10.01-mail-job-schema-erweiterung-PLAN.md — Migration + MailJob DAO-Struct + SQLite-Impl (template_id, repayment_phase_id)
- [x] 10.02-member-document-schema-und-document-type-PLAN.md — Migration + MemberDocumentEntity Auditable-Erweiterung (FROZEN-Order) + SQLite-Impl + DocumentType::RepaymentMail

**Wave 2** *(blocked on Wave 1)*
- [x] 10.03-mail-service-create-job-signature-PLAN.md — MailService::create_job + Impl + alle Call-Sites in genossi_mail (Trait Breaking-Change)
- [x] 10.04-rest-bulk-mail-body-erweiterung-PLAN.md — SendBulkMailRequest + UUID-Parsing + 400 BadRequest bei Invalid (depends_on 10.03)
- [x] 10.05-template-repayment-context-helper-PLAN.md — merge_repayment_context + 4 Unit-Tests + deutsche Lokalisierung (NICHT depends_on; parallel zu 10.03/10.04)

**Wave 3** *(blocked on Wave 2; Worker-Integration)*
- [x] 10.06-worker-repayment-context-und-audited-create-PLAN.md — Worker-Signatur (6 neue Deps) + worker_audit.rs (inlined wegen circular dep, see PATTERNS.md "Critical Finding") + Repayment-Aggregation + MemberDocument-Create + Fail-Tolerance

**Wave 4** *(blocked on Wave 3)*
- [x] 10.07-genossi-bin-worker-wiring-PLAN.md — RestStateImpl::start_mail_worker DI-Wiring um 6 neue DAOs erweitern

**Wave 5** *(blocked on Wave 4)*
- [x] 10.08-e2e-bulk-mail-und-audit-chain-PLAN.md — 5 E2E-Tests: SC#1-4 + Audit-Chain + PII-Safety + Ad-hoc-Skip

#### Phase 11: Export (PDF)

**Goal:** Vorstand exportiert Auszahlungsliste als PDF (Online-Banking-Vorlage) für offene **und** geschlossene Phasen.

**Requirements:** EXPO-01, EXPO-02, EXPO-03, EXPO-05.

> **Scope note (D-12):** CSV-Export (EXPO-04) wurde während Discuss-Phase nach v1.2 deferred. Re-Add ist additiv (neue Format-Variante, neuer Free-Function-Renderer, REST-Whitelist um `csv` erweitern). Phase-Slug bleibt `11-export-pdf-csv` (Pfad-Stabilität).

**Success criteria:**
1. Typst-Template `auszahlungsliste.typ` in `DEFAULT_TEMPLATES`; Repeat-Header-Tabelle mit Mitgliedsnummer, Name, IBAN, share_count, Betrag, Verwendungszweck
2. REST-Endpoint `GET /api/repayment-phase/{id}/export/{format}?include=open|all|paid` liefert PDF (Format-Whitelist nur `pdf`; alles andere → 400); Filename-Schema `auszahlung-{fiscal_year}-{include}.pdf`
3. Export-Service hat `0` `audited_*!`-Aufrufe (Grep-Gate im Test); Vorstand-only via OIDC, `Helper`-Auth liefert 403
4. 6+ E2E-Tests decken: PDF-Erfolg (Happy Path), 403 ohne Vorstand-Auth, 400 unbekanntes Format (`csv` blockiert mit 400), jede `?include`-Variante (`open`/`all`/`paid`), 409 bei `RepaymentPhase` in `Vorbereitung`-Status, 404 bei unbekannter `phase_id`, leere IBAN (Member.bank_account NULL) wird als leere Spalte gerendert

**Plans:** 3/6 plans executed

Plans:
**Wave 1** *(parallel — foundation: Template + Service-Trait)*
- [x] 11-01-PLAN.md — Typst-Template `auszahlungsliste.typ` + DEFAULT_TEMPLATES-Eintrag + `PdfGenerator::render_repayment_list` + `RepaymentExportRow`-Struct
- [x] 11-02-PLAN.md — Service-Trait `RepaymentExportService` + Domain-Types `ExportFormat` (nur Pdf, D-12) + `ExportInclude` (Open/All/Paid, Default=Open, D-03) + `RepaymentExport`-Bundle

**Wave 2** *(blocked on Wave 1 completion)*
- [x] 11-03-PLAN.md — `RepaymentExportServiceImpl` mit Permission-Funnel (D-10/D-11), In-Memory-Include-Filter (D-01/D-02), Sort (D-09), Verwendungszweck-Pre-Computing (D-04), Euro-Format-Pre-Computing, Grep-Gate-Test (EXPO-05)

**Wave 3** *(blocked on Wave 2 completion)*
- [ ] 11-04-PLAN.md — REST-Handler + Format-Whitelist (D-12) + Query-Param-Default (D-03) + lokales `map_export_error` (D-11) + OpenAPI + Router-Mount + RestStateDef-Bound-Erweiterung

**Wave 4** *(blocked on Wave 3 completion)*
- [ ] 11-05-PLAN.md — DI-Wiring in `genossi_bin/src/lib.rs::RestStateImpl::new()` (5 Edit-Stellen, Single-Arc-per-Process)

**Wave 5** *(blocked on Wave 4 completion)*
- [ ] 11-06-PLAN.md — 9 E2E-Tests: PDF-Happy-Path (Open+Closed) + Format-Whitelist (csv/xlsx/json/html → 400) + Status-Gate (Preparation → 409) + 404 + Audit-Chain bleibt valide + Include-Filter-3-Sub-Tests + leere IBAN (D-06) + Pitfall #2 Status-Leak-Defense (403 statt 409)

#### Phase 12: Frontend (Component-First)

**Goal:** Vorstand verwaltet RepaymentPhases im Browser; UI ist component-first und konsistent mit bestehendem Vorstand-Layout.

**Requirements:** UI-01, UI-02, UI-03, UI-04, UI-05, UI-06.

**Success criteria:**
1. Page `/repayment-phases` zeigt Liste aller Phasen mit Status + fiscal_year + share_value + Anzahl-Einträge; sortierbar; Create-Modal verfügbar
2. Page `/repayment-phases/{id}` zeigt 3-Tab-Layout (Stammdaten, Einträge, Export); Lifecycle-Aktionen (öffnen/schließen) sichtbar je nach Status
3. `RepaymentEntryList`-Component existiert in `genossi-frontend/src/component/`; nutzt multi-select, Status-Filter, sortierbar nach Mitgliedsnummer/Status; Component-First-Anker (Grep-Gate gegen inline-RSX-Duplikate in `page/`)
4. Manuelles Add-Entry-Modal mit Mitglied-Picker (Substring-Suche auf Name/Mitgliedsnummer)
5. `ausbezahlt`-Toggle hat Confirm-Dialog mit Warnung „irreversibel, audit-pflichtig, reduziert current_shares"; Backend-Validation-Fehler (PAYO-03) wird im Frontend als Toast angezeigt
6. Massenmail-Aktion im Tabellen-Header funktioniert (multi-select → Template-Picker → Versenden); UAT-Checkliste durchgeklickt mit echtem SMTP-Account auf Staging

## Progress

| Phase                                                           | Milestone | Plans Complete | Status                  | Completed  |
| --------------------------------------------------------------- | --------- | -------------- | ----------------------- | ---------- |
| 1. Assembly-Aggregat + Audit-Hardening                          | v1.0      | 5/5            | Complete                | 2026-05-03 |
| 2. Helfer-Token + Session + AuthContext::Helper                 | v1.0      | 8/8            | Complete                | 2026-05-04 |
| 3. Attendance-Aggregat + Cascade-Invalidation                   | v1.0      | 6/6            | Complete                | 2026-05-04 |
| 4. Frontend (Component-First) + QR + Manual-Code-Fallback       | v1.0      | 11/11          | Complete                | 2026-05-06 |
| 5. Pre-GV-Generalprobe und Operations-Plan                      | v1.0      | 0/0            | SKIPPED (GV produktiv)  | 2026-05-17 |
| 6. Teilnehmerlisten-Export für Generalversammlungen             | v1.0      | 4/4            | Complete                | 2026-05-17 |
| 7. RepaymentPhase Backend (Foundation)                          | v1.1      | 4/5 | In Progress|  |
| 8. RepaymentEntry + Auto-Befüllung                              | v1.1      | 10/10 | Complete   | 2026-05-31 |
| 9. Auszahlungs-Buchung (atomisch + auditiert)                   | v1.1      | 4/5 | In Progress|  |
| 10. Massenmail-Anbindung + Template-Variablen                   | v1.1      | 8/8 | Complete    | 2026-05-31 |
| 11. Export (PDF)                                                | v1.1      | 3/6 | In Progress|  |
| 12. Frontend (Component-First)                                  | v1.1      | 0/?            | Pending                 | —          |

---

*Roadmap created: 2026-05-02*
*Last updated: 2026-05-31 after Phase 11 plans created (6 plans across 5 waves)*
