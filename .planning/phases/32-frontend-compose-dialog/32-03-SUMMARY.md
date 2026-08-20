---
phase: 32-frontend-compose-dialog
plan: 03
subsystem: ui
tags: [dioxus, wasm, application-mail, compose, routing, component-first]

# Dependency graph
requires:
  - phase: 32-frontend-compose-dialog (Plan 32-02)
    provides: "send_application_mail / preview_application_mail / get_application_communications, filter_templates_by_type, last_outbound_summary, MailTemplateTO.template_type, i18n LastSentSummary/NeverSent/SentMailBody"
  - phase: 31 (Backend Application-Mail)
    provides: "admin-gated POST /applications/{id}/mail[/preview], GET /applications/{id}/communications"
provides:
  - "Dedizierte Compose-Vollseite ApplicationCompose (Route /applications/:id/compose)"
  - "Additive TemplateSelector-Props filter_type + initial_template_id (Component-First, backward-kompatibel)"
affects: [32-04 application_detail (verlinkt spaeter auf diese Route)]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Freie Helfer-Funktion (bump_and_preview/schedule_preview) statt geteilter Closure — alle Signale sind Copy und werden by-value hereingereicht"
    - "Debounce ueber Generation-Zaehler: abgelaufene Preview-Laeufe verwerfen ihr Ergebnis, die letzte aufgeloeste Vorschau bleibt stehen (kein Flackern, D-04)"
    - "MailPreviewFrame (reused mail_compose-Component) rendert die aufgeloeste HTML-Vorschau statt der member-scoped TemplatePreview"

key-files:
  created:
    - "genossi-frontend/src/page/application_compose.rs"
  modified:
    - "genossi-frontend/src/component/mail_compose/template_selector.rs"
    - "genossi-frontend/src/page/mod.rs"
    - "genossi-frontend/src/router.rs"

key-decisions:
  - "TemplateSelector additiv um filter_type + initial_template_id erweitert (controlled <select> via neuem selected-Signal); None = unveraendertes MailPage-Verhalten"
  - "Aufgeloeste Vorschau ueber MailPreviewFrame statt TemplatePreview — TemplatePreview ist member-scoped (ruft preview_mail mit Member-Dropdown) und fuer memberlose Antragsteller-Mails unbrauchbar; die Anforderung verlangt explizit preview_application_mail (nicht member-scoped)"
  - "last_outbound_summary auf Outbound-Direction gefiltert, bevor die zuletzt-gesendet-Zeile gebildet wird (korrekter Anti-Doppelversand-Guard, D-06)"
  - "Ungueltige Antrags-ID (nicht-parsbare UUID) rendert einen defensiven Fehlerhinweis statt Requests abzufeuern"

patterns-established:
  - "Dedizierte Compose-Vollseite statt Modal-in-Modal (D-01), 1:1 nach mail_page.rs-Geruest"

requirements-completed: [APMAIL-04, APUI-01, APUI-02]

coverage:
  - id: T1
    description: "TemplateSelector additiv um filter_type + initial_template_id (Component-First, backward-kompatibel)"
    requirement: "APUI-02"
    verification:
      - kind: build
        ref: "nix develop --command cargo build -p genossi-frontend"
        status: pass
      - kind: unit
        ref: "cargo test -p genossi-frontend (339 passed — bestehende MailPage-Nutzung + filter_templates_by_type gruen)"
        status: pass
    human_judgment: false
  - id: T2
    description: "ApplicationCompose-Vollseite, admin-gated, komponiert mail_compose-Bausteine, gefiltert+vorbefuellt, debounced Preview, disabled-during-send ohne form-onsubmit"
    requirement: "APUI-01, APMAIL-04"
    verification:
      - kind: build
        ref: "nix develop --command cargo build -p genossi-frontend"
        status: pass
      - kind: unit
        ref: "tests/no_form_submit_regression.rs (no_submit_type_buttons_in_frontend_source) — beweist: kein submit-Button/form-onsubmit"
        status: pass
    human_judgment: true
    human_judgment_reason: "WASM-Interaktion (Debounce-Timing, disabled-during-send, Rueckkehr+Toast) ist nur im Browser sichtbar — deferred UAT via dx serve"
  - id: T3
    description: "Route::ApplicationCompose { id } registriert und an die Compose-Page gebunden"
    requirement: "APUI-01"
    verification:
      - kind: build
        ref: "nix develop --command cargo build -p genossi-frontend (Router-Makro findet die Komponente; spezifische Route vor generischer)"
        status: pass
    human_judgment: false

