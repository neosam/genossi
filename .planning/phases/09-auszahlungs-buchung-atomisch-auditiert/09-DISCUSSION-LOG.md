# Phase 9: Auszahlungs-Buchung (atomisch + auditiert) - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-31
**Phase:** 9-auszahlungs-buchung-atomisch-auditiert
**Areas discussed:** Audit-Gruppierung (SC #3), REST-Endpoint-Schema, MemberAction-Auto-Felder, Cascade-Mechanik & action_count

---

## Audit-Gruppierung (SC #3)

### Q1: Wie soll SC #3 'gleiche transaction_id' realisiert werden?

| Option | Description | Selected |
|--------|-------------|----------|
| Macro-Erweiterung | Neue `audited_*_with_tx_id!`-Variante mit gemeinsamer UUID; AuditQueryFilter.transaction_id; neuer Sub-Endpoint. | |
| Phase-8-D-03-Pragmatik | process + Timestamp + sequentielle Hash-Chain. Kein Macro-Refactor. | ✓ |
| Du entscheidest | Claude wählt. | |

**User's choice:** Phase-8-D-03-Pragma übernehmen (nach Klarstellungsdialog).

**Notes:** Claude hatte ursprünglich Macro-Erweiterung empfohlen, weil ROADMAP-SC #3 wörtlich „gleiche transaction_id" sagt. User hat berechtigt nachgefragt: „Warum brauchen wir eigentlich so komplexe Audit-Änderungen?" — Claude hat zurückgerudert mit dem Argument: (1) Hash-Chain + same-Tx-Commit beweisen ohnehin Atomarität, (2) Phase 8 D-03 hat das Pattern schon etabliert, (3) kein konkreter Audit-Query-Use-Case fordert UUID-Gleichheit, (4) Macro-Erweiterung bleibt als deferred Idee für später. User hat dann das Phase-8-Pragma bestätigt.

---

## REST-Endpoint-Schema

### Q2: Batch-Variante in Phase 9 oder single-only?

| Option | Description | Selected |
|--------|-------------|----------|
| Single-only — Batch in Phase 12 oder nie | UI-05 Confirm-Dialog pro Eintrag; Cascade sicherheitskritisch; Pattern-konsistent mit Phase 8 D-07. | ✓ |
| Batch in Phase 9 mit All-or-Nothing-Semantik | Analog Phase 8 D-08, alle Cascades in EINER Tx, erster Fehler rollt zurück. | |
| Du entscheidest | Claude wählt. | |

**User's choice:** Single-only — Batch in Phase 12 oder nie.

**Notes:** UI-05 ist explizit pro-Eintrag konzipiert, Cascade ist irreversibel, Bulk-Aktion würde Fehler vervielfachen. Body-Form (kein Body) folgt aus Q3.

---

## MemberAction-Auto-Felder

### Q3: Sollen MemberAction.date und MemberAction.comment vom Vorstand setzbar sein?

| Option | Description | Selected |
|--------|-------------|----------|
| Vollständig automatisch | date=today, comment='Anteils-Rückzahlung Phase {fiscal_year}'. Endpoint ohne Body. | ✓ |
| date auto, comment optional | Body `{ comment? }`. | |
| Beide optional setzbar | Body `{ date?, comment? }` für Backdating/Korrekturen. | |
| Du entscheidest | Claude wählt. | |

**User's choice:** Vollständig automatisch.

**Notes:** Pragmatischer Standard. mark_paid_out korrespondiert atomar mit dem Bank-Transfer, der heute vom Vorstand ausgelöst wird. Backdating bleibt deferred falls real World es fordert.

---

## Cascade-Mechanik & action_count

### Q4: Phase.status == Open als Pre-Condition (Defense-in-Depth)?

| Option | Description | Selected |
|--------|-------------|----------|
| Ja — Phase.status == Open ist Pre-Condition | Defense-in-Depth, kostet 1 find_by_id, schützt vor DB-Korruption. | ✓ |
| Nein — Entry-Status-Guard reicht | Spart Call, verlässt sich auf Phase-8-Garantien. | |
| Du entscheidest | Claude wählt. | |

**User's choice:** Ja — Defense-in-Depth.

### Q5: recalc_migrated nach Cascade aufrufen oder vertrauen?

| Option | Description | Selected |
|--------|-------------|----------|
| Vertrauen — keine recalc-Call | Konsistenz-by-Construction (current_shares-=N, action_count+=1, Verkauf-Action mit -N). | |
| Explizit recalc_migrated | Pattern-Konsistenz mit MemberActionService::create + MemberService::update. | |
| Du entscheidest | Claude wählt. | ✓ |

**User's choice:** Du entscheidest → Claude wählt expliziten recalc_migrated.

**Notes:** Begründung: (1) Pattern-Konsistenz mit MemberActionServiceImpl::create und MemberServiceImpl::update (beide rufen recalc IMMER nach audited write), (2) compute_migration_status hat eine subtile `expected_action_count = member.action_count + 1`-Off-by-one-Konvention (semantisch unklar), Konsistenz-by-Construction wäre fragil, (3) Defense-in-Depth kostet wenig (<1ms), (4) falls spätere Phase die Cascade-Reihenfolge anpasst, hält recalc die Invariante automatisch. Implementations-Detail (`compute_migration_status` pub-machen vs Trait-Methode vs Duplikation) bleibt Planner-Discretion.

---

## Claude's Discretion

- **Audit-Gruppierung Q1** — initial empfahl Claude Macro-Erweiterung, korrigierte zu Phase-8-Pragma nach User-Pushback.
- **recalc_migrated Q5** — Claude wählte expliziten Call.
- **Implementations-Details aus D-10** — pub vs Trait-Methode vs Duplikation für `compute_migration_status` ist explizit Planner-Discretion.
- **Reihenfolge der drei `audited_*!`-Calls (D-09)** — Empfehlung MemberAction → Member → Entry; Planner darf abweichen.
- **Race-Defense-Pfad (D-11)** — Researcher verifiziert, ob SQLITE_BUSY-Konkurrenz reicht oder ob ein UPDATE...RETURNING-Pattern (Phase 2 HLPR-04) wiederverwendet werden soll.
- **OpenAPI-Beispielwerte** — Planner wählt realistische Defaults.

## Deferred Ideas

- Batch-mark_paid_out-Endpoint — Phase 12 oder später, falls UI-Flow Bulk-Bestätigung verlangt
- Vorstand-Input für MemberAction.comment / date — Body `{ comment?, date? }` nachziehbar
- Audit-Macro-Erweiterung `audited_*_with_tx_id!` für gemeinsame transaction_id-UUID — additive Änderung wenn echter Audit-Query-Use-Case auftaucht
- AuditQueryFilter.transaction_id + REST-Endpoint `GET /api/audit/transaction/{tx_id}`
- Auto-Close der Phase wenn letzter Entry PaidOut wird — bewusst NICHT (Vorstand schließt manuell)
