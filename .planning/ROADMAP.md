# Roadmap: Genossi v1.2 Mitgliedschaft-Anpassungen

**Created:** 2026-06-04
**Milestone Goal:** Vorstand kann am Mitglied direkt Kündigung, Teil-Rückgabe, Übertrag oder Aufstockung auslösen; v1.2 erzeugt nur Intent-Datensätze; Anteils-Reduktion und Verkauf-Action bleibt v1.1-PaidOut-Cascade-Aufgabe.

**Phasen-Nummerierung:** Continued from v1.1 (Phase 13 → next = 14).

## Overview

5 Phasen, 31 Requirements, Backend-First → Frontend Build-Order.

| # | Phase | Goal | REQs | Plans (est.) |
|---|-------|------|------|--------------|
| 14 | DAO/Domain Foundation | 4/4 | Complete    | 2026-06-04 |
| 15 | Service+REST: Kündigung + Aufstockung | `cancel_membership` erzeugt Austritts-Action; `increase_shares` erzeugt Aufstockungs-Action + current_shares-Update; Admin-Permission + Datum-Validierung | 11 | 5–7 |
| 16 | Service+REST: Teil-Rückgabe + Auto-Anlegen-Phase | `partial_repayment` erzeugt RepaymentEntry in Ziel-Phase mit Sum-Check; Auto-Anlegen-Ziel-Phase (Variante A/B/C aus Discuss); Auto-Fill-Skip-Pattern für Doppelbuchungs-Prävention | 6 | 5–7 |
| 17 | Service+REST: Übertrag | `transfer_shares` mit 2 atomaren verlinkten MemberActions + gemeinsamem Process-String; Voll-Übertrag erzeugt zusätzliche Austritts-Action; Empfänger-aktiv-Guard; Self-Transfer-Block | 8 | 5–7 |
| 18 | Frontend Component-First | `MembershipAdjustModal` shared Component mit Sub-Choice-Form, Datepicker, MemberSearch-Reuse, Vorschau-Section; Button auf Member-Detail-Page; i18n DE/EN | 4 | 5–7 |

**Total:** 31 REQs in 5 Phasen, alle abgedeckt (Coverage: 100%).

---

## Phase 14 — DAO/Domain Foundation

**Goal:** Pure-Function und DAO-Queries als Foundation für alle Service-Operationen. Keine Schreib-Operation in dieser Phase.

**Requirements (2):**
- **CANC-02**: H1/H2-Stichtag-Berechnung (Pure-Function `compute_effective_date`)
- **TRSF-06**: Empfänger-Search Endpoint `GET /api/members/transfer-recipients?exclude_self={uuid}` (DAO + Service + REST)

**Success Criteria:**
1. `compute_effective_date(willensbekundung: Date) -> (fiscal_year: i32, effective_date: Date)` als Pure-Function in `genossi_service_impl` mit mindestens 6 Edge-Case-Unit-Tests (30.06., 01.07., 31.12., 01.01., Schaltjahr-Februar, Jahres-Anfang/-Ende)
2. `RepaymentEntryDao::find_by_member_and_phase(member_id, phase_id, tx) -> Vec<RepaymentEntryEntity>` als neue DAO-Methode mit 2 Unit-Tests (leere Liste + mehrere Entries)
3. `MemberService::list_transfer_recipients(exclude_member_id)` Service-Methode mit Filter `exit_date IS NULL AND id != exclude_member_id` und 3 Unit-Tests
4. REST-Endpoint `GET /api/members/transfer-recipients` mit admin-Permission + 1 E2E-Test (Happy-Path mit 3 Members: 1 gekündigt → ausgefiltert, 1 self → ausgefiltert, 1 aktiv → enthalten)

**Plans:** 4/4 plans complete
- [x] 14-01-PLAN.md — Pure-Function compute_effective_date + EffectiveDate struct + 6 edge-case tests (CANC-02)
- [x] 14-02-PLAN.md — RepaymentEntryDao::find_by_member_and_phase trait method + SQLite SQL-override + 3 tests (TRSF-06 foundation)
- [x] 14-03-PLAN.md — MemberService::list_transfer_recipients service method + 3 mockall unit tests (TRSF-06 service)
- [x] 14-04-PLAN.md — MemberSlimTO + REST handler + sub-route registration + E2E test (TRSF-06 REST+E2E)

---

## Phase 15 — Service+REST: Kündigung + Aufstockung

**Goal:** Single-Action-Operationen (eine MemberAction pro Vorgang) implementieren. Foundation für Permission-Funnel und Datum-Validierung.

**Requirements (11):**
- **CANC-01, CANC-03, CANC-04, CANC-05**: Kündigung erzeugt `MemberAction::Austritt`; `recalc_dates` setzt `exit_date`; keine Verkauf-Action/RepaymentEntry direkt
- **UPGD-01, UPGD-02, UPGD-03, UPGD-04**: Aufstockung erzeugt `MemberAction::Aufstockung` + erhöht `current_shares` atomar; blockt gekündigte Member
- **PERM-01**: Admin-only via `ADMIN_PRIVILEGE` (etabliert in dieser Phase)
- **PERM-02**: Server-Layer-Datum-Validierung (offenes GJ + nächstes GJ)
- **AUDT-01**: Alle Operationen via `audited_create!`/`audited_update!`-Macros (Convention etabliert)

