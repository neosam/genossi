---
phase: 14-dao-domain-foundation
plan: 02
subsystem: dao
tags: [dao, repayment_entry, sqlite, trsf-06, foundation]
requires:
  - phase 7 RepaymentEntry DAO trait + SQLite impl
provides:
  - RepaymentEntryDao::find_by_member_and_phase trait method (default-impl via dump_all + filter)
  - SQLite SQL-override with WHERE member_id = ? AND phase_id = ? AND deleted IS NULL
  - foundation for Phase-16 sum-check + auto-fill-skip pattern (PITFALLS Kat 1)
affects:
  - genossi_dao/src/repayment_entry.rs
  - genossi_dao_impl_sqlite/src/repayment_entry.rs
tech_stack:
  added: []
  patterns:
    - Default-Impl + SQLite-Override (analog find_by_phase_id Phase 8)
    - Hand-rolled TestRepaymentEntryDao stub for default-impl test (mockall override pitfall)
    - SQLite BLOB binding via Uuid::as_bytes().to_vec()
    - ORDER BY created ASC, id ASC for deterministic test ordering
key_files:
  created: []
  modified:
    - genossi_dao/src/repayment_entry.rs
    - genossi_dao_impl_sqlite/src/repayment_entry.rs
decisions:
  - D-14-08: SQL-Override in SQLite-Impl plus Default-Impl im Trait (beide vorhanden)
  - D-14-09: Return-Type Arc<[RepaymentEntryEntity]> (cheap-clone, codebase-konvention)
metrics:
  duration_minutes: 18
  tasks_completed: 2
  files_modified: 2
  tests_added: 3
  loc_added: 272
completed: 2026-06-04
requirements:
  - TRSF-06
---

# Phase 14 Plan 02: DAO find_by_member_and_phase Summary

**One-liner:** Adds `RepaymentEntryDao::find_by_member_and_phase(member_id, phase_id, tx) -> Arc<[RepaymentEntryEntity]>` as foundation for Phase-16 sum-check + auto-fill-skip pattern, with default-impl in the trait and SQL-override in SQLite for scaling.

## Was geliefert wurde

### Trait-Methode (genossi_dao/src/repayment_entry.rs)

- **`async fn find_by_member_and_phase(member_id, phase_id, tx)`** als Trait-Methode mit Default-Impl
- Default-Impl filtert via `dump_all().filter(member == m AND phase == p AND deleted.is_none())`
- Ausführlicher Doc-Comment mit (1) Foundation-Hinweis für Phase-16-Sum-Check + Auto-Fill-Skip-Pattern (PITFALLS Kat 1), (2) Mockall-Override-Warnung (Service-Tests in Plan 14-03 müssen `.expect_find_by_member_and_phase()` explizit setzen, weil `#[automock]` Default-Impls überschreibt)

### SQL-Override (genossi_dao_impl_sqlite/src/repayment_entry.rs)

- `RepaymentEntryDaoImpl::find_by_member_and_phase` überschreibt mit direktem SQL:
  ```sql
  SELECT id, member_id, phase_id, share_count_to_pay_out, status, created, deleted, version
  FROM repayment_entry
  WHERE member_id = ? AND phase_id = ? AND deleted IS NULL
  ORDER BY created ASC, id ASC
  ```
- Column-Liste **wurde verbatim aus `dump_all` (Z. 78-81) übernommen** — `RepaymentEntryDb`-Row-Mapping bleibt konsistent
- BLOB-Binding via `member_id.as_bytes().to_vec()` und `phase_id.as_bytes().to_vec()` (analog `dump_all`)
- `ORDER BY created ASC, id ASC` liefert deterministische Reihenfolge (Phase-8-Plan-08-02-Lektion über Tie-Breaker bei gleicher Sekunde)

## Test-Ergebnisse

### genossi_dao (Trait-Modul)

| Test | Result |
|------|--------|
| `test_find_by_member_and_phase_default_impl_filters_correctly` | passed |

**Setup:** Hand-rolled `TestRepaymentEntryDao`-Stub im `tests`-Modul implementiert `RepaymentEntryDao` minimal (nur `dump_all`; `create`/`update` sind `unimplemented!`). Default-Impl-Pfad wird gegen 4 Eintraege getestet: `(member_A, phase_X)`, `(member_A, phase_Y)`, `(member_B, phase_X)`, `(member_A, phase_X, deleted=Some(...))`. Erwartung: genau 1 Survivor (e1). Zusätzlich Empty-Input-Edge-Case: `dump_all() -> []` liefert `Arc<[]>` der Länge 0.

Mockall-Falle bewusst umgangen — `#[automock]` würde Default-Impl überschreiben (Phase-3-Plan-03-Lektion, dokumentiert in `repayment_phase.rs:976-989`); hand-rolled Stub ist die canonische Lösung für trait-level Default-Impl-Tests.

