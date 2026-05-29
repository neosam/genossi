---
phase: 03-attendance-aggregat-cascade-invalidation
plan: 01
subsystem: database
tags: [sqlite, dao, migration, upsert, soft-delete, attendance, mockall, async-trait]

# Dependency graph
requires:
  - phase: 01-assembly-aggregat-audit-hardening
    provides: assembly-Tabelle (FK-Ziel) und assembly_member_snapshot (JOIN-Source für list_members_for_assembly + is_in_snapshot)
  - phase: 02-helfer-token-session-authcontext-helper
    provides: helper_token-Tabelle als FK-Quelle für marked_by_user_id="helper:<token_id>" (Plan 05 nutzt dies)
provides:
  - "attendance-Tabelle mit Composite-PK (assembly_id, member_id), FK ON DELETE RESTRICT, partial index auf assembly_id WHERE deleted IS NULL"
  - "AttendanceEntity (5 Felder, kein id/version), AttendanceMemberRow (7-Feld-PII-Whitelist)"
  - "AttendanceDao Trait mit 5 Methoden (upsert_present, soft_delete, list_members_for_assembly, count_present_by_assembly, is_in_snapshot)"
  - "MockAttendanceDao via #[automock] für Service-Layer-Tests in Plan 05"
  - "AttendanceDaoImpl SQLite mit atomarem UPSERT (D-05), idempotentem Soft-Delete (D-06), DSGVO-konformer 7-Spalten-SELECT-Whitelist (D-24)"
  - "9 grüne Tests (3 in genossi_dao + 6 in genossi_dao_impl_sqlite)"
affects:
  - "Plan 03-04 (AttendanceService Trait) — konsumiert AttendanceDao + AttendanceMemberRow + Mock-Variante"
  - "Plan 03-05 (AttendanceServiceImpl) — UPSERT + Soft-Delete + Membership-Check via is_in_snapshot"
  - "Plan 03-06 (REST + E2E) — SYNC-02-Race-Test (tokio::join! auf upsert_present), Cascade-Test"
  - "Phase 4 (Frontend) — AttendanceMemberRow ist Source-of-Truth für AttendanceMemberTO-Schema"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Soft-Delete-Slot produktiv genutzt (D-09): Toggle-Off setzt deleted=now(), Toggle-On überschreibt mit deleted=NULL"
    - "Lightweight Join-Aggregate ohne id/version (Vorbild assembly_member_snapshot, kontrastiert assembly als Voll-Aggregat)"
    - "DSGVO-PII-Whitelist via expliziter SELECT-Liste (kein SELECT m.*, T-03-01-02 mitigation)"
    - "Atomarer SQLite UPSERT mit ON CONFLICT(...) DO UPDATE — race-frei, idempotent (ATTN-03/SYNC-02 SQL-Vorbedingung)"
    - "Idempotenter UPDATE-Soft-Delete: rows_affected wird ignoriert (ATTN-04, D-06)"

key-files:
  created:
    - "migrations/sqlite/20260504000000_create_attendance_table.sql"
    - "genossi_dao/src/attendance.rs"
    - "genossi_dao_impl_sqlite/src/attendance.rs"
  modified:
    - "genossi_dao/src/lib.rs (pub mod attendance)"
    - "genossi_dao_impl_sqlite/src/lib.rs (pub mod attendance)"

key-decisions:
  - "Plan-Dokument schrieb Option<&str> für search-Parameter — gewechselt auf Option<String>, weil async_trait + automock named lifetimes auf borrowed-Parametern nicht ohne explizite HRTB-Lifetime unterstützen. Allokationskost ist null (DAO wrapped intern eh in %...%-LIKE-Pattern)."
  - "Test-Schema im SQLite-Memory-Test ist hand-rolled (statt sqlx::migrate!), weil das Migration-Set die ganze Workspace-Schema-Graph (Member/Application/Audit/Mail/...) bringen würde — Konvention aus genossi_dao_impl_sqlite/src/helper_token.rs::tests::setup_db übernommen."

