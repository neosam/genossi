---
phase: 20-inbox-digest-t-glicher-posteingangs-benachrichtigungs-worker
plan: 03
subsystem: ui
tags: [dioxus, wasm, config, frontend, digest, inbox-notification, validation]

# Dependency graph
requires:
  - phase: 20-inbox-digest (Plan 02 — Backend-Worker)
    provides: "Config-Keys digest_recipients + digest_send_time werden vom Worker gelesen; dieses Plan pflegt sie über die UI"
provides:
  - "Config-Abschnitt 'Posteingangs-Benachrichtigung' auf der Config-Seite (Empfänger + Versand-Uhrzeit + Speichern)"
  - "Clientseitige Inline-Validierung für komma-getrennte E-Mail-Empfänger und HH:MM-Uhrzeit (pure, unit-getestete Funktionen)"
  - "Reload-Populate beider Config-Werte aus dem Config-KV-Store"
affects: [20-inbox-digest worker, config-page, future-config-sections]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Pure, unit-testbare Validierungs-Helfer (validate_digest_recipients / validate_digest_send_time) statt inline-Logik im onclick-Closure — testbar trotz WASM-Bin-Crate"
    - "Eigener CollapsibleSection-Config-Abschnitt im Stil der bestehenden inline SMTP/IMAP-Blöcke (D-12)"

key-files:
  created: []
  modified:
    - genossi-frontend/src/page/config_page.rs

key-decisions:
  - "Plan 20-03: AppError hat KEIN From<String> — clientseitige Validierungsfehler via api::AppError::new(None, msg, None) (konsistent mit config_page.rs:631), nicht via AppError::from()."
  - "Plan 20-03: Validierungslogik in pure free-Funktionen validate_digest_recipients/validate_digest_send_time extrahiert (Rule 2 + User-CLAUDE.md 'always have tests'); 6 Unit-Tests in config_page::tests. Bin-Crate hat kein lib-Target → Tests laufen via 'cargo test --bin genossi-frontend'."
  - "Plan 20-03: Save-Button nutzt r#type: button + onclick (NICHT form-onsubmit) wegen Dioxus-Button-Reload-Bug (Memory feedback_dioxus_button_type)."

patterns-established:
  - "Config-Validierung clientseitig: pure bool-Funktionen + AppError::new bei Fehler, früh-return vor spawn"
  - "Inline-Config-Abschnitt konsistent zum SMTP/IMAP-Bestand (Component-First: kein eigener Component für einmaligen Abschnitt)"

requirements-completed: [DIGEST-01, DIGEST-02, DIGEST-07]

# Metrics
duration: 4min
completed: 2026-06-26
---

# Phase 20 Plan 03: Posteingangs-Benachrichtigung Config-Abschnitt Summary

**Neuer Config-Abschnitt „Posteingangs-Benachrichtigung" auf der Config-Seite mit komma-getrenntem Empfänger-Feld, HH:MM-Uhrzeit, Speichern und pure-funktional unit-getesteter Inline-Validierung — persistiert `digest_recipients` + `digest_send_time` für den Plan-02-Worker.**

## Performance

- **Duration:** ~4 min
- **Started:** 2026-06-26T21:37:04Z
- **Completed:** 2026-06-26T21:40:38Z
- **Tasks:** 2
- **Files modified:** 1

## Accomplishments
- Eigener `CollapsibleSection`-Abschnitt „Posteingangs-Benachrichtigung" im Stil der SMTP/IMAP-Blöcke (D-12), eingefügt zwischen IMAP- und WebDAV-Abschnitt
- `digest_recipients`- und `digest_send_time`-Signals + Reload-Populate aus dem Config-KV-Store (Werte bleiben nach Reload erhalten — DIGEST-01/02)
- Clientseitige Inline-Validierung (D-13): grobe E-Mail-Format-Prüfung je komma-getrennter Adresse + HH:MM-Bereichsprüfung (0–23 / 0–59), mit früh-return + ErrorAlert
- Leeres Empfänger-Feld ist gültig und deaktiviert das Feature (DIGEST-07/D-14) — wird als leerer String gespeichert, Worker skippt
- Validierung in pure, unit-getestete Funktionen extrahiert (6 Tests, alle grün)