### genossi_dao_impl_sqlite (SQLite-Override)

| Test | Result |
|------|--------|
| `test_find_by_member_and_phase_returns_empty_when_no_match` | passed |
| `test_find_by_member_and_phase_filters_correctly` | passed |

**Test 1 (empty):** Echte in-memory SQLite-DB; legt einen unrelated Eintrag an `(random_member, random_phase)` an. Query auf `(target_member, target_phase)` → 0 Eintraege. Verifiziert, dass die WHERE-Klausel wirklich filtert (nicht "alle Eintraege" liefert).

**Test 2 (multi-entry):** 4 Eintraege im Kreuzprodukt (m_A,p_X), (m_A,p_Y), (m_B,p_X), (m_A,p_X). Query auf `(m_A, p_X)` → exakt 2 Survivors (e1 + e4). Verifiziert für jeden zurückgegebenen Eintrag: `member_id == m_a`, `phase_id == p_x`, `deleted.is_none()`.

### Regression

- `cargo test -p genossi_dao --lib repayment_entry`: **9 passed, 0 failed** (8 existierende + 1 neu)
- `cargo test -p genossi_dao_impl_sqlite --lib repayment_entry`: **8 passed, 0 failed** (6 existierende + 2 neu)
- `cargo build -p genossi_dao -p genossi_dao_impl_sqlite`: success
- `cargo clippy -p genossi_dao -p genossi_dao_impl_sqlite --all-targets`: clean (keine Warnings)
- `rustfmt --check`: passed für beide Dateien (rustfmt aus `/nix/store` per Memory-Lektion)

## Doc-Comment-Verifikation

Bestätigt: Doc-Comment auf der Trait-Methode `find_by_member_and_phase` enthält explizit den Mockall-Pitfall-Warnung-Block:

```
/// **Mockall-Hinweis:** `#[automock]` ueberschreibt Default-Impls,
/// daher muessen Service-Unit-Tests `.expect_find_by_member_and_phase()`
/// explizit setzen.
```

Plan 14-03 wird im Service-Layer `MockRepaymentEntryDao` einsetzen — dieser Doc-Comment ist die direkte Anker-Referenz für die `.expect_*()`-Calls dort.

## Column-List-Verbatim-Bestaetigung

Verifiziert via diff zwischen `dump_all` (Z. 78-81) und `find_by_member_and_phase`:

```
SELECT id, member_id, phase_id, share_count_to_pay_out, status, created, deleted, version FROM repayment_entry
```

Beide Stellen verwenden die identische Spalten-Reihenfolge. `RepaymentEntryDb::FromRow` mappt diese 8 Spalten in der gleichen Reihenfolge → `try_from(&RepaymentEntryDb)` bleibt funktionsfähig.

## Deviations from Plan

**None — Plan executed exactly as written.**

- Doc-Comment-Inhalt: 1:1 wie im Plan-Action-Block vorgegeben (Foundation Phase-16, PITFALLS Kat 1, Mockall-Hinweis).
- SQL-String: 1:1 wie im `<interfaces>`-Block (column list + WHERE + ORDER BY).
- Test-Namen: 1:1 wie im Plan vorgegeben.
- Test-Setup: 1:1 wie `<action>`-Block (sample_entity-Helper-Reuse, kein neuer Helper).

## Auth-Gates / Checkpoints

None — Plan 14-02 ist autonomer DAO-Layer-Plan ohne externe Auth.

## Phase-16-Vorbereitung

- Phase 16 (Teil-Rueckgabe) wird `find_by_member_and_phase` an zwei Stellen aufrufen:
  1. **Sum-Check** (`sum(open_entries.share_count_to_pay_out for (member_id, phase_id)) + n <= member.current_shares`) — verhindert Doppelbuchung
  2. **Auto-Fill-Skip-Pattern** (`if find_by_member_and_phase(member, phase, tx).await? .iter().any(|e| e.status != PaidOut) { skip }`) — verhindert Doppel-Anlage beim Auto-Fill
- Beide Pfade sind dokumentiert im Trait-Doc-Comment und im SQL-Override-Inline-Kommentar als Foundation-Anker

## Self-Check

### Created files exist
- N/A — keine neuen Dateien (nur Modifikationen)

### Modified files exist
- `[FOUND]` `genossi_dao/src/repayment_entry.rs` (446 LOC, +154)
- `[FOUND]` `genossi_dao_impl_sqlite/src/repayment_entry.rs` (536 LOC, +118)

### Commits exist
- `[FOUND]` `c347567` feat(14-02): add RepaymentEntryDao::find_by_member_and_phase trait method
- `[FOUND]` `0182f34` feat(14-02): add SQL-override + 2 SQLite tests for find_by_member_and_phase

## Self-Check: PASSED