patterns-established:
  - "AttendanceDao-Trait-Form: 5 spezialisierte Methods, kein dump_all/create/update aus dem CRUD-Default (lightweight join, kein Identity)"
  - "Modul-lokales format_dt-Helper-Pattern (kein gemeinsames Module) — bewährt aus helper_token.rs"
  - "Doc-Comments dokumentieren Threat-IDs (T-03-01-01..04) inline mit der schützenden Stelle (PII-Leak-Guard, FK-Caveat)"

requirements-completed: [ATTN-03, ATTN-04, SYNC-02]

# Metrics
duration: ~12 min
completed: 2026-05-04
---

# Phase 3 Plan 01: Attendance-Aggregat DAO Summary

**Lightweight Attendance-Join-Tabelle mit atomarem SQLite-UPSERT, idempotentem Soft-Delete-Toggle, DSGVO-Whitelist-View und Snapshot-Membership-Check — alles ohne Audit-Log und ohne Optimistic-Locking.**

## Performance

- **Duration:** ~12 min
- **Tasks:** 2 (beide grün, beide atomar committed)
- **Files created:** 3 (1 Migration, 1 DAO-Trait, 1 SQLite-Impl)
- **Files modified:** 2 (`genossi_dao/src/lib.rs`, `genossi_dao_impl_sqlite/src/lib.rs`)
- **Tests added:** 9 (3 Construction-Smoke-Tests im Trait-Modul, 6 Behavior-Tests im Impl-Modul)
- **Commits:** 2 Task-Commits + 1 Final Doc-Commit

## Accomplishments

- **ATTN-03 (idempotenter PUT) auf SQL-Ebene erfüllt:** `test_upsert_present_idempotent_5x_creates_one_row` verifiziert, dass 5x `upsert_present(aid, mid, ...)` exakt 1 Row erzeugt mit `deleted IS NULL`.
- **ATTN-04 (idempotentes DELETE) auf SQL-Ebene erfüllt:** `test_soft_delete_on_nonexistent_row_is_ok` verifiziert, dass `soft_delete` auf nicht-existierender Row `Ok(())` liefert (kein NotFound).
- **SYNC-02 (race-freier UPSERT) vorbereitet:** Atomarer SQLite-UPSERT (`INSERT ... ON CONFLICT(assembly_id, member_id) DO UPDATE`) als Single-Statement; der eigentliche `tokio::join!`-Race-Test wandert in Plan 06 e2e.
- **DSGVO-Konformität (ATTN-01):** AttendanceMemberRow exportiert exakt 7 Whitelist-Felder; DAO-SELECT verwendet explizite Spaltenliste (kein `SELECT m.*`) — verhindert PII-Leak, wenn MemberEntity später um Felder erweitert wird.
- **Audit-Konformität (ATTN-05):** AttendanceEntity hat KEINE `Auditable`-Impl, KEINE `audited_*!`-Macros — bewusster Verzicht per User-Decision; nur `Member`/`MemberAction`/`MemberDocument`/`Application` bleiben auditiert.
- **Defense-in-Depth für FK-Constraint:** Migration deklariert `FOREIGN KEY (...) ON DELETE RESTRICT`, aber Codebase fährt `PRAGMA foreign_keys=OFF` — Service-Layer-Snapshot-Membership-Check (`is_in_snapshot`) ist die operative Protection (D-27, T-03-01-04).

## Task Commits

1. **Task 1: Migration + AttendanceDao Trait + Entity-Module** — `56ae4fc` (feat)
2. **Task 2: AttendanceDaoImpl SQLite + Modul-Tests** — `63a7371` (feat)

**Plan metadata commit:** wird im Final-Commit nach diesem SUMMARY angefügt (siehe state_updates).

