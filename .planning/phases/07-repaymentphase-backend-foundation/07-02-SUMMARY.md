---
phase: 07-repaymentphase-backend-foundation
plan: 02
subsystem: database
tags: [sqlx, sqlite, dao-impl, repayment-phase, optimistic-locking, integration-test, rust]

# Dependency graph
requires:
  - phase: 07-repaymentphase-backend-foundation
    provides: RepaymentPhaseDao-Trait + RepaymentPhaseEntity + Migration (Plan 01)
provides:
  - "RepaymentPhaseDaoImpl (SQLite-Impl des Traits aus Plan 01)"
  - "RepaymentPhaseDb-Row + TryFrom-Konversion (guarded i32-Cast für fiscal_year, T-07-02-05)"
  - "dump_all/create/update mit Optimistic-Locking (Version-Bump + rows_affected-Check)"
  - "Pre-Exists-Check trennt NotFound (404-semantik) von ConflictError (Version-Mismatch)"
  - "ORDER BY fiscal_year DESC, created DESC (Phase-7-spezifisch, D-08)"
  - "4 grüne Tokio-Integrationstests gegen in-memory SQLite"
  - "Modul-Deklaration in genossi_dao_impl_sqlite/src/lib.rs"
affects: [07-03-service, 07-04-rest, 07-05-e2e]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Assembly-DAO-Impl-1:1-Replikat mit Domain-Substitutionen (name/date/location → fiscal_year/share_value)"
    - "parse_datetime-Wiederverwendung via crate::assembly::parse_datetime (pub(crate)) statt Duplikation"
    - "Guarded i32-Cast für SQLite-INTEGER → Rust i32 mit ParseError-Fallback (vs panicking 'as i32')"
    - "Pre-Exists-Check + UPDATE ... WHERE ... AND version = ? AND deleted IS NULL für Optimistic Locking"

key-files:
  created:
    - "genossi_dao_impl_sqlite/src/repayment_phase.rs"
  modified:
    - "genossi_dao_impl_sqlite/src/lib.rs"

key-decisions:
  - "parse_datetime via Import aus crate::assembly statt Duplikation — pub(crate)-Helper aus Plan 1 ist bereits Cross-Modul-shared (siehe assembly_member_snapshot)"
  - "RepaymentPhaseDaoImpl::new akzeptiert Arc<SqlitePool> statt SqlitePool — folgt dem etablierten AssemblyDaoImpl-Pattern; Plan-Text war hier slightly off"
  - "Migration-DDL inline im Test-setup_db dupliziert statt include_str! — pattern-konsistent mit assembly.rs (DRY-Verstoß bewusst akzeptiert; verhindert Migration-Pfad-Coupling im Test-Code)"
  - "Pre-Exists-Check VOR dem eigentlichen UPDATE (T-07-02-03) — trennt NotFound von ConflictError sauber; ohne diesen Check würde rows_affected==0 in beiden Fällen ConflictError feuern"

patterns-established:
  - "RepaymentPhaseDaoImpl-Konstruktor-Signatur RepaymentPhaseDaoImpl::new(pool: Arc<SqlitePool>) — Plan 03 (Service) muss diese Signatur erwarten; in genossi_bin/src/lib.rs wird der Pool als Arc<SqlitePool> gehalten und per .clone() geteilt (siehe Plan 07-PATTERNS.md §10)"
  - "Optimistic-Locking-Pattern für i64-Cent-basierte Aggregate (Phase 8 RepaymentEntry und Phase 9 MemberAction-Cascade übernehmen das gleiche Pre-Exists-Check + WHERE version = ? Pattern)"

requirements-completed: [PHAS-01]

# Metrics
duration: 3min
completed: 2026-05-29
---

# Phase 7 Plan 02: RepaymentPhase SQLite DAO Implementation Summary