**Success Criteria:**
1. `MembershipAdjustService::cancel_membership(member_id, willensbekundung_date, context)` erzeugt `MemberAction::Austritt` mit `effective_date = compute_effective_date(...).exit_date`, `shares_change = 0`; `recalc_dates`-Hook setzt `Member.exit_date` automatisch; 5 E2E-Tests (Happy-Path H1, Happy-Path H2, Permission-Denied 401, Already-Cancelled 409, Audit-Chain-Verify)
2. `MembershipAdjustService::increase_shares(member_id, n, willensbekundung_date, context)` erzeugt `MemberAction::Aufstockung` (`shares_change = +n`, `transfer_member_id = None`) + erhöht `Member.current_shares` atomar in einer Tx; 4 E2E-Tests (Happy-Path, Cancelled-Member-Block 400, Permission-Denied, Audit-Chain-Verify)
3. Server-Layer-Datum-Validierung lehnt Daten ausserhalb des offenen GJ + nächsten GJ ab (HTTP 400); 2 Edge-Case-Tests (Datum im vorletzten GJ, Datum im übernächsten GJ)
4. `cargo test --test e2e_tests` und v1.1-Audit-Hashchain (`/api/audit/verify`) bleiben grün

**Plans:** 4 plans
- [ ] 15-01-PLAN.md — MembershipAdjustService trait + validate_willensbekundung_date pure function + recalc_dates free-function refactor (PERM-02)
- [ ] 15-02-PLAN.md — cancel_membership impl + 4 service unit tests (CANC-01..05, PERM-01, AUDT-01)
- [ ] 15-03-PLAN.md — increase_shares impl + 4 service unit tests (UPGD-01..04, PERM-01, AUDT-01)
- [ ] 15-04-PLAN.md — REST endpoints + DI wiring + 11 E2E tests (full stack + audit-chain-verify)

---

## Phase 16 — Service+REST: Teil-Rückgabe + Auto-Anlegen-Phase

**Goal:** Multi-Datensatz-Operation (RepaymentEntry-Insert + ggf. RepaymentPhase-Auto-Create). Auto-Fill-Skip-Pattern als Doppelbuchungs-Prävention.

**Requirements (6):**
- **PART-01..06**: Teil-Rückgabe-Operation komplett

**Success Criteria:**
1. `MembershipAdjustService::partial_repayment(member_id, n, willensbekundung_date, context)` erzeugt `RepaymentEntry` in der via `compute_effective_date(...).fiscal_year` berechneten Ziel-Phase
2. Service-Layer-Sum-Check: `sum(open_entries.share_count for (member_id, target_phase_id)) + n <= member.current_shares` validiert vor Insert (HTTP 400 bei Verletzung)
3. Auto-Anlegen-Ziel-Phase: wenn für berechnetes `fiscal_year` keine Phase existiert, wird sie via gewählter Variante (A/B/C aus `/gsd-discuss-phase 16`) angelegt mit `share_value` aus Vorgänger-Phase oder Defaults
4. Auto-Fill-Skip-Pattern in `open_repayment_phase` (Erweiterung der existing Logik in `genossi_service_impl/src/repayment_phase.rs:319–395`): wenn `find_by_member_and_phase(member, phase) -> non-empty`, überspringt Auto-Fill den Member (Pitfall-Kategorie-1-Mitigation)
5. 6 E2E-Tests: Happy-Path H1, Happy-Path H2 mit Auto-Anlegen-Phase, Sum-Check-Block 400, Auto-Fill-Skip-Test (Phase-Open nach v1.2-Teilrückgabe erzeugt kein Duplikat), Phase-not-existent-without-auto-create-Fallback, Audit-Chain-Verify

---

## Phase 17 — Service+REST: Übertrag (Atomare 2-Action-Cascade)

**Goal:** Komplexeste Operation. 2 verlinkte MemberActions in einer Tx, Voll-Übertrag mit zusätzlicher Austritts-Action.

**Requirements (8):**
- **TRSF-01..05, TRSF-07**: Übertrag-Logik (inkl. Voll-Übertrag-Austritt + Self-Transfer-Block)
- **AUDT-02**: Gemeinsamer Process-String für Übertrag-Pair
- **PERM-03**: Empfänger-aktives-Mitglied-Guard auf Service-Layer

