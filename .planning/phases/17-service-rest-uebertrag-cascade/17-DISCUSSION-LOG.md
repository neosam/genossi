# Phase 17: service-rest-uebertrag-cascade - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in `17-CONTEXT.md` — this log preserves the alternatives considered.

**Date:** 2026-06-06
**Phase:** 17-service-rest-uebertrag-cascade
**Areas discussed:** Voll-Übertrag-Detection, Process-String + AUDT-02-Test, Race-Pattern-Test, PERM-03 + Self-Transfer + Validation

---

## Voll-Übertrag-Detection

### Frage 1: Wo und wann erkennt der Service current_shares==0?

| Option | Description | Selected |
|--------|-------------|----------|
| Pre-write Service-Check | Service rechnet vor den Writes `will_become_zero = (from.current_shares - n == 0)`; deterministisch, Mock-Tests verifizieren Austritt-Aufruf direkt. Vorbild Phase 9. | ✓ |
| Post-write Re-Read | Pair + Updates schreiben, dann from neu laden, dann prüfen — nutzt echten DB-State, kostet einen Read. | |
| Du entscheidest | Claude wählt. | |

**User's choice:** Pre-write Service-Check
**Notes:** Bestätigt den Phase-9-PaidOut-Cascade-Vorbild. → D-17-01.

### Frage 2: `recalc_dates`-Aufrufe für welche Members?

| Option | Description | Selected |
|--------|-------------|----------|
| Nur A, am Ende vor commit | Nur A könnte exit_date ändern, B bleibt aktiv; spart einen DAO-Read auf B. | ✓ |
| Beide A und B, am Ende | Defensive, ruft `recalc_dates` für beide auf. | |
| Nur A, NUR wenn Voll-Übertrag | Skip recalc_dates komplett bei Teil-Übertrag. | |

**User's choice:** Nur A, am Ende vor commit
**Notes:** Symmetrisch zu Phase 15 D-15-04. → D-17-02.

### Frage 3: `Austritt(A).transfer_member_id`?

| Option | Description | Selected |
|--------|-------------|----------|
| Some(B.id) | Verlinkt Austritt mit Cascade; Audit-Story zeigt drei verlinkte Einträge inkl. eindeutiger Zuordnung "ausgetreten durch Voll-Übertrag". | ✓ |
| None (analog Phase 15) | Standalone Austritt wie CANC; Verlinkung nur über shared Process-String + tx_id. | |
| Du entscheidest | Claude wählt. | |

**User's choice:** Some(B.id)
**Notes:** Bewusste Divergenz von Phase 15 CANC-Austritt. → D-17-03.

---

## Process-String + AUDT-02-Test

### Frage 1: Process-String für 3. Action (Voll-Übertrag-Austritt)?

| Option | Description | Selected |
|--------|-------------|----------|
| Shared `member-adjust.transfer` | Alle 3 Actions + 2 Member-Updates teilen denselben Process-String; eine Audit-Story = eine User-Aktion. | ✓ |
| Distinct `member-adjust.cancel` | Austritt mit Phase-15-Process-String, Pair separat. | |
| Neuer `member-adjust.transfer-full` | Eigene Variante für Voll-Übertrag. | |

**User's choice:** Shared `member-adjust.transfer`
**Notes:** AUDT-02-Pair-Spec wird bewusst auf Triple erweitert. → D-17-04.

### Frage 2: Wie testet AUDT-02-Verifikation?

| Option | Description | Selected |
|--------|-------------|----------|
| Count distinct MemberAction-Rows pro tx | `COUNT(*) WHERE process='member-adjust.transfer' AND entity_type='MemberAction' AND transaction_id=?` muss 2/3 sein. | |
| Count distinct transaction_ids pro Pair | Eine einzige transaction_id pro Vorgang prüfen. | |
| Beides — Doppel-Assertion | (a) eine transaction_id (Atomarität) + (b) exakte Action-Count (Cascade-Vollständigkeit). | ✓ |

**User's choice:** Beides — Doppel-Assertion
**Notes:** Defensive; Helper-Funktion für Wiederverwendung. → D-17-05.

---

## Race-Pattern-Test

### Frage 1: Welches Race-Szenario?

| Option | Description | Selected |
|--------|-------------|----------|
| Same-Direction-Parallel | 2x identischer POST via tokio::join!, analog Phase 9. | |
| Cross-Direction-Parallel | A→B simultan B→A; Deadlock-Probe. | |
| Beides — zwei separate Tests | Test A (Same-Direction-Phase-9-Klon) + Test B (Cross-Direction-Deadlock-Probe). | ✓ |
| Du entscheidest | Claude wählt. | |

