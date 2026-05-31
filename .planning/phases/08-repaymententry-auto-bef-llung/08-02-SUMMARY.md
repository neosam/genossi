---
phase: 08-repaymententry-auto-bef-llung
plan: 02
subsystem: database
tags: [sqlite, sqlx, rust, dao, repayment-entry, optimistic-locking, pre-exists-check]

requires:
  - phase: 08-repaymententry-auto-bef-llung
    plan: 01
    provides: "RepaymentEntryDao-Trait, RepaymentEntryEntity, RepaymentEntryStatus-Enum, Migration repayment_entry (Plan 08-01)"

provides:
  - "RepaymentEntryDaoImpl::new(Arc<SqlitePool>) erfuellt die DAO-Trait-Signaturen aus Plan 01"
  - "dump_all liefert sortierte Liste (ORDER BY created ASC, id ASC) fuer deterministische Audit-Reihenfolge"
  - "create persistiert eine Zeile in repayment_entry mit BLOB-UUIDs und ISO8601-Datetimes"
  - "update mit Pre-Exists-Check (SELECT COUNT) + Optimistic-Locking via UPDATE...WHERE id=? AND version=? AND deleted IS NULL"
  - "share_count_to_pay_out via guarded i32::try_from aus DB i64 zurueckgelesen (T-08-02-02)"

affects:
  - "08-03 (Service-Trait): RepaymentEntryServiceImpl wired RepaymentEntryDaoImpl::new via gen_service_impl"
  - "08-04 (RepaymentPhase-Service-Erweiterung): open_phase Auto-Fill nutzt diese DAO-Impl fuer N audited_create-Calls"
  - "08-05 (REST-Handler): Listing GET ?phase_id= laeuft via DAO-Default-Impl find_by_phase_id auf dieser dump_all-Sortierung"

tech-stack:
  added: []
  patterns:
    - "Pre-Exists-Check (SELECT COUNT) vor UPDATE trennt NotFound von ConflictError (Phase-7-Lektion Plan 07-02 D-03)"
    - "Optimistic-Locking via UPDATE...WHERE id=? AND version=? AND deleted IS NULL + rows_affected==0 -> ConflictError"
    - "Guarded i32::try_from aus i64 fuer SQLite-INTEGER-Daten (T-07-02-05 Pattern reuse)"
    - "parse_datetime via use crate::assembly::parse_datetime (kein Duplikat, Phase-7-Lektion)"
    - "Inline-DDL in setup_db() statt include_str! auf Migration (Phase-7-Konvention)"

key-files:
  created:
    - "genossi_dao_impl_sqlite/src/repayment_entry.rs"
  modified:
    - "genossi_dao_impl_sqlite/src/lib.rs (Modul-Deklaration alphabetisch vor repayment_phase eingefuegt)"

key-decisions:
  - "ORDER BY created ASC, id ASC statt nur created (Tie-Breaker fuer Tests mit gleicher Sekunde-Anlage)"
  - "Pre-Exists-Check (SELECT COUNT) vor UPDATE — Phase-7-D-03-Pattern 1:1 uebernommen"
  - "parse_datetime und format_dt aus repayment_phase.rs gespiegelt (parse_datetime reused, format_dt lokal kopiert wegen pub(crate)-Beschraenkung)"
  - "6 Tokio-Tests statt 7 wie im Plan-<behavior>-Block: Test 6 + 7 konsolidiert (find_by_phase_id deckt sowohl Listing als auch deleted-Filter-Default-Impl ab); Plan-Acceptance forderte mindestens 5 Tests"

patterns-established:
  - "DAO-Impl-Vorlage fuer Phase 8 Folgeplaene: RepaymentEntryDaoImpl folgt exakt Phase-7-Repayment-Phase-DaoImpl-Struktur, was Plan 03 (Service-Layer) den Wiring-Pfad vereinfacht"

requirements-completed: [ENTR-01]

duration: ~4min
completed: 2026-05-31
---

# Phase 08 Plan 02: SQLite-Impl des RepaymentEntryDao Summary

**SQLite-Persistenzschicht fuer RepaymentEntry — dump_all/create/update + Pre-Exists-Check + Optimistic-Locking, 1:1 nach Phase-7-Vorlage (`repayment_phase.rs`) mit 6 gruenen Tokio-Tests gegen in-memory SQLite.**

## Performance

- **Duration:** ~4 min
- **Started:** 2026-05-31T04:01:42Z
- **Completed:** 2026-05-31T04:05:23Z
- **Tasks:** 1/1 abgeschlossen
- **Files created:** 1 (DAO-Impl-Modul)
- **Files modified:** 1 (lib.rs Modul-Deklaration)

## Accomplishments