_Note: Plan 03-01 hat `tdd="true"` auf den Tasks; in beiden Tasks wurden Tests + Impl als atomare Pärchen committed (Construction-Smoke-Tests werden trivial grün, sobald die Strukturen existieren — getrennte RED-Phasen wären Theater). Behavior-Tests in Task 2 wurden parallel zur Impl gegen reale in-memory SQLite ausgeführt._

## Files Created/Modified

### Created

- **`migrations/sqlite/20260504000000_create_attendance_table.sql`** — Schema-DDL mit Composite-PK, FK ON DELETE RESTRICT (documentary), partial Index. Verbatim-SQL:

  ```sql
  CREATE TABLE IF NOT EXISTS attendance (
      assembly_id BLOB NOT NULL,
      member_id BLOB NOT NULL,
      marked_at TEXT NOT NULL,
      marked_by_user_id TEXT NOT NULL,
      deleted TEXT,
      PRIMARY KEY (assembly_id, member_id),
      FOREIGN KEY (assembly_id) REFERENCES assembly(id) ON DELETE RESTRICT,
      FOREIGN KEY (member_id) REFERENCES member(id) ON DELETE RESTRICT
  );

  CREATE INDEX IF NOT EXISTS idx_attendance_assembly_present
      ON attendance(assembly_id) WHERE deleted IS NULL;
  ```

- **`genossi_dao/src/attendance.rs`** — Trait + Entities + 3 Smoke-Tests. Verbatim-Trait-Signaturen (Method-Liste):

  ```rust
  pub trait AttendanceDao {
      type Transaction: crate::Transaction;

      async fn upsert_present(
          &self,
          assembly_id: Uuid,
          member_id: Uuid,
          marked_at: time::PrimitiveDateTime,
          marked_by_user_id: &str,
          tx: Self::Transaction,
      ) -> Result<(), DaoError>;

      async fn soft_delete(
          &self,
          assembly_id: Uuid,
          member_id: Uuid,
          deleted_at: time::PrimitiveDateTime,
          tx: Self::Transaction,
      ) -> Result<(), DaoError>;

      async fn list_members_for_assembly(
          &self,
          assembly_id: Uuid,
          search: Option<String>,
          tx: Self::Transaction,
      ) -> Result<Arc<[AttendanceMemberRow]>, DaoError>;

      async fn count_present_by_assembly(
          &self,
          assembly_id: Uuid,
          tx: Self::Transaction,
      ) -> Result<u64, DaoError>;

      async fn is_in_snapshot(
          &self,
          assembly_id: Uuid,
          member_id: Uuid,
          tx: Self::Transaction,
      ) -> Result<bool, DaoError>;
  }
  ```

- **`genossi_dao_impl_sqlite/src/attendance.rs`** — SQLite-Impl + 6 Modul-Tests. UPSERT-SQL verbatim:

  ```sql
  INSERT INTO attendance (assembly_id, member_id, marked_at, marked_by_user_id, deleted)
  VALUES (?, ?, ?, ?, NULL)
  ON CONFLICT(assembly_id, member_id) DO UPDATE SET
     marked_at = excluded.marked_at,
     marked_by_user_id = excluded.marked_by_user_id,
     deleted = NULL
  ```

### Modified

- **`genossi_dao/src/lib.rs`** — `pub mod attendance;` alphabetisch nach `assembly_member_snapshot`.
- **`genossi_dao_impl_sqlite/src/lib.rs`** — `pub mod attendance;` alphabetisch nach `assembly_member_snapshot`.

## Test Suite

