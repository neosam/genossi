---
phase: 08-repaymententry-auto-bef-llung
plan: 01
subsystem: database
tags: [sqlite, sqlx, rust, dao, auditable, repayment-entry, migration]

requires:
  - phase: 07-repaymentphase-backend-foundation
    provides: "RepaymentPhase-Schema, Auditable-Trait, repayment_phase-Tabelle als FK-Ziel, Phase-7-DAO-Pattern als 1:1-Vorlage"

provides:
  - "Schema-Migration migrations/sqlite/20260530203550_create_repayment_entry_table.sql (8 Spalten, 3 Indizes, CHECK(share_count_to_pay_out > 0))"
  - "RepaymentEntryStatus-Enum mit allen 3 Varianten {Open, Contacted, PaidOut} von Anfang an (D-05)"
  - "RepaymentEntryEntity mit Auditable-Impl (entity_type='repayment_entry', 4 frozen audit_fields)"
  - "RepaymentEntryDao-Trait mit dump_all/create/update + Default-Impls all/find_by_id/find_by_phase_id"
  - "MockRepaymentEntryDao via #[automock] für Service-Layer-Unit-Tests"

affects:
  - "08-02 (SQLite-Impl): RepaymentEntryDaoImpl baut auf Trait + Entity auf"
  - "08-03 (Service-Trait): RepaymentEntryService nutzt DAO-Trait + Status-Enum"
  - "08-04 (RepaymentPhase-Erweiterung): open_phase Auto-Fill + close_phase Pending-Validation nutzen find_by_phase_id"
  - "08-05 (REST-Handler): TOs leiten von RepaymentEntryEntity ab"
  - "08-06 (E2E-Tests): Migration läuft auf Test-DB"
  - "09 (PAYO): Phase-9 hängt MemberAction-Cascade an entity_id+version pro Entry; Status-Toggle PaidOut nutzt selbe Enum"

tech-stack:
  added: []
  patterns:
    - "Auditable mit FROZEN audit_fields-Reihenfolge (Hash-Chain-Konsistenz, Phase-7-Lektion)"
    - "DAO-Default-Impls für Domain-Filter (find_by_phase_id) über dump_all + In-Memory-Filter"
    - "Status-Enum mit allen Phase-9-Varianten von Anfang an, um spätere DB-Migration zu vermeiden (D-05)"

key-files:
  created:
    - "migrations/sqlite/20260530203550_create_repayment_entry_table.sql"
    - "genossi_dao/src/repayment_entry.rs"
  modified:
    - "genossi_dao/src/lib.rs (Modul-Deklaration alphabetisch vor repayment_phase eingefügt)"

key-decisions:
  - "Status-Enum (Open/Contacted/PaidOut) komplett in Phase 8 angelegt — Phase 9 braucht keine DB-Migration mehr (D-05)"
  - "Audit-Felder-Reihenfolge im Test eingefroren — Reihenfolge-Änderung würde Hash-Chain historischer Einträge brechen (Phase-7-Plan-01-Lektion)"
  - "find_by_phase_id als DAO-Default-Impl über dump_all (statt SQL-WHERE im Sub-DAO) — Phase-7-Konvention für Domain-Filter; SQL-Optimierung wäre Premature-Optimization"
  - "CHECK(share_count_to_pay_out > 0) auf Schema-Ebene — Verteidigung in Depth zusätzlich zur Service-Layer-Validierung (T-08-01-03)"
  - "KEIN UNIQUE-Constraint auf (member_id, phase_id) — ENTR-03 erlaubt mehrere Einträge (z.B. Teil-Abtretungen, Korrekturen nach Erst-Auto-Fill)"

patterns-established:
  - "FROZEN-Audit-Fields-Pattern: dedizierter Test test_auditable_fields_count_and_excludes_metadata friert Reihenfolge ein"
  - "DAO-Default-Impl-Pattern für phase-skopierte Listings: find_by_phase_id via dump_all + filter"
  - "Migration-Kommentar-Pattern: WR-03-FK-Doku als Header-Kommentar (kopiert aus Phase-3-attendance-Migration)"

requirements-completed: [ENTR-01, ENTR-03]

duration: 6min
completed: 2026-05-31
---

# Phase 08 Plan 01: RepaymentEntry Schema + DAO-Foundation Summary

**Migration für `repayment_entry`-Tabelle plus DAO-Trait, Entity, Auditable-Impl und drei Default-Methoden — Foundation für alle nachfolgenden Phase-8-Plans (Service, REST, Phase-Erweiterung, E2E).**

## Performance

- **Duration:** ~6 min
- **Started:** 2026-05-31T03:51:08Z
- **Completed:** 2026-05-31T03:56:45Z
- **Tasks:** 2/2 abgeschlossen
- **Files created:** 2 (Migration-SQL, DAO-Modul)
- **Files modified:** 1 (lib.rs Modul-Deklaration)

## Accomplishments

- Migration legt `repayment_entry`-Tabelle mit 8 Pflichtspalten, 3 Indizes (phase_id, (phase_id, status), deleted) und schemaseitigem CHECK-Constraint an
- Status-Enum `RepaymentEntryStatus { Open, Contacted, PaidOut }` mit englischen Strings, `Default = Open`, `as_str`/`from_str`-Roundtrip
- `RepaymentEntryEntity` implementiert `Auditable` mit **frozen** 4-Feld-Reihenfolge (member_id, phase_id, share_count_to_pay_out, status) — Hash-Chain-Konsistenz garantiert per Unit-Test
- `RepaymentEntryDao`-Trait mit 3 Pflicht-Methoden + 3 Default-Impls (`all`, `find_by_id`, `find_by_phase_id`) — bereit für SQLite-Impl in Plan 02 und Service-Konsumption in Plan 03/04
- 8 Unit-Tests grün (Plan forderte mindestens 7)