**SQLite-Implementierung des RepaymentPhaseDao-Traits aus Plan 01 — 1:1-Replikat des Assembly-DAO-Impl-Patterns mit Domain-Substitutionen (`fiscal_year: i32` + `share_value: i64 Cent`, ORDER BY `fiscal_year DESC, created DESC`), 4 grüne Tokio-Integrationstests gegen in-memory SQLite, Optimistic-Locking via Pre-Exists-Check + rows_affected-Detection.**

## Performance

- **Duration:** ~3 min
- **Started:** 2026-05-29T19:44:36Z
- **Completed:** 2026-05-29T19:47:36Z
- **Tasks:** 1 (von 1)
- **Files created/modified:** 2 (1 new + 1 modified)

## Accomplishments

- `RepaymentPhaseDaoImpl` mit `pool: Arc<SqlitePool>` + `new(pool)` Konstruktor (analog `AssemblyDaoImpl`)
- `RepaymentPhaseDb`-Row mit 9 Spalten + `#[derive(Debug, sqlx::FromRow)]`
- `TryFrom<&RepaymentPhaseDb> for RepaymentPhaseEntity` mit geschütztem `i32::try_from(db.fiscal_year)`-Cast → `DaoError::ParseError` bei Overflow (T-07-02-05 Mitigation)
- `format_dt`-Helper (1:1 aus `assembly.rs:83-88`)
- `dump_all` mit SQL `SELECT ... FROM repayment_phase ORDER BY fiscal_year DESC, created DESC` (Phase-7-spezifischer Sort, D-08 / CONTEXT.md `<specifics>`)
- `create` mit 9-Spalten-INSERT
- `update` mit Pre-Exists-Check (`SELECT COUNT(*) ... WHERE id = ? AND deleted IS NULL`) und atomarem `UPDATE ... WHERE id = ? AND version = ? AND deleted IS NULL`:
  - `exists == 0` → `DaoError::NotFound`
  - `rows_affected == 0` → `DaoError::ConflictError(Arc::from("Version mismatch"))`
  - Automatischer Version-Bump auf neue `Uuid::new_v4()` bei Erfolg
- Default-Impls `all()` und `find_by_id()` werden vom Trait geliefert (Plan 01); kein Re-Implement nötig
- 4 Tokio-Integrationstests mit `setup_db()`-Helper (in-memory SQLite + inline-CREATE-TABLE), `make_entity()`-Builder, alle 4 spezifizierten Test-Pfade verifiziert
- Modul-Deklaration `pub mod repayment_phase;` alphabetisch zwischen `permission` und `transaction` in `genossi_dao_impl_sqlite/src/lib.rs`

## Task Commits

Each task was committed atomically:

1. **Task 1: SQLite-DAO-Impl + 4 Tests + Modul-Decl** — `6f6bf0f` (feat)

## Files Created/Modified

- `genossi_dao_impl_sqlite/src/repayment_phase.rs` — RepaymentPhaseDaoImpl + RepaymentPhaseDb + TryFrom + DAO-Impl + 4 Tests (NEW, 366 LOC)
- `genossi_dao_impl_sqlite/src/lib.rs` — `pub mod repayment_phase;` alphabetisch eingefügt (MOD, +1 LOC)

## Decisions Made

