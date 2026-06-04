# Features Research — v1.2 Mitgliedschaft-Anpassungen während des Geschäftsjahres

**Domain:** Genossenschaft-Mitgliedschaftspflege im laufenden Geschäftsjahr
**Researched:** 2026-06-04
**Confidence:** HIGH (Anker: Design-Doc, v1.1-Verhalten, Seeds)

## Kontext

v1.1 hat die **Auszahlungsphase** mit Auto-Befüllung beim Phase-Öffnen implementiert: Beim Start eines neuen GJ werden Kündigungen aus dem Vorjahr automatisch übernommen. v1.2 ergänzt die **laufende Pflege während des GJ**: Vorstand löst am Mitglied direkt eine Adjustment-Operation aus, Software erzeugt alle Folge-Datensätze mit korrekter H1/H2-Stichtagsregel.

## Feature-Kategorien (4 Operationen)

### Kategorie 1: Kündigung (Voll-Rückgabe)

**Table Stakes:**
- Vorstand klickt „Mitgliedschaft anpassen" → „Kündigung" auf Member-Detail-Seite
- Datepicker für Willensbekundungs-Datum (default `today()`, eingeschränkt auf offene/s GJ/e)
- System berechnet H1/H2-Stichtag automatisch (H1 → 31.12. aktuelles GJ, H2 → 31.12. folgendes GJ)
- Vorschau-Confirm-Dialog zeigt: Stichtag, Auswirkung (exit_date wird gesetzt), keine sofortige Anteilsreduktion
- Nach Bestätigung: `Member.exit_date` wird gesetzt; KEINE MemberAction, KEIN RepaymentEntry direkt
- v1.1 erledigt den Rest: Auto-Befüllung beim Phase-Open im Ziel-GJ; PaidOut-Toggle erzeugt MemberAction::Verkauf + reduziert current_shares

