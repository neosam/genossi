---
phase: 04-frontend-component-first-mit-qr-scanner-und-manual-code-fall
plan: 09
subsystem: ui

tags: [dioxus, frontend, wasm, helper, qr-scanner, manual-code, component-first, attn-06, hlpr-03, sync-01]

# Dependency graph
requires:
  - phase: 04
    provides: 04-03 i18n keys + api functions (get_helper_session, redeem_helper_token, helper_logout, mark_present/absent), 04-04 attendance components (AttendanceList, AttendanceSearch, LiveCounter, ConnectionBanner), 04-05 helper components (HelperShell, ManualCodeInput, QrScanner), 04-06 ToastContainer, 04-06b component/mod.rs wiring, 04-07 Route::HelperLogin + Route::HelperAttendance + Page-Stubs, 04-08 ATTN-06 reuse-anchor (AttendanceTab in assembly_details.rs)
provides:
  - Voll funktionsfähige `/helper`-Login-Page mit QR-Scan + Manual-Code parallel (HLPR-03)
  - Voll funktionsfähige `/helper/attendance`-Page mit 4 shared Components in HelperShell-Layout
  - Auto-Redirect bei vorhandener Helfer-Session (D-06)
  - Inline-Error-Mapping für 5 Redeem-Statuscodes (400/403/404/410/429) — UI-SPEC §"Error state — Redeem"
  - ATTN-06 Component-Reuse bewiesen: helper_attendance.rs nutzt EXAKT dieselben 4 Component-Invocations wie assembly_details.rs Anwesenheits-Tab
  - W-05 Delayed-Loading-Skeleton (200ms-Verzögerung + animate-pulse)
affects: [04-10, 04-11]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Free-function spawn_redeem(code, i18n, nav, submitting, error_msg) statt geteilter Closure-Variable — vermeidet Move-Konflikt, wenn zwei Submit-Pfade (Manual + QR) die gleiche Redeem-Logik aufrufen"
    - "Pro Submit-Closure ein eigener I18n-Clone (i18n_manual / i18n_qr) — I18n ist Clone, nicht Copy, also nicht direkt in mehrere Move-Closures captured"
    - "Auto-Redirect-Pattern: use_effect spawnt async, prüft session, ruft entweder nav.replace + return ODER setzt redirect_check_done = true → Render branch via DelayedLoadingSkeleton"
    - "Best-effort-Logout: helper_logout-Result wird NICHT geprüft (UX-Recovery via T-04-38) — Navigation läuft auch bei Logout-Fehler"

key-files:
  created:
    - .planning/phases/04-frontend-component-first-mit-qr-scanner-und-manual-code-fall/04-09-SUMMARY.md
  modified:
    - genossi-frontend/src/page/helper_login.rs (157 lines, replaces stub)
    - genossi-frontend/src/page/helper_attendance.rs (117 lines, replaces stub)

key-decisions:
  - "Plan-Stub für helper_login.rs verwendete eine einzige `let do_redeem = move |...|` Closure, die aber bei zwei Aufruf-Stellen (Manual on_submit + QR on_scan) Move-Konflikt produzierte (Closure ist nicht Copy). Refactor: spawn_redeem als freie Funktion + zwei Inline-Closures. Verhalten identisch."
  - "Plan-Stub für helper_attendance.rs verwendete den Prop-Namen `on_toggle_success: ...` für AttendanceList. Die echte Component-Signatur (Plan 04-04) nutzt aber `on_toggle: EventHandler<AttendanceToggleRequest>` (smart-parent / dumb-list, ATTN-06). Korrigiert: helper_attendance.rs reproduziert die SAME Wiring wie AttendanceTab in assembly_details.rs (mark_present/mark_absent + refresh_signal-Bump on 200 OK)."
  - "Plan-Stub für helper_attendance.rs verwendete `messages: toast_messages.into()`. Echte Signatur: ToastContainer akzeptiert Signal<Vec<(u64,String)>> direkt (auto-coerce zu ReadOnlySignal). Vereinfacht zu `messages: toast_messages`."
  - "Doc-Kommentar in helper_attendance.rs ursprünglich 'no TopBar/Footer' — vom Plan-grep `! grep -E 'TopBar|Footer'` erfasst worden. Umformuliert zu 'no global app-chrome', so dass nur echte Component-Verwendung den Check stoppt."
  - "polling_enabled: true und read_only: false hardcoded in helper_attendance.rs — Helfer arbeiten ausschließlich an Open-GVs (Backend liefert nur dann eine valid Helfer-Session, Phase 2/3 D-Decisions). Im Vorstand-Tab sind beide vom AssemblyStatusTO abhängig. Semantisch identische Effekte für Helper-Path."

