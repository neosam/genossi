---
phase: 32-frontend-compose-dialog
plan: 02
subsystem: ui
tags: [dioxus, wasm, reqwest, serde, i18n, application-mail]

# Dependency graph
requires:
  - phase: 32-frontend-compose-dialog (Plan 32-01)
    provides: "CommunicationEntryTO.rendered_body/rendered_html_body im Frontend-rest-types (D-06 Wire)"
  - phase: 31 (Backend Application-Mail)
    provides: "Endpoints POST /applications/{id}/mail[/preview], GET /applications/{id}/communications (admin-gated)"
provides:
  - "Dedizierte api.rs-Funktionen send_application_mail / preview_application_mail / get_application_communications"
  - "Lokale Send/Preview-Request-Structs + ApplicationPreviewResponse (Landmine 1)"
  - "MailTemplateTO.template_type (clientseitiger Filter-Diskriminator, D-03)"
  - "Reine, getestete Helfer filter_templates_by_type + last_outbound_summary"
  - "i18n-Keys LastSentSummary / NeverSent / SentMailBody in De + En"
affects: [32-03 Compose-Page, 32-04 application_detail]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Lokale Wire-Structs im Frontend, weil Backend-Request-Typen in der zweiten rest-types-Crate nicht verfuegbar sind (Landmine 1)"
    - "Reine, WASM-unabhaengige Helfer kapseln die einzige testbare Compose/Detail-Logik"

key-files:
  created: []
  modified:
    - "genossi-frontend/src/api.rs"
    - "genossi-frontend/src/i18n/mod.rs"
    - "genossi-frontend/src/i18n/de.rs"
    - "genossi-frontend/src/i18n/en.rs"

key-decisions:
  - "Send/Preview-Request-Typen als lokale private Structs gespiegelt (Landmine 1) statt Backend-rest-types zu importieren"
  - "MailTemplateTO.template_type additiv mit #[serde(default)] fuer Backward-Compat mit Legacy-Responses"
  - "last_outbound_summary vertraut der Backend-Sortierung (ORDER BY date DESC) und nimmt entry[0] statt clientseitig zu sortieren"
  - "LastSentSummary als Praefix-Label (nicht als Templatestring), Aufrufer baut die Zeile in Plan 03/04"

patterns-established:
  - "Dedizierte per-Resource-api.rs-Funktionen statt member-scoped Umleitung (APUI-02)"
  - "Testbare reine Funktionen fuer WASM-untestbare UI-Logik"

requirements-completed: [APMAIL-04, APUI-02]

coverage:
  - id: D1
    description: "Dedizierte api.rs-Funktionen fuer Send/Preview/Communications gegen /api/applications/ (keine Member-Umleitung)"
    requirement: "APUI-02"
    verification:
      - kind: unit
        ref: "genossi-frontend/src/api.rs#test_send_application_mail_request_skips_none_optionals"
        status: pass
      - kind: unit
        ref: "genossi-frontend/src/api.rs#test_send_application_mail_request_includes_some_optionals"
        status: pass
    human_judgment: false
  - id: D2
    description: "Preview-Funktion ruft den Backend-Render-Kernel und gibt ApplicationPreviewResponse zurueck (APMAIL-04-Fundament)"
    requirement: "APMAIL-04"
    verification:
      - kind: unit
        ref: "cargo test -p genossi-frontend (compiles + serde roundtrip of ApplicationPreviewResponse)"
        status: pass
    human_judgment: false
  - id: D3
    description: "MailTemplateTO.template_type + filter_templates_by_type liefern nur Antragsteller-Vorlagen (D-03)"
    verification:
      - kind: unit
        ref: "genossi-frontend/src/api.rs#test_filter_templates_by_type_keeps_only_application"
        status: pass
      - kind: unit
        ref: "genossi-frontend/src/api.rs#test_filter_templates_by_type_empty_input"
        status: pass
    human_judgment: false
  - id: D4
    description: "last_outbound_summary leitet (Betreff, Status, Datum) aus dem neuesten Eintrag ab, None bei leer (D-06 anti-double-send)"
    verification:
      - kind: unit
        ref: "genossi-frontend/src/api.rs#test_last_outbound_summary_returns_first_entry"
        status: pass
      - kind: unit
        ref: "genossi-frontend/src/api.rs#test_last_outbound_summary_empty_returns_none"
        status: pass
    human_judgment: false
  - id: D5
    description: "i18n-Keys LastSentSummary / NeverSent / SentMailBody in beiden Locales (De + En)"
    verification:
      - kind: unit
        ref: "cargo build -p genossi-frontend (match exhaustiveness beweist Locale-Paritaet)"
        status: pass
    human_judgment: false

# Metrics
duration: ~10min
completed: 2026-08-21
status: complete
---

# Phase 32 Plan 02: Frontend-API-Schicht + reine Helfer + i18n Summary

