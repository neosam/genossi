# Requirements: Genossi v1.2 Mitgliedschaft-Anpassungen

**Defined:** 2026-06-04
**Core Value:** Genossenschaften verwalten ihre Mitglieder ohne Excel — verbandskonform, nachvollziehbar, mit weniger manueller Arbeit. v1.2 ergänzt die laufende Pflege während des Geschäftsjahres (Kündigung, Teil-Rückgabe, Übertrag, Aufstockung am Mitglied direkt) ohne v1.1's PaidOut-Cascade zu duplizieren.

## v1 Requirements

Requirements für v1.2-Release. Jedes mappt auf genau eine Roadmap-Phase.

### Kündigung (CANC) — Voll-Rückgabe an Genossenschaft

- [x] **CANC-01**: Vorstand kann am Mitglied „Kündigung" auslösen (Single-Button auf Member-Detail-Page)
- [ ] **CANC-02**: System berechnet H1/H2-Stichtag aus Willensbekundungs-Datum (H1 = Monat 1–6 → 31.12. aktuelles GJ; H2 = Monat 7–12 → 31.12. folgendes GJ)
- [x] **CANC-03**: System erzeugt eine `MemberAction::Austritt` mit `date = Willensbekundungs-Datum`, `effective_date = berechneter H1/H2-Stichtag`, `shares_change = 0` (Anteile bleiben bis zur Auszahlung unverändert)
- [x] **CANC-04**: `Member.exit_date` wird via existing `recalc_dates`-Hook automatisch aus der Austritts-Action gesetzt (keine direkte Mutation außerhalb von `recalc_dates`)
- [x] **CANC-05**: System erzeugt KEINE `MemberAction::Verkauf` und KEIN `RepaymentEntry` direkt; `current_shares` bleibt unverändert — v1.1's PaidOut-Cascade übernimmt Anteils-Reduktion und Verkauf-Action beim späteren Ausbezahlt-Toggle (Auto-Befüllung in `open_repayment_phase` picked den Member über `exit_date in fiscal_year` auf)
- [ ] **CANC-06**: Vorschau-Confirm-Dialog zeigt Willensbekundungs-Datum, berechneten Stichtag, prognostizierte Ziel-Auszahlungsphase (fiscal_year) und Wirkungs-Timeline vor dem finalen Commit

### Teil-Rückgabe (PART) — Anteile zurück an die Genossenschaft, Mitgliedschaft bleibt