**Success Criteria:**
1. `MembershipAdjustService::transfer_shares(from_id, to_id, n, transfer_date, context)` erzeugt 2 verlinkte MemberActions atomar in einer Tx mit Pattern aus v1.1 Phase-9-PaidOut-Cascade (`mark_paid_out`-Single-Tx, gemeinsamer Process-String)
2. `process="member-adjust.transfer"` ist gemeinsam für beide Actions; `/api/audit/verify` + Process-Filter findet exakt 2 Einträge pro Übertrag
3. Bei Voll-Übertrag (A.current_shares − n == 0): zusätzlich `MemberAction::Austritt` für A mit `effective_date = transfer_date` in derselben Tx; `recalc_dates`-Hook setzt `Member.exit_date`
4. Self-Transfer-Block (HTTP 400 wenn `from_id == to_id`); Empfänger-aktiv-Guard (HTTP 400 wenn `to.exit_date IS NOT NULL`)
5. 8 E2E-Tests: Teil-Übertrag Happy-Path, Voll-Übertrag mit exit_date-Cascade, Self-Transfer 400, Empfänger-gekündigt 400, Empfänger-soft-deleted 404, Audit-Pair-Verlinkung-Verify, SQLITE_BUSY-Race (akzeptiert `[200, 409|500]`), Multi-Endpoint-Audit-Verify

---

## Phase 18 — Frontend Component-First

**Goal:** `MembershipAdjustModal` als shared Component; Integration auf Member-Detail-Page.

**Requirements (4):**
- **UI-01, UI-02, UI-03, UI-04, CANC-06**: Modal + Datepicker + Vorschau + Button

> Hinweis: CANC-06 ist hier gemappt (Vorschau-Confirm-Dialog für Kündigung als Spezialfall von UI-04).

**Success Criteria:**
1. `MembershipAdjustModal` als shared Component in `genossi-frontend/src/component/membership_adjust_modal.rs` mit Sub-Choice-Form (Variante aus `/gsd-discuss-phase 18` — 4 flat vs. 3 mit Nesting vs. Kündigung-Quickpath) und vier Operation-Sub-Views (Kündigung, Teil-Rückgabe, Übertrag, Aufstockung)
2. Datepicker-Component mit GJ-Bounds (default `today()`, erlaubt nur Daten im aktuell offenen GJ und im nächsten GJ); MemberSearch-Reuse aus v1.1 Phase 12 für Übertrag-Empfänger
3. Vorschau-Section in jeder Sub-View zeigt konkrete Zahlen vor Commit:
   - Kündigung: „Member: X Anteile → Stichtag: 31.12.YYYY → Auszahlung in Phase FYYYYY"
   - Teil-Rückgabe: „Member: X → X-n Anteile (nach Auszahlung)"
   - Übertrag: „Member A: X → X-n · Member B: Y → Y+n"
   - Aufstockung: „Member: X → X+n Anteile"
4. Button „Mitgliedschaft anpassen" auf Member-Detail-Page (`genossi-frontend/src/page/member_details.rs`), Admin-only via `RequirePrivilege`; i18n DE/EN mit mindestens 20 neuen Keys; ManualUAT-Sektion mit Browser-Test-Anleitung

---

## Constraints (per Phase)

- **Phase-14-19 alle:** v1.1-Audit-Hashchain bleibt valid (`/api/audit/verify`); cargo workspace builds clean; v1.1-Tests bleiben grün
- **Phase 15+16+17:** alle Schreib-Operationen via `audited_*!`-Macros (Grep-Gate: 0 direkte DAO-`create`/`update`-Calls außerhalb der Macros in v1.2-Code)
- **Phase 17 spezifisch:** Single-Tx-Pattern mit shared `tx.clone()`; gemeinsamer Process-String; Pre-Tx-Rollback-Test
- **Phase 18:** Component-First — keine inline-RSX; alle Operationen-UI in `MembershipAdjustModal`

## Discuss-Phase-Decisions (offen)

Folgende Entscheidungen müssen pro Phase im `/gsd-discuss-phase` geklärt werden, bevor `/gsd-plan-phase`:

- **Phase 15:** ActionType-Persistenz (TEXT vs. INTEGER) verifizieren; Datum-Bounds-Implementierung (Service vs. existing Validator)
- **Phase 16:** Auto-Anlegen-Strategie Variante A vs. B vs. C aus PITFALLS-Kategorie-2; Sum-Check-Service vs. DAO-Query
- **Phase 17:** Voll-Übertrag-Detection-Logik (sofortiger Service-Check vs. `recalc_dates`-Trigger); Race-Pattern-Test analog v1.1 Phase 9
- **Phase 18:** Sub-Choice-Form aus FEATURES.md (4 flat vs. 3 mit Nesting vs. Kündigung-Quickpath); Component-Reuse mit Phase-12-Pattern

## Cross-Reference

- Master-Design-Doc: `.planning/notes/membership-adjust-design.md`
- Requirements: `.planning/REQUIREMENTS.md` (31 REQs, 7 Kategorien)
- Research: `.planning/research/SUMMARY.md`, `STACK.md`, `FEATURES.md`, `ARCHITECTURE.md`, `PITFALLS.md`

---
*Roadmap created: 2026-06-04*
*Numbering: continued from v1.1 (last phase: 13)*