**User's choice:** Beides — zwei separate Tests
**Notes:** 2 von 8 SC-#5 E2E-Tests werden Race-Tests. → D-17-06.

### Frage 2: Cross-Direction-Accept-Set?

| Option | Description | Selected |
|--------|-------------|----------|
| [200, 200] erlaubt + Konsistenz-Check | Akzeptiert `[(200,200), (200, 409|500)]`, aber nicht `[409|500, 409|500]`; Post-Check Summe + Audit-valid. | ✓ |
| Strikt mindestens ein Gewinner | Alles außer `[409|500, 409|500]`. | |
| Nur [200, 409|500] | Strikt analog Same-Direction. | |

**User's choice:** [200, 200] erlaubt + Konsistenz-Check
**Notes:** SQLite kann orthogonale Member-Locks unter Umständen serialisieren. → D-17-06.

---

## PERM-03 + Self-Transfer + Validation

### Frage 1: Empfänger gekündigt (PERM-03) → HTTP-Status?

| Option | Description | Selected |
|--------|-------------|----------|
| 409 Conflict | Analog Phase 15 D-15-12 Already-Cancelled-Pattern; Resource-State-Conflict. | ✓ |
| 400 BadRequest | ROADMAP-SC #4 wortwörtlich; konsistent mit Self-Transfer 400. | |
| 404 NotFound | "Nicht (mehr) auffindbar als aktives Mitglied". | |

**User's choice:** 409 Conflict
**Notes:** Bewusste Divergenz von Roadmap-SC #4-Lesart; Konsistenz mit Phase-15-Audit-Story. → D-17-07.

### Frage 2: Self-Transfer (TRSF-07) → HTTP-Status?

| Option | Description | Selected |
|--------|-------------|----------|
| 400 BadRequest via ValidationError | Input-Fehler in Pure-Function-Validation. | ✓ |
| 409 Conflict | Symmetrisch zu PERM-03. | |
| 422 Unprocessable Entity | Semantisch präziser; Präzedenzfall. | |

**User's choice:** 400 BadRequest via ValidationError
**Notes:** Konsistent mit Phase 15 D-15-08 Validation-Pattern. → D-17-08.

### Frage 3: Wo lebt die Eingabe-Validierung?

| Option | Description | Selected |
|--------|-------------|----------|
| Pure-Function `validate_transfer_inputs` | Analog Phase 15 D-15-05; deterministische Edge-Case-Tests. PERM-03 bleibt separater Service-Branch (DAO-Read). | ✓ |
| Inline im Service | Validierung direkt in Service-Methode. | |
| Gemischt | Pure-Function für n + Self-Transfer; PERM-03 separat (effektiv selbe Trennung). | |

**User's choice:** Pure-Function `validate_transfer_inputs`
**Notes:** → D-17-09, D-17-10 Error-Mapping-Tabelle.

---

## Claude's Discretion

- Response-DTO-Naming: anonymes JSON oder benannter `TransferSharesResponseTO` (Planner entscheidet basierend auf OpenAPI-Klarheit).
- Handler-Datei-Placement: `genossi_rest/src/membership_adjust.rs` erweitern; falls >600 LOC: Split in Submodule.
- `recalc_migrated`-Aufruf: NICHT aufrufen (Übertrag berührt `migrated`-Flag nicht); Planner verifiziert.
- Edge-Case-Test "from hat PaidOut-reduzierte current_shares": Planner darf Mini-Test ergänzen.
- OpenAPI-Annotationen: Utoipa mit 200/400/401/403/404/409/500.
- `audited_*!`-Macro-Reihenfolge: Planner darf minimal umstellen, solange Atomarität + finale recalc_dates(from) erhalten bleiben.
- `tokio::time::sleep(1ms)` Pool-Warm-up: kopieren aus v1.1 Phase 9.
- `busy_timeout` PRAGMA: nur falls Race-Tests intermittierend rot werden.

## Deferred Ideas

- MembershipAdjustService Voll-Verschmelzung mit MemberActionService (v2-Refactor).
- Storno-Knopf für Übertrag (bleibt manuelle MemberAction; PROJECT.md Out-of-Scope).
- Bulk-Übertrag (kein User-Case in v1.2).
- Voll-Übertrag-Austritt mit `transfer_member_id = None` als zukünftiger Vereinheitlichungs-Pfad.
- `busy_timeout` PRAGMA generisch setzen.
- `recalc_migrated` Free-Function-Refactor.
- Frontend-Vorschau-Dialog für Voll-Übertrag-Edge-Case (Phase 18).