**Dedizierte Application-Mail-api.rs-Funktionen (send/preview/communications) mit lokalen Wire-Structs, MailTemplateTO.template_type, zwei unit-getestete reine Helfer (Filter + last-sent) und drei i18n-Keys in De+En — das Wave-1-Fundament fuer die Compose-Seite (Plan 32-03/04).**

## Performance

- **Duration:** ~10 min
- **Started:** 2026-08-21T00:48:04+02:00 (erster Task-Commit)
- **Completed:** 2026-08-21T00:50:39+02:00 (letzter Task-Commit)
- **Tasks:** 3 (Task 2 als TDD: RED+GREEN)
- **Files modified:** 4

## Accomplishments
- Drei dedizierte, nicht-umgeleitete api.rs-Funktionen gegen die admin-gated Application-Endpoints (APUI-02); Preview trifft den Backend-Render-Kernel (APMAIL-04-Fundament).
- Lokale Send/Preview-Request-Structs + `ApplicationPreviewResponse` mit `skip_serializing_if`/`serde(default)` (Landmine 1: Backend-Request-Typen im Frontend nicht importierbar).
- `MailTemplateTO.template_type` additiv ergaenzt + reine `filter_templates_by_type` (D-03) — nur Antragsteller-Vorlagen inkl. Seed …0003.
- Reine `last_outbound_summary` (D-06 anti-double-send-Guard) — (Betreff, Status, Datum) aus dem neuesten Eintrag, `None` bei leerer Historie.
- i18n-Keys `LastSentSummary` / `NeverSent` / `SentMailBody` in beiden Locales, Paritaet via Match-Exhaustiveness beim Build bewiesen.

## Task Commits

Each task was committed atomically:

1. **Task 1: Dedizierte api.rs-Funktionen + lokale Structs + MailTemplateTO.template_type** - `c19c45c` (feat)
2. **Task 2 (TDD RED): failing tests for filter_templates_by_type + last_outbound_summary** - `e9444af` (test)
3. **Task 2 (TDD GREEN): implement filter_templates_by_type + last_outbound_summary** - `1711561` (feat)
4. **Task 3: i18n-Keys LastSentSummary / NeverSent / SentMailBody (De + En)** - `6cbdd8b` (feat)

**Plan metadata:** siehe abschliessender docs-Commit.

## Files Created/Modified
- `genossi-frontend/src/api.rs` - 3 dedizierte Application-Mail-Funktionen, lokale Wire-Structs, `ApplicationPreviewResponse`, `MailTemplateTO.template_type`, 2 reine Helfer, 8 neue Unit-Tests.
- `genossi-frontend/src/i18n/mod.rs` - 3 neue `Key`-Varianten.
- `genossi-frontend/src/i18n/de.rs` - deutsche Arme (primaer, laut UI-SPEC Copywriting Contract).
- `genossi-frontend/src/i18n/en.rs` - englische Arme.

## Decisions Made
- Send/Preview-Request-Typen als **lokale** private Structs gespiegelt (Landmine 1), spiegelbildlich zum bestehenden `PreviewRequest`.
- `template_type` additiv mit `#[serde(default)]` — Backward-Compat mit Legacy-Responses.
- `last_outbound_summary` vertraut der Backend-Sortierung (`ORDER BY date DESC`) und nimmt `entries[0]`, statt clientseitig zu sortieren — haelt den Helfer rein und deterministisch.
- `Key::LastSentSummary` ist ein Praefix-**Label** ("Zuletzt gesendet"); die vollstaendige Zeile "Zuletzt gesendet: {Betreff} — {Status} am {Datum}" baut der Aufrufer in Plan 03/04.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
- `cargo test -p genossi-frontend` vom Workspace-Root schlaegt fehl: die Frontend-Crate ist im Root-`Cargo.toml` via `exclude` ausgeschlossen (eigene Flake). Tests wurden korrekt aus `genossi-frontend/` via `nix develop --command cargo test` ausgefuehrt (339 Tests gruen). Kein Code-Problem, nur Ausfuehrungsort.
- `genossi-frontend/Cargo.lock` wurde beim Build automatisch von `2026.211.1-dev` auf die in `Cargo.toml` deklarierte `2026.221.1-dev` synchronisiert (kein inhaltlicher Dependency-Change). Wird im abschliessenden docs-Commit mitgefuehrt.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Plan 32-03 (Compose-Page) kann `send_application_mail` / `preview_application_mail` / `filter_templates_by_type` direkt konsumieren.
- Plan 32-04 (application_detail) kann `get_application_communications` + `last_outbound_summary` + die drei i18n-Keys fuer den last-sent-Guard nutzen.
- UI-Gating (RequirePrivilege) folgt planmaessig in Plan 03/04 (T-32-05: Endpoints bereits serverseitig admin-gated).

## Self-Check: PASSED

---
*Phase: 32-frontend-compose-dialog*
*Completed: 2026-08-21*
