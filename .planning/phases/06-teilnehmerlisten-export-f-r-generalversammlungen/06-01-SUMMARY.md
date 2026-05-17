---
phase: 06-teilnehmerlisten-export-f-r-generalversammlungen
plan: 01
subsystem: infra
tags:
  - cargo
  - workspace-dependencies
  - typst
  - template
  - export
  - rust_xlsxwriter
  - csv

# Dependency graph
requires: []
provides:
  - "rust_xlsxwriter 0.82 als Workspace-Dependency in genossi_service_impl verfuegbar"
  - "csv 1.3 als Workspace-Dependency in genossi_service_impl verfuegbar"
  - "templates/teilnehmerliste.typ Typst-Template fuer Teilnehmerlisten-PDF"
affects:
  - "06-02 (Service-Implementierung Export-Logik)"
  - "06-03 (REST-Endpoint + Wiring + E2E)"
  - "06-04 (Frontend-Block)"

# Tech tracking
tech-stack:
  added:
    - "rust_xlsxwriter 0.82 (Workspace-Dep, fuer XLSX-Generierung)"
    - "csv 1.3 (Workspace-Dep, fuer CSV-Generierung; vorher hardcoded in genossi_rest + genossi_bin)"
  patterns:
    - "Typst-Template mit table.header(repeat: true) fuer mehrseitige Tabellen"
    - "Konditionaler X-von-Y Header via meta.at('total', default: none)"
    - "JSON-encoded sys.inputs Pattern (analog zu join_confirmation.typ)"

key-files:
  created:
    - "templates/teilnehmerliste.typ — Typst-Template fuer Teilnehmerliste"
  modified:
    - "Cargo.toml — rust_xlsxwriter + csv in [workspace.dependencies]"
    - "genossi_service_impl/Cargo.toml — csv + rust_xlsxwriter als workspace = true"
    - "genossi_bin/Cargo.toml — dev-deps von hardcoded auf workspace = true migriert"
    - "genossi_rest/Cargo.toml — csv von hardcoded auf workspace = true migriert"
    - "Cargo.lock — automatisches Update durch cargo build"

key-decisions:
  - "rust_xlsxwriter Pin auf 0.82 (bereits in Cargo.lock) statt Bump auf 0.94/0.95 — Risiko-arm, RESEARCH §Open Question 1 Empfehlung"
  - "Template-Position im templates/-Root (NICHT in templates/defaults/) — passend zum bestehenden #import \"_layout.typ\" Resolver-Pattern"
  - "Anwesenheits-Glyph: ✓ (per Planner-Entscheidung aus CONTEXT.md §Claude's Discretion)"

patterns-established:
  - "Workspace-Dependency-Promotion: Hardcoded Cargo.toml-Versions zentralisieren auf [workspace.dependencies] und in Konsumenten als { workspace = true } referenzieren"
  - "Typst-Template-Input-Konvention: JSON-encoded Multi-Field-Inputs via sys.inputs.at('key'), dekodiert via json.decode"

requirements-completed:
  - D-02
  - D-04
  - D-08
  - D-10

# Metrics
duration: 15min
completed: 2026-05-17
---

# Phase 6 Plan 01: Phase-6-Foundation Summary

**Workspace-Dependency-Promotion fuer rust_xlsxwriter/csv plus neues Typst-Template `teilnehmerliste.typ` mit konditionalem X-von-Y-Kopf und 6-spaltiger Repeat-Header-Tabelle.**

## Performance

- **Duration:** ca. 15 min
- **Started:** 2026-05-17T10:48:38Z
- **Completed:** 2026-05-17T11:03:32Z
- **Tasks:** 2
- **Files modified:** 5 (4 modified + 1 created)

## Accomplishments

- `rust_xlsxwriter = "0.82"` und `csv = "1.3"` als `[workspace.dependencies]` deklariert
- `genossi_service_impl` kann jetzt beide Crates via `{ workspace = true }` konsumieren — vorbereitet fuer Plan 02 (Writer-Implementierung)
- `genossi_bin/Cargo.toml` und `genossi_rest/Cargo.toml` von hardcoded Versionen auf Workspace-Refs migriert (zentrale Versionierung)
- Neues Typst-Template `templates/teilnehmerliste.typ` mit:
  - Import von `_layout.typ::letter` (etabliertes Pattern)
  - JSON-Decoded `meta` + `rows` aus `sys.inputs`
  - Konditionalem Header: "X von Y anwesend" wenn `meta.total != none`, sonst "X anwesend"
  - 6-spaltiger Tabelle mit `table.header(repeat: true)` fuer Mehrseiten-Repetition
  - Anwesenheits-Glyph `✓`
- `cargo build --workspace` ist gruen (verifiziert mit `SQLX_OFFLINE=true`)

