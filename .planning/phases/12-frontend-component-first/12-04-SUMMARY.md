---
phase: 12-frontend-component-first
plan: 04
subsystem: frontend
tags: [frontend, page, listing, modal, tdd, component-first]

# Dependency graph
requires:
  - phase: 12-frontend-component-first
    provides: "Plan 12-01: list_repayment_phases / list_repayment_entries / create_repayment_phase API + RepaymentPhaseTO + CreateRepaymentPhaseRequest + RepaymentPhases/EmptyHint/FiscalYear/ShareValue/EntryCount i18n keys"
  - phase: 12-frontend-component-first
    provides: "Plan 12-02: RepaymentPhaseStatusBadge component + parse_euro_to_cents + format_payout_eur helpers"
  - phase: 12-frontend-component-first
    provides: "Plan 12-03: Route::RepaymentPhases + Route::RepaymentPhaseDetails registered in router.rs, page/mod.rs declares pub mod repayment_phases"
provides:
  - "RepaymentPhases #[component] in genossi-frontend/src/page/repayment_phases.rs — full Listen-Page implementation (UI-01)"
  - "RepaymentPhaseListRow sub-component with per-row use_resource for entry count (UI-01-SC#1)"
  - "CreateRepaymentPhaseForm sub-component — Modal form with parse_euro_to_cents Euro→Cent conversion"
  - "sort_phases_default(phases) -> Vec<RepaymentPhaseTO> pure function (fiscal_year DESC, created DESC; D-14)"
  - "3 unit tests for sort_phases_default in page::repayment_phases::tests"
affects:
  - "Plan 12-05 (Detail-Page) — Listen-Page Link entry-point ready for Route::RepaymentPhaseDetails landing"
  - "Phase-12 UAT (Plan 12-15) — manual smoke test ready (dx serve → /repayment-phases)"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Pure-Function-Helper (sort_phases_default) in page-file with #[cfg(test)] mod tests — analog filter_members in member_search.rs"
    - "Per-row use_resource pattern for N+1 listing (acceptable per CONTEXT.md <specifics>: <20 Phasen/Jahr)"
    - "Detail-Page Link via dioxus::prelude Link to Route::RepaymentPhaseDetails (route registered Plan 12-03)"
    - "Listen-Page + Create-Modal-Pattern 1:1-Klon aus assemblies.rs (Phase 4 D-08)"
    - "Component-First: Status via RepaymentPhaseStatusBadge (Plan 12-02), Euro-Parse via parse_euro_to_cents (Plan 12-02) — keine inline Tailwind-Pill-Klassen, keine lokale Inline-Konvertierung"

key-files:
  created:
    - "genossi-frontend/src/page/repayment_phases.rs (312 Zeilen — 3 Components: RepaymentPhases, RepaymentPhaseListRow, CreateRepaymentPhaseForm + sort_phases_default + 3 tests)"
  modified: []

key-decisions:
  - "Listen-Page hat 12-column Grid mit dediziertem Header-Row (Geschäftsjahr | Anteilswert | Status | Anzahl Einträge | created) — D-14 Discretion auf Layout-Detail"
  - "Anzahl-Einträge-Spalte: per-Row use_resource auf list_repayment_entries, Loading='…', Erfolg=count, Fehler='?' — N+1 explizit erlaubt (CONTEXT.md <specifics>, UI-01-SC#1)"
  - "Detail-Page-Verlinkung via dioxus_router Link statt <a href> — Route::RepaymentPhaseDetails ist in Plan 12-03 registriert worden, daher kann der typed Link genutzt werden (kein String-Format)"
  - "Validation in CreateRepaymentPhaseForm: fiscal_year ∈ [1900, 9999] + share_value > 0 via parse_euro_to_cents — clientseitig minimal, Backend ist Backstop (Phase 7 D-12)"
  - "TDD-Sequenz Task 1 RED → GREEN: RED commit (3e5427d) hat stub sort_phases_default das phases.to_vec() liefert, 2/3 Tests fail; GREEN commit (68bfd94) liefert echtes sort_by → 3/3 PASS"

requirements-completed: [UI-01]

# Metrics
duration: 6 min 48 s
completed: 2026-06-01T12:01:50Z
task-count: 2
file-count: 1
test-count-added: 3
test-count-total: 155
commits:
  - {sha: 3e5427d, type: test, task: 1, scope: "page/repayment_phases.rs + page/mod.rs (RED)"}
  - {sha: 68bfd94, type: feat, task: "1+2", scope: "page/repayment_phases.rs (GREEN + full implementation)"}