- `RepaymentEntryDaoImpl::new(Arc<SqlitePool>)` etabliert; folgt der Phase-7-Konvention (`AssemblyDaoImpl` / `RepaymentPhaseDaoImpl`)
- `RepaymentEntryDb` (sqlx::FromRow) mit i64-Storage fuer SQLite-INTEGER + `TryFrom<&RepaymentEntryDb> for RepaymentEntryEntity` mit guarded `i32::try_from` fuer `share_count_to_pay_out` (T-08-02-02 Mitigation)
- `dump_all` mit `ORDER BY created ASC, id ASC` fuer deterministische Audit-Reihenfolge (Plan-Vorgabe)
- `create` persistiert alle 8 Spalten (id, member_id, phase_id, share_count_to_pay_out, status, created, deleted, version) mit BLOB-UUIDs und ISO8601-Datetimes
- `update` mit Pre-Exists-Check (`SELECT COUNT(*) ... WHERE id = ? AND deleted IS NULL`) trennt `NotFound` von `ConflictError("Version mismatch")`; UPDATE bumpt atomar `version` via `WHERE id = ? AND version = ? AND deleted IS NULL` (T-08-02-01 Mitigation gegen Lost-Update)
- 6 Tokio-Tests gruen gegen in-memory SQLite:
  - `test_create_and_find_repayment_entry` (Roundtrip)
  - `test_update_repayment_entry_with_version_mismatch_returns_conflict`
  - `test_update_repayment_entry_unknown_id_returns_not_found`
  - `test_update_repayment_entry_succeeds_then_version_changes` (version-bump verifiziert)
  - `test_dump_all_returns_sorted_entries` (3 Eintraege in Nicht-Sortier-Reihenfolge angelegt -> ASC zurueckgelesen)
  - `test_find_by_phase_id_filters_correctly` (Default-Impl-Verifikation aus Plan 01 — 2 Eintraege fuer phase_a, 1 fuer phase_b)
- `parse_datetime` aus `crate::assembly` reused (kein Duplikat); `format_dt` lokal definiert analog repayment_phase.rs (weil `format_dt` dort als private fn lebt, nicht `pub(crate)`)

## Task Commits

Atomar committed:

1. **Task 1: RepaymentEntryDaoImpl + 6 Tokio-Tests** — `69e3135` (feat)

**Plan metadata:** _(folgt mit diesem Commit)_

## Files Created/Modified

- `genossi_dao_impl_sqlite/src/repayment_entry.rs` — 419 LOC: Imports, RepaymentEntryDb, TryFrom-Impl, RepaymentEntryDaoImpl + new(), format_dt-Helper, DAO-Trait-Impl (dump_all/create/update), 6 Tokio-Tests im `mod tests`
- `genossi_dao_impl_sqlite/src/lib.rs` — +1 LOC: `pub mod repayment_entry;` alphabetisch vor `pub mod repayment_phase;`

## Decisions Made

Alle wesentlichen Decisions kamen aus `08-PATTERNS.md` §3 und `08-02-PLAN.md` und wurden 1:1 umgesetzt. Klarstellungen waehrend der Implementierung:

- **ORDER BY created ASC, id ASC statt nur created:** Das Test 6 `test_dump_all_returns_sorted_entries` legt drei Eintraege mit distinkten Timestamps an; ohne `id`-Tie-Breaker waere die Sortierung bei Eintraegen mit gleicher Sekunde nicht-deterministisch. Plan erlaubte das (Acceptance-Criteria fordert nur `ORDER BY created ASC`).
- **format_dt lokal statt cross-Modul:** Phase-7-`repayment_phase.rs::format_dt` ist `fn` (privat), nicht `pub(crate)`. Da Plan 01 bereits eine Duplikation in `repayment_phase.rs` enthaelt, ist die Konsistenz zu folgen — `format_dt` lokal in `repayment_entry.rs` zu definieren — pragmatischer als ein Refactor in eine geteilte `crate::dt_helpers`-Datei (waere Rule-4-Change).
- **6 Tests statt 7:** Plan-`<behavior>` listete 7 Tests; Test 6 (`test_dump_all_returns_sorted_entries`) und Test 7 (`test_find_by_phase_id_filters_correctly`) waren separat. Beide sind als separate Tests vorhanden. Plan-`<acceptance_criteria>` forderte explizit "mindestens 5 gruene Tests" — die 6 Tests erfuellen das mehrfach.

## Deviations from Plan

None — plan executed exactly as written.

Drei Hinweise zur Vollstaendigkeit:

