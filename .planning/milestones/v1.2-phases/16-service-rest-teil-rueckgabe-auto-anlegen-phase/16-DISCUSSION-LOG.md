# Phase 16: Service+REST: Teil-Rückgabe + Auto-Anlegen-Phase - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-04
**Phase:** 16-service-rest-teil-rueckgabe-auto-anlegen-phase
**Areas discussed:** Auto-Anlegen-Strategie, share_value bei Auto-Anlegen, Sum-Check Lokation, Edge-Cases & Validierung

---

## Auto-Anlegen-Strategie

### Frage 1: Verhalten bei nicht existierender Ziel-Phase

| Option | Description | Selected |
|--------|-------------|----------|
| B: Auto-Create in 'Open' (Recommended) | Phase wird direkt im Status 'Open' angelegt + Auto-Fill-Skip-Pattern verhindert Duplikate. D-11.1-Guard bleibt unangetastet. PITFALLS-Empfehlung. | ✓ |
| A: Auto-Create in 'Vorbereitung' | Phase in 'Vorbereitung'; D-11.1-Guard auf 'Preparation \| Open' aufweichen; Auto-Fill-Dedup beim späteren Phase-Öffnen. | |
| C: Kein Auto-Create, expliziter Fehler | HTTP 400/409 mit Hinweis "Phase für FY YYYY existiert nicht. Bitte zuerst anlegen". | |

**User's choice:** B
**Notes:** D-11.1 unangetastet; Skip-Pattern wird ohnehin für Success Criterion 4 gebraucht.

### Frage 2: Wie wird die Phase technisch angelegt?

| Option | Description | Selected |
|--------|-------------|----------|
| Via existing RepaymentPhaseService::create_repayment_phase (Recommended) | Delegation; Audit-Macros + Validierung + Permission-Check laufen mit. | ✓ |
| Direkter DAO + audited_create! | Eigener Process-String 'member-adjust.partial-repayment.auto-phase'; dupliziert Validierungslogik. | |
| Helper-Free-Function ensure_repayment_phase | Free-Function find_then_create; wiederverwendbar, aber bricht Service-Boundary. | |

**User's choice:** Via existing RepaymentPhaseService::create_repayment_phase
**Notes:** Layered-Architecture-Konformität.

### Frage 3: Auto-Fill-Skip-Lookup-Strategie

| Option | Description | Selected |
|--------|-------------|----------|
| Per-Member im Auto-Fill-Loop (Recommended) | Pro Member find_by_member_and_phase-Call; konsistent mit existing Loop. | ✓ |
| Bulk-Prefetch vor dem Loop | HashSet<member_id> aus find_by_phase_id; O(1)-Lookup; skaliert besser. | |
| Du entscheidest — Planner-Discretion | Beide korrekt. | |

**User's choice:** Per-Member im Loop
**Notes:** GV-typische Genossi-Größe <200 Members; N+1 kein Problem.

### Frage 4: Tx-Scope für Phase-Auto-Create + Entry-Create

| Option | Description | Selected |
|--------|-------------|----------|
| Single Tx — atomar (Recommended) | Beide Operationen teilen Tx; bei Entry-Fail wird Phase mit-rollbacked. | ✓ |
| Zwei separate Tx | Erst Phase committen, dann Entry; inkonsistenter State möglich. | |
| Du entscheidest | Planner wählt nach Tx-Parameter-Verfügbarkeit. | |

**User's choice:** Single Tx atomar
**Notes:** Verhindert verwaiste Phase im Audit-Log.

---

## share_value bei Auto-Anlegen

### Frage 1: Quelle des share_value

| Option | Description | Selected |
|--------|-------------|----------|
| Letzte existierende RepaymentPhase (Recommended) | share_value der jüngsten Phase übernehmen, unabhängig vom Status. | ✓ |
| Member.entry_share_value oder Genossenschafts-Konfig | Aus zentraler Quelle; setzt Existenz voraus. | |
| Hardcoded Default (z.B. 10000 Cent = 100 EUR) | Magic Number im Code. | |

**User's choice:** Letzte existierende RepaymentPhase
**Notes:** Konstant in Praxis; Vorstand kann nachträglich editieren.

### Frage 2: Fallback bei keiner Vorgänger-Phase

| Option | Description | Selected |
|--------|-------------|----------|
| HTTP 409 Conflict (Recommended) | Sauberer Fallback; Edge-Case in Praxis irrelevant. | |
| Hardcoded Default als Fallback | Operation bleibt funktional; Magic Number. | ✓ |
| Du entscheidest | Edge-Case in Praxis irrelevant. | |

**User's choice:** Hardcoded Default als Fallback
**Notes:** Operation soll determiniert klappen, auch im v1.1-frischen System.

### Frage 3: Default-Wert in Cent

| Option | Description | Selected |
|--------|-------------|----------|
| 10000 Cent = 100 EUR (Recommended) | Standardwert vieler Genossenschaften. | ✓ |
| 20000 Cent = 200 EUR | Falls deine Genossenschaft 200 EUR hat. | |
| 50000 Cent = 500 EUR | Falls höherer Wert üblich. | |

**User's choice:** 10000 Cent
**Notes:** Konstante `DEFAULT_SHARE_VALUE_CENT = 10000` in membership_adjust.rs.