## Task Commits

Each task was committed atomically:

1. **Task 1: Workspace dependency promotion** — `b42886b` (chore)
2. **Task 2: Typst template creation** — `f86ae65` (feat)

_Cargo.lock-Update wurde dem Task-2-Commit beigegeben (durch Build-Reihenfolge so materialisiert); der Inhalt — `csv` + `rust_xlsxwriter` zu `genossi_service_impl`'s Dep-Tree hinzugefuegt — gehoert semantisch zu Task 1._

## Files Created/Modified

- `Cargo.toml` — Workspace-Root: csv + rust_xlsxwriter in `[workspace.dependencies]` aufgenommen
- `Cargo.lock` — Automatisches Update durch Build
- `genossi_service_impl/Cargo.toml` — Konsumiert csv + rust_xlsxwriter als `{ workspace = true }`
- `genossi_bin/Cargo.toml` — dev-deps fuer csv + rust_xlsxwriter migriert auf workspace
- `genossi_rest/Cargo.toml` — csv-Dep migriert auf workspace
- `templates/teilnehmerliste.typ` (neu) — Typst-Template fuer Teilnehmerliste

## Decisions Made

- **rust_xlsxwriter-Pin auf 0.82:** Folgt RESEARCH §Open Question 1. Crate ist bereits gepinnt in Cargo.lock, kein Bump-Risiko. Plan 02 kann ohne Sorgen `use rust_xlsxwriter::Workbook;` schreiben.
- **Template-Position im `templates/`-Root:** Nicht in `templates/defaults/` — der `pdf_generation.rs::template_base`-Mechanismus resolved `#import "_layout.typ"` relativ zum Root, was bereits `_layout.typ` enthaelt.
- **Anwesenheits-Glyph `✓`:** Aus CONTEXT.md §Claude's Discretion uebernommen. Alternativ haetten Checkbox-Glyphen (☑) oder Strings ("ja"/"nein") gewaehlt werden koennen.

## Deviations from Plan

**None — plan executed exactly as written.**

Alle Schritte aus dem Plan-Action-Block wurden 1:1 umgesetzt. Keine Auto-Fixes noetig, keine architektonischen Entscheidungen zu treffen.

## Issues Encountered

- **`cargo build --workspace` ohne `SQLX_OFFLINE=true` schlaegt fehl:** Die `sqlx::query!` Macros in `genossi_dao_impl_sqlite` benoetigen entweder eine `DATABASE_URL` oder Offline-prepared queries. Der Worktree hat keinen `genossi.db`-File. Loesung: `SQLX_OFFLINE=true` setzt, nutzt den vorhandenen `.sqlx/`-Cache. Das ist projekt-uebliches Verhalten und betrifft nicht die Aenderungen dieses Plans.
- **Pre-existing staged changes:** Beim Start waren `.planning/research/SUMMARY.md` (added) und `.planning/phases/06-.../06-UI-SPEC.md` (deleted from index) im Index gestaged — nicht aus dieser Plan-Ausfuehrung. Habe sie via `git reset HEAD` unstaged, um sie nicht in meine Task-Commits zu mischen. Sie bleiben als untracked/working-tree-state im Repo fuer den User zu bereinigen.

## User Setup Required

None — keine externen Service-Konfigurationen noetig.

## Next Phase Readiness

- **Plan 02 (Service-Impl) ist freigeschaltet:** Kann ohne weitere Cargo.toml-Aenderungen `use rust_xlsxwriter::Workbook;` und `use csv::WriterBuilder;` in `genossi_service_impl/src/attendance_export.rs` schreiben.
- **Plan 02/03 (PDF-Render):** Kann das neue Template via `PdfGenerator::render_attendance_list(...)` (neue Methode, in Plan 02 zu implementieren) gegen `templates/teilnehmerliste.typ` aufrufen. Das Template existiert und passt zum bestehenden `_layout.typ`.
- **Keine Blocker.**

## Self-Check: PASSED

Files verifiziert:
- FOUND: `Cargo.toml`
- FOUND: `genossi_service_impl/Cargo.toml`
- FOUND: `genossi_bin/Cargo.toml`
- FOUND: `genossi_rest/Cargo.toml`
- FOUND: `templates/teilnehmerliste.typ`
- FOUND: `.planning/phases/06-teilnehmerlisten-export-f-r-generalversammlungen/06-01-SUMMARY.md`

Commits verifiziert:
- FOUND: `b42886b` (Task 1: Workspace dep promotion)
- FOUND: `f86ae65` (Task 2: Typst template)

Build verifiziert: `SQLX_OFFLINE=true cargo build --workspace` Exit-Code 0.

---
*Phase: 06-teilnehmerlisten-export-f-r-generalversammlungen*
*Completed: 2026-05-17*
