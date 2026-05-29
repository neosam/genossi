# Phase 1: Assembly-Aggregat + Audit-Hardening - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-02
**Phase:** 1-Assembly-Aggregat + Audit-Hardening
**Areas discussed:** Member-Universe-Snapshot-Speicherung, Lifecycle-Audit-Granularität, Phase-Grenze ASSY-06, Naming-Konvention

---

## Member-Universe-Snapshot-Speicherung

### Frage 1: Persistenz-Strategie

| Option | Description | Selected |
|--------|-------------|----------|
| Eigene Tabelle | `assembly_member_snapshot(assembly_id, member_id, captured_at)`. Indexierbar, audit-fähig. | ✓ |
| JSON-Blob im Assembly-Datensatz | TEXT-Spalte mit Member-UUID-Liste. Einfachere Migration, kein JOIN. | |
| Computed-on-demand | Kein persistenter Snapshot, Filter via `created < opened_at AND deleted IS NULL`. | |

**User's choice:** Eigene Tabelle (Recommended)
**Notes:** —

### Frage 2: Aktivitäts-Kriterium

| Option | Description | Selected |
|--------|-------------|----------|
| `deleted IS NULL` | Einfachstes Kriterium, kein Status-Wissen | |
| `deleted IS NULL AND status = Normal` | Schließt FehlerhaftErfasst aus | |
| Du entscheidest | Claude entscheidet basierend auf Praxis | |

**User's choice:** Free-text — "Deleted is null, austrittsdatum, falls gesetzt muss in der Zukunft liegen, und Status muss normal sein."
**Notes:** Diese Logik existiert bereits in `genossi_dao/src/member.rs:182` als `count_active`. Übernahme: `member.deleted IS NULL AND (member.exit_date IS NULL OR member.exit_date > opened_at) AND member.status = MemberStatus::Normal`.

### Frage 3: Snapshot-Felder

| Option | Description | Selected |
|--------|-------------|----------|
| Nur `(assembly_id, member_id)` | Schlanke Tabelle, JOIN auf `member` | ✓ |
| ID + eingefrorene Stamm-Felder | Speichert Snapshot von Mitgliedsnr/Name/Anrede | |
| Du entscheidest | | |

**User's choice:** Nur `(assembly_id, member_id)` (Recommended)
**Notes:** Soft-Delete-Konvention macht hard-deletes unwahrscheinlich; Umbenennungen sind im Protokoll korrekt-gewollt.

### Frage 4: Y-Berechnung

| Option | Description | Selected |
|--------|-------------|----------|
| `COUNT(*)` ad-hoc | Read on demand, kein Cache | ✓ |
| Cached in `assembly.member_universe_count` | Schnellerer Read, Cache-Drift-Risiko | |
| Du entscheidest | | |

**User's choice:** `COUNT(*)` ad-hoc (Recommended)
**Notes:** —

### Continue-Check

| Option | Description | Selected |
|--------|-------------|----------|
| Nächste Area | Snapshot ist klar entschieden | |
| Mehr zum Snapshot | Es gibt offene Punkte | |

**User's choice:** Free-text — "Was empfiehlst du?" → Claude empfahl, weiter zur nächsten Area.
**Notes:** Snapshot solide entschieden; Index-Strategie und ON-DELETE-Verhalten sind Implementierungs-Details für die Plan-Phase.

---

## Lifecycle-Audit-Granularität

### Frage 1 (initial): Audit-Sichtbarkeit

| Option | Description | Selected |
|--------|-------------|----------|
| Generic `audited_update!` (Status-Diff) | Status-Wechsel als Field-Diff | |
| Dedizierte Lifecycle-Events (Recommended) | Klar benannte Process-Identifier | |
| Beides | Doppelte Markierung | |

**User's choice:** Free-text — "Warum Audit? Wollten wir das nicht ausschließen? Das war doch nicht notwendig."
**Notes:** Claude klärte den Scope — ATTN-05 (Anwesenheits-Markierungen) ist out-of-scope, aber ASSY-07 (Lifecycle-Audit) bleibt explizit drin laut REQUIREMENTS.md und Phase-1-Goal.