---

# Phase 12 Plan 04: RepaymentPhases Listen-Page (UI-01) Summary

**One-liner:** Voll funktionale Listen-Page `/repayment-phases` mit Liste, Sortierung (fiscal_year DESC, created DESC), Create-Modal (parse_euro_to_cents Reuse) und per-Row Anzahl-Einträge via `use_resource` — Component-First-Pattern mit RepaymentPhaseStatusBadge und Plan-12-02-Helpers.

## What Was Built

Eine einzige Datei (`genossi-frontend/src/page/repayment_phases.rs`, 312 Zeilen) mit drei `#[component]`-Definitionen plus einer reinen Helper-Funktion und drei Unit-Tests — vollständige Listen-Page-UI inklusive Create-Phase-Modal.

### Komponenten

**`RepaymentPhases() -> Element`** (öffentliche Page-Component, mounted unter Route `/repayment-phases`):

- `RequirePrivilege { privilege: "admin", fallback: AccessDeniedPage }` wrapping (D-25)
- `TopBar` + Container mit Header („Anteils-Rückzahlung" + Create-Button)
- Drei Render-Modi:
  - **Loading:** `p { ... "{i18n.t(Key::Loading)}" }`
  - **Empty:** zentrierte Box mit `RepaymentPhaseEmpty`+`RepaymentPhaseEmptyHint` und Create-CTA (Pattern Z. 59-69 assemblies.rs)
  - **Liste:** 12-column Grid mit Header-Row + `for p in phases.read().iter() { RepaymentPhaseListRow { key, phase } }`
- Create-Modal mounted bei `*show_create.read() == true` → `CreateRepaymentPhaseForm` als `Modal`-Kind
- `ToastContainer` als Sibling für asynchrone API-Fehler

**`RepaymentPhaseListRow(phase: RepaymentPhaseTO) -> Element`** (Sub-Component, eine pro Phase):

- Eigener `use_resource(... list_repayment_entries(phase_id) ...)`-Hook pro Row → ergibt Anzahl Einträge
- Anzeige je Ressource-Status:
  - `None` → `"…"` (Loading-Placeholder)
  - `Some(Ok(list))` → `list.len().to_string()` (Erfolg)
  - `Some(Err(_))` → `"?"` (Defensiv-Fallback bei API-Fehler)
- Row-Klick wandert via `Link { to: Route::RepaymentPhaseDetails { id: phase.id.to_string() } }` zur Detail-Page (Plan 12-05)
- Status-Spalte rendert `RepaymentPhaseStatusBadge { status: phase.status }` (Plan 12-02 Component-First)
- Anteilswert-Spalte via `format_payout_eur(1, phase.share_value)` mit Suffix „/ Anteil"

**`CreateRepaymentPhaseForm(on_close, on_created, on_error) -> Element`** (Modal-internes Form):

- `<form onsubmit={...}>` mit synchroner `e.prevent_default()` direkt vor `spawn(async)` (D-01)
- Felder:
  - `fiscal_year` (number-input, i32)
  - `share_value_euro` (text-input, parse via `parse_euro_to_cents` aus Plan 12-02 — KEIN lokales `(euros * 100.0).round() as i64`)
- Validation: `parse_euro_to_cents` ist Some + `1900 ≤ fiscal_year ≤ 9999`; sonst `on_error.call(invalid_msg)` mit deutscher Fehlermeldung
- Submit-Button `r#type: "submit"` mit `disabled: *submitting.read()` (D-01)
- Cancel-Button `r#type: "button"` (D-01)

### Pure-Function

**`sort_phases_default(&[RepaymentPhaseTO]) -> Vec<RepaymentPhaseTO>`** (D-14):

```rust
fn sort_phases_default(phases: &[RepaymentPhaseTO]) -> Vec<RepaymentPhaseTO> {
    let mut result: Vec<RepaymentPhaseTO> = phases.to_vec();
    result.sort_by(|a, b| {
        b.fiscal_year
            .cmp(&a.fiscal_year)
            .then_with(|| b.created.cmp(&a.created))
    });
    result
}
```

Rust's `sort_by` ist stable → bei `fiscal_year + created`-Ties bleibt die Eingabe-Reihenfolge erhalten (D-14 Stability-Garantie).

### Tests (3, alle PASS in 155 Gesamt-Tests)

1. **`sort_by_fiscal_year_desc`** — 3 Phasen mit Jahren 2023/2025/2024 → sortiert [2025, 2024, 2023]
2. **`sort_by_created_desc_within_same_year`** — 3 Phasen mit fiscal_year=2025 und unterschiedlichen `created`-Timestamps → sortiert nach `created` DESC
3. **`sort_empty_returns_empty`** — leerer Slice → leerer Vec

## Render-Pfad (Datenfluss)

```
RepaymentPhases (mount)
  ↓ use_effect → load()
  ↓ spawn → api::list_repayment_phases(&config)
  ↓ Ok(list) → phases.set(sort_phases_default(&list))
  ↓ Render-Tree:
    RequirePrivilege
      TopBar
      Container
        Header (h1 + Create-Button)
        Branch: Loading | Empty | Liste
          Liste: Grid-Header-Row + ForEach RepaymentPhaseListRow
            RepaymentPhaseListRow
              use_resource → spawn → api::list_repayment_entries(&config, phase_id)
              Link { to: Route::RepaymentPhaseDetails { id: phase.id.to_string() } }
                Grid-Cells (fiscal_year | format_payout_eur | RepaymentPhaseStatusBadge | entry_count | created)
        Modal? (when show_create == true)
          CreateRepaymentPhaseForm
      ToastContainer
```

**Per-Row Anzahl-Einträge-Verhalten** (UI-01-SC#1):
- Während fetch: `"…"` als Placeholder
- Bei Erfolg: `list.len().to_string()` (z.B. `"5"`)
- Bei API-Fehler: `"?"` als Defensiv-Fallback (kein Crash, sichtbare Fehler-Spur — analog `MEMBERS`-Pattern)

**Create-Modal-Datenfluss**:
- User tippt EUR-String (z.B. `"60,00"`) ins `share_value_euro`-Signal
- `onsubmit` triggert `parse_euro_to_cents(&share_value_euro.read())` aus Plan 12-02 (kanonisch)
- Bei `Some(cents)` UND `1900 ≤ year ≤ 9999`: `spawn(api::create_repayment_phase(&config, &req))`
- `Ok(_) → on_created()` (closes modal, reloads list)
- `Err(e) → on_error.call(e.message)` (Toast)

## How It Was Verified

```bash
# Build + Tests
$ cd genossi-frontend && cargo build
warning: `genossi-frontend` generated 36 warnings (unused i18n keys for future plans)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 17.40s

$ cargo test page::repayment_phases::tests
test page::repayment_phases::tests::sort_by_created_desc_within_same_year ... ok
test page::repayment_phases::tests::sort_by_fiscal_year_desc ... ok
test page::repayment_phases::tests::sort_empty_returns_empty ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 152 filtered out

$ cargo test
test result: ok. 155 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

# D-01 Button-Gate (kein button ohne r#type:)
$ rg -P '(?ms)button\s*\{(?:(?!\}).)*?\}' genossi-frontend/src/page/repayment_phases.rs \
    | grep -v 'r#type:' | grep -c 'button {'
0

# Component-First Anker
$ rg -c "RepaymentPhaseStatusBadge \{" genossi-frontend/src/page/repayment_phases.rs
1

# API-Verdrahtung
$ rg -c "api::list_repayment_phases|api::create_repayment_phase" \
    genossi-frontend/src/page/repayment_phases.rs
2

# Entry-Count-Spalte (UI-01-SC#1)
$ rg -c "Key::RepaymentPhaseEntryCount" genossi-frontend/src/page/repayment_phases.rs
1

# Parse-Reuse (kein lokales Inline-Parse)
$ rg -c "parse_euro_to_cents" genossi-frontend/src/page/repayment_phases.rs
4
$ rg -c "\(\s*euros\s*\*\s*100\.0\s*\)\.round\(\)" genossi-frontend/src/page/repayment_phases.rs
# (no output = 0)

# Modal + Auth-Gate
$ rg -c "Modal \{" genossi-frontend/src/page/repayment_phases.rs
1
$ rg -c "RequirePrivilege" genossi-frontend/src/page/repayment_phases.rs
2

# Stub-Marker entfernt
$ rg -c "TODO Plan 12-04" genossi-frontend/src/page/repayment_phases.rs
# (no output = 0)
```

Alle Plan-Done-Criteria sind erfüllt.

## Decisions Made

**Detail-Page Link via dioxus_router `Link` statt `<a href>`:**

Der initiale Plan-Action-Block referenziert `Link { to: Route::RepaymentPhaseDetails { ... } }`. Zum Zeitpunkt von Task 1 (RED) war Plan 12-03 noch nicht in HEAD; ich hätte alternativ `<a href="/repayment-phases/{id}">` nutzen müssen, um zu kompilieren. Da Plan 12-03 jedoch zwischen RED- und GREEN-Commit in den Worktree gemerged wurde, kann der typed `Link` verwendet werden — dies ist der konsistentere Pattern (Type-Safety, Refactor-Freundlichkeit). Pattern-Anker: `access_denied.rs:26-30`, `mail_page.rs:732`.

**12-column Grid statt Card-Layout:**

Der Plan-Action-Block spezifiziert dezidiert eine 12-column Grid mit Header-Row, weil die Anzahl-Einträge-Spalte (UI-01-SC#1) eine klare tabellarische Darstellung verlangt. `assembly_list_row.rs` nutzt ein Card-Layout (`flex items-center justify-between`) — bei nur 2-3 Datenpunkten pro Row ausreichend. Phase 12 hat 4 Datenpunkte + Created-Date → Grid bringt visuelle Ordnung. Pattern-Anker: keine direkte Vorlage; Plan-Discretion gemäß CONTEXT D-14.

**Rule 3 Auto-Fix während Task 1 RED:**

Beim ersten Test-Run war `genossi-frontend/src/page/mod.rs` noch ohne `pub mod repayment_phases;`-Eintrag (Plan 12-03 war noch nicht angekommen). Rule 3 (blockierender Test-Compile) erforderte die Ergänzung. Diese ein-Zeilen-Änderung in mod.rs landete im RED-Commit `3e5427d` — beim späteren GREEN-Commit war Plan 12-03 bereits mit voller mod.rs-Erweiterung in HEAD, sodass der GREEN-Commit nur noch `repayment_phases.rs` selbst änderte.

**TDD-Sequenz Strict-RED → GREEN:**

Task 1 (`tdd="true"`) wurde als 2 Commits ausgeführt — RED mit Stub (3e5427d), GREEN mit echter Sortierung (68bfd94). Task 2 wurde mit dem GREEN-Commit kombiniert, weil die Page-Component und die Pure-Funktion im selben File leben und die Page das Sort-Verhalten direkt konsumiert. Das ist konsistent mit dem TDD-Gate Pattern aus Plan 12-02 (das ähnlich strukturiert war).

## Deviations from Plan

**1. [Rule 3 — Blocking Fix] page/mod.rs Eintrag in RED-Commit hinzugefügt**

- **Found during:** Task 1 RED (vor Plan 12-03-Merge)
- **Issue:** `cargo test page::repayment_phases::tests` schlug fehl mit „could not find module repayment_phases", weil das Modul nicht in `page/mod.rs` registriert war
- **Fix:** `pub mod repayment_phases;` hinzugefügt — diese Zeile wurde später von Plan 12-03 (in einem separaten Commit `3816a3f`, der zwischen meinem RED und GREEN in HEAD landete) mit einem Kommentar-Block und `pub mod repayment_phase_details;` ergänzt. Plan 12-03 ist der eigentliche Owner — Phase 12 Wave 2 hat 12-03 als parallel-erlaubten Plan, der einfacher in zwei Worktrees gleichzeitig die mod.rs anfasst. Kein Konflikt, weil 12-03's Erweiterung additiv ist.
- **Files modified:** genossi-frontend/src/page/mod.rs (1 Zeile, RED-Commit)
- **Commit:** 3e5427d

**2. [Rule 3 — Workaround entfernt] `<a href>` durch `Link { to: Route::... }` ersetzt**

- **Found during:** Task 1 GREEN (nach Plan 12-03-Merge)
- **Issue:** Im RED-Stub habe ich initial `<a href="/repayment-phases/{id}">` verwendet, um Route::RepaymentPhaseDetails nicht zu importieren (Plan 12-03 lieferte das noch nicht in HEAD)
- **Fix:** Da Plan 12-03 zwischen RED und GREEN landete, ist `Link { to: Route::RepaymentPhaseDetails { id: phase.id.to_string() } }` jetzt verfügbar — der typed Link ist konsistenter mit `access_denied.rs:26-30` und `mail_page.rs:732`
- **Files modified:** genossi-frontend/src/page/repayment_phases.rs
- **Commit:** 68bfd94

Keine weiteren Abweichungen — alle anderen Plan-Action-Codes wurden 1:1 übernommen, alle Done-Criteria sind erfüllt.

## Known Stubs

**None.**

Die Listen-Page ist voll funktional. Der Link in `RepaymentPhaseListRow` zeigt auf `Route::RepaymentPhaseDetails`, dessen Ziel-Component aus Plan 12-03 ein Stub mit „TODO Plan 12-05" ist — das ist ein Plan-12-03-Artefakt, kein Plan-12-04-Stub. Plan 12-05 implementiert die Detail-Page.

## Self-Check: PASSED

Verifizierte Artefakte (alle FOUND/VERIFIED):

- [FOUND] `genossi-frontend/src/page/repayment_phases.rs` (312 Zeilen, > 150 min_lines)
- [FOUND] `#[component] pub fn RepaymentPhases()` Zeile 47
- [FOUND] `#[component] fn RepaymentPhaseListRow(phase: RepaymentPhaseTO)` Zeile 132
- [FOUND] `#[component] fn CreateRepaymentPhaseForm(...)` Zeile 176
- [FOUND] `fn sort_phases_default` Zeile 35 (pure function)
- [FOUND] 3 `#[test]`-Markierte Tests in `tests`-Submodul
- [VERIFIED] `cargo build` exit 0 (genossi-frontend)
- [VERIFIED] `cargo test page::repayment_phases::tests` → 3/3 PASS
- [VERIFIED] `cargo test` (all) → 155/155 PASS
- [VERIFIED] D-01 Button-Gate: `rg -P '(?ms)button\s*\{(?:(?!\}).)*?\}' ... | grep -v 'r#type:' | grep -c 'button {'` = 0
- [VERIFIED] Component-First-Anker via `RepaymentPhaseStatusBadge {` = 1
- [VERIFIED] Parse-Reuse: `parse_euro_to_cents` = 4 Treffer (Import + Aufruf); kein Inline-Parse
- [VERIFIED] API-Verdrahtung: `api::list_repayment_phases` + `api::create_repayment_phase` = 2 Treffer
- [VERIFIED] `list_repayment_entries` per-Row Resource = 1 Aufruf (+ 2 Doku-Kommentare)
- [VERIFIED] `Key::RepaymentPhaseEntryCount` = 1 Render
- [VERIFIED] `Modal {` = 1, `RequirePrivilege` = 2
- [VERIFIED] Stub-Marker `TODO Plan 12-04` entfernt
- [FOUND] Commit 3e5427d (test(12-04): add failing tests for sort_phases_default)
- [FOUND] Commit 68bfd94 (feat(12-04): implement RepaymentPhases list page (UI-01))

## TDD Gate Compliance

- **RED gate:** `3e5427d` — `test(12-04): add failing tests for sort_phases_default`. Stub `sort_phases_default` liefert `phases.to_vec()` (Identity), 2 von 3 Tests schlagen fehl (`sort_empty_returns_empty` passt trivially weil leerer Vec auch unsortiert leer ist).
- **GREEN gate:** `68bfd94` — `feat(12-04): implement RepaymentPhases list page (UI-01)`. Echte `sort_by` mit `b.fiscal_year.cmp(&a.fiscal_year).then_with(|| b.created.cmp(&a.created))` → alle 3 Tests grün.
- **REFACTOR gate:** keine Refactor-Commits — Implementation war minimal und Codebase-konform.
- **Task 2 Strict-RED skipped:** Task 2 ist die Page-Component selbst, die per Definition Render-Output produziert; RED-Tests für UI-Render ohne WASM-Setup sind nicht praktikabel. Die Verify-Stufe ist `cargo build exit 0` + Pure-Func-Tests + Plan-12-15-UAT (Phase-12-Verify-Stage).

---

*Phase: 12-frontend-component-first*
*Plan: 04 — RepaymentPhases Listen-Page (UI-01)*
*Completed: 2026-06-01T12:01:50Z (6 min 48 s)*