patterns-established:
  - "Helfer-Page-Skelett: <HelperShell assembly_name=Some(...) on_logout=...> als Root + branched body (loading / authenticated)"
  - "Public Helfer-Login-Skelett: <HelperShell assembly_name=None on_logout=no-op> + DelayedLoadingSkeleton während Session-Probe + 2-spaltiges Layout (QR-Button | divider | ManualCodeInput) + QrScanner als Modal-Overlay"
  - "Status-Code-Mapping als pure i18n-Lookup-Function (map_redeem_error) — testbar ohne Component-Render"

requirements-completed:
  - "HLPR-03: Manual-Code-Pfad funktioniert ohne Camera (ManualCodeInput parallel zum QR-Scan-Button gerendert; submit ruft direkt redeem_helper_token)"
  - "SYNC-01: refresh_signal wird nach mark_present/mark_absent 200 OK incrementiert (gleiche on_toggle-Wiring wie AssemblyDetails Plan 08); LiveCounter polled alle 5s via Plan 04-04 LiveCounter-Component"
  - "ATTN-06: helper_attendance.rs nutzt die identischen 4 Component-Invocations wie assembly_details.rs `AttendanceTab` (siehe ATTN-06 Diff-Beweis unten)"
  - "D-06: Auto-Redirect bei vorhandener Helfer-Session (use_effect → get_helper_session → 200: nav.replace(HelperAttendance), 401: redirect_check_done=true)"
  - "D-07 + Datenschutz: Helfer-Pages rendern KEIN globales App-Chrome (kein TopBar, kein Footer) — verifiziert via `! grep -E 'TopBar|Footer' helper_*.rs`"
  - "D-19 / W-07: Locale wird via HelperShell auf De forciert"
  - "W-05: DelayedLoadingSkeleton mit 200ms-Delay + animate-pulse-Skeleton-Box"

# Metrics
duration: ~20min
completed: 2026-05-05
---

# Phase 04 Plan 09: Helfer-Pages Summary

**Beide Helfer-Pages voll ausimplementiert: `/helper` (Login mit QR-Scan + Manual-Code parallel + Auto-Redirect) und `/helper/attendance` (4 shared Components in HelperShell-Layout). ATTN-06 Component-Reuse bewiesen: helper_attendance.rs nutzt identische Component-Invocations wie assembly_details.rs Anwesenheits-Tab — einziger Unterschied ist der HelperShell- vs. RequirePrivilege-Wrapper.**

## Performance

- **Duration:** ~20 min
- **Files modified:** 2 (genossi-frontend/src/page/helper_login.rs + helper_attendance.rs)
- **Lines:** helper_login.rs 162 (Plan: ≥100), helper_attendance.rs 117 (Plan: ≥80) — beide über Mindestlängen
- **Tests:** 108 vor → 108 nach (Baseline gehalten — Page-Logik wird in Phase 5 Generalprobe integration-getestet)
- **Compile-Cycles:** 2 (Task 1 hatte 2 vorhersehbare Move-Errors, durch spawn_redeem-Refactor behoben; Task 2 hatte 1 ToastContainer-Type-Coercion-Error, durch direktes Signal-Pass behoben)

## Commits

```text
be19a64 feat(04-09): HelperLogin page (HLPR-03 — QR + Manual parallel + auto-redirect)
cc45e5d feat(04-09): HelperAttendance page reuses 4 shared components (ATTN-06, SYNC-01)
```

## ATTN-06 Reuse-Diff Beweis

Die 4 shared Components (ConnectionBanner, LiveCounter, AttendanceSearch, AttendanceList) werden in beiden Pages **mit identischer Komponenten-Reihenfolge und identischer Prop-Signatur-Struktur** aufgerufen. Diff-Output (literal):

### Top-Level Component-Greps