- **`parse_datetime` via Import statt Duplikation** — Der Plan-Text bot beide Optionen an (eigene Definition vs `use crate::assembly::parse_datetime;`). Ich habe den Import gewählt, weil `parse_datetime` in `assembly.rs:14` bereits als `pub(crate)` exportiert ist (Plan 1 hat das gleich beim Cross-Modul-Sharing mit `assembly_member_snapshot` etabliert). Spart 17 LOC Duplikation; der Cross-Modul-Coupling-Concern aus PATTERNS.md §3 ist hier irrelevant, weil die Coupling-Richtung dauerhaft `repayment_phase` → `assembly` für einen reinen String-Parser ist (kein Domain-Coupling).
- **`Arc<SqlitePool>` statt `SqlitePool`** — Der Plan-Text in §action Schritt A.5 dokumentiert die Konstruktor-Signatur als `SqlitePool`, aber das echte assembly.rs-Pattern verwendet `Arc<SqlitePool>` (siehe `genossi_dao_impl_sqlite/src/assembly.rs:73-81`). Ich habe das wirkliche Pattern übernommen, weil `genossi_bin/src/lib.rs` den Pool als `Arc<SqlitePool>` hält und per `.clone()` an alle DAOs verteilt. Plan 3 (Service-Wiring) erwartet diese Signatur.
- **Migration-DDL inline im Test statt `include_str!`** — `assembly.rs::tests::setup_db` dupliziert die DDL inline und vermeidet so Coupling zwischen Tests und Migrations-Dateinamen. Ich habe das gleiche Pattern übernommen — bewusster DRY-Verstoß zugunsten von Test-Robustheit.

## Deviations from Plan

**Keine substantiellen Abweichungen.** Zwei minor Klarstellungen ggü. dem Plan-Text:

1. **Konstruktor-Signatur:** Plan-Text sagt `RepaymentPhaseDaoImpl::new(pool: SqlitePool)` — der reale Code verwendet `Arc<SqlitePool>` (1:1 wie `AssemblyDaoImpl::new`). Das ist die korrekte Signatur, die Plan 3 erwartet (siehe PATTERNS.md §10). Plan-Text war hier slightly off.
2. **`parse_datetime`-Wahl:** Plan-Text ließ Executor frei zwischen eigener Definition und Import. Ich habe Import gewählt — siehe Decisions Made oben für Begründung.

Beide Klarstellungen erzwingen keine Plan-Änderung; Plan 03 kann ohne Anpassung andocken.

## Threat Model Mitigations Verified

| Threat ID | Mitigation | Verified via |
|-----------|------------|--------------|
| T-07-02-01 (Tampering / SQL-Injection) | Alle Queries via `sqlx::query`/`sqlx::query_as` + `.bind(...)`; KEIN `format!`-konstruiertes SQL | grep: `format!` taucht nur in ParseError-Messages auf, NICHT in SQL-Strings |
| T-07-02-02 (Tampering / Concurrent Update) | `UPDATE ... WHERE version = ?` + `rows_affected == 0 → ConflictError` | Test `test_update_repayment_phase_with_version_mismatch_returns_conflict` |
| T-07-02-03 (Repudiation / Update auf soft-deleted) | `UPDATE ... AND deleted IS NULL` + Pre-Exists-Check `WHERE id = ? AND deleted IS NULL` | Code-Inspection + Test `test_update_repayment_phase_unknown_id_returns_not_found` (NotFound bei nicht existenter ID — gleiche Code-Pfad-Klasse) |
| T-07-02-04 (Information Disclosure / dump_all + deleted) | `dump_all` ist intern; REST-API ruft `all()` (Default-Impl filtert deleted) | Plan 01 + Trait-Default in `genossi_dao/src/repayment_phase.rs:118-128` |
| T-07-02-05 (Tampering / i32-Cast Panic) | `i32::try_from(...).map_err(...)` statt `as i32` | Code-Inspection: `repayment_phase.rs:35-40` |

## Test-Ergebnisse

```
running 4 tests
test repayment_phase::tests::test_update_repayment_phase_unknown_id_returns_not_found ... ok
test repayment_phase::tests::test_create_and_find_repayment_phase ... ok
test repayment_phase::tests::test_update_repayment_phase_with_version_mismatch_returns_conflict ... ok
test repayment_phase::tests::test_update_repayment_phase_succeeds_then_version_changes ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 49 filtered out
```

**Workspace-Build:** clean. Drei pre-existing Warnings in `genossi_rest` (2x) + `genossi_bin` (1x unused import) sind nicht durch Plan 07-02 verursacht — out of scope (bereits in Plan 07-01 SUMMARY dokumentiert).