## Task Commits

Jede Task wurde atomar committed:

1. **Task 1: Migration-SQL für repayment_entry-Tabelle** — `6896c93` (feat)
2. **Task 2: DAO-Trait + Entity + Auditable in genossi_dao/src/repayment_entry.rs** — `2b348bd` (feat)

**Plan metadata:** _(folgt mit diesem Commit)_

## Files Created/Modified

- `migrations/sqlite/20260530203550_create_repayment_entry_table.sql` — DDL mit CHECK, 3 Indizes, dokumentarischen FKs
- `genossi_dao/src/repayment_entry.rs` — Status-Enum, Entity, Auditable-Impl, DAO-Trait mit Mock, 8 Tests
- `genossi_dao/src/lib.rs` — `pub mod repayment_entry;` alphabetisch vor `repayment_phase`

## Decisions Made

Alle wesentlichen Decisions kamen aus `08-CONTEXT.md` (D-02, D-05, ENTR-03) und `08-PATTERNS.md` §1/§2 und wurden 1:1 umgesetzt. Keine zusätzlichen Architektur-Entscheidungen während der Execution.

Klarstellungen während der Implementierung:

- **Audit-Test eingefroren als zweistufige Garantie:** Erstens `test_auditable_fields_count_and_excludes_metadata` (Länge=4, exakte Names-Liste, keine Metadaten); zweitens zusätzlicher Test `test_auditable_audit_fields_member_id_first_phase_id_second` als Index-basierte Reihenfolge-Garantie (verteidigt gegen Refactorings, die nur die Reihenfolge tauschen aber Länge und Namen erhalten).
- **CHECK-Constraint statt nur Service-Validation:** Auf Schema-Ebene zusätzlich zur Service-Layer-Validierung — verteidigt gegen Daten-Korruption durch direkte DB-Zugriffe (z.B. Migrations, manuelle Korrekturen).

## Deviations from Plan

None — plan executed exactly as written.

Drei Hinweise zur Vollständigkeit:

1. **8 Tests statt 7:** Plan forderte „mind. 7 Tests grün"; ich habe einen zusätzlichen Index-basierten Reihenfolge-Test (`test_auditable_audit_fields_member_id_first_phase_id_second`) als doppelte Sicherung gegen Reihenfolge-Refactorings ergänzt. Das ist additiv und ändert das Plan-Verhalten nicht.
2. **Rustfmt angewendet:** Datei wurde mit `rustfmt --edition 2021` formatiert (cargo fmt ist auf dem System nicht installiert; rustfmt-binary aus `/nix/store` genutzt gemäß Memory-Notiz „Nix-Toolchain nicht sofort aufgeben"). Kein Verhaltens-Impact, nur Code-Style.
3. **Workspace-Build durchgeführt:** Zusätzlich zur in den Acceptance Criteria geforderten `cargo build -p genossi_dao` habe ich `cargo build --workspace` ausgeführt, um sicherzustellen, dass das neue Modul nicht versehentlich downstream-Crates bricht. Ergebnis: clean, nur pre-existing Warnings.

## Issues Encountered

- **`grep -ci "UNIQUE"` matched Kommentar:** Die initiale Acceptance-Criteria-Prüfung lieferte `grep -ci "UNIQUE.*member_id" "$M"` == 1, weil mein Header-Kommentar das Wort „UNIQUE" enthält. Habe per `grep -v "^--"` die SQL-Statements ohne Kommentare gefiltert: 0 UNIQUE-Constraints im tatsächlichen DDL — Erfüllung ENTR-03 verifiziert.
- **SQLite-direkter CHECK-Test:** Ergänzend zur grep-Verifikation habe ich `sqlite3` gegen eine Temp-DB den CHECK-Constraint mit `INSERT VALUES(..., 0, ...)` und `INSERT VALUES(..., -5, ...)` getestet — beide korrekt mit „CHECK constraint failed" abgelehnt; Default-Status `'Open'` per `INSERT` ohne Status-Spalte verifiziert.

## User Setup Required

None — Migration läuft automatisch beim Server-Start via `sqlx::migrate!`. Keine externen Service-Konfigurationen, keine Environment-Variablen, keine manuellen Schritte.

## Next Phase Readiness

- **Plan 02 (SQLite-Impl):** Foundation komplett. Plan 02 braucht: Migration läuft (✓), DAO-Trait + Entity (✓), Status-Enum mit `as_str`/`from_str` für SQL-Roundtrip (✓), `parse_datetime`-Helper aus `crate::assembly` wiederverwendbar (vorhanden in Phase-7-DAO-Impl).
- **Plan 03 (Service-Trait):** Kann `RepaymentEntryDao`-Trait + `MockRepaymentEntryDao` direkt importieren. `find_by_phase_id` ist Default-Impl, also auf jedem Mock automatisch verfügbar.
- **Plan 04 (Phase-Erweiterung):** `RepaymentPhaseServiceImpl`-Deps müssen um `RepaymentEntryDao` erweitert werden — Trait-Identität ist jetzt stabil.
- **Keine Blocker.**

## Self-Check: PASSED

**Verified files exist:**
- `migrations/sqlite/20260530203550_create_repayment_entry_table.sql`: FOUND
- `genossi_dao/src/repayment_entry.rs`: FOUND
- `genossi_dao/src/lib.rs` (modified): FOUND

**Verified commits exist:**
- `6896c93` (Task 1): FOUND in git log
- `2b348bd` (Task 2): FOUND in git log

**Verified tests pass:**
- 8/8 in `repayment_entry::tests` modul: passed

---

*Phase: 08-repaymententry-auto-bef-llung*
*Completed: 2026-05-31*
