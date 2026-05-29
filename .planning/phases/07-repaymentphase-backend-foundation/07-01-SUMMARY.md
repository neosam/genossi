---
phase: 07-repaymentphase-backend-foundation
plan: 01
subsystem: database
tags: [sqlx, sqlite, dao, audit, repayment-phase, migration, rust]

# Dependency graph
requires:
  - phase: 01-assembly-aggregat-audit-hardening
    provides: Assembly-Aggregat-Pattern (Migration + DAO + Auditable) als 1:1-Vorlage
provides:
  - "Migration `repayment_phase` Tabelle (9 Spalten, 2 Indizes, kein UNIQUE auf fiscal_year)"
  - "RepaymentPhaseStatus enum (Preparation/Open/Closed, English strings per D-01)"
  - "RepaymentPhaseEntity (id, fiscal_year: i32, share_value: i64 Cent, status, opened_at, closed_at, created, deleted, version)"
  - "Auditable impl mit 5 audit_fields ohne id/version/created/deleted (Audit-Konvention)"
  - "RepaymentPhaseDao trait (dump_all/create/update + default all/find_by_id mit deleted-IS-NULL-Filter)"
  - "Modul-Deklaration in genossi_dao/src/lib.rs alphabetisch eingefügt"
affects: [07-02-sqlite-impl, 07-03-service, 07-04-rest, 07-05-e2e, 08-repayment-entries, 09-payout-cascade, 11-export]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Lifecycle-Aggregat-Pattern (3-State enum + opened_at/closed_at als Spalten + Auditable)"
    - "Cent-as-i64 für monetäre Werte (etabliert Pattern für Phase 9 PAYO und Phase 11 Export)"
    - "Auditable-Impl mit format_dt-Sentinel statt unwrap_or_default (WR-08-Lesson übernommen)"

key-files:
  created:
    - "migrations/sqlite/20260529190437_create_repayment_phase_table.sql"
    - "genossi_dao/src/repayment_phase.rs"
  modified:
    - "genossi_dao/src/lib.rs"

key-decisions:
  - "Status-Strings Englisch (Preparation/Open/Closed) — pattern-konsistent mit AssemblyStatus, Frontend übersetzt via i18n (D-01)"
  - "Kein UNIQUE-Constraint auf fiscal_year — mehrere Phasen pro Geschäftsjahr in beliebigen Statuskombinationen erlaubt (D-08)"
  - "opened_at/closed_at als optionale Spalten persistiert (nicht nur im Audit-Log) — analog Assembly, nützlich für Phase 11 Filename-Schema (D-13)"
  - "fiscal_year als i32 (DB INTEGER), share_value als i64 (DB INTEGER, Cent) — Range-/Positivitäts-Checks im Service-Layer (Plan 03, D-11/D-12)"
  - "Auditable-Impl schließt id/version/created/deleted explizit aus (Konvention; verifiziert per Unit-Test)"
  - "format_dt-Closure mit tracing::error! + Sentinel '<invalid datetime>' statt unwrap_or_default — WR-08-Lesson aus Assembly übernommen, NICHT vereinfacht"

patterns-established:
  - "Assembly-1:1-Replikation: assembly.rs ist direkte Vorlage für neue Lifecycle-Aggregate; Domain-Substitutionen sind die einzigen Änderungen"
  - "i64-Cent-Konvention für monetäre Werte (vs. f64) — wird in Phase 8 RepaymentEntry.amount, Phase 9 MemberAction-Cascade und Phase 11 Export übernommen"

requirements-completed: [PHAS-01, PHAS-05]

# Metrics
duration: 5min
completed: 2026-05-29
---

# Phase 7 Plan 01: RepaymentPhase Backend Foundation Summary

**Migration für `repayment_phase`-Tabelle plus RepaymentPhaseDao-Trait + RepaymentPhaseEntity + Auditable-Impl mit 7 grünen Unit-Tests — direktes 1:1-Replikat des Assembly-Aggregats mit Domain-Substitutionen (fiscal_year: i32, share_value: i64 Cent)**

## Performance

- **Duration:** ~5 min (4 min 34 s)
- **Started:** 2026-05-29T19:35:03Z
- **Completed:** 2026-05-29T19:39:37Z
- **Tasks:** 2 (von 2)
- **Files created/modified:** 3 (2 new + 1 modified)

## Accomplishments

