---
phase: 32-frontend-compose-dialog
plan: 04
subsystem: ui
tags: [dioxus, wasm, tailwind, application-mail, communication-timeline, is_email_empty]

# Dependency graph
requires:
  - phase: 32-01
    provides: CommunicationEntryTO.rendered_body/rendered_html_body (gespeicherter Body)
  - phase: 32-02
    provides: get_application_communications + last_outbound_summary API-Funktionen
  - phase: 32-03
    provides: Route::ApplicationCompose { id } (scoped Compose-Seite)
provides:
  - "Geteiltes util/email.rs mit is_email_empty (Component-First, getestet)"
  - "Additiver on_entry_click-Prop an CommunicationTimeline (backward-kompatibel)"
  - "application_detail: Senden-Button, zuletzt-gesendet-Zeile, Historie-Abschnitt, Inline-Body-Detail-Panel"
affects: [antragsteller-kommunikation, application-detail, member-details]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Additive optionale EventHandler-Props zur Rueckwaerts-kompatiblen Komponenten-Erweiterung (Landmine 3)"
    - "Geteilte reine Util-Funktionen mit Inline-Tests (util/email.rs analog util/format.rs)"
    - "Gespeicherten Body HTML-escaped in max-h-96 overflow-auto whitespace-pre-wrap zeigen (kein dangerous_inner_html, kein Re-Render)"

key-files:
  created:
    - genossi-frontend/src/util/email.rs
  modified:
    - genossi-frontend/src/util/mod.rs
    - genossi-frontend/src/page/member_details.rs
    - genossi-frontend/src/component/communication_timeline.rs
    - genossi-frontend/src/component/application_detail.rs

key-decisions:
  - "is_email_empty aus member_details.rs in geteiltes util/email.rs gehoben; beide Nutzer teilen die Logik (keine Duplikate)"
  - "CommunicationTimeline additiv erweitert (#[props(default)] on_entry_click); ohne Handler exakt der bestehende Link-Pfad — Member-Nutzung unveraendert"
  - "Body-Panel bevorzugt rendered_body (Klartext) vor rendered_html_body, HTML-escaped im Scroll-Container — konsistent mit mail_recipient_rendered_content.rs (bewusst kein dangerous_inner_html) und T-32-03"

patterns-established:
  - "Optionale EventHandler-Props als backward-kompatibler Erweiterungspfad fuer geteilte Komponenten"

requirements-completed: [APMAIL-03, APUI-01, APUI-03]

coverage:
  - id: D1
    description: "Geteiltes is_email_empty in util/email.rs; member_details nutzt es (Duplikat entfernt)"
    requirement: "APUI-01"
    verification:
      - kind: unit
        ref: "genossi-frontend/src/util/email.rs#is_email_empty_* (5 Tests)"
        status: pass
    human_judgment: false
  - id: D2
    description: "CommunicationTimeline additiv um on_entry_click erweitert; Member-Pfad ohne Handler unveraendert"
    requirement: "APUI-03"
    verification:
      - kind: integration
        ref: "cd genossi-frontend && nix develop --command cargo build -p genossi-frontend"
        status: pass
    human_judgment: false
  - id: D3
    description: "application_detail: Senden-Button disabled/annotiert bei fehlender Adresse, nav -> Route::ApplicationCompose"
    requirement: "APMAIL-03"
    verification:
      - kind: integration
        ref: "cd genossi-frontend && nix develop --command cargo build -p genossi-frontend"
        status: pass
      - kind: manual_procedural
        ref: "dx serve: Button ohne Adresse disabled + Hinweis; Klick oeffnet Compose scoped auf application_id"
        status: unknown
    human_judgment: true
    rationale: "Interaktives WASM-Verhalten (disabled-Button, Navigation) nur in dx-serve-Smoke-Session vor Milestone-Merge pruefbar"
  - id: D4
    description: "zuletzt-gesendet-Zeile (Betreff+Status+Datum via last_outbound_summary), sonst NeverSent"
    requirement: "APUI-03"
    verification:
      - kind: unit
        ref: "genossi-frontend/src/api.rs#test_last_outbound_summary_* (aus Plan 32-02, weiterhin gruen)"
        status: pass
    human_judgment: false
  - id: D5
    description: "Body-Detail-Panel zeigt echten gespeicherten Body (kein Re-Render) im begrenzten Scroll-Container"
    requirement: "APUI-03"
    verification:
      - kind: manual_procedural
        ref: "dx serve: Timeline-Klick oeffnet Inline-Panel mit echtem Body; lange/HTML-Bodies bleiben im max-h-96-Scroll-Container"
        status: unknown
    human_judgment: true
    rationale: "long-text-Backstop (UI-SPEC): visueller Held-out-Check, dass sehr lange/HTML-Bodies nicht ueberlaufen — nur interaktiv pruefbar"

# Metrics
duration: 5min
completed: 2026-08-20
status: complete
---

# Phase 32 Plan 04: Application-Detail Kommunikations-Erweiterungen Summary

**"✉ E-Mail senden"-Button (disabled/annotiert bei fehlender Adresse) plus zuletzt-gesendet-Zeile, prop-getriebene Historie und Inline-Body-Panel in application_detail — auf Basis eines geteilten is_email_empty und einer additiv erweiterten CommunicationTimeline.**

## Performance

- **Duration:** ~5 min
- **Started:** 2026-08-20T23:10:40Z
- **Completed:** 2026-08-20T23:15:13Z
- **Tasks:** 3
- **Files modified:** 5 (1 erstellt, 4 geaendert)

