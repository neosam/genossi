# Phase 8: RepaymentEntry + Auto-Befüllung - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-30
**Phase:** 8-repaymententry-auto-bef-llung
**Areas discussed:** Auto-Fill + Audit-Granularität, Status-Lifecycle & Toggle-API, REST-Pfad-Schema (Nested vs Flat), Close-Validation (PHAS-03)

---

## Auto-Fill + Audit-Granularität

### Q1 — FY-Range-Definition

| Option | Description | Selected |
|--------|-------------|----------|
| Kalenderjahr 1.1.–31.12. | `exit_date BETWEEN {fy}-01-01 AND {fy}-12-31`. Einfachste Bedeutung; deutsche Genossenschaften nutzen überwiegend Kalenderjahr als GJ. | ✓ |
| Konfigurierbar pro Phase (Start/Ende-Datum) | Mehr Flexibilität, mehr Felder zu pflegen + Migration. | |
| Konfigurierbar global (Settings) | Globale Setting `fiscal_year_start_month`. Mittelweg. | |

**User's choice:** Kalenderjahr (Recommended).
**Notes:** Pattern-konsistent mit deutscher Genossenschaftspraxis. Bei Bedarf später nachziehbar.

### Q2 — Member-Filter beim Auto-Fill

| Option | Description | Selected |
|--------|-------------|----------|
| Strikt: deleted IS NULL + exit_date IN FY + current_shares > 0 | Nur aktive Mitglieder mit Anteilen > 0 erzeugen Einträge. Verhindert leere Einträge. | ✓ |
| Liberal: deleted IS NULL + exit_date IN FY (auch share=0) | Auch Member mit 0 Anteilen, Vorstand prüft manuell. | |
| Strikt + zusätzlich status.is_normal() | Aber: Ausgeschiedene haben oft Status != Normal nach exit_date — würde Zielgruppe ausschließen. | |

**User's choice:** Strikt (Recommended).
**Notes:** `is_normal()`-Filter bewusst ausgelassen, weil Ausgeschiedene oft != Normal-Status haben und genau die die Zielgruppe sind.

### Q3 — Audit-Granularität beim Auto-Fill

| Option | Description | Selected |
|--------|-------------|----------|
| N einzelne audited_create! pro Eintrag | Volle Audit-Granularität, gruppiert via transaction_id. Pattern-konsistent mit Member/MemberAction. | ✓ |
| Batch ohne Audit (Assembly-Snapshot-Pattern) | Einträge als "Daten zum Open-Akt", nur die Phase wird auditiert. Nachteil: keine Hash-Chain pro Eintrag. | |
| Hybrid: 1 Audit-Summary + N raw inserts | Forensik weiß N, aber Details nicht in Chain. | |

**User's choice:** N einzelne audited_create! (Recommended).
**Notes:** RepaymentEntries sind Lifecycle-Träger (Phase 9 cascade-bucht daran), nicht „nur Daten" — daher volle Audit-Granularität.

### Q4 — Nachzügler nach Phase-Open

| Option | Description | Selected |
|--------|-------------|----------|
| Nur manueller Add via REST | Auto-Fill nur einmal beim Open. Nachzügler via POST /api/repayment-entry (ENTR-02). | ✓ |
| Re-Fill-Endpoint `POST /{id}/refill` | Vorstand kann erneuten Auto-Fill anstoßen, ergänzt nur fehlende Member. | |
| Automatisch via Member-Update-Hook | Magisch, undurchsichtig im Audit, Cross-Service-Coupling. | |

**User's choice:** Nur manueller Add (Recommended).
**Notes:** Klare Audit-Story; Phase-Open ist ein Bilanz-Stichtag.

---

## Status-Lifecycle & Toggle-API

### Q1 — Reversibilität Open ↔ Contacted

| Option | Description | Selected |
|--------|-------------|----------|
| Bidirektional reversibel | Open ↔ Contacted in beide Richtungen erlaubt (Mail-Korrektur). PaidOut bleibt einseitig (Phase 9). | ✓ |
| Einbahnstraße (Open → Contacted only) | Rückwechsel nur via Soft-Delete + Neuanlage. | |
| Bidirektional mit Audit-Historie | Wie Option A, Audit zeigt die Toggle-Sequenz. Überlappt semantisch mit A. | |

**User's choice:** Bidirektional reversibel (Recommended).

### Q2 — Batch-Toggle-API-Design

| Option | Description | Selected |
|--------|-------------|----------|
| Dedizierter Batch-Endpoint mit ID-Liste | `POST /api/repayment-entry/batch-status` mit `{ entry_ids, target_status }`. Ein Roundtrip. | ✓ |
| PUT pro Entry (kein Batch) | N parallele PUTs vom Frontend. Simpler Server, Frontend sammelt Errors. | |
| Phase-scoped Batch via Filter | `POST /api/repayment-phase/{id}/mark-contacted?status=Open` markiert ALLE. Nicht selektiv genug. | |

**User's choice:** Dedizierter Batch-Endpoint (Recommended).

### Q3 — Batch-Fehler-Semantik

| Option | Description | Selected |
|--------|-------------|----------|
| All-or-nothing | Eine Tx, erster Fehler rollt zurück, 409 mit Error-Detail. | ✓ |
| Best-effort | Per-Entry success/fail. Pragmatisch wie Phase-10-Mail-Pattern. | |
| All-or-nothing mit dry-run | `?dry_run=true` zum Validieren. Mehr API-Komplexität. | |

**User's choice:** All-or-nothing (Recommended).
**Notes:** Status-Toggle ist atomare State-Machine-Transition, kein I/O-Resilience-Problem.

### Q4 — PaidOut-Repräsentation in Phase 8

