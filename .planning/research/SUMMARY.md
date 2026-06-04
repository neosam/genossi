# Research Summary — v1.2 Mitgliedschaft-Anpassungen während des Geschäftsjahres

**Researched:** 2026-06-04
**Sources:** STACK.md, FEATURES.md, ARCHITECTURE.md, PITFALLS.md
**Confidence:** HIGH

---

## TL;DR

v1.2 fügt 4 Operationen am Mitglied hinzu (Kündigung / Teil-Rückgabe / Übertrag / Aufstocken), die **bewusst KEINE** MemberAction::Verkauf erzeugen und **NICHT** `current_shares` reduzieren, wenn die Genossenschaft später Geld auszahlt. Das macht v1.1's PaidOut-Cascade — kein Doppelbuchen.

Stack: 100% existing — keine neuen Dependencies. Architektur-Anker ist `MemberActionService` (Service-Extension statt neuer Service). Größte Risiken: Doppelbuchung (Auto-Fill + v1.2-Trigger), Auto-Anlegen Ziel-Phase bei H2-Wirksamkeit folgendes GJ, ActionType-Erweiterung-Konsistenz.

---

## Stack Additions

**Keine.** v1.2 nutzt:
- Rust 2021, Tokio 1.35+, Axum 0.8.3, SQLx 0.8 (SQLite), Utoipa 5.0
- Dioxus 0.6.3 + Tailwind im Frontend
- `time` 0.3 für H1/H2-Stichtag-Pure-Function
- Existing Audit-Macros (`audited_create!`, `audited_update!`, `audited_delete!`)
- Existing Permission-Service (`ADMIN_PRIVILEGE`)

Mögliche Migration: ActionType-Enum-Erweiterung in `migrations/sqlite/` falls TEXT-Spalte mit CHECK-Constraint (zu verifizieren in Discuss-Phase).

---

## Feature Table Stakes

| Operation | v1.2 erzeugt | v1.1 macht später |
|-----------|--------------|-------------------|
| **Kündigung** (Voll) | `exit_date` (H1/H2-Stichtag) | Auto-Fill beim Phase-Open + PaidOut-Cascade |
| **Teil-Rückgabe** | `RepaymentEntry` in Ziel-Phase (H1/H2-GJ) | PaidOut-Cascade beim Ausbezahlt-Toggle |
| **Übertragen** (Teil/voll) | 2 verlinkte MemberActions (neuer Typ) + `current_shares` sofort + ggf. `exit_date` | nicht involviert |
| **Aufstocken** | MemberAction (neuer Typ) + `current_shares` sofort | nicht involviert |

Querschnitt: Admin-only Permissions, One-Click mit Vorschau-Confirm, Single-Button auf Member-Detail-Page (nicht in Liste).

---

## Feature Differentiators

- **H1/H2-Stichtagsregel** als unified rule: gilt genau dann, wenn die Genossenschaft Geld auszahlen muss (Kündigung + Teil-Rückgabe). Übertrag und Aufstocken sind sofort wirksam.
- **Übertrag-Pair** mit gemeinsamem `process="member-adjust.transfer"` und `transfer_member_id`-Bidir-Link → ein Audit-Query findet beide.
- **Datepicker-Bounds**: nur aktuelles + nächstes offenes GJ (für H2-Wirksamkeit erforderlich).
- **Component-First-Frontend**: `MembershipAdjustModal` als shared Component; Vorschau-Dialog reusable.
- **Self-Action-Warning**: Vorstand, der sich selbst kündigt, bekommt extra Warn-Modal.

---

## Architecture Highlights

- **Service-Extension** auf `MemberActionService` statt neuer Service (Cohesion, kein neuer Crate-Mount nötig).
- **Pure-Function** `compute_effective_date(willensbekundung) -> (fiscal_year, exit_date)` für H1/H2-Logik — unit-testbar, kein I/O.
- **Übertrag-Atomarität** analog v1.1 PaidOut-Cascade-Pattern (Single-Tx, gemeinsamer Process-String).
- **Auto-Fill-Skip-Pattern** beim `open_repayment_phase` — wenn Member schon Entry hat, Auto-Fill überspringt (Duplikat-Prävention).
- **Neue DAO-Query** `find_by_member_and_phase(member_id, phase_id)` als Foundation für Duplikat-Detection + Sum-Check.
- **Frontend-Integration**: neuer Button auf `genossi-frontend/src/page/member_details.rs`, Modal mit `MemberSearch` (für Übertrag-Empfänger), Datepicker, Vorschau-Sektion.

---

## Watch Out For (Kritische Pitfalls)

### Severity: KRITISCH

