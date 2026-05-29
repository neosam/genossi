---
phase: 04-frontend-component-first-mit-qr-scanner-und-manual-code-fall
plan: 06b
subsystem: ui
tags: [dioxus, components, mod.rs, single-writer, wave-serialization]

requires:
  - phase: 04-04
    provides: attendance_list, attendance_search, live_counter, connection_banner Component-Files
  - phase: 04-05
    provides: manual_code_input, qr_scanner, qr_card, helper_shell Component-Files
  - phase: 04-06
    provides: assembly_status_badge, assembly_list_row, tab_strip, toast, token_row, create_token_form, basics_tab Component-Files
provides:
  - mod.rs Single-Writer Update — alle 15 Wave-2.1-Components sind als pub mod deklariert
  - 15 pub use Re-Exports — Pages 04-07/08/09 koennen `use crate::component::{AttendanceList, QrScanner, ...}` verwenden
  - Append-only Diff fuer mod.rs (kein Re-Order, keine bestehenden Zeilen geaendert)
affects: [04-07, 04-08, 04-09]

tech-stack:
  added: []
  patterns:
    - "Single-Writer-Pattern fuer mod.rs in Wave-Serialization (vermeidet Merge-Konflikte zwischen parallelen Plans)"

key-files:
  created: []
  modified:
    - genossi-frontend/src/component/mod.rs
    - genossi-frontend/src/component/tab_strip.rs (Wave-2.1 Bug-Fix)
    - genossi-frontend/src/component/basics_tab.rs (Wave-2.1 Bug-Fix)
    - genossi-frontend/src/component/assembly_list_row.rs (Wave-2.1 Bug-Fix)

key-decisions:
  - "Append-only Edit ohne Sortierung der bestehenden 26 pub mod / 19 pub use Zeilen"
  - "Drei zusaetzliche Re-Exports ueber must_haves hinaus: AttendanceToggleRequest, CameraPath, decide_camera_path — externe Helfer aus Wave-2.1 die Pages 07-09 benoetigen"
  - "Wave-2.1-Bugs (Debug-Derives, fehlende Route) werden in einem separaten fix-Commit minimal-invasiv behoben — sonst ist cargo build / cargo test rot und das Plan-Ziel grueener Build unerreichbar"
  - "BasicsMode bleibt privat (kein pub) und wird nicht re-exportiert — der Test im Modul-File braucht aber Debug fuer assert_eq!"

patterns-established:
  - "Phase-Kommentar-Bloecke `// === Phase 4 Plan 04 ===` strukturieren neue Sektionen visuell"

requirements-completed: []

duration: ~10min
completed: 2026-05-05
---

# Phase 04 Plan 06b: mod.rs Single-Writer Summary

**mod.rs nimmt 15 Wave-2.1-Components in Empfang — Pages 07-09 koennen jetzt sauber importieren.**

## Performance

- **Tasks:** 1 (Append `pub mod` + `pub use` fuer alle 15 neuen Component-Files)
- **Files modified:** 4 (1 Plan-Hauptaufgabe + 3 Wave-2.1-Bug-Fixes)

## Accomplishments

- 15 `pub mod` Zeilen append-only ergaenzt (12 plan-must-have + 3 W-04-Extraction-Components)
- 15 `pub use` Re-Exports ergaenzt — alle in `must_haves` gelisteten Public Types plus drei zusaetzliche externe Helfer (`AttendanceToggleRequest`, `CameraPath`, `decide_camera_path`)
- `cargo test` Lauf: **108 Tests pass** (Baseline 68 vor Wiring; +40 neue Tests durch Wave-2.1-Components werden jetzt ausgefuehrt)
- Diff fuer `mod.rs` ist append-only verifiziert (`git diff` zeigt nur `+` Zeilen, keine `-` Zeilen, keine bestehenden Zeilen modifiziert)

## Task Commits

1. **Plan-Hauptaufgabe: mod.rs Single-Writer** — `96a0c0e` (feat)
2. **Wave-2.1 Bug-Fixes** — `9de254e` (fix)
3. **Plan SUMMARY** — separater Commit (docs)

## Files Created/Modified

- `genossi-frontend/src/component/mod.rs` — +46 Zeilen (15 `pub mod` + 15 `pub use`); keine bestehenden Zeilen geaendert
- `genossi-frontend/src/component/tab_strip.rs` — `#[derive(Clone, PartialEq)]` -> `#[derive(Clone, Debug, PartialEq)]` auf `TabDef` (Test-Fixture braucht Debug fuer `assert_eq!`)
- `genossi-frontend/src/component/basics_tab.rs` — `#[derive(Copy, Clone, PartialEq, Eq)]` -> `#[derive(Copy, Clone, Debug, PartialEq, Eq)]` auf `BasicsMode`
- `genossi-frontend/src/component/assembly_list_row.rs` — `Link { to: Route::AssemblyDetails {...} }` ersetzt durch `a { href: "/assemblies/{id}" }` weil `Route::AssemblyDetails`-Variante erst in Plan 04-09 zum Router hinzugefuegt wird