## Task Commits

1. **Task 1: Digest-Signals + reload-Populate** - `d812eba` (feat)
2. **Task 2: CollapsibleSection-Abschnitt + Inline-Validierung + Save-Flow** - `340bd6f` (feat)

## Files Created/Modified
- `genossi-frontend/src/page/config_page.rs` - Digest-Signals, reload-Populate, neuer CollapsibleSection-Abschnitt, pure Validierungs-Helfer + 6 Unit-Tests

## Decisions Made
- **AppError-Konstruktion:** `api::AppError` hat kein `From<String>` (verifiziert in api.rs:20-44). Clientseitige Validierungsfehler werden via `api::AppError::new(None, msg, None)` erzeugt — konsistent mit dem bestehenden Pattern in config_page.rs:631.
- **Validierung als pure Funktionen:** Statt der vom Plan vorgegebenen inline-Logik im onclick-Closure wurde die Validierung in `validate_digest_recipients` / `validate_digest_send_time` extrahiert, um sie testbar zu machen (User-CLAUDE.md: „Always make sure you have tests"). Das onclick-Closure ruft nur noch die Helfer auf. Verhalten identisch zum Plan.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Validierungslogik in pure, unit-getestete Funktionen extrahiert**
- **Found during:** Task 2 (CollapsibleSection-Abschnitt mit Inline-Validierung)
- **Issue:** Der Plan platzierte die E-Mail-/Uhrzeit-Validierung als inline-Block direkt im onclick-Closure. Das ist nicht unit-testbar, während User-CLAUDE.md („Always make sure you have tests") und das Plan-success_criteria („validation logic should be unit-tested if extracted into a pure function") Tests fordern.
- **Fix:** Validierung in zwei pure free-Funktionen `validate_digest_recipients(&str) -> bool` und `validate_digest_send_time(&str) -> bool` extrahiert; onclick ruft sie auf und setzt bei `false` einen ErrorAlert + früh-return. 6 Unit-Tests in `config_page::tests` decken leer/gültig/ungültig für beide Felder ab (inkl. D-14 leeres Feld = gültig).
- **Files modified:** genossi-frontend/src/page/config_page.rs
- **Verification:** `cargo test --bin genossi-frontend config_page::tests` → 6 passed; 0 failed. `cargo check` exit 0.
- **Committed in:** 340bd6f (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 missing critical — Testabdeckung)
**Impact on plan:** Verhalten 1:1 wie geplant; einzige Änderung ist die Testbarkeit (pure Funktionen statt inline). Kein Scope-Creep.

## Issues Encountered
- `cargo test --lib` schlägt fehl („no library targets") — das Frontend ist ein Bin-Crate. Tests laufen via `cargo test --bin genossi-frontend`. Test-Filter musste auf den vollen Modulpfad `config_page::tests` (bzw. `page::config_page::tests`) gesetzt werden, da die Funktionsnamen sonst herausgefiltert wurden.

## User Setup Required
None - no external service configuration required. (Empfänger + Uhrzeit werden vom Vorstand direkt in der UI gepflegt.)

## Known Stubs
None - der Abschnitt ist voll an den Config-KV-Store (`api::set_config_entry` / `get_config_value`) verdrahtet; keine hartkodierten Platzhalter-Daten.

## Next Phase Readiness
- Config-Keys `digest_recipients` + `digest_send_time` sind über die UI pflegbar und persistent — der Worker aus Plan 02 liest exakt dieselben Keys.
- Phase 20 vollständig, sobald Worker (Plan 02) und Migration/DAO (Plan 01) integriert sind.

## Self-Check: PASSED

- FOUND: 20-03-SUMMARY.md
- FOUND: genossi-frontend/src/page/config_page.rs
- FOUND: commit d812eba (Task 1)
- FOUND: commit 340bd6f (Task 2)

---
*Phase: 20-inbox-digest-t-glicher-posteingangs-benachrichtigungs-worker*
*Completed: 2026-06-26*
