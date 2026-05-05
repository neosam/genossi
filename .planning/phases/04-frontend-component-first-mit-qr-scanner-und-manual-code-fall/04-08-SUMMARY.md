---
phase: 04-frontend-component-first-mit-qr-scanner-und-manual-code-fall
plan: 08
subsystem: ui

tags: [dioxus, frontend, wasm, vorstand, attendance, component-first, attn-06]

# Dependency graph
requires:
  - phase: 04
    provides: 04-03 i18n keys + api functions, 04-04 attendance components, 04-05 QrCard, 04-06 vorstand components (TabStrip, AssemblyListRow, AssemblyStatusBadge, ToastContainer, BasicsTab, TokenRow, CreateTokenForm), 04-06b component/mod.rs wiring, 04-07 Route::Assemblies + Route::AssemblyDetails + Page-Stubs
provides:
  - Voll funktionsfähige Assemblies-Liste mit Create-Modal (Vorstand-only)
  - Voll funktionsfähige AssemblyDetails 3-Tab-Page (Stammdaten / Tokens / Anwesenheit)
  - ATTN-06 Reuse-Anker: Anwesenheits-Tab nutzt EXAKT dieselben 4 Components, die Plan 04-09 für /helper/attendance verwenden wird
  - Smart-Page-Pattern für AttendanceList: Page besitzt mark_present/mark_absent + refresh_signal-Bump (SYNC-01 acceptance)
affects: [04-09, 04-10, 04-11]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Free-function show_toast + Signal<Vec<(u64,String)>> + Signal<u64> als Toast-Pattern (Plan 04-06)"
    - "Modal als Wrapper (props.children) für Create-Forms"
    - "Smart-Parent für AttendanceList: on_toggle EventHandler ruft API + bumped refresh_signal — AttendanceList bleibt dumb"
    - "Just-Created-QrCard inline rendern (nicht in Modal) für Druckbarkeit; one-time-show via Signal<Option<HelperTokenCreateResponseTO>>"

key-files:
  created: []
  modified:
    - genossi-frontend/src/page/assemblies.rs
    - genossi-frontend/src/page/assembly_details.rs

key-decisions:
  - "AttendanceList exposes on_toggle (NICHT on_toggle_success wie im Plan-Briefing) — Page übernimmt API-Call (mark_present/mark_absent) und bumped refresh_signal bei 200 OK. Das ist der ATTN-06-Anker: gleiche dumb-list Component, smart-parent unterscheidet zwischen /helper/attendance und /assemblies/{id} via auth-context"
  - "AttendanceTab als Page-internal Component extrahiert — kapselt das vier-Komponenten-Shell + die SMART Toggle-Wiring in einem #[component] fn AttendanceTab. Plan 04-09 helper_attendance.rs kann denselben Wrapper-Code 1:1 übernehmen"
  - "Zwei Page-Stubs (helper_login.rs / helper_attendance.rs) bewusst NICHT verändert — gehören zu Plan 04-09"
  - "search_query als Signal<String> auf Page-Level definiert + an AttendanceList als ReadOnlySignal weitergereicht (Dioxus-0.6 auto-coerce: Signal<T> → ReadOnlySignal<T>)"
  - "TokensTab inline gehalten (Page-internal) — koordiniert page-spezifischen state (just_created One-Time-Show, Phase 2 D-21). Reused W-04-Components TokenRow + CreateTokenForm aus crate::component::*"

patterns-established:
  - "Vorstand-Page-Skelett: <RequirePrivilege privilege=\"admin\" fallback=AccessDeniedPage> + <TopBar> + container + <ToastContainer>"
  - "Create-Form-Modal-Pattern: Inner-Component nimmt on_close + on_created + on_error EventHandler, wrapping Page entscheidet via show_create-Signal"
  - "Smart-Parent / Dumb-Component für Attendance: Component dispatcht AttendanceToggleRequest, Parent ruft API + bumped Refresh-Signal"

