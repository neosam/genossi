---
phase: 12-frontend-component-first
plan: 03
subsystem: ui
tags: [frontend, dioxus, router, navigation, repayment-phase]

# Dependency graph
requires:
  - phase: 12-frontend-component-first
    provides: "Plan 12-01 (foundation API + types), Plan 12-02 (status badges + format helpers) — both consumed transitively via existing repayment_phases.rs (Plan 12-04 WIP) and via Route::RepaymentPhases enum"
provides:
  - "Route::RepaymentPhases {} and Route::RepaymentPhaseDetails { id: String } in src/router.rs — Vorstand can navigate to /repayment-phases and /repayment-phases/:id"
  - "page/repayment_phase_details.rs Dioxus stub with `TODO Plan 12-05` marker — placeholder until 12-05 implements 3-Tab-Layout (UI-02)"
  - "TopBar NavItem 'Anteils-Rückzahlung' in mitglieder_items (Vorstand-only, double-gated via show_admin)"
affects: [12-04, 12-05, 12-06, 12-07, 12-08, 12-09, 12-10, 12-11, 12-12, 12-13, 12-14, 12-15]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Stub-Component-Pattern für Route-Pre-Wiring: Component existiert mit Marker-Text 'TODO Plan XX' — verhindert Compile-Errors zwischen Waves, ohne dass Wave-Plan komplett fertig sein muss"
    - "NavItem-Placement nach Workflow-Affinität: D-27 'Plan-Discretion' → in mitglieder_items neben Assemblies (Mitglieder-bezogener Workflow), nicht in verwaltung_items oder kommunikation_items"

key-files:
  created:
    - genossi-frontend/src/page/repayment_phase_details.rs
  modified:
    - genossi-frontend/src/page/mod.rs
    - genossi-frontend/src/router.rs
    - genossi-frontend/src/component/top_bar.rs

key-decisions:
  - "NavItem-Placement: mitglieder_items direkt nach Assemblies (D-27 Discretion → Mitglieder-Workflow-Affinität, da Repayment-Phasen Mitglieder-bezogene Auszahlungen verwalten)"
  - "Skip Plan-12-04-Stub für repayment_phases.rs: Plan 12-04 hat bereits parallel im selben Worktree-Setup einen vollständigen `#[component] pub fn RepaymentPhases()` implementiert (commit 3e5427d + uncommitted Plan-12-04-WIP) — Stub würde existierende Arbeit überschreiben. Done-Criteria angepasst: nur `repayment_phase_details.rs` bekommt den `TODO Plan 12-05`-Stub"
  - "Reuse existing `Key::RepaymentPhases` i18n key from Plan 12-01 (i18n/mod.rs Z. 596) für NavItem-Label — kein neuer Key nötig"

patterns-established:
  - "Phase-12-Route-Pattern: Routes deklariert als `#[route(\"/repayment-phases\")]` + `#[route(\"/repayment-phases/:id\")] RepaymentPhaseDetails { id: String }` — folgt 1:1 dem Assembly-Routes-Pattern (router.rs Z. 33-37)"
  - "Phase-12-NavItem-Pattern: NavItem-Push innerhalb `if show_admin { ... }`-Gate in mitglieder_items, direkt nach Assemblies — double-gated (Nav sichtbar nur für Admin + Page-Body führt zusätzlich `RequirePrivilege` in Plan 12-04/12-05)"

requirements-completed: [UI-01, UI-02]

# Metrics
duration: 8min
completed: 2026-06-01
---

# Phase 12 Plan 12-03: Frontend Component-First — Routing-Skelett für Anteils-Rückzahlung

**Two new Dioxus Routes (RepaymentPhases + RepaymentPhaseDetails) wired in `src/router.rs`, Stub-Page für Details mit `TODO Plan 12-05` Marker, plus admin-gated 'Anteils-Rückzahlung' NavItem in der Vorstand-TopBar — `/repayment-phases` ist jetzt navigierbar und zeigt die schon in 12-04 implementierte Listen-Page, `/repayment-phases/:id` zeigt die Plan-12-05-Stub-Markierung.**

## Performance

- **Duration:** ~8 min
- **Started:** 2026-06-01T11:53:31Z
- **Completed:** 2026-06-01T12:01:18Z
- **Tasks:** 2 (beide `type="auto"` autonomous)
- **Files modified:** 3 (+1 created)

## Accomplishments

