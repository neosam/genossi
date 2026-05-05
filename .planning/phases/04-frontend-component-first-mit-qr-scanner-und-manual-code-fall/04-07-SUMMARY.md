---
phase: 04-frontend-component-first-mit-qr-scanner-und-manual-code-fall
plan: 07
subsystem: ui

tags: [dioxus, router, frontend, wasm, layout-branching]

# Dependency graph
requires:
  - phase: 04
    provides: 04-04 i18n keys, 04-05 helper components, 04-06 vorstand components, 04-06b component/mod.rs wiring
provides:
  - 4 neue Route-Variants (HelperLogin, HelperAttendance, Assemblies, AssemblyDetails)
  - 4 Page-Stubs in src/page/ damit Route-Enum compiled
  - Layout-Branching in app.rs (/helper* ohne Auth/TopBar/Footer)
affects: [04-08, 04-09, 04-10, 04-11]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Layout-Branching via web_sys::window().location().pathname() — Option A aus 04-RESEARCH"
    - "Helfer-Routes ohne <Auth>-Wrapper, keine Vorstand-Navigation sichtbar (D-07)"

key-files:
  created:
    - genossi-frontend/src/page/helper_login.rs
    - genossi-frontend/src/page/helper_attendance.rs
    - genossi-frontend/src/page/assemblies.rs
    - genossi-frontend/src/page/assembly_details.rs
  modified:
    - genossi-frontend/src/router.rs
    - genossi-frontend/src/app.rs
    - genossi-frontend/src/page/mod.rs

key-decisions:
  - "Plan-Route-Variant für /helper heißt HelperLogin {} — UI-SPEC-Beispiel (Helper {}) war veraltet; Plan-Body und Briefing definieren HelperLogin als Variant-Name passend zum Page-Stub"
  - "Layout-Branch via pathname.starts_with(\"/helper\") — minimaler Eingriff, näher am bestehenden Pattern (RESEARCH Option A statt Dioxus #[layout(...)])"

patterns-established:
  - "Helfer-Layout: keine TopBar, kein Footer, kein Auth-Wrapper — nur DropdownBase + Stylesheet + Router"
  - "Vorstand-Layout: bestehender flex flex-col min-h-screen-Aufbau bleibt unverändert"

requirements-completed: []

# Metrics
duration: ~10min
completed: 2026-05-05
---

# Phase 04 Plan 07: Routing Foundation Summary

**Vier neue Phase-4-Routes (HelperLogin, HelperAttendance, Assemblies, AssemblyDetails) verkabelt + app.rs branched bei /helper*-Pfaden auf Helfer-Layout ohne Auth/TopBar/Footer.**

## Performance

- **Duration:** ~10 min
- **Tasks:** 2
- **Files modified:** 7 (4 created, 3 modified)
- **Tests:** 108 passed (unverändert vor/nach Plan)

## Accomplishments
- 4 neue Route-Varianten in `Route`-Enum (HelperLogin, HelperAttendance, Assemblies, AssemblyDetails { id })
- 4 Page-Stubs in `src/page/` mit den von Plan 04-08/04-09 erwarteten public Type-Namen
- Layout-Branching in `app.rs` über `pathname.starts_with("/helper")` — Helfer-Routes umgehen `<Auth>`, `<TopBar>` und `<Footer>` (D-05/D-07)
- `cargo check` grün, `cargo test` grün (108/108)

## Task Commits

1. **Task 1: Page-Stubs anlegen + Route-Enum erweitern** — `46508c9` (feat)
2. **Task 2: app.rs Layout-Branching für /helper*-Routes** — `a577447` (feat)

## Files Created/Modified
- `genossi-frontend/src/page/helper_login.rs` — Stub `pub fn HelperLogin() -> Element` (Plan 04-09 ersetzt Body)
- `genossi-frontend/src/page/helper_attendance.rs` — Stub `pub fn HelperAttendance() -> Element` (Plan 04-09)
- `genossi-frontend/src/page/assemblies.rs` — Stub `pub fn Assemblies() -> Element` (Plan 04-08)
- `genossi-frontend/src/page/assembly_details.rs` — Stub `pub fn AssemblyDetails(id: String) -> Element` (Plan 04-08)
- `genossi-frontend/src/page/mod.rs` — `pub mod` + `pub use` für die 4 neuen Pages
- `genossi-frontend/src/router.rs` — 4 neue `pub use`-Re-Exports + 4 neue Route-Varianten in `Route`-Enum
- `genossi-frontend/src/app.rs` — `is_helper_route`-Branch vor dem RSX-Block; Helfer-Branch ohne `<Auth>`/`<TopBar>`/`<Footer>`, Vorstand-Branch wie zuvor

## Page-Stub Public Type-Namen (für Plan 04-08/04-09)
- `crate::page::HelperLogin` (Route `/helper`)
- `crate::page::HelperAttendance` (Route `/helper/attendance`)
- `crate::page::Assemblies` (Route `/assemblies`)
- `crate::page::AssemblyDetails { id: String }` (Route `/assemblies/:id`)

Alle vier sind in `router.rs` und `page/mod.rs` re-exportiert; `cargo check` bestätigt Auflösung.

## Decisions Made
- **Variant-Name `HelperLogin` für `/helper`** statt UI-SPEC-Beispiel `Helper {}`: Briefing und Plan-Task-1 definieren explizit `HelperLogin {}` als Variant-Namen passend zum Page-Stub `pub fn HelperLogin`. UI-SPEC-Code-Block (S. 705-706) ist veraltet und wird in Plan 04-10 oder via Doku-Sync angepasst.
- **Layout-Branch via `pathname()`** (RESEARCH Option A) statt Dioxus `#[layout(...)]`: minimaler Eingriff, näher am bestehenden if/else-Pattern in `app.rs`, leichter rückbaubar. Migration auf Layout-Annotation wäre Phase-5+-Refactor.
- **HelperShell NICHT in app.rs** verkabelt: Plan-07-Briefing erwähnt HelperShell als Wrapper, aber die Plan-Task-2-Action zeigt explizit nur `Router::<Route> {}` im Helfer-Branch (kein `HelperShell {}`). Plan 04-09 wird `HelperShell` innerhalb der einzelnen Helper-Pages verwenden, sodass der Wrapper-Scope page-lokal bleibt — passt zum Component-First-Pattern.

## Deviations from Plan
None — Plan exakt wie spezifiziert ausgeführt. Plan 04-06 `assembly_list_row.rs`-Migration zu `Route::AssemblyDetails`-Link wurde NICHT durchgeführt (im Briefing als optional markiert, Tasks-Liste enthielt sie nicht).

## Issues Encountered
None.

## User Setup Required
None.

## Next Phase Readiness
- Plan 04-08 (Vorstand): kann `Assemblies`/`AssemblyDetails`-Page-Stubs ausimplementieren — Route-Enum + Re-Export bereit.
- Plan 04-09 (Helper): kann `HelperLogin`/`HelperAttendance`-Page-Stubs ausimplementieren; `is_helper_route`-Branch garantiert kein Vorstand-Layout-Leak.
- `HelperShell` ist über `crate::component::HelperShell` weiterhin verfügbar (Wave 2.2 mod.rs-Wiring) und wartet auf Verwendung in Plan 04-09.

---
*Phase: 04-frontend-component-first-mit-qr-scanner-und-manual-code-fall*
*Completed: 2026-05-05*