1. **Doppelbuchung** Auto-Fill + v1.2-Trigger
   - Risiko: v1.2-Kündigung setzt `exit_date`, v1.1-Auto-Fill picked beim Phase-Open auf → wenn v1.2 zusätzlich Entry erzeugt → Duplikat
   - Mitigation: Service-Layer-Sum-Check + Auto-Fill-Skip-Pattern + neue DAO-Query `find_by_member_and_phase`

### Severity: MITTEL-HOCH

2. **Auto-Anlegen Ziel-Phase** bei H2-Wirksamkeit folgendes GJ
   - Risiko: Teil-Rückgabe braucht RepaymentEntry in Ziel-Phase, die noch nicht existiert
   - Mitigation: Discuss-Phase entscheidet zwischen Auto-Create-in-Vorbereitung (A), Auto-Create-in-Open (B), Explicit-Error (C)

3. **ActionType-Erweiterung** ohne PaidOut-Cascade-Seiteneffekt
   - Risiko: v1.2 nutzt fälschlich `ActionType::Verkauf` → Audit-Story verwirrt
   - Mitigation: Neue Varianten `Übertragung-Aus`/`Übertragung-Ein`/`Aufstockung` mit `validate_action`-Regeln; Grep-Gate „nur eine Verkauf-Stelle in mark_paid_out"

### Severity: MITTEL

4. **Audit-Verlinkung Übertrag** (2 Actions in einer Tx)
   - Mitigation: Single-Tx-Pattern + gemeinsamer `process`-String + E2E-Verifikations-Pattern

5. **H1/H2-Edge-Cases** (Schaltjahr, 30.06./01.07., 31.12.)
   - Mitigation: Pure-Function mit explizitem Grenz-Doc-Kommentar + 6 Edge-Case-Unit-Tests

6. **current_shares-Race** (Optimistic-Lock 409)
   - Mitigation: klare Fehler-Message + Frontend-Re-Read-Pattern

7. **Empfänger-Search** (Soft-Delete + Self-Transfer)
   - Mitigation: neue Service-Methode `list_transfer_recipients(exclude=A.id)` mit Filter `exit_date IS NULL AND id != exclude`

8. **Vorstand-Self-Kündigung**
   - Mitigation: Frontend-Warn-Modal (kein Service-Guard nötig)

### Severity: NIEDRIG

9. SQLITE_BUSY in E2E-Cascade-Tests — Memory-Pool mit `busy_timeout(5000)` analog v1.1 Phase 9
10. `recalc_migrated`-Konsistenz nach v1.2-Operationen — Helper in Service-Code-Konvention

---

## Discuss-Phase-Items (Top 5)

1. **Auto-Anlegen-Phase-Strategie:** A/B/C aus PITFALLS Kategorie 2 fixieren — bestimmt Teil-Rückgabe-Pipeline
2. **H1/H2-Grenze:** im Code-Kommentar explizit fixieren (Monat 1–6 / 7–12); Datepicker-Scope (aktuelles GJ vs. + nächstes)
3. **ActionType-Naming:** Deutsch (Übertragung-Aus/Ein, Aufstockung) oder English (TransferOut/In, Increase)?
4. **Sub-Choice-Form im Dialog:** 4 flat vs. 3 mit Nesting vs. Kündigungs-Quickpath
5. **Migration nötig?** ActionType-Persistenz als TEXT mit CHECK-Constraint vs. INTEGER-Discriminator

---

## Phasen-Vorschlag (für Roadmap)

Grobe Schätzung — Roadmapper-Schritt verfeinert:

| # | Phase | Goal | Voraussichtliche Pläne |
|---|-------|------|------------------------|
| 14 | DAO/Domain-Foundation | ActionType-Erweiterung, `compute_effective_date` Pure-Function, neue DAO-Queries (`find_by_member_and_phase`, `list_transfer_recipients`) | 4–5 |
| 15 | Service-Layer | `MembershipAdjustService` (oder Extension von `MemberActionService`): Kündigung, Teil-Rückgabe, Übertrag, Aufstocken mit allen Audit/Tx-Cascades; Auto-Anlegen-Phase je nach Discuss-Entscheidung | 6–8 |
| 16 | REST + DI-Wiring | 4 neue Endpoints + OpenAPI + DI in `genossi_bin/src/lib.rs` | 3–4 |
| 17 | Frontend Component-First | `MembershipAdjustModal` + Sub-Components (Datepicker, MemberSearch, Vorschau) + Button auf Member-Detail | 5–7 |
| 18 | E2E + Cross-Cutting | E2E-Tests, Audit-Verifikation, SQLITE_BUSY-Pool-Setup, alle Pitfall-Mitigations verifizieren | 3–4 |

Total geschätzt: 5 Phasen, 21–28 Pläne. Roadmapper feilt am exakten Cut.

---

*Synthesis from: STACK.md, FEATURES.md, ARCHITECTURE.md, PITFALLS.md*
*Researched: 2026-06-04*