requirements-completed:
  - SYNC-01: refresh_signal wird nach mark_present/mark_absent 200 OK incrementiert (siehe AttendanceTab on_toggle Wiring); LiveCounter polled alle 5s (Plan 04-04 erbt)

# Metrics
duration: ~25min
completed: 2026-05-05
---

# Phase 04 Plan 08: Vorstand-Pages Summary

**Zwei Vorstand-Pages voll ausimplementiert: `/assemblies` (Liste + Create-Modal) und `/assemblies/{id}` (3-Tab-Detail mit Stammdaten, Tokens, Anwesenheit). Anwesenheits-Tab nutzt EXAKT dieselben 4 Components, die Plan 04-09 für /helper/attendance verwenden wird — ATTN-06 Reuse-Anker etabliert.**

## Performance

- **Duration:** ~25 min
- **Tasks:** 2 (1 Page pro Task)
- **Files modified:** 2
- **Tests:** 108 passed (unverändert vor/nach Plan — keine neuen Page-Tests; Page-Logik wird in Phase 5 Generalprobe integration-getestet)
- **Lines:** assemblies.rs 182 LoC, assembly_details.rs 301 LoC

## Accomplishments

### `/assemblies` (genossi-frontend/src/page/assemblies.rs, 182 LoC)
- Wrapped in `<RequirePrivilege privilege="admin" fallback=AccessDeniedPage>`
- `<TopBar>` + Container-Layout
- Header mit `<h1>` + Create-Button (right-aligned)
- Liste rendert `<AssemblyListRow>` pro Versammlung (Card-Style mit Status-Badge)
- Empty-State: zentriertes Layout mit Headline + Hint + großem Create-Button
- Create-Modal: `<Modal>` wrapping inner `CreateAssemblyForm` (page-internal Component)
  - Felder: name (required, validiert), date (datetime-local, optional), location (optional)
  - Submit ruft `api::create_assembly`; on success → close + reload; on error → Toast
- `<ToastContainer>` für Fehlermeldungen

### `/assemblies/{id}` (genossi-frontend/src/page/assembly_details.rs, 301 LoC)
- Wrapped in `<RequirePrivilege privilege="admin" fallback=AccessDeniedPage>`
- Header: Assembly-Name + `<AssemblyStatusBadge>`
- `<TabStrip>` mit 3 Tabs: Stammdaten / Tokens / Anwesenheit
- **Stammdaten-Tab:** delegiert vollständig an `<BasicsTab>` (Plan 04-06 W-04). BasicsTab handlet Edit-Mode-Toggle, update_assembly, Open/Close-Confirm-Dialogs.
- **Tokens-Tab:** page-internal `<TokensTab>` Component
  - Liste rendert `<TokenRow>` (Plan 04-06 W-04) pro Token mit Revoke-Button
  - "+ Helfer-Token erstellen" öffnet `<Modal>` mit `<CreateTokenForm>` (Plan 04-06 W-04)
  - Bei erfolgreichem Token-Create: `<QrCard>` inline (NICHT in Modal — Druckbarkeit)
  - One-Time-Show via `Signal<Option<HelperTokenCreateResponseTO>>` (Phase 2 D-21)
  - "Schließen"-Button setzt just_created auf None — danach unrecoverable
- **Anwesenheits-Tab:** page-internal `<AttendanceTab>` Component (ATTN-06-Anker)
  - Status == Preparation: Hinweis-Text "Anwesenheits-Tab erst nach Eröffnung" (`AssemblyAttendanceNotOpenYet`)
  - Status == Open: alle 4 Components verkabelt mit polling_enabled=true, read_only=false
  - Status == Closed: alle 4 Components verkabelt mit polling_enabled=false, read_only=true
  - Smart-Parent-Wiring: `on_toggle: EventHandler<AttendanceToggleRequest>` ruft `mark_present` / `mark_absent` und bumped `refresh_signal` bei 200 OK (SYNC-01)
- `<ToastContainer>` für alle Tab-übergreifenden Fehlermeldungen