| Option | Description | Selected |
|--------|-------------|----------|
| Enum hat 3 Werte, Toggle in Phase 8 blockiert | `{ Open, Contacted, PaidOut }` von Anfang an. Phase 8 blockt PaidOut-Toggle mit 409. Keine Schema-Migration in Phase 9. | ✓ |
| Enum hat erst 2 Werte | Phase 9 erweitert. Cleaner Boundary, aber Migration nötig. | |
| Enum offen lassen (TEXT in DB) | Rust parst nur 2 in Phase 8, Phase 9 erweitert. Hacky, pattern-inkonsistent. | |

**User's choice:** 3-Wert-Enum von Anfang an (Recommended).

---

## REST-Pfad-Schema (Nested vs Flat)

### Q1 — Pfad-Stil

| Option | Description | Selected |
|--------|-------------|----------|
| Flat `/api/repayment-entry` mit phase_id im Body/Query | Konsistent mit Member/Application/MemberAction/Attendance. | ✓ |
| Nested `/api/repayment-phase/{phase_id}/entry/...` | Hierarchie sichtbar. Pattern-Anker: Assembly-Attendance/Helper-Tokens. | |
| Hybrid: GET nested, mutations flat | Spiegelt Praxis, mehr API-Oberfläche. | |

**User's choice:** Flat (Recommended).

### Q2 — Listing-Filter-Optionen

| Option | Description | Selected |
|--------|-------------|----------|
| ?phase_id=<uuid> | Alle Einträge zu einer Phase — Hauptanwendung. | ✓ |
| ?member_id=<uuid> | Einträge eines Members über alle Phasen. | |
| ?status=<Open\|Contacted\|PaidOut> | Status-Filter für Frontend. | |
| ?include_deleted=true | Audit-Tooling, default off. | |

**User's choice:** Nur `?phase_id=<uuid>`.
**Notes:** Status- und Member-Filter werden client-side gemacht oder kommen später.

### Q3 — Create-Validierungen beim manuellen Add

| Option | Description | Selected |
|--------|-------------|----------|
| Phase ist im Status Open | Sonst 409 Conflict. | ✓ |
| Member existiert und nicht soft-deleted | Sonst 400/404. | ✓ |
| share_count > 0 AND ≤ Member.current_shares | Sonst ValidationError. | ✓ |
| Sum aller offenen+angeschriebenen Entries ≤ Member.current_shares | Strikte Aggregat-Validation. Nicht recommended. | |

**User's choice:** Erste drei (alle Recommended). Sum-Check explizit nicht gewollt — Phase 9 catches.

---

## Close-Validation (PHAS-03)

### Q1 — Definition von "pending"

| Option | Description | Selected |
|--------|-------------|----------|
| status != PaidOut AND deleted IS NULL | Open- und Contacted-Einträge blocken; verbandskonform. | ✓ |
| Nur status = Open blockiert | Bricht Workflow-Sinn. | |
| Liberal: nichts blockiert | Für Audit schlecht. | |

**User's choice:** status != PaidOut AND deleted IS NULL (Recommended).

### Q2 — 0-Entry-Phase Close-Verhalten

| Option | Description | Selected |
|--------|-------------|----------|
| Close erlaubt | 0 pending = 0 Konflikt; Phase darf direkt wieder schließen. | ✓ |
| Warnung mit explizitem Confirm | 422 + Frontend-Dialog. Mehr Friction. | |
| Close blockt mit 409 | Zu paranoid. | |

**User's choice:** Close erlaubt (Recommended).

### Q3 — 409-Body-Detail beim pending-Close

| Option | Description | Selected |
|--------|-------------|----------|
| Anzahl + Mitgliedsnummern-Liste (max 20, +N weitere) | Vorstand sieht direkt wo das Problem ist. | ✓ |
| Nur Anzahl + Generic-Message | Frontend muss filtern. | |
| Anzahl + Entry-UUIDs | UUIDs sind dem Vorstand unvertraut. | |

**User's choice:** Anzahl + Mitgliedsnummern-Liste (Recommended).

---

## Claude's Discretion

- **PUT-Body-Schema** für `PUT /api/repayment-entry/{id}` — `{ share_count_to_pay_out?, status?, version }` mit Edit-Matrix-Check im Service; PaidOut als PUT-Target → 409.
- **Auto-Fill-Reihenfolge** der `audited_create!`-Calls — deterministisch z.B. nach `member_number ASC`.
- **Batch-Toggle-Größenlimit** — optional Max-Batch-Size (z.B. 500) als DoS-Schutz, Planner entscheidet.
- **Indizes auf `repayment_entry`** — Migration darf `phase_id`, `(phase_id, status)`, `member_id` indizieren falls Listing-Filter-Workload das rechtfertigt.
- **`MemberAction::Verkauf`-Audit-Coupling vorbereiten** — Phase 8 muss noch nichts wiren; Phase 9 bekommt `transaction_id`-Verkettung automatisch über `audited_update!`-Macro-Tx-Group.

## Deferred Ideas

- `?member_id=<uuid>` Listing-Filter — Phase 12 Member-Detail-Page
- `?status=<...>` Listing-Filter — später, jetzt client-side
- `?include_deleted=true` Listing-Filter — Audit-Tooling, später
- Re-Fill-Endpoint `POST /api/repayment-phase/{id}/refill` — explizit verworfen
- Member-Status-Filter (`is_normal()`) beim Auto-Fill — bewusst nicht angewendet
- Sum-Check über mehrere Entries pro `(member_id, phase_id)` — Phase 9 fängt es
- DB-Unique-Index auf `(phase_id, member_id, status='Open')` — würde ENTR-03 brechen
- PUT-Body-Status-Toggle vs. nur Batch-Endpoint — Planner-Discretion
- Max-Batch-Size — Planner-Discretion
