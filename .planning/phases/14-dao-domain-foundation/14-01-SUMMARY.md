---
phase: 14-dao-domain-foundation
plan: 01
subsystem: service-foundation
tags: [v1.2, CANC-02, pure-function, h1-h2, foundation]
requires: []
provides:
  - "membership_adjust::compute_effective_date Pure-Function (pub(crate))"
  - "membership_adjust::EffectiveDate Struct (pub(crate)) mit fiscal_year + effective_date"
affects:
  - genossi_service_impl
tech_stack_added: []
patterns_used:
  - "Pure-Function pub(crate) + #[cfg(test)] mod tests (Analog: member_action::compute_dates)"
  - "Struct-Return mit Copy-Derive (D-14-01)"
  - "Verbands-Konvention im ///-Doc-Kommentar verankert (D-14-07)"
key_files_created:
  - genossi_service_impl/src/membership_adjust.rs
key_files_modified:
  - genossi_service_impl/src/lib.rs
decisions:
  - "D-14-01..03 angewendet: Struct EffectiveDate, neue Datei membership_adjust.rs, pub(crate)-Sichtbarkeit"
  - "D-14-04..07 angewendet: month <= 6 für H1, 31.12.→H2-next-year, Schaltjahr-Test, ausführlicher Doc-Kommentar"
  - "Module-Registrierung in alphabetischer Position (nach member_import, vor pdf_generation) ohne pub use Re-Export (D-14-03)"
metrics:
  duration_minutes: 11
  tasks_completed: 2
  files_touched: 2
  tests_added: 6
  tests_passing: 6
completed: 2026-06-04
---

# Phase 14 Plan 01: Pure-Function compute_effective_date Summary

**One-liner:** H1/H2-Stichtag-Berechnung als pure `compute_effective_date(Date) -> EffectiveDate` mit Struct-Return, 6 Edge-Case-Tests und verbands-konformem Doc-Kommentar — Foundation für Phase 15-17 Kündigung/Teil-Rückgabe/Übertrag.

## What Was Built

Plan 14-01 liefert die Foundation für CANC-02 (Verbands-Halbjahres-Stichtagsregel). Die Pure-Function `compute_effective_date` ermittelt aus einem Willensbekundungsdatum den wirksamen Stichtag (`fiscal_year` + `effective_date = 31.12.`) nach den Verbands-Konventionen:

- **H1 (Monat 1-6):** `fiscal_year = year(input)`, `effective_date = 31.12. desselben Jahres`
- **H2 (Monat 7-12):** `fiscal_year = year(input) + 1`, `effective_date = 31.12. des Folgejahres`

Konsumiert wird die Funktion in Phase 15 (`cancel_membership`), Phase 16 (`partial_repayment` für RepaymentPhase-Lookup über `fiscal_year`) und Phase 17 (`transfer_shares` bei Voll-Übertrag mit Austritt-Action).

### Files Created

| File | Lines | Purpose |
|------|-------|---------|
| `genossi_service_impl/src/membership_adjust.rs` | 115 | Pure-Function `compute_effective_date` + Struct `EffectiveDate` + 6 inline Edge-Case-Tests |

### Files Modified

| File | Diff | Purpose |
|------|------|---------|
| `genossi_service_impl/src/lib.rs` | +1 | Modul-Registrierung `pub mod membership_adjust;` in alphabetischer Position |

## Tasks

### Task 1 — Pure-Function + Struct + 6 Tests

- **Commit:** `4e10763`
- **Files:** `genossi_service_impl/src/membership_adjust.rs` (NEW, 115 LOC)
- **Acceptance:**
  - `pub(crate) fn compute_effective_date` count = 1 ✓
  - `pub(crate) struct EffectiveDate` count = 1 ✓
  - `#[derive(Debug, Clone, Copy, PartialEq, Eq)]` count = 1 ✓
  - `fn test_compute_effective_date_` count = 6 ✓
  - Edge-Case-Namen vollständig (30_juni, 01_juli, 31_dezember, 01_januar, schaltjahr_29_februar, mittiges_datum_15_maerz) = 6 ✓
  - Doc-Kommentar-Keywords (H1, H2, Verbands-Konvention, 30.06, 01.07) = 8 Treffer ✓

### Task 2 — Module Registration

- **Commit:** `43eccd4`
- **Files:** `genossi_service_impl/src/lib.rs` (+1 line)
- **Acceptance:**
  - `^pub mod membership_adjust;$` count = 1 ✓
  - `pub use.*membership_adjust` count = 0 (per D-14-03 kein Re-Export) ✓
  - `cargo build -p genossi_service_impl` grün ✓
  - `cargo test -p genossi_service_impl --lib membership_adjust` → `6 passed; 0 failed` ✓

## Tests

```
running 6 tests
test membership_adjust::tests::test_compute_effective_date_30_juni_is_h1 ... ok
test membership_adjust::tests::test_compute_effective_date_31_dezember_is_h2_next_year ... ok
test membership_adjust::tests::test_compute_effective_date_01_juli_is_h2 ... ok
test membership_adjust::tests::test_compute_effective_date_01_januar_is_h1 ... ok
test membership_adjust::tests::test_compute_effective_date_schaltjahr_29_februar_is_h1 ... ok
test membership_adjust::tests::test_compute_effective_date_mittiges_datum_15_maerz_is_h1 ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 352 filtered out
```