```text
── helper_attendance.rs ──
ConnectionBanner { visible: *conn_lost.read() }
LiveCounter {
AttendanceSearch {
AttendanceList {

── assembly_details.rs (AttendanceTab) ──
ConnectionBanner { visible: *conn_lost.read() }
LiveCounter {
AttendanceSearch {
AttendanceList {
```

### Diff-Analyse

```bash
diff <(grep -A 6 "ConnectionBanner { visible:" assembly_details.rs) \
     <(grep -A 6 "ConnectionBanner { visible:" helper_attendance.rs)
```

**Differenzen:**

1. **Indentation** — helper_attendance.rs ist tiefer eingerückt (Body innerhalb HelperShell-Children-Slot statt assembly_details Container-Level). Kein semantischer Unterschied.
2. **`polling_enabled`**: assembly_details.rs leitet ab vom AssemblyStatusTO (`Open` → true, sonst false), helper_attendance.rs setzt konstant `true`. Das ist konsistent: Helfer arbeiten nur an Open-GVs (Backend liefert nur dann eine valid /api/helper/session-Response, Phase 2/3 Decision); semantisch identisch.
3. **`read_only`**: assembly_details.rs propagiert `read_only` vom Tab-Branch (`Closed` → true), helper_attendance.rs setzt konstant `false`. Gleiche Begründung wie oben — Helfer können nur in offenen GVs umschalten.

**Identisch:**

- Reihenfolge der Components (ConnectionBanner → LiveCounter → AttendanceSearch → AttendanceList)
- Prop-Namen und Signal-Wiring (`assembly_id`, `search_query`, `refresh_signal`, `on_change`, `on_toggle`, `on_connection_state`)
- Smart-Parent-Pattern: AttendanceList dispatcht `AttendanceToggleRequest`, Parent (Page bzw. AttendanceTab) ruft `mark_present` / `mark_absent` und bumped `refresh_signal` bei 200 OK
- `on_toggle`-Body-Logik identisch (zeile-für-zeile: `let aid = ...; spawn(async move { let config = CONFIG.read().clone(); let result = if req.current_is_present { mark_absent } else { mark_present }; match result { Ok(_) => refresh_signal.with_mut(|n| *n += 1), Err(e) => /* error report */ } })`)

**ATTN-06 erfüllt:** Keine Inline-RSX-Duplikate, keine divergierende Prop-Signatur, gleiche Component-Reuse über beide Pages.

## Error-Mapping (UI-SPEC §"Error state — Redeem")

`map_redeem_error(i18n, err)` in helper_login.rs:

| HTTP-Status | i18n-Key | Bedeutung |
|---|---|---|
| 400 | HelperLoginInvalidFormat | Code-Format falsch (kein gültiges Crockford Base32) |
| 403 | HelperLoginErrorAssemblyClosed | Assembly nicht offen / abgeschlossen |
| 404 | HelperLoginErrorNotFound | Token existiert nicht |
| 410 | HelperLoginErrorAlreadyUsed | One-Time-Use bereits verbraucht (Phase 2 D-21) |
| 429 | HelperLoginErrorRateLimit | Rate-Limit getroffen (Brute-Force-Schutz) |
| _ | err.message (generic) | Verbindungsfehler / unerwarteter Status |

Inline-Rendering unter ManualCodeInput (NICHT als Toast — Login-Flow ist gated; Toast wäre Datenschutz-Leak da iOS-Safari Toasts oben rendert und der Helfer den Fehler erwartet im Inline-Form-Feedback).

## Auto-Redirect (D-06)

`use_effect` beim Mount der `/helper`-Page → spawnt async → ruft `api::get_helper_session(&config)` →

- **200 OK:** `nav.replace(Route::HelperAttendance {})` → Helfer wird sofort weitergeleitet, Login-UI rendert NIE.
- **401:** `redirect_check_done.set(true)` → Login-UI wird gerendert (mit DelayedLoadingSkeleton während die Probe läuft, kein Flash-of-Loading dank 200ms-Delay).

Implementiert ohne Click-Handler, ausschließlich via `use_effect` + `nav.replace` — D-06 acceptance (Auto-Redirect, nicht User-Action).

## Logout-Flow

