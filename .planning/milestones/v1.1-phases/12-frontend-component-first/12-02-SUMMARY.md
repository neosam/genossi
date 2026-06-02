---
phase: 12-frontend-component-first
plan: 02
subsystem: ui
tags: [frontend, dioxus, component, badge, formatting, tdd]

# Dependency graph
requires:
  - phase: 12-frontend-component-first
    provides: "Plan 12-01: RepaymentPhaseStatusTO + RepaymentEntryStatusTO enums in api.rs; i18n Key variants RepaymentPhaseStatus{Preparation,Open,Closed} + RepaymentEntryStatus{Open,Contacted,PaidOut}"
provides:
  - "RepaymentPhaseStatusBadge component (#[component]) — Pill-Badge mit Farbpalette Vorbereitung=grau / Offen=blau / Abgeschlossen=grün"
  - "RepaymentEntryStatusBadge component (#[component]) — Pill-Badge mit Farbpalette Offen=grau / Angeschrieben=blau / Ausbezahlt=grün"
  - "format_payout_eur(share_count: i32, share_value_cents: i64) -> String — kanonische deutsche EUR-Formatierung mit € (NICHT i18n::format_price das EUR liefert)"
  - "parse_euro_to_cents(input: &str) -> Option<i64> — kanonischer Parser für User-Inputs (Komma/Punkt, Whitespace-Trim, > 0 Constraint)"
  - "Re-Exports in genossi-frontend/src/component/mod.rs"
affects:
  - "Plan 12-04 (Create-Modal — reused parse_euro_to_cents für share_value-Eingabe)"
  - "Plan 12-05 (Detail-Page-Header — reused RepaymentPhaseStatusBadge)"
  - "Plan 12-06 (share_value-Inline-Edit — reused parse_euro_to_cents)"
  - "Plan 12-07 (Listen-Page — reused RepaymentPhaseStatusBadge)"
  - "Plan 12-08 (RepaymentEntryList — reused RepaymentEntryStatusBadge + format_payout_eur für Betrag-Spalte)"
  - "Plan 12-10 (PaidOut-Confirm — reused format_payout_eur für Gesamtsumme)"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Pure-Helper-Modul in component/ ohne #[component] — testbar via cargo test ohne WASM-Setup"
    - "1:1-Klon-Pattern aus assembly_status_badge.rs mit angepasstem Status-Enum + Farbpalette"
    - "TDD RED→GREEN für pure helpers (separate test/feat commits)"

key-files:
  created:
    - "genossi-frontend/src/component/repayment_format.rs"
    - "genossi-frontend/src/component/repayment_phase_status_badge.rs"
    - "genossi-frontend/src/component/repayment_entry_status_badge.rs"
  modified:
    - "genossi-frontend/src/component/mod.rs"

key-decisions:
  - "format_payout_eur ist eigener Helper (NICHT i18n::format_price) — D-10 verlangt €-Symbol, i18n liefert EUR-Suffix"
  - "parse_euro_to_cents lehnt 0 und negative Werte ab — Backend-Phase-7 D-12 erfordert share_value > 0"
  - "Module bewusst NICHT als pub use re-exportiert — Aufrufer sehen Modulpfad crate::component::repayment_format::{format_payout_eur, parse_euro_to_cents} (verhindert namespace-Pollution)"
  - "Phase-12-Adaption RepaymentPhaseStatusBadge dreht Open/Closed-Farben gegenüber AssemblyStatusBadge (Phase=Open ist blau, Closed ist grün; Assembly hat es umgekehrt) — D-14 + Claude's Discretion"

patterns-established:
  - "Pure-Function-Helper-Modul: format_/parse_-Funktionen ohne UI-Rendering, mit #[cfg(test)] mod tests inline (analog member_search.rs::filter_members Z.9-35)"
  - "Status-Badge-Klon: 1:1-Übernahme von assembly_status_badge.rs Struktur (status_label + status_badge_class + #[component] mit rsx! span); nur Status-Enum und Farb-Match-Arms tauschen"

requirements-completed: [UI-01, UI-02, UI-03]

# Metrics
duration: 4min 13s
completed: 2026-06-01
---

# Phase 12 Plan 02: Frontend-Component-First Foundation — Summary