- Migration `20260529190437_create_repayment_phase_table.sql` mit 9 Spalten + 2 Indizes (status, deleted), KEIN UNIQUE auf fiscal_year (D-08)
- `RepaymentPhaseStatus` enum mit drei englischen Varianten (Preparation/Open/Closed), `as_str`/`from_str`/`Default::Preparation`
- `RepaymentPhaseEntity` struct mit allen 9 Feldern aus CONTEXT.md §entity-Skeleton
- `impl Auditable` mit `entity_type = "repayment_phase"`, `entity_id = self.id`, exakt 5 audit_fields (fiscal_year, share_value, status, opened_at, closed_at) — schließt id/version/created/deleted explizit aus (Auditable-Konvention)
- `RepaymentPhaseDao` trait mit 3 Pflicht-Methoden + 2 Default-Impls (all/find_by_id mit deleted-IS-NULL-Filter)
- 7 Unit-Tests grün, 60/60 genossi_dao gesamt grün, Workspace-Build clean
- Modul-Deklaration in `genossi_dao/src/lib.rs` alphabetisch zwischen `permission` und `user_preference` eingefügt

## Task Commits

Each task was committed atomically:

1. **Task 1: Migration `repayment_phase` Tabelle** — `b28cf91` (feat)
2. **Task 2: DAO-Modul + Status enum + Entity + Auditable + Trait + 7 Unit-Tests + lib.rs-Decl** — `a8f38fe` (feat)

**Plan metadata:** _TBD_ (final docs commit after SUMMARY + STATE updates)

## Files Created/Modified

- `migrations/sqlite/20260529190437_create_repayment_phase_table.sql` — CREATE TABLE + 2 Indizes (NEW, 14 LOC)
- `genossi_dao/src/repayment_phase.rs` — Status enum + Entity + Auditable + DAO-Trait + 7 Unit-Tests (NEW, 258 LOC)
- `genossi_dao/src/lib.rs` — `pub mod repayment_phase;` alphabetisch eingefügt (MOD, +1 LOC)

## Decisions Made

- **Pattern-Anker explizit beibehalten:** WR-08-`format_dt`-Closure mit `tracing::error!` und Sentinel-String wurde NICHT vereinfacht — Audit-Forensik-Lesson aus Phase 1 ist hier dauerhaft etabliert.
- **`fiscal_year: i32` (statt u16):** DB INTEGER → Rust i32 ist die Genossi-Konvention; Range-Check (2000..=2100) erfolgt im Service-Layer (Plan 03), nicht via Type-Safety.
- **`share_value: i64` (Cent):** Neue Cent-Konvention für monetäre Werte. SQLite INTEGER ist 8-Byte → Rust i64. Etabliert das Pattern für Phase 8 (`RepaymentEntry.amount`), Phase 9 (`MemberAction::Verkauf` shares_change-Cascade), Phase 11 (Export-Multiplikation). Validierung `> 0` ohne Obergrenze (User-Decision aus CONTEXT.md D-12).
- **Audit-fields-Reihenfolge `fiscal_year, share_value, status, opened_at, closed_at`:** Frozen — T-07-01-01 (Hash-Chain-Konsistenz). Reihenfolge-Test verifiziert im neuen `test_auditable_fields_count_and_excludes_metadata`.

## Deviations from Plan

None — Plan exakt wie geschrieben ausgeführt.

Sub-Repos sind nicht konfiguriert (Single-Repo); jj+git colocated, normale `git commit` verwendet (kein `--no-verify`).

## Domain-Substitutionen für Folgeplans (02-05)

Plan 02 (SQLite-Impl) und nachfolgende Plans müssen diese Domain-Substitutionen 1:1 vom Assembly-Aggregat übernehmen:

- **Tabellenname:** `assembly` → `repayment_phase` (überall in DDL und SELECT/UPDATE/INSERT)
- **Spalten:**
  - `name TEXT NOT NULL` → entfällt
  - `date TEXT NOT NULL` → `fiscal_year INTEGER NOT NULL` (sqlx liest als i64, cast zu i32 in TryFrom)
  - `location TEXT` → `share_value INTEGER NOT NULL` (i64-Bind)