# Metrics
duration: ~25min
completed: 2026-08-21
status: complete
---

# Phase 32 Plan 03: Application-Mail-Compose-Vollseite Summary

**Dedizierte, admin-gated `ApplicationCompose`-Vollseite (Route `/applications/:id/compose`) — 1:1 nach dem `mail_page.rs`-Geruest aus den bestehenden `mail_compose/*`-Bausteinen komponiert, mit Antragsteller-gefiltertem und „Zahlungserinnerung"-vorbefuelltem TemplateSelector, debouncter aufgeloester Live-Vorschau ueber `preview_application_mail` und confirm-before-send ohne form-onsubmit; plus additive, backward-kompatible TemplateSelector-Props.**

## Performance

- **Tasks:** 3 (alle `type="auto"`)
- **Files:** 1 erstellt, 3 modifiziert
- **Build:** `cargo build -p genossi-frontend` gruen; `cargo test -p genossi-frontend` 339 passed

## Accomplishments

- **TemplateSelector additiv erweitert** (`filter_type`, `initial_template_id`, beide `#[props(default)]`): bei gesetztem `filter_type` werden die Optionen ueber `filter_templates_by_type` gefiltert; `initial_template_id` wird als controlled `<select>`-Wert (neues `selected`-Signal) gespiegelt. `None` = unveraendertes MailPage-Verhalten (keine neuen Pflichtprops).
- **Neue Compose-Vollseite `ApplicationCompose`**: `RequirePrivilege(PRIVILEGE_ADMIN)` → `flex flex-col min-h-screen` → `TopBar` → `container mx-auto px-4 py-8` → `h1.text-3xl.font-bold.mb-6` → Compose-Card. Beim Mount: `get_application_communications` + `list_mail_templates`, Antragsteller-Filter, „Zahlungserinnerung" (Seed …0003) als Betreff/Body/`selected_template_id` vorbefuellt (D-03).
- **Debounced aufgeloeste Live-Vorschau (D-04/D-05)**: `preview_application_mail` (nicht member-scoped) mit Generation-Zaehler-Debounce (400ms); die letzte aufgeloeste Vorschau bleibt waehrend Pending sichtbar (kein Flackern). Die Vorschau ist der Fokuspunkt und die Bestaetigung — kein separates Confirm-Modal.
- **Anti-Doppelversand-Guard (D-06)**: prominente „zuletzt gesendet"-Zeile (`LastSentSummary` mit Betreff + Status + Datum, `NeverSent` bei leerer Historie) direkt ueber dem Senden-Button, aus outbound-gefilterten Communications via `last_outbound_summary`.
- **Senden ohne form-onsubmit**: `button` + `onclick` + `r#type: "button"` (Vorbild `repayment_phases.rs`); `disabled: *sending.read() || subject.read().is_empty()`. Fehler via `ErrorAlert` (nie stilles 200); Erfolg → `show_toast(MailJobCreated)` + `nav.push(Route::ApplicationsPage)`.
- **Route registriert**: `#[route("/applications/:id/compose")] ApplicationCompose { id: String }` vor der generischen `/applications`-Route.

## Task Commits

1. **Task 1: TemplateSelector additiv (filter_type + initial_template_id)** — `22c9eb0` (feat)
2. **Task 2: Compose-Vollseite application_compose.rs + page/mod.rs Re-Export** — `ff939ff` (feat)
3. **Task 3: Route::ApplicationCompose in router.rs** — `dc02b4d` (feat)

## Files Created/Modified

- `genossi-frontend/src/page/application_compose.rs` (NEU) — `ApplicationCompose`-Page inkl. `schedule_preview`/`bump_and_preview`-Helfer und der Vorbefuell-Konstanten.
- `genossi-frontend/src/component/mail_compose/template_selector.rs` — additive Props `filter_type` + `initial_template_id`, controlled `<select>` via `selected`-Signal.
- `genossi-frontend/src/page/mod.rs` — `pub mod application_compose;` + `pub use application_compose::ApplicationCompose;`.
- `genossi-frontend/src/router.rs` — Route-Variante + `pub use`.

## Deviations from Plan

### Auto-fixed / discretion applied