## Accomplishments
- `is_email_empty` aus `member_details.rs` in geteiltes `util/email.rs` gehoben (5 Unit-Tests wandern mit, bleiben gruen); `member_details.rs` importiert es nun statt eigener Duplikat-Definition — Verhalten des Member-Buttons unveraendert (Component-First, APUI-01).
- `CommunicationTimeline` additiv um `#[props(default)] on_entry_click: Option<EventHandler<CommunicationEntryTO>>` erweitert: mit Handler wird die Betreff-Zelle ein klickbares `span` (gleiche Link-Optik), ohne Handler bleibt exakt der bestehende `Link`-Pfad — Member-Nutzung backward-kompatibel (Landmine 3, APUI-03).
- `application_detail.rs` um vier Elemente erweitert: (1) "✉ E-Mail senden"-Button `disabled`+`title`+italic-Hinweis bei leerer Adresse, sonst `nav.push(Route::ApplicationCompose { id })` — nie stiller Fehlversuch (APMAIL-03); (2) Kommunikation via `get_application_communications` beim Mount geladen; (3) "zuletzt gesendet"-Zeile via `last_outbound_summary` (Betreff+Status+Datum), sonst `NeverSent`; (4) Inline-Body-Detail-Panel mit echtem gespeicherten Body (kein Re-Render, kein Modal-in-Modal).

## Task Commits

Jede Task atomar committed:

1. **Task 1: is_email_empty in geteiltes util/email.rs heben** - `8b72ee4` (refactor)
2. **Task 2: CommunicationTimeline additiv um on_entry_click** - `72ae842` (feat)
3. **Task 3: application_detail Button + last-sent + Timeline + Body-Panel** - `d79cf2f` (feat)

**Plan-Metadaten:** siehe abschliessenden docs-Commit.

## Files Created/Modified
- `genossi-frontend/src/util/email.rs` (NEU) - geteiltes `pub fn is_email_empty` + 5 Unit-Tests
- `genossi-frontend/src/util/mod.rs` - `pub mod email;`
- `genossi-frontend/src/page/member_details.rs` - nutzt `crate::util::email::is_email_empty`; Duplikat-Funktion und -Tests entfernt
- `genossi-frontend/src/component/communication_timeline.rs` - additiver `on_entry_click`-Prop; `render_entry` durchgereicht
- `genossi-frontend/src/component/application_detail.rs` - Senden-Button, last-sent-Zeile, Timeline-Abschnitt, Inline-Body-Panel; `outbound_status_label`-Helfer

## Decisions Made
- **Body-Panel bevorzugt `rendered_body` (Klartext) vor `rendered_html_body`.** Der Plan nannte die HTML-Variante zuerst; die Anzeige erfolgt jedoch HTML-escaped mit `whitespace-pre-wrap` (kein `dangerous_inner_html`) — konsistent mit dem bestehenden `mail_recipient_rendered_content.rs` und T-32-03 (kein Live-Re-Render von User-HTML). Klartext ist fuer diese escaped Text-Anzeige die lesbarere Primaerquelle; HTML dient als Fallback. Der echte gespeicherte Body wird in beiden Faellen gezeigt (D-06 gewahrt).
- **`outbound_status_label`-Helfer** spiegelt das sent/failed/pending-Mapping der Timeline-Badges fuer die zuletzt-gesendet-Zeile (konsistente Status-Uebersetzung).

## Deviations from Plan

### Auto-fixed / bewusste Praezisierungen

**1. [Rule 1 - Korrektheit/Sicherheit] Body-Quelle: rendered_body vor rendered_html_body**
- **Found during:** Task 3 (Body-Detail-Panel)
- **Issue:** Der Plan nannte `rendered_html_body` als Primaerquelle; da die Anzeige HTML-escaped (`whitespace-pre-wrap`, kein `dangerous_inner_html`) erfolgt, wuerden HTML-Tags roh als Text erscheinen.
- **Fix:** `rendered_body` (Klartext) bevorzugt, `rendered_html_body` als Fallback — konsistent mit dem etablierten `mail_recipient_rendered_content.rs`-Muster und T-32-03; zeigt weiterhin den echten gespeicherten Body ohne Re-Render.
- **Files modified:** genossi-frontend/src/component/application_detail.rs
- **Verification:** Build gruen; Anzeige im begrenzten Scroll-Container (`max-h-96 overflow-auto whitespace-pre-wrap`)
- **Committed in:** d79cf2f (Task-3-Commit)

---

**Total deviations:** 1 bewusste Praezisierung (Korrektheit/Sicherheit, kein Scope-Creep)
**Impact on plan:** Alle Muss-Kriterien erfuellt; die Praezisierung verbessert Lesbarkeit und haelt die etablierte No-`dangerous_inner_html`-Sicherheitslinie ein.

## Issues Encountered
None - Build und alle 339 Frontend-Unit-Tests (+1 no-form-submit-Regressionstest) gruen nach jeder Task.

## User Setup Required
None - keine externen Dienste konfiguriert.

## Next Phase Readiness
- Phase 32 (frontend-compose-dialog) mit diesem Plan vollstaendig; alle Wave-1/2/3-Outputs integriert.
- Deferred UAT: `dx serve`-Vorstands-Smoke-Session vor Milestone-Merge (disabled-Button, Navigation, Body-Panel, long-text-Backstop) — siehe coverage D3/D5.
- Blocking UI-SPEC-Safety-Gate laeuft anschliessend (32-UI-SPEC.md-Konformitaet).

## Self-Check: PASSED

---
*Phase: 32-frontend-compose-dialog*
*Completed: 2026-08-20*