## Components Reused (ATTN-06 Verifikation)

### `/assemblies/{id}` Anwesenheits-Tab nutzt:
1. `<ConnectionBanner visible=...>` (Plan 04-04)
2. `<LiveCounter assembly_id=... polling_enabled=... on_connection_state=...>` (Plan 04-04)
3. `<AttendanceSearch on_change=...>` (Plan 04-04)
4. `<AttendanceList assembly_id=... search_query=... read_only=... refresh_signal=... on_toggle=...>` (Plan 04-04)

→ Plan 04-09 `helper_attendance.rs` wird die EXAKT gleichen 4 Components mit identischer Wiring nutzen, lediglich der `read_only`-Flag und der Token-Source unterscheiden sich. Hard-Anker via `grep -E "AttendanceList|AttendanceSearch|LiveCounter|ConnectionBanner"` → 9 Treffer in assembly_details.rs.

### `/assemblies/{id}` weitere Component-Imports (alles aus crate::component::*):
- `AssemblyStatusBadge` (Plan 04-06)
- `TabStrip` + `TabDef` (Plan 04-06)
- `Modal` (existing)
- `ToastContainer` + `show_toast` (Plan 04-06)
- `TopBar` (existing)
- `BasicsTab` (Plan 04-06 W-04 extracted)
- `TokenRow` (Plan 04-06 W-04 extracted)
- `CreateTokenForm` (Plan 04-06 W-04 extracted)
- `QrCard` (Plan 04-05)

### `/assemblies` Component-Imports:
- `AssemblyListRow` (Plan 04-06)
- `Modal` (existing)
- `ToastContainer` + `show_toast` (Plan 04-06)
- `TopBar` (existing)

→ Beide Pages bleiben strikt Component-First: keine inline-Definitionen von BasicsTab/TokenRow/CreateTokenForm in assembly_details.rs (grep-verifiziert: `! grep -E '^\s*fn (BasicsTab|TokenRow|CreateTokenForm)\('`).

## Toast/Modal Pattern Used

- **Toast:** Plan-04-06-Pattern — free-function `show_toast(&mut Signal<Vec<(u64,String)>>, &mut Signal<u64>, msg)`. Caller hält beide Signals, gibt sie als ReadOnlySignal an `<ToastContainer>` weiter (Dioxus 0.6 auto-coerce). Auto-Dismiss nach 5s ist intern in show_toast.
- **Modal:** Existing `<Modal>` Component (props.children). Create-Forms (CreateAssemblyForm, CreateTokenForm) werden als Children in `<Modal>` gewrappt; Page hält `show_*`-Signal zur Sichtbarkeit.
- **Confirm-Dialogs (Open/Close/Revoke):** Liegen alle in den extrahierten W-04-Components (BasicsTab, TokenRow); Page muss nichts dafür tun.

## Task Commits

1. **Task 1: Assemblies List-Page mit Create-Modal** — `7e26ccb` (feat)
2. **Task 2: AssemblyDetails 3-Tab-Page** — `4ad52d3` (feat)

## Files Created/Modified

- `genossi-frontend/src/page/assemblies.rs` — Stub-Replacement; 182 LoC. Liste + Create-Modal + Toast.
- `genossi-frontend/src/page/assembly_details.rs` — Stub-Replacement; 301 LoC. 3-Tab-Layout + 2 Page-internal Components (TokensTab, AttendanceTab).

## Decisions Made