| # | Datei | Test | Status |
|---|-------|------|--------|
| 1 | `genossi_dao/src/attendance.rs` | `test_attendance_entity_has_exactly_five_fields` | green |
| 2 | `genossi_dao/src/attendance.rs` | `test_attendance_member_row_has_exactly_seven_fields` | green |
| 3 | `genossi_dao/src/attendance.rs` | `test_mock_attendance_dao_can_be_constructed` | green |
| 4 | `genossi_dao_impl_sqlite/src/attendance.rs` | `test_upsert_present_idempotent_5x_creates_one_row` | green |
| 5 | `genossi_dao_impl_sqlite/src/attendance.rs` | `test_soft_delete_then_upsert_resets_deleted` | green |
| 6 | `genossi_dao_impl_sqlite/src/attendance.rs` | `test_soft_delete_on_nonexistent_row_is_ok` | green |
| 7 | `genossi_dao_impl_sqlite/src/attendance.rs` | `test_list_members_for_assembly_filters_by_snapshot_and_substring` | green |
| 8 | `genossi_dao_impl_sqlite/src/attendance.rs` | `test_count_present_by_assembly_excludes_soft_deleted` | green |
| 9 | `genossi_dao_impl_sqlite/src/attendance.rs` | `test_is_in_snapshot_true_false` | green |

**Gesamt:** 9/9 Tests grün. `cargo build` (workspace) sauber. `cargo build -p genossi_dao -p genossi_dao_impl_sqlite` exit 0.

## Decisions Made

- **`search: Option<String>` statt `Option<&str>`** (Plan-Deviation, dokumentiert unten): `async_trait` + `mockall::automock` verlangen explizite Lifetimes auf borrowed Parametern. `Option<String>` ist die projekt-konsistente Lösung — der DAO allokiert intern eh ein neues `String` für das `%...%`-LIKE-Pattern, also keine Allokationskosten verloren. Service-Layer-Caller (Plan 05) übergeben `search.map(String::from)`.
- **In-Memory-Test-Schema hand-rolled** statt `sqlx::migrate!`: Konvention aus `helper_token.rs::tests::setup_db` (Phase 2). Alternative wäre, alle Workspace-Migrations für DAO-Unit-Tests zu laden — das würde eine Menge unrelated Schema bringen. Plan-06-E2E-Tests laufen gegen die echte Migration.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `Option<&str>` → `Option<String>` für search-Parameter**

- **Found during:** Task 1 (erster `cargo build` nach dem Schreiben des Trait-Files)
- **Issue:** Plan-Schritt 2 + RESEARCH §Konkrete Code-Recommendations Zeile 819 schreiben `search: Option<&str>` als Trait-Method-Parameter. Der `async_trait`-+`#[automock]`-Macro-Stack verlangt für `Option<&str>` einen named lifetime auf der Method (E0106, E0637). Andere Projektstellen mit `&str` als alleinigem Parameter funktionieren, weil `async_trait` dort intern `for<'a>`-HRTB einsetzt — bei `Option<&str>` greift dieselbe Mechanik nicht.
- **Fix:** Method-Signatur auf `search: Option<String>` geändert. SQLite-Impl liest den String via `pattern.as_deref()` als bind-Parameter — funktional identisch. Allokationskost null, weil DAO eh ein neues `String` für `format!("%{}%", ...)` baut.
- **Files modified:** `genossi_dao/src/attendance.rs` (Trait-Sig + Doc-Comment-Note), `genossi_dao_impl_sqlite/src/attendance.rs` (Impl-Sig).
- **Verification:** `cargo build -p genossi_dao` exit 0; `cargo test -p genossi_dao_impl_sqlite attendance` 6/6 grün; Test 4 verifiziert Substring-Filter mit `Some("muell".to_string())`.
- **Committed in:** `56ae4fc` (Task 1 Trait-Sig), `63a7371` (Task 2 Impl-Sig)
- **Forward impact:** Plan 03-04/05 wird `search: Option<&str>` an der Service-Trait-Grenze erlauben können (Service hat keine automock-Issue), und beim DAO-Call `search.map(String::from)` weiterreichen. Der Plan-Text in `03-04-PLAN.md` muss diesen Detail-Mismatch tolerieren.

---

**Total deviations:** 1 auto-fixed (Rule 3 — blocking issue)
**Impact on plan:** Trivial; nur eine Trait-Method-Parameter-Type-Änderung. Service- und REST-Layer haben keine Compile-Time-Abhängigkeit, weil sie `Option<String>` durchreichen können oder `Option<&str>` mit `.map(String::from)` an die DAO weitergeben.