**Vier Pure-Reuse-Bausteine (format_payout_eur, parse_euro_to_cents, RepaymentPhaseStatusBadge, RepaymentEntryStatusBadge) als Component-First-Foundation für alle nachfolgenden Phase-12-Plans.**

## Performance

- **Duration:** 4 min 13 s
- **Started:** 2026-06-01T11:34:26Z
- **Completed:** 2026-06-01T11:38:39Z
- **Tasks:** 2 (TDD-Task 1 in 2 Commits + Task 2 in 1 Commit)
- **Files created:** 3 (`repayment_format.rs`, `repayment_phase_status_badge.rs`, `repayment_entry_status_badge.rs`)
- **Files modified:** 1 (`component/mod.rs`)

## Accomplishments

- **`format_payout_eur(share_count, share_value_cents) -> String`** — kanonische deutsche EUR-Formatierung mit Euro-Symbol „60,00 €". Vermeidet bewusst `i18n::format_price`, das „60,00 EUR" liefert (D-10).
- **`parse_euro_to_cents(input) -> Option<i64>`** — kanonischer Parser für User-Inputs. Akzeptiert Komma- ODER Punkt-Dezimaltrennung, trimmt Whitespace, lehnt 0 / negative / garbage / Suffix-Inputs ab. Backend-Phase-7-D-12-konform (`share_value > 0` als Backend-CHECK).
- **`RepaymentPhaseStatusBadge`** — Pill-Badge für Phase-Status mit Farbpalette Vorbereitung=grau, Offen=blau, Abgeschlossen=grün (CONTEXT D-14 + Claude's Discretion).
- **`RepaymentEntryStatusBadge`** — Pill-Badge für Entry-Status mit Farbpalette Offen=grau, Angeschrieben=blau, Ausbezahlt=grün (CONTEXT D-14).
- **17 Unit-Tests gesamt:** 9 für `repayment_format.rs` (4 format_payout_eur + 5 parse_euro_to_cents-Tests), 4 pro Badge-File (3 Farb-Branches + 1 Pill-Styling-Check).

## Task Commits

Each task was committed atomically:

1. **Task 1 RED: failing tests for format_payout_eur + parse_euro_to_cents** — `5677792` (`test`)
2. **Task 1 GREEN: implement format_payout_eur + parse_euro_to_cents** — `1fb1bc5` (`feat`) — 9/9 tests PASS
3. **Task 2: RepaymentPhaseStatusBadge + RepaymentEntryStatusBadge** — `cc27420` (`feat`) — 8 Tests (4 pro Badge); standalone-Build blocked durch cross-wave-Dependency auf Plan 12-01

_Note: TDD task hatte RED+GREEN split; Task 2 wurde als ein commit gemacht, da die compile-Blocker rein external sind (Plan-12-01-Types, kein Logik-Bug)._

## Farbpalette-Mapping

| Status-Badge | Variante | Tailwind-Klasse | Hex (Tailwind 100/800) |
|---|---|---|---|
| `RepaymentPhaseStatusBadge` | Preparation | `bg-gray-100 text-gray-800` | bg #F3F4F6, fg #1F2937 |
| `RepaymentPhaseStatusBadge` | Open | `bg-blue-100 text-blue-800` | bg #DBEAFE, fg #1E40AF |
| `RepaymentPhaseStatusBadge` | Closed | `bg-green-100 text-green-800` | bg #DCFCE7, fg #166534 |
| `RepaymentEntryStatusBadge` | Open | `bg-gray-100 text-gray-800` | bg #F3F4F6, fg #1F2937 |
| `RepaymentEntryStatusBadge` | Contacted | `bg-blue-100 text-blue-800` | bg #DBEAFE, fg #1E40AF |
| `RepaymentEntryStatusBadge` | PaidOut | `bg-green-100 text-green-800` | bg #DCFCE7, fg #166534 |

**Gemeinsames Pill-Styling für alle 6 Varianten:** `px-2 py-1 rounded text-xs font-medium`.

**Visuelle Konsistenz-Notiz:** RepaymentPhase und RepaymentEntry teilen sich dieselben Hex-Werte je Semantik-Position (grau=initial, blau=in-progress, grün=done). Verifizierbar in der Phase-12-UI-Verify, indem ein RepaymentPhase mit Status Open und ein RepaymentEntry mit Status Contacted nebeneinander gerendert werden — beide sollen visuell identische Badges zeigen.

## Files Created/Modified

- `genossi-frontend/src/component/repayment_format.rs` (created, 126 Zeilen) — pure helpers + 9 Unit-Tests
- `genossi-frontend/src/component/repayment_phase_status_badge.rs` (created, 86 Zeilen) — Badge-Component + 4 Tests
- `genossi-frontend/src/component/repayment_entry_status_badge.rs` (created, 84 Zeilen) — Badge-Component + 4 Tests
- `genossi-frontend/src/component/mod.rs` (modified) — Phase-12-Re-Exports-Block hinzugefügt:
  - `pub mod repayment_format`
  - `pub mod repayment_phase_status_badge` + `pub use RepaymentPhaseStatusBadge`
  - `pub mod repayment_entry_status_badge` + `pub use RepaymentEntryStatusBadge`

## Decisions Made

- **`format_payout_eur` lebt in eigenem Helper-Modul, NICHT in i18n** — Phase 12 default-Sprache ist Deutsch (D-Default); explizites Locale-Switching wäre ein Phase-13-Feature. Aktuelle Implementation ist bewusst nicht Locale-aware (kein `i18n` -Argument).
- **`parse_euro_to_cents` ist die EINZIGE kanonische Parse-Stelle** — Plan 12-04 (Create-Modal) und Plan 12-06 (share_value-Inline-Edit) reusen direkt; KEIN lokales `(euros * 100.0).round() as i64` mehr in jenen Plans.
- **Re-Export-Stil:** Badges sind `pub use`d für direkten Component-Aufruf (`RepaymentPhaseStatusBadge { ... }`); `repayment_format` ist nur als `pub mod` exportiert (Aufruf via `crate::component::repayment_format::format_payout_eur(...)` — bewusste namespace-Pollution-Vermeidung wie im Plan-Action spezifiziert).

## Deviations from Plan

**None - plan executed exactly as written.**

(Alle Code-Templates aus dem `<action>`-Block des Plans wurden 1:1 übernommen. Einzige Anpassung: Strict-TDD-Sequenz für Task 1 (RED-Stub-Commit vor GREEN-Impl-Commit) statt single-commit, wie es das `tdd="true"` Attribut + die TDD-Execution-Flow-Richtlinie verlangen.)

## Issues Encountered

### Cross-Wave-Dependency in Parallel-Worktree-Execution

**Beobachtung:** Plan 12-02 ist in der Plan-Frontmatter als `wave: 1, depends_on: []` markiert — parallel ausführbar zu Plan 12-01. Task 2 (Status-Badges) braucht aber `crate::api::{RepaymentPhaseStatusTO, RepaymentEntryStatusTO}` und die i18n-Keys `RepaymentPhaseStatus{Preparation,Open,Closed}` + `RepaymentEntryStatus{Open,Contacted,PaidOut}`, die Plan 12-01 erst erzeugt.

**Effekt im Worktree:**
- `cargo check` läuft mit 8 Errors (`E0432 unresolved import` × 2, `E0599 no variant Key::RepaymentPhase…` × 6) — exakt die erwarteten Cross-Plan-Imports.
- Task 1 (`repayment_format.rs`) ist davon NICHT betroffen — die 9 Tests laufen grün isoliert.
- Task-2-Tests (`repayment_phase_status_badge::tests`, `repayment_entry_status_badge::tests`) lassen sich in diesem Worktree nicht ausführen, weil die Datei nicht kompiliert.

**Resolution-Strategie:** Code exakt wie im Plan vorgesehen geschrieben. Die Orchestrator-Merge der Wave-1-Worktrees (12-01 + 12-02) bringt die Plan-12-01-Types und Plan-12-02-Konsumenten zusammen, sodass nach Merge `cargo build -p genossi-frontend` und `cargo test component::repayment_phase_status_badge component::repayment_entry_status_badge` grün laufen werden.

**Verifier-Hinweis (Phase 12 Verify):** Nach Wave-1-Merge bitte folgende Kommandos prüfen:
```bash
cd genossi-frontend && cargo check  # erwartet: 0 errors
cd genossi-frontend && cargo test component::repayment_format component::repayment_phase_status_badge component::repayment_entry_status_badge
# erwartet: 17 PASSED (9 + 4 + 4)
```

### Workspace-Exclude (genossi-frontend ist nicht im Cargo-Workspace)

**Beobachtung:** `cargo test -p genossi-frontend` aus dem Repo-Root schlägt fehl, weil `genossi-frontend/` im Root-`Cargo.toml` als `exclude` markiert ist. Tests müssen aus `genossi-frontend/`-Verzeichnis heraus ausgeführt werden.

**Konsequenz:** Plan 12-02 verify-Block `<automated>cargo test -p genossi-frontend --lib component::repayment_format</automated>` muss in der Praxis als `cd genossi-frontend && cargo test component::repayment_format` ausgeführt werden. Kein Defect — eine Kommando-Anpassung für künftige Verify-Schritte.

### Worktree-bezogene Cargo.toml-/STATE.md-Modifikationen (extern)

Der Orchestrator hat während dieser Session `Cargo.toml` (Worktree-Exclude-Liste) und `.planning/STATE.md` (last_updated, current focus) modifiziert. Diese sind explizit NICHT vom Worktree-Agent zu committen (parallel_execution-Regel), sodass sie als uncommitted Working-Tree-Diffs zurückbleiben — der Orchestrator handelt das auf der Wave-Merge-Ebene.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

**Bereit für Wave 2 Plan-12-04, 12-05, 12-06, 12-07, 12-08, 12-10:**
- `crate::component::RepaymentPhaseStatusBadge { status: phase.status }` (Plan 12-05 + 12-07)
- `crate::component::RepaymentEntryStatusBadge { status: entry.status }` (Plan 12-08)
- `crate::component::repayment_format::format_payout_eur(entry.share_count_to_pay_out, phase.share_value)` (Plan 12-08 + 12-10)
- `crate::component::repayment_format::parse_euro_to_cents(user_input.trim())` (Plan 12-04 + 12-06)

**Blocker:** Keine — die Foundation-Bausteine stehen. Nach Wave-1-Merge ist Wave 2 startfähig.

## Self-Check: PASSED

Verifizierte Artefakte:

- [FOUND] `genossi-frontend/src/component/repayment_format.rs` (126 Zeilen)
- [FOUND] `genossi-frontend/src/component/repayment_phase_status_badge.rs` (86 Zeilen, > 40 min_lines)
- [FOUND] `genossi-frontend/src/component/repayment_entry_status_badge.rs` (84 Zeilen, > 40 min_lines)
- [FOUND] `pub mod repayment_format` in `genossi-frontend/src/component/mod.rs`
- [FOUND] `pub use repayment_phase_status_badge::RepaymentPhaseStatusBadge` in `mod.rs`
- [FOUND] `pub use repayment_entry_status_badge::RepaymentEntryStatusBadge` in `mod.rs`
- [FOUND] `pub fn format_payout_eur` (1 Definition, keine Duplikate in genossi-frontend/src/)
- [FOUND] `pub fn parse_euro_to_cents` (1 Definition, keine Duplikate in genossi-frontend/src/)
- [FOUND] `#[component]` × 2 (einmal pro Badge-File)
- [FOUND] Commit `5677792` (RED test)
- [FOUND] Commit `1fb1bc5` (GREEN feat)
- [FOUND] Commit `cc27420` (Task 2 feat)
- [VERIFIED] Task-1-Tests: 9/9 PASS via `cargo test component::repayment_format` (in genossi-frontend/)
- [DEFERRED] Task-2-Tests: blockiert durch Cross-Wave-Dep auf Plan 12-01 — nach Merge ausführbar (siehe Issues Encountered)

## TDD Gate Compliance

- **RED gate:** `5677792` (`test(12-02): add failing tests …`) — 7 von 9 Tests schlugen fehl (2 stub-cases passten zufällig durch None/Empty-Default).
- **GREEN gate:** `1fb1bc5` (`feat(12-02): implement …`) — alle 9 Tests grün.
- **REFACTOR gate:** keine Refactor-Commits — Implementation war minimal und sauber, kein Refactor nötig.
- **Task 2:** kein expliziter RED-Commit, da die compile-Blocker rein external sind (Plan-12-01-Types). Strict-TDD-RED hätte hier keinen Erkenntnisgewinn gebracht (Tests können wegen fehlender Types gar nicht kompilieren). Pragmatic-Single-Commit ist ok für Klon-Pattern wo `assembly_status_badge.rs` der Test-Beweis ist.

---

*Phase: 12-frontend-component-first*
*Plan: 02*
*Completed: 2026-06-01*
