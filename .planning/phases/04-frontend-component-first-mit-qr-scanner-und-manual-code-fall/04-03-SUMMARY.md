---
phase: 04-frontend-component-first-mit-qr-scanner-und-manual-code-fall
plan: 03
subsystem: ui
tags: [dioxus, wasm, rust, i18n, reqwest, api-client]

requires:
  - phase: 01-assembly-aggregat-audit-hardening
    provides: AssemblyTO/AssemblyStatusTO Schema und REST-Endpoints für /api/assembly
  - phase: 02-helfer-token-session-authcontext-helper
    provides: HelperTokenTO/RedeemRequest/RedeemResponse + 410-Gone-Semantik + /api/helper/* Endpoints
  - phase: 03-attendance-aggregat-cascade-invalidation
    provides: AttendanceMemberTO 7-Feld-Whitelist + AttendanceStatsTO + /api/attendance/* Endpoints
provides:
  - 14 Phase-4-TOs als Rust-Structs in genossi-frontend/src/api.rs
  - 16 async API-Funktionen für Assembly-Lifecycle, Helper-Token-CRUD, Attendance, Helper-Session/Logout
  - status_to_message-Erweiterung um 410 Gone "Bereits eingelöst"
  - 67 neue i18n-Keys in beiden Locales (de.rs + en.rs)
affects:
  - 04-04 LiveCounter (verwendet AttendanceCounterLong/AttendanceCounterUnknown)
  - 04-05 AttendanceList/AttendanceSearch (verwendet AttendanceMemberTO + list_attendance_members)
  - 04-06 ConnectionBanner (verwendet AttendanceConnectionLost/Restored)
  - 04-07 QrScanner/ManualCodeInput (verwendet HelperLogin*-Keys + redeem_helper_token)
  - 04-08 Vorstand-Pages (verwendet AssemblyTO + alle Assembly/HelperToken-Funktionen)
  - 04-09 Helfer-Pages (verwendet HelperShell* + AttendanceList + helper_logout)

tech-stack:
  added: []
  patterns:
    - "Frontend-eigene rest-types-Crate bleibt unangetastet — Phase-4-TOs liegen direkt in api.rs (analog zu ApplicationTO/ConfigEntryTO)"
    - "PII-Whitelist-Doc-Comment auf AttendanceMemberTO als zweite Verteidigungslinie"
    - "Security-Guards: weder code noch memo werden geloggt (T-04-13)"

key-files:
  created: []
  modified:
    - "genossi-frontend/src/api.rs (Phase-4-TOs + 16 Funktionen + 410-Mapping)"
    - "genossi-frontend/src/i18n/mod.rs (67 neue Key-Varianten)"
    - "genossi-frontend/src/i18n/de.rs (67 deutsche Übersetzungen)"
    - "genossi-frontend/src/i18n/en.rs (67 englische Übersetzungen)"

key-decisions:
  - "js_sys::encode_uri_component statt urlencoding-Crate (kein Cargo.toml-Touch — liegt außerhalb des Plan-03-Scopes; js_sys ist bereits Dep)"
  - "AttendanceCounterLong als reines Wort 'anwesend' / 'present' — LiveCounter komponiert die {x}-/{y}-Zahlen inline (i18n-System hat kein Format-String-Interpolation)"
  - "Datetime-Felder als Option<String> (ISO8601) — konsistent mit ApplicationTO-Pattern in derselben Datei"

patterns-established:
  - "Phase-4-TO-Sektion am Ende der api.rs vor #[cfg(test)] mod tests"
  - "Banner-/Empty-State-/Confirm-Title+Text-i18n-Tripel-Pattern (z.B. AssemblyOpenConfirmTitle + AssemblyOpenConfirmText)"

requirements-completed: []

duration: ~25min
completed: 2026-05-05
---

# Phase 4-03: Frontend API Surface + i18n Foundation Summary

**14 Phase-4-TOs, 16 async API-Funktionen, 410-Mapping und 67 i18n-Keys (de+en) — Voraussetzung für alle Wave-2-Components und Pages.**

## Performance

- **Duration:** ~25 min
- **Tasks:** 2 (alle automatisiert, beide grün im ersten Anlauf nach Build)
- **Files modified:** 4
- **Commits:** 2 + 1 SUMMARY

## Accomplishments

- 14 Phase-4-TOs in `genossi-frontend/src/api.rs`: `AssemblyTO`, `AssemblyStatusTO`, `CreateAssemblyRequest`, `UpdateAssemblyRequest`, `HelperTokenTO`, `HelperTokenStatusTO`, `HelperTokenCreateResponseTO`, `CreateHelperTokenRequest`, `RedeemRequest`, `RedeemResponse`, `HelperSessionTO`, `AttendanceMemberTO` (7-Feld-Whitelist mit Doc-Comment), `AttendanceStatsTO`. Alle haben `serde::Deserialize` + `Clone` + `Debug` (Request-Structs nur `Serialize`).
- 16 async API-Funktionen: `list_assemblies`, `get_assembly`, `create_assembly`, `update_assembly`, `open_assembly`, `close_assembly`, `list_helper_tokens`, `create_helper_token`, `revoke_helper_token`, `redeem_helper_token`, `get_helper_session`, `helper_logout`, `list_attendance_members` (mit Substring-Search via `js_sys::encode_uri_component`), `mark_present`, `mark_absent`, `get_assembly_stats`. Alle nutzen das bestehende `AppError`/`check_response`/`reqwest`-Pattern.
- `status_to_message(410)` liefert `"Bereits eingelöst"` (Helfer-Token-redeemed-Status). Existing Cargo-Test `test_status_to_message_known_codes` um eine `assert_eq!`-Zeile ergänzt; alle 68 Frontend-Tests grün.
- 67 neue i18n-Keys in beiden Locales: `Close` (generic) + 21 Assembly + 17 Helper-Token + 16 Helper-Login + 2 Helper-Shell + 13 Attendance. Beide Locales haben identische Key-Sets (Compile-Error wäre sonst durch `match`-Exhaustiveness sichergestellt). Locale-Parität via grep verifiziert (de=69, en=69 inkl. Phase-4-Prefix-Matches).

## Task Commits

1. **Task 1: Phase-4 TOs + 16 API functions + 410 status mapping** — `0c66232` (feat)
2. **Task 2: ~67 i18n keys for Phase 4 in both locales (de/en)** — `9f5de3b` (feat)

## Files Created/Modified

- `genossi-frontend/src/api.rs` (+330 Zeilen) — TO-Definitionen, API-Funktionen, 410-Eintrag in `status_to_message`, Test-Assertion erweitert
- `genossi-frontend/src/i18n/mod.rs` (+67 Key-Varianten in 6 Phase-4-Sektionen)
- `genossi-frontend/src/i18n/de.rs` (+92 Match-Arme inkl. Sektions-Kommentare)
- `genossi-frontend/src/i18n/en.rs` (+91 Match-Arme inkl. Sektions-Kommentare)

## Decisions Made

- **`js_sys::encode_uri_component` statt `urlencoding`-Crate:** Der Plan-Action-Schritt 4 schlug vor, `urlencoding = "2.1"` ggf. zu `Cargo.toml` zu ergänzen. Der Orchestrator-Constraint erlaubt jedoch nur Edits in `api.rs` und `i18n/`. `js_sys` ist bereits Dep des Frontends, `encode_uri_component` deckt den Use-Case (Querystring-Werte sicher escapen) ab — kein Cargo.toml-Touch nötig.
- **`AttendanceCounterLong` als reines Wort statt Format-String:** Das i18n-System der Codebase verwendet `Rc<str>`-Translations ohne Format-String-Interpolation. Plan-Action-Schritt 2 erlaubte explizit "Plan-Discretion"; gewählt wurde der pragmatischste Pfad: `Key::AttendanceCounterLong => "anwesend"` (de) / `"present"` (en); die LiveCounter-Component (Plan 04-04) komponiert `{x} von {y} anwesend` inline. Konsequenz: en-Locale erzeugt grammatisch tolerable, aber nicht idiomatische Strings ("12 von 47 present"); akzeptabel weil die Helfer-Page laut D-19 deutsch-only ist und en-Locale nur für Vorstand-View existiert.
- **Datetime-Felder als `Option<String>`:** Konsistent mit `ApplicationTO` in derselben Datei (Frontend hält ISO8601 als String, formatiert via `i18n.format_datetime`). Vermeidet Komplexität von `time::PrimitiveDateTime` in den TOs.

## Deviations from Plan

**Insgesamt 67 statt der im Plan genannten "~50" i18n-Keys.** Plan-Tasks listen explizit 67 Keys auf (1 Generic + 21 Assembly + 17 Helper-Token + 16 Helper-Login + 2 Helper-Shell + 13 Attendance) — die "~50"-Aussage in der Plan-Frontmatter war eine grobe Schätzung, die exakte Aufzählung in `<action>` Schritt 1 ist autoritativ. Keine echte Abweichung, nur Zähl-Mismatch zwischen Frontmatter-Hinweis und Action-Spezifikation.

**Keine echten Plan-Abweichungen sonst** — alle Phase-4-TOs, Funktions-Signaturen, Verify-Commands und Verifier-Patterns wurden exakt umgesetzt. Plan-Verify-Block hatte einen kleinen Bug (`grep 'Key::HelperTokenCardManualHint' mod.rs` matched nicht, weil mod.rs die Variants ohne `Key::`-Prefix listet); Sache wurde manuell mit korrektem Pattern verifiziert.

## Issues Encountered

- Initialer `cargo check -p genossi-frontend` aus dem Workspace-Root failte (`package ID specification did not match`). Ursache: `genossi-frontend` ist im Top-Level-Cargo.toml als `excluded` aufgeführt (eigener Workspace). Lösung: Build aus `genossi-frontend/`-Verzeichnis (ohne `-p`-Flag). Entspricht der `genossi-frontend/CLAUDE.md`-Anweisung (`cargo check`).

## Next Phase Readiness

- **Wave 2** (Plans 04-04 bis 04-07: Components LiveCounter, AttendanceList/Search, ConnectionBanner, QrScanner/ManualCodeInput) hat alle benötigten TOs + API-Funktionen + i18n-Keys.
- **Wave 3** (Plans 04-08, 04-09: Pages) kann ohne Blocker mit den vollständigen Component-Sets starten.
- Alle Tests grün, Build clean.

---
*Phase: 04-frontend-component-first-mit-qr-scanner-und-manual-code-fall*
*Plan: 03*
*Completed: 2026-05-05*