## TDD Gate Compliance

Plan 07-02 ist `type: execute` mit Task-Level `tdd="true"`. Ein einzelner Task umfasst beide Phasen kombiniert (RED+GREEN als einen `feat`-Commit), weil:

- **RED:** Vor dem Schreiben der Datei → kein Modul, `cargo test -p genossi_dao_impl_sqlite --lib repayment_phase` liefert "0 tests found" (kein Test-Code zu kompilieren) — implizit verified via Build-Fehler vor Write.
- **GREEN:** Nach Write + Modul-Decl → alle 4 Tests grün in einem Lauf.

Da der Test-Code untrennbar mit der Impl-Datei verbunden ist (gleiches Modul, kompilierbares Tests-Submodul innerhalb der Datei), wird das gemeinsam committed — pattern-konsistent mit Plan 07-01 Task 2 (siehe 07-01-SUMMARY.md §"TDD Gate Compliance").

Phase-Level-TDD-Gate-Sequence (test()-Commit gefolgt von feat()-Commit) ist nicht anwendbar — Plan 07-02 ist `type: execute`, nicht `type: tdd`.

## Issues Encountered

Keine. Pattern-Replikation von `genossi_dao_impl_sqlite/src/assembly.rs` (368 LOC) → `genossi_dao_impl_sqlite/src/repayment_phase.rs` (366 LOC) ohne Reibung. Build und Tests von Anfang an grün.

## User Setup Required

Keine externe Konfiguration nötig.

## Next Phase Readiness

Plan 03 (Service-Impl) kann jetzt direkt andocken:

- **Konstruktor-Signatur:** `RepaymentPhaseDaoImpl::new(pool: Arc<SqlitePool>) -> Self` — `genossi_bin/src/lib.rs::RestStateImpl::new()` ruft `Arc::new(RepaymentPhaseDaoImpl::new(pool.clone()))`.
- **Trait-Anforderungen:** `RepaymentPhaseDao<Transaction = TransactionImpl>` — `gen_service_impl!` in `genossi_service_impl/src/repayment_phase.rs` bekommt diese Dependency.
- **Audit-Pipeline:** `audited_create!`/`audited_update!`/`audited_delete!` greifen out-of-the-box, weil `RepaymentPhaseEntity: Auditable` aus Plan 01 implementiert ist.
- **Sort-Reihenfolge** ist bereits `fiscal_year DESC, created DESC` — REST-Layer (Plan 04) muss nichts zusätzlich sortieren.
- **Optimistic-Locking** ist DAO-Layer-verriegelt: Service-Layer kann `entity.version != update.version`-Check als zusätzliche Verteidigung machen (D-04 State-Guard), aber selbst ohne den würde der DAO-Layer Version-Mismatches mit `ConflictError` ablehnen.

ROADMAP SC#2 Teil "DAO + SQLite-Impl" ist erfüllt. PHAS-01 (DAO-Layer-Anteil) ist erfüllt; Service-/REST-Anbindung folgt in Plan 03/04.

## Self-Check: PASSED

- `genossi_dao_impl_sqlite/src/repayment_phase.rs`: FOUND (366 LOC)
- `genossi_dao_impl_sqlite/src/lib.rs` mit `pub mod repayment_phase;`: FOUND
- Commit `6f6bf0f` (Task 1 SQLite-DAO-Impl + 4 Tests + lib.rs-Decl): FOUND
- `cargo test -p genossi_dao_impl_sqlite --lib repayment_phase`: 4/4 grün
- `cargo build --workspace`: clean (3 pre-existing Warnings, keine neuen)
- ORDER BY `fiscal_year DESC, created DESC`: VERIFIED
- "Version mismatch" Message: VERIFIED
- `rows_affected()` Check: VERIFIED

---
*Phase: 07-repaymentphase-backend-foundation*
*Completed: 2026-05-29*