- [ ] **PART-01**: Vorstand kann am Mitglied „Teil-Rückgabe" auslösen mit Anteils-Anzahl `n` (1..current_shares) und Willensbekundungs-Datum
- [ ] **PART-02**: System berechnet H1/H2-Stichtag (Ziel-fiscal_year) analog Kündigung
- [ ] **PART-03**: System erzeugt `RepaymentEntry` in der Ziel-Phase mit `share_count_to_pay_out = n`, Status `Open`
- [ ] **PART-04**: System validiert `sum(open_entries.share_count for member in target_phase) + n <= member.current_shares` (Sum-Check verhindert Über-Rückgabe)
- [ ] **PART-05**: System legt Ziel-RepaymentPhase automatisch an, falls für das berechnete fiscal_year noch nicht existent (exakte Variante A/B/C aus PITFALLS-Kat-2 wird in `/gsd-discuss-phase` fixiert)
- [ ] **PART-06**: System erzeugt KEINE MemberAction und reduziert NICHT `current_shares` direkt (das macht v1.1's PaidOut-Cascade beim Ausbezahlt-Toggle)

### Übertrag (TRSF) — Anteile an aktives Mitglied (Teil oder voll)

- [ ] **TRSF-01**: Vorstand kann am Mitglied „Übertragen an Mitglied" auslösen mit Empfänger-Mitglied und Anteils-Anzahl `n` (1..source.current_shares)
- [ ] **TRSF-02**: Übertrag ist sofort wirksam (kein H1/H2-Stichtag, da kein Geldfluss aus der Genossenschaft)
- [ ] **TRSF-03**: System erzeugt 2 verlinkte MemberActions atomar in einer Tx: `MemberAction::UebertragungAbgabe(A: shares_change=−n, transfer_member_id=B.id)` + `MemberAction::UebertragungEmpfang(B: shares_change=+n, transfer_member_id=A.id)` (existing ActionType-Varianten)
- [ ] **TRSF-04**: System aktualisiert `Member.current_shares` für A (−n) und B (+n) atomar in derselben Tx
- [ ] **TRSF-05**: Bei Voll-Übertrag (A.current_shares → 0 nach Übertrag) erzeugt System zusätzlich eine `MemberAction::Austritt` für A mit `date = Übertrags-Datum` und `effective_date = Übertrags-Datum`; `Member.exit_date` wird via `recalc_dates` automatisch gesetzt (gleiche Austritt-Konsistenz-Story wie CANC-03)
- [ ] **TRSF-06**: Empfänger-Search liefert nur aktive Mitglieder (`exit_date IS NULL AND id != source_id`) — neuer REST-Endpoint `GET /api/members/transfer-recipients?exclude_self={uuid}`
- [ ] **TRSF-07**: Self-Transfer ist verboten — Service-Layer-Guard liefert HTTP 400 bei `from_member_id == to_member_id`

### Aufstockung (UPGD)

- [ ] **UPGD-01**: Vorstand kann am Mitglied „Aufstocken" auslösen mit Anteils-Anzahl `n` und Willensbekundungs-Datum
- [ ] **UPGD-02**: Aufstockung ist sofort wirksam (kein H1/H2, kein Geldfluss)
- [ ] **UPGD-03**: System erzeugt eine `MemberAction::Aufstockung(shares_change=+n, transfer_member_id=None)` (existing ActionType-Variante) und erhöht `Member.current_shares` um n atomar in einer Tx
- [ ] **UPGD-04**: Aufstockung ist blockiert für gekündigte Mitglieder (`exit_date IS NOT NULL` → HTTP 400)

### UI

- [ ] **UI-01**: Single-Button „Mitgliedschaft anpassen" auf Member-Detail-Page (nicht in Mitgliederliste — Audit-Bewusstsein durch extra Klick)
- [ ] **UI-02**: `MembershipAdjustModal` als shared Component in `genossi-frontend/src/component/`; eine Modal mit Operation-Sub-Choice und vier Sub-Views (Sub-Choice-Form wird in `/gsd-discuss-phase` fixiert)
- [ ] **UI-03**: Datepicker mit GJ-Bounds — erlaubt nur Daten im aktuell offenen GJ und im nächsten GJ (für H2-Wirksamkeit erforderlich)
- [ ] **UI-04**: Vorschau-Section mit konkreter Zahlen-Anzeige vor Commit (z.B. „Member A: 5 → 3 Anteile · Member B: 2 → 4 Anteile" für Übertrag; analog für andere Operationen)

### Audit (AUDT)

- [x] **AUDT-01**: Alle v1.2-Operationen erzeugen Audit-Einträge via `audited_create!`/`audited_update!`-Macros (kein direkter DAO-Write außerhalb der Macros)
- [ ] **AUDT-02**: Übertrag-Pair (Aus + Ein) teilt sich gemeinsamen Process-String `process="member-adjust.transfer"` und kann via `/api/audit/verify` + Process-Filter als zusammenhängender Vorgang gefunden werden

### Permission & Validation (PERM)

- [x] **PERM-01**: Alle 4 Operationen sind admin-only via `check_permission(ADMIN_PRIVILEGE, ...)` (Vorstand)
- [x] **PERM-02**: Server-Layer validiert das Willensbekundungs-Datum: muss im aktuell offenen GJ oder nächsten GJ liegen (zusätzlich zum Datepicker-Frontend-Guard)
- [ ] **PERM-03**: Empfänger beim Übertrag muss aktives Mitglied sein (`exit_date IS NULL`) — Service-Layer-Guard zusätzlich zum Search-Filter (TRSF-06)

## v2 Requirements (Deferred)

Nicht in v1.2-Scope, aber benannt für späteres Aufgreifen:

### Verband / Compliance

- **VRBD-01**: Self-Action-Warn-Modal — Vorstand, der sich selbst kündigt, bekommt extra Warn-Step (im aktuellen Q&A nicht ausgewählt, kann später nachgereicht werden)
- **VRBD-02**: Zwei-Stufen-Workflow (Antrag → Genehmigung) als Konfig-Option für Vier-Augen-Prinzip

### Anteils-Übertrag erweitert

- **TRSF-A**: Übertrag an Mitgliedsantragsteller mit Auto-Vollmitgliedschaft (siehe Seed `.planning/seeds/transfer-to-applicant.md` — bleibt unaktiviert)

### Operations

- **OPS-01**: Bulk-Operationen (z.B. „alle Mitglieder mit Status X kündigen") — bisher kein Use-Case
- **OPS-02**: Storno-Knopf für ausgelöste Kündigungen (im v1.2 über manuelle MemberAction als negative Gegenbuchung)

## Out of Scope

Explizit ausgeschlossen für v1.2:

| Feature | Reason |
|---------|--------|
| Rückwirkende Erfassung in abgeschlossene GJs | sehr individuell; Vorstand nutzt bestehende manuelle MemberAction-UI |
| Übertrag an Mitgliedsantragsteller mit Auto-Vollmitgliedschaft | koppelt Application+Member+Anteile+Action atomar → zu komplex; bleibt als Seed |
| Storno-Knopf für ausgelöste Kündigungen | manuelle MemberAction-UI reicht als negative Gegenbuchung |
| Zwei-Stufen-Workflow (Antrag → Genehmigung) | One-Click mit Vorschau-Confirm ist Default; Vier-Augen ist v2 |
| `MemberAction::Verkauf`-Erzeugung durch v1.2 | v1.1's PaidOut-Cascade ist Single-Source-of-Truth; v1.2 erzeugt nur Intent-Datensätze |
| `current_shares`-Reduktion durch v1.2 bei Kündigung/Teil-Rückgabe | v1.1's PaidOut-Cascade macht das beim Ausbezahlt-Toggle |
| Pessimistische Member-Locks während v1.2-Dialog | optimistic-locking + Re-Read reicht (Tech-Debt v1.3+) |
| Bulk-Operationen | nicht im Use-Case-Scope |

## Traceability

Welche Phasen welche Requirements abdecken. Wird vom Roadmapper-Schritt befüllt.

| Requirement | Phase | Status |
|-------------|-------|--------|
| CANC-01 | Phase 15 | Complete |
| CANC-02 | Phase 14 | Pending |
| CANC-03 | Phase 15 | Complete |
| CANC-04 | Phase 15 | Complete |
| CANC-05 | Phase 15 | Complete |
| CANC-06 | Phase 18 | Pending |
| PART-01 | Phase 16 | Pending |
| PART-02 | Phase 16 | Pending |
| PART-03 | Phase 16 | Pending |
| PART-04 | Phase 16 | Pending |
| PART-05 | Phase 16 | Pending |
| PART-06 | Phase 16 | Pending |
| TRSF-01 | Phase 17 | Pending |
| TRSF-02 | Phase 17 | Pending |
| TRSF-03 | Phase 17 | Pending |
| TRSF-04 | Phase 17 | Pending |
| TRSF-05 | Phase 17 | Pending |
| TRSF-06 | Phase 14 | Pending |
| TRSF-07 | Phase 17 | Pending |
| UPGD-01 | Phase 15 | Pending |
| UPGD-02 | Phase 15 | Pending |
| UPGD-03 | Phase 15 | Pending |
| UPGD-04 | Phase 15 | Pending |
| UI-01 | Phase 18 | Pending |
| UI-02 | Phase 18 | Pending |
| UI-03 | Phase 18 | Pending |
| UI-04 | Phase 18 | Pending |
| AUDT-01 | Phase 15 | Complete |
| AUDT-02 | Phase 17 | Pending |
| PERM-01 | Phase 15 | Complete |
| PERM-02 | Phase 15 | Complete |
| PERM-03 | Phase 17 | Pending |

**Coverage:**
- v1.2 requirements: 31 total
- Mapped to phases: 31 (all)
- Unmapped: 0 ✓

## Cross-Reference

- Master-Design-Doc: `.planning/notes/membership-adjust-design.md`
- Research SUMMARY: `.planning/research/SUMMARY.md`
- Pitfalls: `.planning/research/PITFALLS.md` (10 Pitfall-Kategorien, jede mit Mitigation)
- Architecture: `.planning/research/ARCHITECTURE.md` (Service-Extension, Pure-Function, Übertrag-Atomarität-Pattern)
- Seeds: `.planning/seeds/membership-adjust-during-fiscal-year.md`, `transfer-to-applicant.md` (deferred)

---
*Requirements defined: 2026-06-04*
*Last updated: 2026-06-04 after initial definition*