**1. [Rule 1/3 - Blocking] Aufgeloeste Vorschau ueber `MailPreviewFrame` statt `TemplatePreview`**
- **Gefunden bei:** Task 2.
- **Problem:** Der Plan-Task nennt `TemplatePreview` als Vorschau-Baustein, doch `TemplatePreview` ist fest member-scoped: es ruft intern die member-scoped `preview_mail` und bietet ein Member-Auswahl-Dropdown (`member_ids`, `preview_member_id`). Fuer eine memberlose Antragsteller-Mail ist das Dropdown leer und es feuert nie ein Preview-Request — funktional unbrauchbar. Zugleich verlangt die Anforderung (D-04, Akzeptanzkriterium) ausdruecklich `preview_application_mail`, **nicht** die member-scoped `preview_mail`.
- **Fix:** Die aufgeloeste HTML-Vorschau wird ueber die ebenfalls in der UI-SPEC gelistete, wiederverwendete `mail_compose`-Component `MailPreviewFrame` (Desktop-Modus, `preview_srcdoc` + `inject_asset_src`) gerendert, gespeist aus dem Ergebnis von `preview_application_mail`; der aufgeloeste Betreff steht als Label darueber. Kein geforktes UI — es bleibt bei wiederverwendeten `mail_compose`-Bausteinen.
- **Dateien:** `genossi-frontend/src/page/application_compose.rs`
- **Commit:** `ff939ff`

**2. [Rule 3 - Blocking] Kein `Key::NotFound` vorhanden**
- **Gefunden bei:** Task 2 (Build-Fehler `E0599`).
- **Problem:** Fuer den defensiven Invalid-UUID-Zweig gibt es keinen passenden i18n-Key.
- **Fix:** Defensiver Zweig rendert eine deutsche Literal-Zeile („Ungültige Antrags-ID.") statt einen neuen i18n-Key ausserhalb des Plan-Scopes hinzuzufuegen. Dieser Zweig ist ein reiner Schutz gegen manipulierte URLs.
- **Dateien:** `genossi-frontend/src/page/application_compose.rs`
- **Commit:** `ff939ff`

**3. [Rule 2 - Korrektheit] `last_outbound_summary` auf Outbound-Direction gefiltert**
- **Gefunden bei:** Task 2.
- **Problem:** `get_application_communications` liefert ein- UND ausgehende Eintraege; der Plan schlug `last_outbound_summary(&communications)` vor, was bei einer neueren eingehenden Antwort den falschen (inbound) Eintrag als „zuletzt gesendet" zeigen wuerde.
- **Fix:** Vor `last_outbound_summary` werden die Eintraege auf `CommunicationDirection::Outbound` gefiltert — der Guard reflektiert damit tatsaechlich die zuletzt **gesendete** Mail (D-06).
- **Dateien:** `genossi-frontend/src/page/application_compose.rs`
- **Commit:** `ff939ff`

## Issues Encountered

- `cargo build/test` muss aus `genossi-frontend/` via `nix develop --command …` laufen (Crate ist im Root-Workspace `exclude`d, Toolchain nur in der Flake-Devshell). Kein Code-Problem, nur Ausfuehrungsort.

## User Setup Required

None.

## Next Phase Readiness

- Plan 32-04 (`application_detail`) kann jetzt den „✉ E-Mail senden"-Button auf `Route::ApplicationCompose { id }` verlinken.
- Deferred UAT (Vorstands-Smoke via `dx serve`): Compose oeffnet vorbefuellt, Vorschau aktualisiert debounced, Senden deaktiviert waehrend Request, Erfolg → Rueckkehr zur Antragsliste + Toast. Long-text/HTML-Backstop der UI-SPEC ist ebenfalls dort zu pruefen.

## Test-Notiz

Die WASM-UI ist nicht host-unit-testbar; die zugrundeliegende Logik (`filter_templates_by_type`, `last_outbound_summary`, Request-Serialisierung) wurde in Plan 32-02 unit-getestet und wird hier konsumiert. Das Build-Gate beweist Typ-/Router-/Prop-Korrektheit; `tests/no_form_submit_regression.rs` beweist die Einhaltung der „kein form-onsubmit"-Regel projektweit. 339 Frontend-Tests gruen. Konsistent mit der Projektpraxis fuer reine View-Seiten ohne zusaetzliche host-testbare Logik.

## Self-Check: PASSED

- Alle erstellten/modifizierten Dateien vorhanden (5/5).
- Alle Task-Commits im git-Log (22c9eb0, ff939ff, dc02b4d).