- **ORDER BY in `dump_all`:** `ORDER BY date DESC` → `ORDER BY fiscal_year DESC, created DESC` (CONTEXT.md `<specifics>`)
- **Detail-Wrapper:** Phase 7 hat KEINEN `AssemblyDetail`-äquivalenten Wrapper-Typ — `get_repayment_phase` liefert direkt `RepaymentPhase` (CONTEXT.md `<canonical_refs>` / PATTERNS §4)
- **Anzahl-Asserts in Tests:** 5 statt 6 (kein `name` mehr)
- **OpenAPI-Beispielwerte:** `fiscal_year: 2026`, `share_value: 12000` (Cent = 120,00 EUR)

## Test-Ergebnisse

```
running 7 tests
test repayment_phase::tests::test_auditable_fields_count_and_excludes_metadata ... ok
test repayment_phase::tests::test_repayment_phase_status_default_is_preparation ... ok
test repayment_phase::tests::test_repayment_phase_status_invalid_string ... ok
test repayment_phase::tests::test_repayment_phase_status_roundtrip ... ok
test repayment_phase::tests::test_auditable_diff_detects_status_change ... ok
test repayment_phase::tests::test_auditable_entity_type_is_repayment_phase ... ok
test repayment_phase::tests::test_repayment_phase_status_strings_are_english ... ok

test result: ok. 7 passed; 0 failed
```

Volle `genossi_dao`-Suite: 60 passed; 0 failed.
Workspace-Build: clean (nur 3 pre-existing warnings in genossi_rest/genossi_bin, nicht durch Plan 07-01 verursacht — out of scope).

## TDD Gate Compliance

Plan 07-01 ist `type: execute` (nicht `type: tdd`), nutzt aber Task-Level TDD (`tdd="true"` an jedem Task):

- **Task 1 Migration:** RED (test -f Datei fehlt) → GREEN (Write + grep checks + cargo build) — als ein `feat`-Commit zusammengefasst, weil "Test" hier rein Verification-Greps + cargo build ist, nicht ausführbarer Rust-Test.
- **Task 2 DAO-Modul:** RED (test -f Modul fehlt) → GREEN (Write + lib.rs-Decl) inklusive der 7 ausführbaren Rust-Unit-Tests im selben Commit. Tests prüfen Status-Roundtrip, English-only-Validierung (T-07-01-05), Default, entity_type, audit_fields-Reihenfolge und -Exklusionen, Diff-Detection.

## Issues Encountered

Keine. Pattern-Replikation von `genossi_dao/src/assembly.rs` (251 LOC) → `genossi_dao/src/repayment_phase.rs` (258 LOC) ohne Reibung. Tests von Anfang an grün.

## User Setup Required

Keine externe Konfiguration nötig.

## Next Phase Readiness

Phase-7-Plans 02-05 hängen alle an diesem Plan (depends_on: [01]). Sie können in der dokumentierten Reihenfolge starten:

- **Plan 02 (SQLite-Impl):** Migration ist eingespielt-bereit; `RepaymentPhaseDao`-Trait existiert → `genossi_dao_impl_sqlite/src/repayment_phase.rs` kann jetzt mit dem in PATTERNS.md §3 dokumentierten Pattern angelegt werden. ORDER BY ist `fiscal_year DESC, created DESC` (vs. Assembly `date DESC`).
- **Plan 03 (Service):** `Auditable`-Trait ist implementiert → `audited_create!`/`audited_update!`/`audited_delete!`-Macros greifen out-of-the-box. Field-Validation (fiscal_year 2000..=2100, share_value > 0) als Inline-Helper.
- **Plan 04 (REST):** TO-Layer kann mit `From<&RepaymentPhase>`-Pattern aus PATTERNS.md §7 erstellt werden. Singular-Pfad `/api/repayment-phase` (D-14).
- **Plan 05 (E2E):** create → open → update share_value → close, Audit-Chain-Verifikation via `/api/audit/verify`.

ROADMAP SC#1 (Migration legt Tabelle an) und SC#2 Teil-1 (Auditable-Trait-Impl) sind erfüllt. SC#2 Teil-2 (Macros greifen) wird in Plan 03 verifiziert.

## Self-Check: PASSED

- `migrations/sqlite/20260529190437_create_repayment_phase_table.sql`: FOUND
- `genossi_dao/src/repayment_phase.rs`: FOUND
- `genossi_dao/src/lib.rs`: FOUND (mit `pub mod repayment_phase;`)
- Commit `b28cf91` (Task 1 Migration): FOUND
- Commit `a8f38fe` (Task 2 DAO + lib.rs): FOUND

---
*Phase: 07-repaymentphase-backend-foundation*
*Completed: 2026-05-29*