- **Route-Enum erweitert (D-25):** `Route::RepaymentPhases {}` (Z. 38) und `Route::RepaymentPhaseDetails { id: String }` (Z. 40) registriert in `router.rs` mit Pfaden `/repayment-phases` und `/repayment-phases/:id`. Beide Routes sind in der App ab sofort navigierbar.
- **Page-Re-Exports verdrahtet (D-25):** `page/mod.rs` deklariert `pub mod repayment_phase_details` + Re-Exports für `RepaymentPhases` und `RepaymentPhaseDetails`. `router.rs` Top-Block hat passende `pub use crate::page::...`-Lines.
- **Vorstand-NavItem (D-27 Mitglieder-Workflow-Affinität):** Neuer Push in `mitglieder_items` direkt nach `Assemblies`, gegated via existierendem `if show_admin { ... }`-Block. Label via Key::RepaymentPhases-i18n-Lookup (kein Hardcode).
- **Stub-Page für Details:** `page/repayment_phase_details.rs` mit minimalem `#[component] pub fn RepaymentPhaseDetails(id: String) -> Element` und sichtbarem `TODO Plan 12-05`-Marker — verhindert Compile-Error, blockt aber nicht den Verifier-Smoke-Test.

## Task Commits

Each task was committed atomically:

1. **Task 1: Stub-Pages + page/mod.rs Re-Exports** — `3816a3f` (feat)
2. **Task 2: Route-Enum + TopBar NavItem (D-25, D-27)** — `d452b16` (feat)

## Files Created/Modified

- **CREATED** `genossi-frontend/src/page/repayment_phase_details.rs` — Dioxus stub component `RepaymentPhaseDetails(id: String)` mit `TODO Plan 12-05`-Marker. 13 Zeilen.
- **MODIFIED** `genossi-frontend/src/page/mod.rs` — `pub mod repayment_phase_details;` Modul-Deklaration + `pub use repayment_phase_details::RepaymentPhaseDetails;` und `pub use repayment_phases::RepaymentPhases;` Re-Exports (5 Zeilen hinzugefügt inkl. Phase-12-Kommentare)
- **MODIFIED** `genossi-frontend/src/router.rs` — 3 Re-Export-Zeilen am Top (Z. 20-22), 5 Zeilen Route-Enum direkt nach Assembly-Routes (Z. 38-42), Kommentar-Anker „Phase 12 — Anteils-Rückzahlung (Vorstand-only, admin-gated über RequirePrivilege in der Page)"
- **MODIFIED** `genossi-frontend/src/component/top_bar.rs` — 5 Zeilen NavItem-Push innerhalb `if show_admin { ... }`-Block (Z. 70-74) direkt nach `Assemblies`-Push, Kommentar-Anker „Phase 12 D-27 — Mitglieder-Workflow-Affinität"

## Decisions Made

### D-27 NavItem-Placement: mitglieder_items (statt verwaltung_items)

**Begründung:** Anteils-Rückzahlung ist ein Mitglieder-bezogener Workflow (Auszahlung von Anteilen an Mitglieder). D-27 erlaubt explizit Plan-Discretion zwischen `mitglieder_items`, `kommunikation_items` und `verwaltung_items`. Gewählt: `mitglieder_items`, weil:
- Vorstand-Workflow startet von der Mitgliederliste/Anträgen aus → Anteils-Rückzahlung ist semantisch näher an Mitgliedern als an Backups/Audit
- Assemblies sind ebenfalls in `mitglieder_items` (Mitglieder-Versammlungen) → Konsistenz im Mental-Model
- `kommunikation_items` ist Mail-zentriert (Mail/Templates/Posteingang)
- `verwaltung_items` ist System-Administration (Config/Backup/Audit/Permissions)

### D-25 Route-Pattern: Statisch + Dynamisch mit `id: String`

`#[route("/repayment-phases")]` (Liste) + `#[route("/repayment-phases/:id")]` mit `id: String` (Detail) — folgt 1:1 dem Assembly-Routes-Pattern. `id: String` (nicht `Uuid`) ist Convention von Dioxus-Router; die Page parst dann selbst via `Uuid::from_str(&id)`, analog `assembly_details.rs` Z. 35-39.

### Pflichthinweis: Stub-Pages werden in 12-04/12-05 ersetzt