## Verification

- `cargo fmt --check` (rustfmt --edition 2021): **PASS** für beide modifizierten Dateien (`membership_adjust.rs` + `lib.rs`).
- `cargo clippy -p genossi_service_impl --lib --no-deps`: keine neuen Warnings auf Phase-14-01-Code. Es bleiben 2 erwartete `dead_code`-Warnings auf `compute_effective_date` + `EffectiveDate` — werden in Phase 15-17 konsumiert (CANC-02 Service-Methoden).
- `cargo check -p genossi_service_impl`: **PASS** (Crate baut sauber mit neuem Modul).

## Decisions Applied

Alle Decisions aus `14-CONTEXT.md` für Plan 14-01 wurden 1:1 umgesetzt:

- **D-14-01** Struct-Return statt Tuple — `EffectiveDate { fiscal_year, effective_date }` mit named fields.
- **D-14-02** Neue Datei `membership_adjust.rs` statt Ergänzung in `member_action.rs`.
- **D-14-03** `pub(crate)`-Visibility (kein `pub` Re-Export im `lib.rs`).
- **D-14-04** Grenzwert `month <= 6` für H1.
- **D-14-05** 31.12. → H2 → 31.12. Folgejahr (Test 3 verifiziert).
- **D-14-06** Schaltjahr 29.02.2024 → H1 → 31.12.2024 (Test 5 verifiziert).
- **D-14-07** Ausführlicher `///`-Doc-Kommentar mit H1/H2-Konvention + Grenzwert-Beispielen.

## Deviations from Plan

**None** — Plan 14-01 wurde exakt nach Template aus `14-PATTERNS.md` und Interface-Spec aus `14-01-PLAN.md` umgesetzt. Code stimmt 1:1 mit der vorgegebenen Implementierungs-Vorlage überein.

### Environmental Notes (NICHT vom Plan verursacht)

Der Worktree enthielt beim Start drei **fremde** modifizierte Files aus parallel-Wave-Bleed-Over (Plan 14-02 und 14-03 work-in-progress vom Orchestrator):

- `genossi_dao/src/repayment_entry.rs` — Plan 14-02 DAO-Trait-Erweiterung (`find_by_member_and_phase`)
- `genossi_service/src/member.rs` — Plan 14-03 Service-Trait-Methode (`list_transfer_recipients`)
- `.planning/STATE.md` — Orchestrator-Bookkeeping

Während meiner Arbeit erschien zusätzlich `genossi_dao_impl_sqlite/src/repayment_entry.rs` (vermutlich Plan 14-02 SQLite-Override).

Diese Files wurden NICHT modifiziert oder committed durch mich. Sie führen zu einer Build-Inkonsistenz im `genossi_service_impl/src/member.rs` (`E0046: missing list_transfer_recipients in impl`), die NICHT von Plan 14-01 verursacht oder behoben werden kann. Die Plan-14-01-Acceptance-Checks (`cargo test -p genossi_service_impl --lib membership_adjust::tests`) laufen trotzdem grün, weil `genossi_service_impl` als Lib sauber baut (der Trait-impl-Fehler tritt nur unter `--tests` auf, wenn die `MemberServiceImpl`-Tests den Trait neu prüfen).

Per `destructive_git_prohibition` und Permission-Block des Auto-Mode-Classifiers wurden diese fremden Modifikationen NICHT discarded. Die parallelen Wave-Agents für Plan 14-02 und 14-03 werden ihre Trait-Methoden konsistent ergänzen, sobald sie ihre eigenen Tasks committen.

## Anchor for Phases 15-17

Die Funktion ist `pub(crate)`. Callers innerhalb `genossi_service_impl` importieren via:

```rust
use crate::membership_adjust::{compute_effective_date, EffectiveDate};
```

Typischer Call-Site-Pattern (Phase 15+):

```rust
let result = compute_effective_date(willensbekundung);
// Phase 15 Kündigung:
member_action.effective_date = Some(result.effective_date);
// Phase 16 Teil-Rückgabe:
let phase = repayment_phase_dao.find_by_fiscal_year(result.fiscal_year, tx).await?;
```

`EffectiveDate` ist `Copy` — keine `.clone()`-Aufrufe nötig.

## Known Stubs

Keine. Die Pure-Function ist vollständig implementiert; Test-Coverage deckt alle 6 in PLAN/CONTEXT spezifizierten Edge-Cases ab.

## Self-Check: PASSED

- `genossi_service_impl/src/membership_adjust.rs`: **FOUND** (`[ -f ... ] && echo FOUND` → exit 0).
- `genossi_service_impl/src/lib.rs`: **FOUND** (modified, change includes `pub mod membership_adjust;`).
- Commit `4e10763`: **FOUND** (`git log --oneline | grep 4e10763` → present).
- Commit `43eccd4`: **FOUND** (`git log --oneline | grep 43eccd4` → present).
- Tests passing: **VERIFIED** via `cargo test -p genossi_service_impl --lib membership_adjust` → `6 passed; 0 failed`.

All claims verified. No missing artifacts.