## Decisions Made

**Append-only ohne Re-Order:** Plan verlangt explizit kein Sortieren. Bestehende 26 `pub mod` + 19 `pub use` Zeilen sind unveraendert.

**Zusaetzliche Re-Exports ueber must_haves hinaus:**
- `AttendanceToggleRequest` (struct in `attendance_list.rs`) — Bridge-Struct das Pages 07/08 fuer den `on_toggle` EventHandler benoetigen
- `CameraPath` + `decide_camera_path` (qr_scanner.rs) — testbare Helfer, die Plan 04-08 in Helper-Logout/Camera-Switch wiederverwendet
- `ConnState` (live_counter.rs) — bereits in plan must_haves, korrekt re-exportiert

**Wave-2.1-Bugs:** 5 distinkte Compile-Fehler wurden durch das Wiring sichtbar (vorher kompilierten die Files nicht da sie nicht referenziert waren):
1. `tab_strip::TabDef` Test braucht Debug
2. `basics_tab::BasicsMode` Test braucht Debug (2x assert_eq!/assert_ne!)
3. `assembly_list_row` referenziert `Route::AssemblyDetails` die nicht existiert
Diese sind Wave-2.1-Implementations-Bugs, nicht mod.rs-Wiring-Probleme. Minimal-invasiv im separaten fix-Commit behoben damit `cargo build` und `cargo test` gruen sind (Plan-Success-Criteria).

## Deviations from Plan

### Auto-fixed Issues

**1. Wave-2.1 Component-Bugs blockieren Build nach Wiring**
- **Found during:** Task 1 (mod.rs Wiring), nach `cargo test`
- **Issue:** 5 Compile-Fehler in `tab_strip.rs`, `basics_tab.rs`, `assembly_list_row.rs` (Debug-Derives + fehlende Route)
- **Fix:** Debug-Derives ergaenzt; `Route::AssemblyDetails` durch plain `<a href>` ersetzt bis Plan 04-09 die Route definiert
- **Files modified:** Drei Component-Files (kein mod.rs!)
- **Verification:** `cargo test` -> 108 passed, 0 failed
- **Committed in:** `9de254e` (separat von mod.rs-Commit)

**2. Drei zusaetzliche Re-Exports ueber must_haves hinaus**
- **Found during:** Vorbereitung Wiring
- **Issue:** Wave-2.1-Components exportieren `AttendanceToggleRequest`, `CameraPath`, `decide_camera_path` als oeffentliche Bridge-/Helper-Types die nicht im `must_haves`-Block stehen
- **Fix:** Alle drei in mod.rs re-exportiert — Pages 07-09 brauchen sie laut Wave-2.1-Plans
- **Verification:** Build gruen, Tests gruen
- **Committed in:** `96a0c0e`

**Total deviations:** 2 auto-fixed (1 Bug-Fix-Sweep, 1 Re-Export-Erweiterung)
**Impact on plan:** Notwendig fuer Success-Criteria "Frontend cargo build gruen". Kein Scope-Creep — alles innerhalb des Plan-04-06b-Ziels (Wave-2.1 Components in mod.rs einbinden).

## Issues Encountered

Keine ueber die im "Deviations"-Block dokumentierten hinaus.

## Next Phase Readiness

- Pages 04-07 (Helper-Login-Page): kann `use crate::component::{HelperShell, ManualCodeInput, QrScanner, QrCard, decide_camera_path}` direkt nutzen
- Pages 04-08 (Helper-Anwesenheit-Page): kann `AttendanceList`, `AttendanceSearch`, `LiveCounter`, `ConnState`, `ConnectionBanner`, `AttendanceToggleRequest` importieren
- Pages 04-09 (Vorstand-Anwesenheit + AssemblyDetails-Page): kann `AssemblyStatusBadge`, `AssemblyListRow`, `TabStrip`, `TabDef`, `ToastContainer`, `show_toast`, `TokenRow`, `CreateTokenForm`, `BasicsTab` importieren
- Plan 04-09 muss `Route::AssemblyDetails` zum Router hinzufuegen und `assembly_list_row.rs` ggf. wieder auf typed Link migrieren

---
*Phase: 04-frontend-component-first-mit-qr-scanner-und-manual-code-fall*
*Completed: 2026-05-05*