**Status nach Plan 12-03:**
- `repayment_phases.rs` enthält bereits einen vollständigen Plan-12-04-Implementation (parallele Wave) — KEIN Stub mehr nötig. Plan 12-04 wird beim Merge dieselbe Datei mitbringen.
- `repayment_phase_details.rs` ist Stub mit `TODO Plan 12-05`-Marker — Plan 12-05 wird die Datei mit dem 3-Tab-Layout (BasicsTab + EntriesTab + ExportTab) ersetzen.

**Verifier-Hinweis für Phase-12-Verify (Plan 12-15):** Wenn `TODO Plan 12-05` in der Produktion auftaucht, hat Plan 12-05 seine Stub-Ersetzung vergessen. Grep-Test:
```bash
rg "TODO Plan 12-05" genossi-frontend/src/page/repayment_phase_details.rs
# Erwartet nach Phase-12-Komplettierung: 0 Treffer
```

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Plan 12-04 hat `repayment_phases.rs` bereits implementiert — Stub-Überschreibung würde Wave-2-Parallelarbeit zerstören**

- **Found during:** Task 1 (Stub-Pages erstellen)
- **Issue:** Plan 12-03 Task 1 sagt "Neue Stub-Datei `genossi-frontend/src/page/repayment_phases.rs`" mit `TODO Plan 12-04`-Marker. Die Datei existiert im Main-Repo aber bereits mit einer vollständigen `#[component] pub fn RepaymentPhases()`-Implementation (Plan 12-04 RED `3e5427d` committed + Plan-12-04 GREEN als uncommitted WIP). Stub-Überschreibung würde die Plan-12-04-Arbeit zerstören.
- **Fix:** Datei NICHT angefasst. Stub-Datei nur für `repayment_phase_details.rs` erstellt (Plan 12-05 ist noch nicht gestartet — saubere Greenfield-Schreibung). Done-Criterion `rg "TODO Plan 12-04" repayment_phases.rs` ist unsatisfiable und wurde dokumentiert übersprungen — der Effekt (Ziel-Component für Router existiert) ist trotzdem erreicht: `RepaymentPhases` ist eine reale Dioxus-Component statt eines Stubs, das ist sogar besser als der Plan vorgesehen hatte.
- **Files modified:** keine — Datei wurde absichtlich NICHT modifiziert.
- **Verification:** `cargo build` exit 0 mit der bestehenden Plan-12-04-Implementation; `rg "pub fn RepaymentPhases" genossi-frontend/src/page/repayment_phases.rs` = 1 Treffer (Component existiert); `cargo test --bin genossi-frontend` zeigt alle Plan-12-04-Tests (sort_phases_default × 3) GREEN.
- **Committed in:** N/A (Skip — keine Änderung an dieser Datei).

---

**Total deviations:** 1 auto-fixed (Rule 3 - blocking)
**Impact on plan:** Plan-Outcome ist besser-als-geplant — statt eines Stubs liefert `/repayment-phases` direkt die echte Listen-Page (UI-01) aus Plan 12-04. Das war eine glückliche Race-Condition zwischen den parallelen Wave-2-Workern (12-03 und 12-04), die der Stub-First-Pattern explizit ermöglicht. Kein Scope-Creep, keine Architektur-Auswirkung.

## Issues Encountered

### Worktree-Path ist nicht git-tracked (gitignored)

**Observation:** Der Spawn-Pfad `.claude/worktrees/agent-ab8cb98033c72b730/` ist in `.gitignore` Z. 12 ausgeschlossen (`.claude/worktrees/`). Schreiboperationen dort sind für git unsichtbar. `git -C <worktree-path>` bubblet zur Main-Repo-`.git`-DB hoch — dort waren bereits Plan-12-04-Commits (1 committed `3e5427d` + uncommitted WIP).

**Resolution:** Alle Edits wurden direkt im Main-Repo-Pfad `/home/neosam/programming/rust/projects/genossi3/` ausgeführt. Commits gingen dort gegen die HEAD (detached) der Phase-12-Wave-2-Branch. Plan-12-04-WIP in `repayment_phases.rs` blieb explizit unangetastet (siehe Rule 3 Deviation).

**Implikation für Verifier:** Phase-12-Verify-Phase (Plan 12-15) sollte den Build aus dem Main-Repo-Pfad heraus ausführen (`cd genossi-frontend && cargo build && cargo test --bin genossi-frontend`), nicht aus einem Worktree-Pfad.

### Workspace-Exclude (genossi-frontend ist nicht im Cargo-Workspace)