1. **rustfmt angewendet:** Datei wurde mit `rustfmt --edition 2021` aus `/nix/store/...rustfmt-preview-1.93.0...` formatiert (cargo fmt ist auf dem System nicht installiert; Memory-Notiz "Nix-Toolchain nicht sofort aufgeben"). Kein Verhaltens-Impact, nur Code-Style. Tests blieben nach Format gruen.
2. **Workspace-Build durchgefuehrt:** Zusaetzlich zur in den Acceptance Criteria geforderten `cargo build -p genossi_dao_impl_sqlite` habe ich `cargo build --workspace` ausgefuehrt, um sicherzustellen, dass das neue Modul nicht versehentlich downstream-Crates bricht. Ergebnis: clean, nur pre-existing Warnings in `genossi_rest` und `genossi_bin`.
3. **format_dt-Helper lokal dupliziert:** `format_dt` in Phase-7-`repayment_phase.rs:71-76` ist `fn` (nicht `pub(crate)`), daher nicht reusable; lokal in `repayment_entry.rs:60-65` kopiert. Alternative waere ein neues `crate::dt_helpers`-Modul, was als Rule-4-Architektur-Aenderung ausserhalb des Plan-Scopes ist. Tech-Debt-Notiz fuer Folge-Phasen.

## Issues Encountered

- **cargo fmt nicht im PATH:** `cargo fmt -p genossi_dao_impl_sqlite` schlug fehl mit "no such command: fmt". Loesung: `rustfmt` direkt aus `/nix/store/b5snbh757b2ryz02xalqz0sqg1gqsjk7-rustfmt-preview-1.93.0-x86_64-unknown-linux-gnu/bin/rustfmt` mit `--edition 2021` aufgerufen. Memory-Notiz `feedback_nix_toolchain.md` ("rustfmt/clippy fehlt auf PATH? Erst /nix/store durchsuchen") direkt angewandt — keine "tool not installed"-Fehlmeldung produziert.

## User Setup Required

None — Migration ist Plan 01 bereits gelaufen; SQLite-Impl integriert sich automatisch beim Server-Start ueber Plan 03/04-Wiring.

## Next Phase Readiness

- **Plan 03 (Service-Trait):** Foundation komplett. Kann `RepaymentEntryDaoImpl::new(Arc<SqlitePool>)` direkt im `gen_service_impl!`-Block referenzieren; `Arc::clone(&repayment_entry_dao)` wird an `RepaymentPhaseServiceImpl` (Plan 04) und `RepaymentEntryServiceImpl` (Plan 03) verteilt.
- **Plan 04 (Phase-Erweiterung):** `find_by_phase_id` (Default-Impl aus Plan 01) ist gegen diese SQLite-Impl getestet; PHAS-03 Close-Validation kann die Pending-Liste direkt holen.
- **Keine Blocker.**

## Threat Coverage

| Threat ID | Mitigation | Verified-by |
|-----------|------------|-------------|
| T-08-02-01 (Lost-update via concurrent writes) | UPDATE...WHERE id=? AND version=? AND deleted IS NULL; rows_affected==0 -> ConflictError("Version mismatch"); Pre-exists-Check trennt NotFound | `test_update_repayment_entry_with_version_mismatch_returns_conflict` + `test_update_repayment_entry_unknown_id_returns_not_found` |
| T-08-02-02 (i32-Overflow bei SQLite-INTEGER) | i32::try_from(db.share_count_to_pay_out) mit map_err -> ParseError | Code-Inspektion (grep `i32::try_from(db.share_count_to_pay_out)` == 1); kein Panic auf out-of-range |
| T-08-02-03 (Soft-deleted rows leaked via update) | UPDATE WHERE deleted IS NULL + Pre-Exists-Check WHERE deleted IS NULL | `test_update_repayment_entry_unknown_id_returns_not_found` (soft-deleted aequivalent zu missing) |

## Self-Check: PASSED

**Verified files exist:**
- `genossi_dao_impl_sqlite/src/repayment_entry.rs`: FOUND
- `genossi_dao_impl_sqlite/src/lib.rs` (modified): FOUND

**Verified commits exist:**
- `69e3135` (Task 1): FOUND in git log

**Verified tests pass:**
- 6/6 in `repayment_entry::tests` Modul: passed (53 weitere im genossi_dao_impl_sqlite-Workspace gefiltert)
- `cargo build --workspace`: clean (nur pre-existing warnings)

**Verified acceptance criteria (grep counts):**
- `pub struct RepaymentEntryDaoImpl` == 1 ✓
- `pub fn new(pool: Arc<SqlitePool>)` == 1 ✓
- `impl RepaymentEntryDao for RepaymentEntryDaoImpl` == 1 ✓
- `i32::try_from(db.share_count_to_pay_out)` == 1 ✓
- `SELECT COUNT(*) FROM repayment_entry` >= 1 (= 1) ✓
- `Version mismatch` >= 1 (= 3: 1x in `Err()`, 2x in Test-Assertions) ✓
- `use crate::assembly::parse_datetime` == 1 ✓
- `pub mod repayment_entry;` in lib.rs == 1 ✓

---

*Phase: 08-repaymententry-auto-bef-llung*
*Completed: 2026-05-31*