- **AttendanceList API-Vertrag korrigiert:** Briefing erwähnte `on_toggle_success`, der existierende Component (Plan 04-04) exposed aber `on_toggle: EventHandler<AttendanceToggleRequest>`. Die Page hält die SMART-Logik (mark_present/mark_absent + refresh_signal-Bump). Das ist exakt der ATTN-06-Anker: dumb-list, smart-parent. Plan 04-09 wird denselben Wrapper-Code für /helper/attendance verwenden — nur die auth-cookie-Bedingung unterscheidet sich.
- **AttendanceTab als Page-internal Component:** Statt das 4-Components-Shell + die Toggle-Wiring inline in den match-Arm zu schreiben, in einen `#[component] fn AttendanceTab` extrahiert. Damit ist der Code-Block exakt wiederverwendbar in Plan 04-09 helper_attendance.rs (1:1 copy-paste, lediglich Imports + assembly_id-Source unterscheiden sich).
- **search_query auf Page-Level:** Damit `<AttendanceSearch on_change>` und `<AttendanceList search_query>` über dasselbe Signal kommunizieren, lebt es im `AssemblyDetails`-Scope und wird durch ATTN-Tab geleitet.
- **show_toast als &mut-Pattern:** Plan-04-06 SUMMARY-konform; alternative wäre Context-Provider, was aber Refactor von members.rs/applications_page.rs implizieren würde — bewusst nicht im Scope von Plan 04-08.
- **Closed-Status zeigt read_only=true:** UI-SPEC §AttendanceList: `read_only={assembly_status == Closed}`. Vorstand kann nach Schließen keine Toggles mehr machen. Open-Status erlaubt Toggles (read_only=false).

## Deviations from Plan

- **Plan-Briefing erwähnte `on_toggle_success`** — der tatsächliche Component-Vertrag (Plan 04-04, attendance_list.rs:59) ist `on_toggle: EventHandler<AttendanceToggleRequest>`. Korrekt umgesetzt: SMART-Parent verdrahtet via `mark_present` / `mark_absent` und bumped `refresh_signal` bei 200 OK. Das ist die intendierte ATTN-06-Architektur (Component dumb, Page smart).
- **`<TokensTab>` und `<AttendanceTab>` als Page-internal Components:** Plan-Vorlage hatte `TokensTab` inline + Anwesenheits-RSX inline. Ich habe Anwesenheits-RSX in `<AttendanceTab>` extrahiert, weil die Toggle-Wiring + 4-Component-Shell exakt von Plan 04-09 gespiegelt wird — daher als Page-Boundary klarer trennbar. KEINE Verschiebung in `crate::component::` (würde mit Plan-04-09-Wave kollidieren).
- Keine sonstigen Abweichungen.

## Issues Encountered

- **`ToastContainer { messages: toast_messages.into() }`** scheiterte an Dioxus-Type-Inference (E0283). Lösung: `messages: toast_messages` (Dioxus 0.6 auto-coerced `Signal<T>` → `ReadOnlySignal<T>`). Dasselbe Pattern in `<AttendanceList search_query: search_query>` und `<AttendanceList refresh_signal: refresh_signal>` ohne `.into()` verwendet — kompiliert sauber.

## User Setup Required

None.

## Next Phase Readiness

- **Plan 04-09 (Helper-Pages):** Kann `<AttendanceTab>`-Pattern als Vorlage verwenden — die Toggle-Wiring (mark_present/mark_absent + refresh_signal-Bump) ist identisch. Lediglich `read_only` per HelperSession-Status und Cookie-basierte Auth unterscheidet sich.
- **Plan 04-10 (Cross-Cutting Verifications):** ATTN-06 Reuse-Anker bereit für grep-Vergleich:
  ```
  grep -E "AttendanceList|AttendanceSearch|LiveCounter|ConnectionBanner" \
    genossi-frontend/src/page/assembly_details.rs \
    genossi-frontend/src/page/helper_attendance.rs
  ```
  Beide Files müssen die 4 Components erwähnen — assembly_details.rs erfüllt das bereits (9 Treffer); helper_attendance.rs folgt in Plan 04-09.
- **SYNC-01 acceptance:** Wiring zu refresh_signal ist im Code; Phase-5-Generalprobe-Test wird Live-Update via 5s-Polling-Refresh nach Toggle empirisch verifizieren.

---
*Phase: 04-frontend-component-first-mit-qr-scanner-und-manual-code-fall*
*Completed: 2026-05-05*
