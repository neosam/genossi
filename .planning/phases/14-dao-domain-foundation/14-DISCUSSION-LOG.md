# Phase 14: DAO/Domain Foundation - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-04
**Phase:** 14-dao-domain-foundation
**Areas discussed:** Pure-Function Shape + Placement, H1/H2 Edge-Case-Semantik, Transfer-Recipients Endpoint Shape, DAO-Query Impl-Strategy

---

## Pure-Function Shape + Placement

### Return-Type

| Option | Description | Selected |
|--------|-------------|----------|
| Struct `EffectiveDate { fiscal_year, effective_date }` | Named-Felder, selbsterklärend am Call-Site. Vorbild: `RepaymentPhaseSubmission`-Struct. | ✓ |
| Tuple `(i32, Date)` mit Doc-Kommentar | Roadmap-Vorlage, minimal, aber Call-Site weniger lesbar. Vorbild: `compute_dates`. | |
| Zwei separate Funktionen | `compute_fiscal_year` + `compute_effective_date`. Doppelte Halbjahres-Branch-Logik. | |

**User's choice:** Struct `EffectiveDate { fiscal_year, effective_date }` (D-14-01)

### Placement

| Option | Description | Selected |
|--------|-------------|----------|
| Neue Datei `membership_adjust.rs` | Foundation für Phase 15-17, sauberer Modul-Schnitt. | ✓ |
| Ergänzen in `member_action.rs` | Neben `compute_dates`. Aber: würde Datei aufblähen bei v1.2-Ausbau. | |
| Neues Sub-Modul `domain/effective_date.rs` | Saubere Trennung, aber kein Vorbild im v1.1-Codebase. | |

**User's choice:** Neue Datei `genossi_service_impl/src/membership_adjust.rs` (D-14-02)

### Visibility

| Option | Description | Selected |
|--------|-------------|----------|
| `pub(crate)` — nur intern testbar | Konsistent mit `compute_dates`. Service wrapped die Logik. | ✓ |
| `pub` — Re-Export für REST/Tests | Breiter API-Surface, falls REST-Layer direkt prüft. | |

**User's choice:** `pub(crate)` (D-14-03)

---

## H1/H2 Edge-Case-Semantik

### H1-Grenze

| Option | Description | Selected |
|--------|-------------|----------|
| 30.06. = H1 (`month <= 6`) | ARCHITECTURE.md §3 Vorlage. Verbands-Konvention. | ✓ |
| 30.06. = H2 (`month > 6` mit Tag-Offset) | Sonderfall ohne Beleg. | |

**User's choice:** 30.06. = H1 (D-14-04)

### 31.12.-Edge

| Option | Description | Selected |
|--------|-------------|----------|
| 31.12.YYYY → H2 → 31.12.YYYY+1 | Konsistent mit `month <= 6`-Regel, "Dezember-Kündigung wirkt im Folgejahr". | ✓ |
| 31.12.YYYY → H1 (Sonderfall) | Verbands-untypisch, würde Logik komplexer machen. | |

**User's choice:** 31.12.YYYY → H2 → 31.12.YYYY+1 (D-14-05)

### Schaltjahr

| Option | Description | Selected |
|--------|-------------|----------|
| Ja — expliziter Test 29.02.2024 → H1 → 31.12.2024 | Defensive Coverage, Roadmap-Vorgabe. | ✓ |
| Nein — Schaltjahr ist time-crate-Verantwortung | Mathematisch redundant aber Test kostet wenig. | |

**User's choice:** Expliziter Test (D-14-06)

### Doc-Kommentar

| Option | Description | Selected |
|--------|-------------|----------|
| Ja — ausführlicher `///`-Doc-Kommentar | Verbands-Konvention im Code verankert. | ✓ |
| Nein — Tests dokumentieren das Verhalten | Code-First-Convention, aber verbands-rechtliche Regel verdient Doc. | |

**User's choice:** Ausführlicher Doc-Kommentar (D-14-07)

---

## Transfer-Recipients Endpoint Shape

### Liste vs. Search

| Option | Description | Selected |
|--------|-------------|----------|
| Volle gefilterte Liste (kein Search-Param) | <200 Members, Frontend filtert client-seitig (Phase 12 D-10 MEMBERS-Signal). | ✓ |
| Mit Search-Query-Param `?q=foo` | Skaliert besser, aber neues Pattern + Pagination-Folge. | |
| Volle Liste als MemberSlim-TO | Datenschutz/Bandbreite, schmaleres TO. (Wurde später separat als Response-TO gewählt — siehe unten.) | |

**User's choice:** Volle gefilterte Liste

### Path/Param

| Option | Description | Selected |
|--------|-------------|----------|
| `GET /api/members/transfer-recipients?exclude_self={uuid}` | Roadmap-Vorlage, Sub-Route von `/api/members`. | ✓ |
| `GET /api/members/transfer-recipients?exclude={uuid}` | Kürzer, generischer. | |
| `GET /api/members/transfer-recipients/{source_id}/candidates` | REST-puristisch, aber komplexer Path. | |