### Frage 1b: Soll Lifecycle auditiert bleiben?

| Option | Description | Selected |
|--------|-------------|----------|
| Ja, auditiert lassen (Recommended) | ASSY-07 bleibt unverändert | ✓ |
| Nein, kein Audit für Assembly | Scope-Änderung, ASSY-07 streichen | |

**User's choice:** Ja, auditiert lassen (Recommended)
**Notes:** —

### Frage 2: Granularität

| Option | Description | Selected |
|--------|-------------|----------|
| Generic `audited_update!` Status-Diff | Process-String generic, Hash-Chain-Eintrag pro Feld | |
| Dedizierte Process-Identifier (Recommended) | `assembly.create` / `.open` / `.close` als Process-Name | ✓ |
| Du entscheidest | | |

**User's choice:** Dedizierte Process-Identifier (Recommended)
**Notes:** Verbandsprüfer können im Audit-Log direkt nach Lifecycle-Aktion filtern, ohne Field-Diffs interpretieren zu müssen.

### Frage 3: Auditable-Trait

| Option | Description | Selected |
|--------|-------------|----------|
| Auditable-Trait implementieren (Recommended) | Konsistent mit Member/Application-Pattern | ✓ |
| Nur Lifecycle-Events, kein Field-Diff | Custom-Audit ohne Trait | |
| Du entscheidest | | |

**User's choice:** Auditable-Trait implementieren (Recommended)
**Notes:** —

### Frage 4: CI-E2E-Test-Standort

| Option | Description | Selected |
|--------|-------------|----------|
| `genossi_bin/tests/e2e_tests.rs` erweitern (Recommended) | Bestehende E2E-Test-Infrastruktur | ✓ |
| Neue Datei `audit_e2e_tests.rs` | Saubere Trennung, mehr Boilerplate | |
| Service-Layer-Test ohne HTTP | Kein voller Stack | |

**User's choice:** `genossi_bin/tests/e2e_tests.rs` erweitern (Recommended)
**Notes:** —

### Continue-Check

| Option | Description | Selected |
|--------|-------------|----------|
| Nächste Area | Audit ist klar entschieden | |
| Mehr zum Audit | Offene Punkte | |

**User's choice:** Free-text — "Was empfiehlst du?" → Claude empfahl, weiter zur nächsten Area.
**Notes:** —

---

## Phase-Grenze ASSY-06 (Post-Close-Korrektur)

### Frage 1: Phase-1-Lieferumfang

| Option | Description | Selected |
|--------|-------------|----------|
| Phase 1 baut nur Assembly + Snapshot, ASSY-06 nach Phase 3 (Recommended) | Saubere Trennung Assembly vs. Attendance | ✓ |
| Phase 1 baut Attendance-Tabelle leer mit | Nur Migration, Service später | |
| Phase 1 baut alles für ASSY-06 self-contained | Größere Phase 1 | |

**User's choice:** Phase 1 baut nur Assembly + Snapshot, ASSY-06 nach Phase 3 (Recommended)
**Notes:** ROADMAP-Update zwingend (deferred section in CONTEXT.md).

### Frage 2: Cascade-Hook in close_assembly

| Option | Description | Selected |
|--------|-------------|----------|
| Hook erst in Phase 3 (Recommended) | Phase 1 hat schlankes close_assembly | |
| Trait-Slot vorbereiten in Phase 1 | AssemblyClosedListener-Pattern | |
| Du entscheidest | | |

**User's choice:** Du entscheidest → Claude wählt: Hook erst in Phase 3 (YAGNI).
**Notes:** —

### Frage 3: Stamm-Felder

| Option | Description | Selected |
|--------|-------------|----------|
| Datum + Titel (Recommended) | ASSY-01-Minimum | |
| Datum + Titel + Ort | Plus optionales Ort-Feld | ✓ |
| Datum + Titel + Ort + Tagesordnung | Volle Niederschrift-Felder | |
| Du entscheidest | | |

**User's choice:** Datum + Titel + Ort
**Notes:** —

### Frage 4: Datums-Typ