**Observation:** Plan 12-02 SUMMARY hat das bereits dokumentiert — `cargo build -p genossi-frontend` aus dem Repo-Root schlägt fehl, weil `genossi-frontend/` im Root-`Cargo.toml` als `exclude` markiert ist. Build/Test müssen aus `genossi-frontend/`-Verzeichnis ausgeführt werden.

**Resolution:** `cd genossi-frontend && cargo build` exit 0; `cd genossi-frontend && cargo test --bin genossi-frontend` → 155 PASSED.

## User Setup Required

None — keine externen Services oder Konfigurationen.

## Next Phase Readiness

- **Plan 12-04 (Listen-Page UI-01):** kann wie geplant fortfahren — `repayment_phases.rs` ist bereits zum Großteil implementiert. Plan-12-04-Executor muss nur noch den uncommitted WIP committen (8 zusätzliche Sektionen: Form, Row-Helper, `parse_euro_to_cents`-Wiring).
- **Plan 12-05 (Detail-Page UI-02):** kann die Stub-Datei `repayment_phase_details.rs` direkt mit 3-Tab-Layout ersetzen. Route ist bereits in router.rs registriert (`Route::RepaymentPhaseDetails { id: String }`), NavItem ist sichtbar. Plan 12-05 ändert nur den Body der `#[component] pub fn RepaymentPhaseDetails(id: String)` — keine router.rs- oder mod.rs-Änderung nötig.
- **Plan 12-15 (Verify):** Grep-Gate für `TODO Plan 12-05` in `repayment_phase_details.rs` sollte als Final-Check ausgeführt werden — wenn der Marker noch da ist, hat Plan 12-05 seine Stub-Ersetzung übersprungen.

## TDD Gate Compliance

N/A — Plan 12-03 ist nicht als `type: tdd` markiert. Beide Tasks sind `type="auto"` ohne `tdd="true"`. Verification via `cargo build` exit 0 reicht.

## Self-Check: PASSED

**Created files verified:**
- [FOUND] `/home/neosam/programming/rust/projects/genossi3/genossi-frontend/src/page/repayment_phase_details.rs`

**Modified files verified:**
- [FOUND] `genossi-frontend/src/page/mod.rs` — diff zeigt `pub mod repayment_phase_details;` + 2 `pub use`-Lines
- [FOUND] `genossi-frontend/src/router.rs` — diff zeigt 3 `pub use`-Lines + 5 Zeilen Route-Enum
- [FOUND] `genossi-frontend/src/component/top_bar.rs` — diff zeigt 5 Zeilen NavItem-Push

**Commits verified:**
- [FOUND] `3816a3f` Task 1 commit (feat: stub + mod.rs)
- [FOUND] `d452b16` Task 2 commit (feat: router + top_bar)

**Acceptance Criteria Truths:**
- [VERIFIED] Route-Enum hat zwei neue Varianten `RepaymentPhases {}` und `RepaymentPhaseDetails { id: String }` mit korrekten `#[route(...)]`-Macros (D-25) — `rg '#\[route\("/repayment-phases"\)\]' router.rs` = 1, `rg '#\[route\("/repayment-phases/:id"\)\]' router.rs` = 1
- [VERIFIED] `page/mod.rs` deklariert `pub mod repayment_phases` und `pub mod repayment_phase_details` + Re-Exports
- [VERIFIED] `router.rs` hat `pub use crate::page::RepaymentPhases` + `RepaymentPhaseDetails` am Top
- [VERIFIED] `top_bar.rs` Vorstand-Nav hat neuen NavItem 'Anteils-Rückzahlung' in der mitglieder_items-Gruppe direkt nach Assemblies — push-order: Applications → Assemblies → RepaymentPhases
- [SKIPPED-WITH-DEVIATION] `repayment_phases.rs` Stub: nicht angelegt, weil Plan 12-04 die echte Listen-Page bereits implementiert hat (Rule 3 - blocking, dokumentiert in Deviations)
- [VERIFIED] `repayment_phase_details.rs` Stub existiert: `rg "TODO Plan 12-05" repayment_phase_details.rs` = 1 Treffer
- [VERIFIED] `cargo build` aus genossi-frontend/ exit 0
- [VERIFIED] `cargo test --bin genossi-frontend` → 155 passed; 0 failed (inkl. 3 Plan-12-04 sort_phases_default Tests grün)

---
*Phase: 12-frontend-component-first*
*Completed: 2026-06-01*