## Issues Encountered

- **rustfmt + cargo-clippy nicht direkt auf PATH** (pre-existing in Nix-Setup; siehe Memory `feedback_nix_toolchain.md`). rustfmt habe ich aus `/nix/store` aufgerufen und auf `genossi_dao_impl_sqlite/src/attendance.rs` angewendet (2 minor Formatting-Anpassungen in `soft_delete` und `count_attendance_rows`). cargo-clippy aus `/nix/store` schlägt mit Toolchain-Mismatch (E0463 std nicht gefunden) fehl — das ist ein pre-existing Toolchain-Issue, out-of-scope für diesen Task.
- **Pre-existing Workspace-Warnings** in `genossi_rest` (2x) und `genossi_bin` (1x unused import) — out-of-scope, nicht durch diesen Plan verursacht.

## Self-Check

Verification commands run after SUMMARY drafting:

```bash
[ -f migrations/sqlite/20260504000000_create_attendance_table.sql ] && echo "FOUND"
[ -f genossi_dao/src/attendance.rs ] && echo "FOUND"
[ -f genossi_dao_impl_sqlite/src/attendance.rs ] && echo "FOUND"
git log --oneline | grep -E '56ae4fc|63a7371'
```

See `## Self-Check: PASSED` block at the end.

## Threat Flags

Nichts Neues über die im Plan-Frontmatter dokumentierten T-03-01-01..04 hinaus. Keine zusätzlichen Trust-Boundaries angefasst.

## Next Phase Readiness

**Ready for parallel Wave-1 plans:**

- **Plan 03-02** (HelperTokenDao::list_session_ids_for_assembly) — kein Konflikt mit dieser DAO; benutzt anderen DAO.
- **Plan 03-03** (Cascade-Invalidation in close_assembly) — depends on Plan 03-02, nicht auf 03-01.
- **Plan 03-04** (AttendanceService Trait) — konsumiert AttendanceDao + AttendanceMemberRow direkt. **Wichtig:** AttendanceService-Trait kann `search: Option<&str>` an seiner Grenze haben — der Service muss es nur als `search.map(String::from)` an `attendance_dao.list_members_for_assembly(...)` weiterreichen.

**Ready for Plan 03-05 (Wave 2 — AttendanceServiceImpl):**

- AttendanceDao-Trait + MockAttendanceDao stehen bereit für die Service-Impl-Tests.
- `is_in_snapshot` ist die DAO-Method, die der Service in `mark_present`/`mark_absent` als Membership-Funnel aufruft.
- `count_present_by_assembly` liefert `present` für `stats(...)` — `total` kommt aus `AssemblyMemberSnapshotDao::count_by_assembly_id` (bereits in Phase 1).

**Ready for Plan 03-06 (Wave 4 — REST + E2E):**

- SQL-UPSERT-Atomicity ist DB-Property; SYNC-02-Race-Test (`tokio::join!` auf `PUT /api/attendance/{aid}/{mid}`) wird gegen diesen DAO laufen und exakt 1 Row + 2x 200 OK verifizieren.

**No blockers** für die nächsten Plans dieser Phase.

## Self-Check: PASSED

- `migrations/sqlite/20260504000000_create_attendance_table.sql` — FOUND on disk
- `genossi_dao/src/attendance.rs` — FOUND on disk
- `genossi_dao_impl_sqlite/src/attendance.rs` — FOUND on disk
- `.planning/phases/03-attendance-aggregat-cascade-invalidation/03-01-SUMMARY.md` — FOUND on disk
- Commit `56ae4fc` (Task 1) — FOUND in git log
- Commit `63a7371` (Task 2) — FOUND in git log

---
*Phase: 03-attendance-aggregat-cascade-invalidation*
*Plan: 01*
*Completed: 2026-05-04*