| Option | Description | Selected |
|--------|-------------|----------|
| `PrimitiveDateTime` mit Uhrzeit (Recommended) | Konsistent mit Genossi | ✓ |
| `time::Date` | Nur Tag, einfacher | |
| Du entscheidest | | |

**User's choice:** `PrimitiveDateTime` mit Uhrzeit (Recommended)
**Notes:** —

### Frage 5: Permission

| Option | Description | Selected |
|--------|-------------|----------|
| Bestehende `admin`-Permission (Recommended) | Vorstand hat bereits admin | ✓ |
| Neue `manage_assemblies`-Permission | Feinere Granularität | |
| Du entscheidest | | |

**User's choice:** Bestehende `admin`-Permission (Recommended)
**Notes:** —

### Continue-Check

| Option | Description | Selected |
|--------|-------------|----------|
| Letzte Area (Naming) | Phase-Grenze klar | ✓ |
| Mehr zur Phase-Grenze | | |

**User's choice:** Letzte Area
**Notes:** —

---

## Naming-Konvention

### Frage 1: Code-Identifier-Sprache

| Option | Description | Selected |
|--------|-------------|----------|
| Englisch: `Assembly` / `assembly` (Recommended) | Konsistent mit Member/Application | ✓ |
| Deutsch: `Generalversammlung` / `generalversammlung` | Domänen-nah | |
| Hybrid (Tabelle deutsch, Code englisch) | Verwirrend | |

**User's choice:** Englisch (Recommended)
**Notes:** —

### Frage 2: Status-Werte

| Option | Description | Selected |
|--------|-------------|----------|
| Deutsch: Vorbereitung / Offen / Geschlossen (Recommended) | Konsistent mit MemberStatus-Pattern | |
| Englisch: Preparation / Open / Closed | Sauber englisch, Frontend braucht i18n | ✓ |
| Du entscheidest | | |

**User's choice:** Englisch: Preparation / Open / Closed
**Notes:** Bruch mit MemberStatus-Pattern (deutsch) — bewusste Entscheidung. Frontend-Mapping zu deutschen Labels in Phase 4.

### Frage 3: Endpoint-URL

| Option | Description | Selected |
|--------|-------------|----------|
| Durchgängig englisch (Recommended) | `/api/assembly` | ✓ |
| Deutsch: `/api/generalversammlung` | URL deutsch | |

**User's choice:** Durchgängig englisch (Recommended)
**Notes:** —

---

## Done-Check

| Option | Description | Selected |
|--------|-------------|----------|
| Bereit für CONTEXT.md | Alle Decisions gefangen | ✓ |
| Mehr Areas erkunden | Weitere unklare Punkte | |

**User's choice:** Bereit für CONTEXT.md
**Notes:** —

---

## Claude's Discretion

- **Cascade-Invalidation-Hook in `close_assembly`** — User sagte „Du entscheidest". Claude wählte: kein Hook in Phase 1, direkter Erweiterungs-Pfad in Phase 3 (YAGNI, Genossi-Konvention erlaubt späteres Erweitern ohne Architektur-Bruch).
- **Index-Strategie auf `assembly_member_snapshot`** — wird in der Plan-Phase entschieden (vermutlich `(assembly_id)` für COUNT, plus `UNIQUE(assembly_id, member_id)` für Snapshot-Idempotenz).
- **ON-DELETE-Verhalten der Foreign Keys** — wird in der Plan-Phase entschieden (vermutlich `RESTRICT`, da Soft-Delete-Konvention).

## Deferred Ideas

- **ROADMAP-Update zwingend nach diesem Discuss:** ASSY-06 von Phase 1 nach Phase 3 verschieben; Phase-1-Goal, Phase-1-SC, Phase-3-SC, REQUIREMENTS-Traceability anpassen.
- **Cascade-Invalidation der Helfer-Sessions in `close_assembly`** — Phase 3.
- **Live-Counter-Endpoint `GET /api/assembly/{id}/stats`** — Phase 3 (ASSY-04).
- **Frontend für Assembly-Verwaltung** — Phase 4 (Component-First-Prinzip).
- **`manage_assemblies`-Permission feiner Granularität** — wenn Sub-Rollen entstehen, später nachziehen.