**Differentiators:**
- Vorschau-Dialog zeigt eine kompakte Timeline: „heute → exit_date → Auszahlung in RepaymentPhase $FY"
- Wenn Vorstand sich selbst kündigt → extra Warn-Modal („Sie sind dabei, sich selbst zu kündigen")

**Research-Anker:**
- `compute_effective_date(date) -> (fy, exit_date)` Pure-Function (siehe ARCHITECTURE.md §3)
- Datepicker-Bounds: aktuelles GJ + nächstes GJ (für H2-Wirksamkeit)

### Kategorie 2: Teil-Rückgabe an Genossenschaft

**Table Stakes:**
- Vorstand wählt „Teil-Rückgabe an Genossenschaft", gibt Anteils-Anzahl `n` (1..current_shares) und Willensbekundungs-Datum ein
- System berechnet H1/H2-Stichtag analog Kündigung
- System erzeugt einen `RepaymentEntry` in der Ziel-Phase (fiscal_year laut H1/H2-Regel) mit `share_count_to_pay_out = n`, Status `Open`
- KEINE MemberAction, KEINE `current_shares`-Reduktion direkt — Mitglied bleibt aktiv, kein `exit_date`
- v1.1 erledigt: PaidOut-Toggle erzeugt MemberAction::Verkauf(n) + reduziert current_shares

**Edge-Cases:**
- Ziel-Phase existiert nicht (H2-Stichtag im folgenden GJ ohne offene Phase) — Verhalten zu klären in Discuss-Phase (Option A/B/C aus PITFALLS Kategorie 2)
- Mehrere Teil-Rückgaben im selben GJ am selben Mitglied — Service-Layer-Sum-Check gegen `current_shares`
- `n > current_shares` — Service-Layer-Validierung (400 BadRequest)

**Differentiators:**
- Vorschau zeigt: „Member behält $remaining Anteile; Auszahlung von $n Anteilen × $share_value = $eur"

### Kategorie 3: Übertragen an aktives Mitglied (Teil oder voll)

**Table Stakes:**
- Vorstand wählt „Übertragen an anderes Mitglied", gibt Anteilszahl `n` und Empfänger-Member ein (Search-Field, filtert auf `exit_date IS NULL AND id != source_id`)
- Übertrag ist sofort wirksam (kein H1/H2-Stichtag, da kein Geldfluss aus der Genossenschaft)
- System erzeugt 2 verlinkte MemberActions in einer Tx:
  - `Übertragung-Aus(A: −n, transfer_member_id=B.id)`
  - `Übertragung-Ein(B: +n, transfer_member_id=A.id)`
- `Member.current_shares` wird atomar aktualisiert (A: −n, B: +n)
- Bei Voll-Übertrag (A's `current_shares` → 0): `exit_date` an A wird auf das Übertrags-Datum gesetzt
- KEIN RepaymentEntry, KEINE Auszahlungs-Logik

**Edge-Cases:**
- Empfänger ist soft-deleted / gekündigt — wird im Search nicht angezeigt (Filter `exit_date IS NULL`)
- Self-Transfer (A=B) — Service-Layer-Guard 400
- Partielle Tx-Fehler (Action 1 ok, Action 2 fail) — gesamte Tx rollback, gemeinsamer `process="member-adjust.transfer"`

**Differentiators:**
- Vorschau-Dialog zeigt: „Member A: $current → $remaining Anteile; Member B: $current → $new Anteile; sofortige Wirksamkeit"
- Beide MemberActions sind über `transfer_member_id` und Process-String im Audit-Log verlinkt → eine Verifikations-Query findet beide

### Kategorie 4: Aufstocken

**Table Stakes:**
- Vorstand wählt „Aufstocken", gibt Anteilszahl `n` und Willensbekundungs-Datum ein
- Aufstockung ist sofort wirksam (kein H1/H2, kein Geldfluss aus Genossenschaft)
- System erzeugt 1 MemberAction `Aufstockung(+n, transfer_member_id=None)` + erhöht `Member.current_shares` atomar
- KEIN RepaymentEntry, KEIN exit_date-Change

**Edge-Cases:**
- Mitglied ist gekündigt (`exit_date IS NOT NULL`) — Service-Layer-Guard verhindert Aufstockung (oder Discuss-Phase-Entscheidung: erlaubt mit Warnung?)
- Member ist soft-deleted — Operation auf soft-deleted Members ist generell blockiert (existing Behavior)

## Querschnitt-Features (alle 4 Operationen)

### Permission
- Admin-only via `check_permission(ADMIN_PRIVILEGE, ...)` (analog v1.1)
- Erweiterte Rollen (Schriftführer, Stellvertreter) ist **Future** (User-Q&A bestätigt: nur Vorstand)

### Workflow
- **One-Click mit Vorschau-Confirm-Dialog** (User-Q&A bestätigt)
- Zwei-Stufen-Workflow (Antrag → Genehmigung) ist Out-of-Scope für v1.2

### Audit
- Alle erzeugten MemberActions/RepaymentEntries/Member-Updates über `audited_create!` / `audited_update!`
- Übertrag erzeugt 2 Actions mit gemeinsamem `process="member-adjust.transfer"`
- Hash-Chain bleibt valid; `/api/audit/verify` muss durchlaufen

### UI
- Single-Button „Mitgliedschaft anpassen" auf Member-Detail-Page (nicht in Mitgliederliste, Audit-Bewusstsein)
- Sub-Choice-Form (4 flat vs. 3 mit Nesting vs. Kündigung-Quickpath) — bewusst offen, Discuss-Phase
- Confirm-Modal-Reuse aus v1.1 Phase 12 (`RepaymentEntry`-PaidOut-Confirm-Pattern)
- Component-First: `MembershipAdjustModal` in `genossi-frontend/src/component/`

## Out of Scope (explizite Boundaries)

| Feature | Reason |
|---------|--------|
| Rückwirkende Erfassung in abgeschlossene GJs | sehr individuell, Vorstand nutzt bestehende manuelle MemberAction-UI |
| Übertrag an Mitgliedsantragsteller mit Auto-Vollmitgliedschaft | koppelt Application+Member+Anteile+Action atomar → zu komplex; eigener Seed `transfer-to-applicant` deferred |
| Storno-Knopf für ausgelöste Kündigungen | bestehende manuelle MemberAction-UI als negative Gegenbuchung |
| Zwei-Stufen-Workflow (Antrag → Genehmigung) | One-Click mit Vorschau-Confirm ist Default; Vier-Augen-Prinzip ist Future |
| MemberAction::Verkauf-Erzeugung durch v1.2 | v1.1's PaidOut-Cascade ist Single-Source-of-Truth; v1.2 erzeugt nur Intent-Datensätze |
| current_shares-Reduktion durch v1.2 bei Kündigung/Teil-Rückgabe | v1.1's PaidOut-Cascade macht das beim Ausbezahlt-Toggle |
| Pessimistische Member-Locks während v1.2-Dialog | optimistic-locking + Re-Read reicht (Tech-Debt für v1.3+) |
| Bulk-Operationen („alle Mitglieder mit X kündigen") | nicht im Use-Case-Scope; einzeln pro Mitglied bewusst |

## Sub-Choice-Form (Discuss-Phase-Item)

Drei Varianten im Design-Doc offen gelassen:

| Variante | Pro | Contra |
|----------|-----|--------|
| **4 flat Buttons** (Kündigung / Teil-Rückgabe / Übertrag / Aufstocken) | Klar, kein Nesting | UI breit, Reduzieren-Operationen unterschiedlich |
| **3 mit Nesting** (Reduzieren {→ Genossenschaft / Mitglied} / Aufstocken / [optional Kündigung]) | Reduzieren-Konzept zusammen | Tieferes Menü, schwieriger Discover |
| **Kündigung-Quickpath** (Großer „Kündigen"-Button + „Andere Anpassung..." Untermenü) | Häufigster Fall einfach | Andere Fälle versteckt |

→ Entscheidung in `/gsd-discuss-phase 14`.

## Sources

- `.planning/notes/membership-adjust-design.md` (Master-Doc mit allen Designentscheidungen)
- `.planning/seeds/membership-adjust-during-fiscal-year.md` + `transfer-to-applicant.md`
- v1.1 Phase 8 RepaymentEntry-Patterns (`genossi_service_impl/src/repayment_entry.rs`)
- v1.1 Phase 9 PaidOut-Cascade-Pattern (`mark_paid_out` 12-Schritt)
- v1.1 Phase 12 RepaymentEntryList-UI als Component-Reuse-Anker
- User-Q&A 2026-06-04: Vorstand-only Permissions; One-Click Workflow; Phase Auto-Anlegen gewünscht

---

*Features research for: Genossi v1.2 Mitgliedschaft-Anpassungen*
*Researched: 2026-06-04*