**User's choice:** `?exclude_self={uuid}` (D-14-10)

### Permission

| Option | Description | Selected |
|--------|-------------|----------|
| Admin-only (`ADMIN_PRIVILEGE`) | Konsistent mit allen v1.2-Operationen, schützt vor Helfer-Auth. | ✓ |
| Wie `GET /api/members` (kein extra Privilege) | Konsistenz, aber öffnet für Helfer. | |

**User's choice:** Admin-only (D-14-11)

### Response-TO

| Option | Description | Selected |
|--------|-------------|----------|
| Voller `MemberTO` | Konsistent mit `GET /api/members`. | |
| Neuer `MemberSlimTO` (id, member_number, first_name, last_name, title, salutation) | Klarer API-Vertrag, kein Datenleck. | ✓ |

**User's choice:** Neuer `MemberSlimTO` (D-14-12)

---

## DAO-Query Impl-Strategy

### DAO-Impl

| Option | Description | Selected |
|--------|-------------|----------|
| Default-Impl auf Trait via `dump_all().filter(...)` | Konsistent mit `find_by_phase_id` Z. 138. Bei <200 Entries unproblematisch. | |
| Trait-Methode + SQL-Override in SQLite-Impl | WHERE-Klausel-Filter, skaliert, bricht aber Default-Pattern. | ✓ |
| Default-Impl jetzt, SQL-Override später als Tech-Debt | YAGNI-Move mit dokumentiertem Folge-Todo. | |

**User's choice:** Trait + SQL-Override in SQLite-Impl (D-14-08)

### Soft-Delete

| Option | Description | Selected |
|--------|-------------|----------|
| Ja — `deleted IS NULL` Filter | Konsistent, verhindert False-Positives in PITFALLS-Kat-1-Sum-Check. | ✓ |
| Nein — alle Entries inkl. soft-deleted | Bricht Soft-Delete-Konvention. | |

**User's choice:** `deleted IS NULL` Filter (Bestandteil D-14-08)

### Return-Type

| Option | Description | Selected |
|--------|-------------|----------|
| `Arc<[RepaymentEntryEntity]>` | Konsistent mit v1.1-Codebase, cheap-clone. | ✓ |
| `Vec<RepaymentEntryEntity>` | Eigentümer-Vec, aber bricht v1.1-Konvention. | |

**User's choice:** `Arc<[RepaymentEntryEntity]>` (D-14-09)

### Test-Ort

| Option | Description | Selected |
|--------|-------------|----------|
| DAO-Trait + SQLite + Service — vollständig | Test-Pyramide, jeder Layer eigene Verantwortung. | ✓ |
| SQLite + Service + E2E (Default-Impl-Test weglassen) | Weniger Tests, weniger Wartung — aber bei SQL-Override-Pattern legitim. | |

**User's choice:** Vollständige Coverage (D-14-14)

---

## Claude's Discretion

- Default-Impl-Strategie (Trait-Default als Fallback für Mock-Generierung vs. nur abstract method) — Planner entscheidet.
- `MemberSlimTO`-Field-Set-Erweiterung (z.B. `current_shares` für UI-Display) — Planner darf ergänzen, keine sensiblen Felder.
- OpenAPI-Schema-Details (Status-Codes 200/400/401/403) — Standard Utoipa-Pattern.
- Funktionsname-Inline-Helper (`is_h1(month) -> bool`) — Planner darf hinzufügen.
- Doc-Kommentar-Sprache (Deutsch im `///`-Kommentar, englisch im Code) — etabliert.
- Plan-Datei-Aufteilung (Pure-Function / DAO / Service / REST+E2E) — Planner-Discretion.
- Endpoint-OpenAPI-Schema-Definition für `MemberSlimTO`.
- Test-Count-Floor: 6 Pure-Function / 2 DAO / 3 Service / 1 E2E sind Pflicht; Planner darf nach oben ergänzen (D-14-15).

## Deferred Ideas

- Search-Query-Parameter `?q=foo` für serverseitige Substring-Suche (heute YAGNI).
- Pagination für die Empfänger-Liste (heute YAGNI).
- `current_shares`-Anzeige im `MemberSlimTO` (Phase 18 darf entscheiden).
- `compute_effective_date` als `pub`-Re-Export (Phase 15 PERM-02 prüft nach).
- DAO-Override-SQL-Migration für `find_by_phase_id` (Konsistenz-Folge-Quick, derzeit nicht akut).
- `MembershipAdjustService`-Trait + Impl als ganzes Modul — Phase 14 hat nur die Pure-Function; Phase 15-17 füllen den Service-Trait + Impl.

## Reviewed Todos (not folded)

None — discussion stayed within phase scope.