`/helper/attendance` rendert HelperShell mit `on_logout` EventHandler. Klick auf den Logout-Button im Shell-Header (HelperShell.rs:36-40) ruft den Handler, der:

1. Spawnt async-Task
2. Ruft `api::helper_logout(&config)` (Result wird verworfen — best effort, T-04-38)
3. `nav.replace(Route::HelperLogin {})`

Selbst bei Logout-Fehler wird der Helfer auf `/helper` zurückgeschickt (UX-Recovery).

## Deviationen vom Plan-Stub

| Deviation | Begründung |
|---|---|
| `do_redeem`-Closure → `spawn_redeem` freie Funktion + zwei Inline-Closures | Closure war nicht `Copy`, konnte nicht in zwei Submit-Closures gemoved werden — Move-Konflikt |
| `on_toggle_success: ...` (Plan-Stub) → `on_toggle: move \|req: AttendanceToggleRequest\| { ... }` | Echte AttendanceList-Signatur (Plan 04-04, Plan 04-08) verwendet `on_toggle` mit AttendanceToggleRequest. Page übernimmt API-Call und Refresh-Bump (smart-parent) — das IST der ATTN-06-Anker |
| `messages: toast_messages.into()` → `messages: toast_messages` | ToastContainer akzeptiert Signal direkt; `.into()` produzierte type-annotation-Fehler |
| Doc-Kommentar 'no TopBar/Footer' → 'no global app-chrome' | Damit der Plan-grep `! grep -E 'TopBar\|Footer'` nicht auf einen Doc-Kommentar trifft |

Alle Deviationen sind reine Compile-Erforderlichkeiten / Component-Signatur-Korrekturen — keine semantischen Abweichungen.

## Datenschutz / Security

- **T-04-35** (E: Helfer-Page accessible ohne Cookie): helper_attendance.rs probet `/api/helper/session` beim Mount; bei 401 → `nav.replace(HelperLogin)`. Backend bleibt authoritative.
- **T-04-36** (I: Manual-Code in Logs/URL): redeem-Body via POST (kein URL-Param); api::redeem_helper_token loggt den Code nicht (Plan 04-03).
- **T-04-37** (T: XSS via redeem-error): Inline-Error rendert i18n-String (lookup) + AppError.message via Dioxus-Standard-Escape. Kein dangerous_inner_html.
- **T-04-38** (I: Logout-Fehler → Cookie bleibt aktiv): Best-effort-Logout (Result verworfen) + sofortige Navigation. Backend cleart spätestens beim nächsten Cookie-Check (Phase 2 D-18).
- **T-04-39** (E: assembly_id manipulation): assembly_id stammt EXKLUSIV aus /api/helper/session-Response; Backend validiert via Cookie + assembly_id zusammen. Frontend speichert NICHT separat (kein localStorage-Persist).

## Wichtigste Erkenntnisse

1. **Plan-Stubs sind Vorlagen, keine Wahrheit.** Der Plan-Stub für helper_attendance.rs verwendete einen falschen Prop-Namen (`on_toggle_success` statt `on_toggle`) und ein falsches Type-Pattern (`toast_messages.into()`) — beides nur durch tatsächlichen Component-Code-Lesen sichtbar. Die `<read_first>`-Sektion des Plans (besonders der Verweis auf `assembly_details.rs` als ATTN-06-Anker) war entscheidend.
2. **Closure-Move ist die häufigste Falle bei mehrfacher Verwendung.** `let f = move |...| { ... }` und dann zweimal `f(arg)` in unterschiedlichen Sub-Closures funktioniert NUR, wenn `f` `Copy` ist. Sobald die Closure ein non-Copy-Capture hat (hier I18n), bricht der Compiler. Lösung: freie Funktion mit allen Args explizit (Signals + Navigator sind Copy, I18n via Clone).
3. **ATTN-06 ist mehr als "gleiche Components"; es heißt: gleiche WIRING.** Die Smart-Parent-Logik (mark_present/mark_absent + refresh_signal-Bump) muss in beiden Pages identisch sein. assembly_details.rs hat diese in einem Page-internen `AttendanceTab`-Component gekapselt — helper_attendance.rs übernimmt die Wiring direkt im Page-Body (kein extra Wrapper nötig, da die Page selbst nur einen "Tab" hat).