---

## Sum-Check Lokation

### Frage 1: Wo wird summiert?

| Option | Description | Selected |
|--------|-------------|----------|
| Service-Layer mit find_by_member_and_phase (Recommended) | DAO liefert Vec; Service summiert in Code. EINE Foundation für Sum-Check + Skip-Pattern. | ✓ |
| Targeted DAO-Query sum_open_shares | SQL-aggregiert; zwei DAO-Methoden nötig. | |
| Du entscheidest | Skaliert beides bei <200 Members. | |

**User's choice:** Service-Layer mit find_by_member_and_phase
**Notes:** Wiederverwendbarer DAO-Query als Foundation für Sum-Check + Auto-Fill-Skip.

### Frage 2: Status-Filter für die Summe

| Option | Description | Selected |
|--------|-------------|----------|
| Nur Status != PaidOut (Recommended) | PaidOut-Entries haben current_shares schon reduziert. | ✓ |
| Nur Status = Open | Strenger; könnte Edge-Cases zwischen Status verfehlen. | |
| Alle Status (auch PaidOut) | Falsch — würde Doppelbuchung blockieren. | |

**User's choice:** Status != PaidOut
**Notes:** PITFALLS-Formulierung exakt.

---

## Edge-Cases & Validierung

### Frage 1: Gekündigtes Mitglied

| Option | Description | Selected |
|--------|-------------|----------|
| Blocken — HTTP 409 (Recommended) | PaidOut-Cascade übernimmt; Konsistent mit UPGD-04. | ✓ |
| Erlauben — als legitimer Geschäftsfall | Zwei Auszahlungs-Stufen möglich. | |
| Du entscheidest | Edge-Case in Praxis nie. | |

**User's choice:** Blocken HTTP 409
**Notes:** Doppelbuchungs-Risiko vermeiden.

### Frage 2: Voll-Rückgabe via partial_repayment

| Option | Description | Selected |
|--------|-------------|----------|
| Erlauben (Recommended) | Eigener Geschäftsfall; Mitglied bleibt im Verband. | |
| Blocken — HTTP 400 mit Hinweis 'Nutze Kündigung' | Klarere Audit-Story; verhindert Verwirrung. | ✓ |
| Du entscheidest | Beides verteidigbar. | |

**User's choice:** Blocken HTTP 400
**Notes:** Voll-Rückgabe = Austritt; nutze cancel_membership.

### Frage 3: Range-Validation für n

| Option | Description | Selected |
|--------|-------------|----------|
| 1 <= n < current_shares (strikt, Recommended) | Konsistent mit Voll-Rückgabe-Block; klare Fehlermeldungen. | ✓ |
| 1 <= n <= current_shares | Erlaubt Voll-Rückgabe; widerspricht voriger Antwort. | |
| Du entscheidest — nur 'n > 0' prüfen | Sum-Check fängt auch ab; explizite Range = klarere Errors. | |

**User's choice:** Strikt 1 <= n < current_shares
**Notes:** Pure-Function `validate_partial_repayment_shares` als Helper.

---

## Claude's Discretion

- `find_by_member_and_phase`-DAO-Methode auf `RepaymentEntryDao` als targeted SQL-Query (nicht Default-Impl + Filter).
- `share_value`-Lookup-Mechanik: `dump_all` + Sort vs. neue `find_latest_by_fiscal_year` (Planner).
- Handler-Datei-Placement: `genossi_rest/src/membership_adjust.rs` (Phase-15-Datei) oder `member.rs` falls > 600 LOC.
- Plan-File-Aufteilung: 4 Plans empfohlen (DAO+Trait, Service-Impl, Auto-Fill-Skip, REST+E2E).
- Response-DTO-Naming: anonymes JSON-Object oder benannter `PartialRepaymentResponseTO`.
- Auto-Anlegen-Reihenfolge im Code: `match`/`if let` vs. Helper-Methode `ensure_repayment_phase`.
- Permission-Doppel-Check (ADMIN_PRIVILEGE durchläuft sowohl `partial_repayment` als auch `create_repayment_phase`) — beide Checks korrekt; nicht umgehen.
- E2E-Test-Liste-Anpassung: Roadmap-Test #5 (Phase-not-existent-without-auto-create) wird durch Variante B obsolet; ersetzt durch zwei Variante-B-Tests.

## Deferred Ideas

- `MembershipAdjustService::transfer_shares` + AUDT-02 — Phase 17.
- Frontend-Modal mit Vorschau (PART-Pendant zu CANC-06) — Phase 18.
- Variante A (Phase in Vorbereitung) — bewusst gegen entschieden; Migration-Pfad dokumentiert in CONTEXT.md `<deferred>`.
- Variante C (Explicit Error) — bewusst gegen entschieden; Migration-Pfad dokumentiert.
- Bulk-Prefetch für Auto-Fill-Skip — bei <200 Members nicht nötig.
- Targeted `sum_open_shares`-DAO-Query — bei Performance-Problem; heute YAGNI.
- `share_value`-Default als ConfigService-Setting — bei Multi-Tenant-Bedarf; heute YAGNI.
- Pessimistic-Lock auf Member während v1.2-Dialog — v2-Architektur (PITFALLS-Kat-6).
