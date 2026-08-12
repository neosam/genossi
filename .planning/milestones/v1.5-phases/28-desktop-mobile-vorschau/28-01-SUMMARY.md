---
phase: 28-desktop-mobile-vorschau
plan: 01
subsystem: api
tags: [ammonia, minijinja, axum, sanitize, mail-preview, xss]

# Dependency graph
requires:
  - phase: 23-html-mail-backend
    provides: "ammonia-Sanitizer (`sanitize.rs`) + `sanitize_body_html_opt`-Helper (D-03 Store-Boundary)"
  - phase: 24-wysiwyg-frontend-editor
    provides: "`PreviewRequest.body_html` / `PreviewResponse.body_html` Wire-Seam + `render_html_template` (autoescape env)"
  - phase: 27-bild-support-backend-editor-upload
    provides: "gehärtete `<img>`-ammonia-Policy (nur `data-genossi-asset-id` überlebt, `src`/`srcset` gestrippt)"
provides:
  - "`POST /api/mail/preview` sanitisiert `body_html` mit ammonia, BEVOR minijinja rendert (D-01, D-02)"
  - "Preview-Response trägt garantiert dieselbe HTML-Fassung, die der Empfänger bekommt"
  - "`<img>` in der Vorschau kann keine externe URL mehr laden — nur `data-genossi-asset-id` überlebt"
  - "vier e2e-Tests, die den Sanitize-vor-Render-Contract am echten HTTP-Endpoint festnageln"
affects: [28-02-frontend-asset-src-injection, 28-03-device-vorschau-iframe, 28-05-uat]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Sanitize-vor-Render: ammonia zuerst, Jinja danach — spiegelt die Produktion (Store-Boundary vor Send-Worker)"
    - "Wiederverwendung von `sanitize_body_html_opt` statt neuer `Option`-Verzweigung (None in ⇒ None out)"

key-files:
  created:
    - ".planning/phases/28-desktop-mobile-vorschau/deferred-items.md"
  modified:
    - "genossi_mail/src/rest.rs"
    - "genossi_bin/tests/e2e_tests.rs"

key-decisions:
  - "Sanitize-vor-Render (D-02) statt Render-vor-Sanitize: Member-Werte werden in Produktion autoescaped, nicht sanitisiert — die umgekehrte Reihenfolge wäre asymmetrisch zur Produktion"
  - "Kein Diff-Banner (D-04): das dargestellte sanitisierte Ergebnis ist der Beweis; Attribut-Platzhalter werden sichtbar gestrippt und das ist gewollt"
  - "`cargo fmt`-Kollateraländerung an `sanitize.rs` per `git checkout HEAD --` zurückgenommen, weil der Plan `sanitize.rs` explizit als unverändert fordert"
  - "Der zweite vorbestehende Fehlschlag `preview_body_html_round_trips_to_response` wurde NICHT gefixt — Task 2 verbietet jede Änderung an bestehenden Tests; nach deferred-items.md ausgelagert"

patterns-established:
  - "Nicht-Regressions-Beweis per temporärem `git show HEAD~1:<file> > <file>`-Rücksetzer statt Behauptung: der fragliche Test wird gegen den Vor-Zustand laufen gelassen und die Panic-Zeile byte-verglichen"
  - "Falsch gewordene Rationale-Kommentare werden im selben Commit mitgeändert (Kommentar-Konvention `// Phase NN (REQ, D-XX): <Was> — <Warum>`)"

requirements-completed: [PREV-02]

coverage:
  - id: D1
    description: "POST /api/mail/preview entfernt on*-Attribute und <script>-Tags aus body_html, interpoliert danach Jinja-Platzhalter aus dem Text-Content"
    requirement: "PREV-02"
    verification:
      - kind: e2e
        ref: "genossi_bin/tests/e2e_tests.rs#preview_body_html_is_sanitized_before_render"
        status: pass
    human_judgment: false
  - id: D2
    description: "<img> in der Vorschau verliert seine externe src, behält data-genossi-asset-id samt UUID (kein Tracking-Pixel, kein SSRF-Trigger)"
    requirement: "PREV-02"
    verification:
      - kind: e2e
        ref: "genossi_bin/tests/e2e_tests.rs#preview_body_html_img_keeps_asset_id_strips_src"
        status: pass
    human_judgment: false
  - id: D3
    description: "Jinja-Platzhalter in Text-Content-Position überleben ammonia auch in verschachtelten Allowlist-Tags und werden anschließend interpoliert; keine rohe Platzhalter-Syntax bleibt übrig"
    requirement: "PREV-02"
    verification:
      - kind: e2e
        ref: "genossi_bin/tests/e2e_tests.rs#preview_body_html_jinja_in_text_survives_sanitize"
        status: pass
    human_judgment: false
  - id: D4
    description: "Preview-Request ohne body_html-Key liefert unverändert keinen body_html-Key auf der Wire (kein Some(\"\")-Sentinel, backward-compatible)"
    requirement: "PREV-02"
    verification:
      - kind: e2e
        ref: "genossi_bin/tests/e2e_tests.rs#preview_without_body_html_stays_backward_compatible"
        status: pass
    human_judgment: false
  - id: D5
    description: "Nebeneffekt aus D-01: die bestehende TemplatePreview-Component rendert ab jetzt die sanitisierte Fassung statt des ungefilterten Editor-DOMs"
    verification: []
    human_judgment: true
    rationale: "Sichtbare Darstellungsänderung im Browser (dangerous_inner_html in template_preview.rs). Der Backend-Contract ist automatisiert bewiesen, aber ob der Vorstand die neue Darstellung als korrekt und nicht als Datenverlust wahrnimmt, ist eine Beurteilungsfrage für die UAT in Plan 28-05."

# Metrics
duration: 13min
completed: 2026-07-28
status: complete
---

# Phase 28 Plan 01: Sanitize-vor-Render im Preview-Endpoint Summary

**`POST /api/mail/preview` schickt `body_html` jetzt zuerst durch ammonia und erst danach durch minijinja — die Vorschau zeigt exakt die HTML-Fassung, die der Empfänger bekommt, statt des ungefilterten `contenteditable`-DOMs.**

## Performance

- **Duration:** ~13 min
- **Started:** 2026-07-28T04:38:00Z
- **Completed:** 2026-07-28T04:51:00Z
- **Tasks:** 2
- **Files modified:** 2 (+1 neue Doku-Datei)

## Accomplishments

- `crate::service::sanitize_body_html_opt` im `preview_mail`-Handler verdrahtet; `render_html_template` matcht jetzt auf die sanitisierte Bindung statt auf `body.body_html`. Keine neue `Option`-Verzweigung — der Helper garantiert `None` in ⇒ `None` out.
- Der Rationale-Kommentar, der bis eben `Read-only preview — no sanitization here` behauptete, wurde ersetzt; die Aussage war durch D-02 falsch geworden.
- Vier neue e2e-Tests am echten HTTP-Endpoint decken Sanitize-vor-Render, `<img>`-Härtung, den Jinja-Text-Content-Contract und die Backward-Compat des None-Pfads ab.
- T-28-01 (Tampering), T-28-02 und T-28-03 (Information Disclosure) aus dem Threat Register sind mitigiert und jeweils durch einen benannten Test belegt.

## Task Commits

1. **Task 1: Sanitize-vor-Render im preview_mail-Handler verdrahten (D-01, D-02)** — `f51ab45` (feat)
2. **Task 2: Vier e2e-Tests für Sanitize-vor-Render am echten HTTP-Endpoint** — `2c8c143` (test)

## Files Created/Modified

- `genossi_mail/src/rest.rs` — `sanitized_body_html`-Bindung + ersetzter Rationale-Kommentar im `preview_mail`-Handler (+20 / -3)
- `genossi_bin/tests/e2e_tests.rs` — vier neue `#[tokio::test]`-Funktionen (+263 / -0, ausschließlich Additionen)
- `.planning/phases/28-desktop-mobile-vorschau/deferred-items.md` — vier out-of-scope-Funde dokumentiert

## (a) Exakte Form des neuen Kommentars

```rust
            // Phase 24 (EDIT-05, D-04): if the caller supplied an HTML sibling,
            // render it through the autoescape env (member values escaped, author
            // markup structurally preserved).
            //
            // Phase 28 (PREV-02, D-01/D-02): sanitize BEFORE render — die Vorschau
            // zeigt exakt die HTML-Fassung, die der Empfänger bekommt, statt des
            // ungefilterten `contenteditable`-DOMs. Die Reihenfolge ist bindend und
            // spiegelt die Produktion: ammonia greift am Store-Boundary (Phase 23
            // D-03), das Jinja-Rendering erst im Send-Worker. Render-dann-sanitize
            // wäre asymmetrisch, weil Member-Werte in Produktion autoescaped und
            // nicht sanitisiert werden.
            // Jinja-Platzhalter im TEXT-Content (`<p>Hallo {{ first_name }}</p>`)
            // überleben ammonia unverändert (siehe `sanitize.rs` Zeilen 30-34).
            // Platzhalter in ATTRIBUTEN (`<a href="{{ link }}">`) sind seit Phase 24
            // out-of-contract und werden hier erstmals sichtbar gestrippt — gewollt,
            // kein Bug, und laut D-04 ohne Diff-Banner: die Darstellung des
            // sanitisierten Ergebnisses ist der Beweis.
            // `sanitize_body_html_opt` garantiert `None` in ⇒ `None` out, deshalb
            // keine zusätzliche Verzweigung (kein `Some("")`-Sentinel).
            let sanitized_body_html =
                crate::service::sanitize_body_html_opt(body.body_html.as_deref());
            let rendered_body_html: Option<String> = match sanitized_body_html.as_deref() {
```

Der Phase-24-Teil (EDIT-05, D-04: autoescape env, Member-Werte escaped, Autoren-Markup
strukturell erhalten) ist erhalten geblieben und wurde ergänzt, nicht gelöscht. Der
`plain_from_html`-Block direkt darunter, die `PreviewResponse`-Konstruktion, Statuscode
und Header sind buchstäblich unverändert.

## (b) Pre-existing-Failure-Baseline im Wortlaut

**Baseline-Lauf VOR jeder Änderung** (`cargo test -p genossi_bin --test e2e_tests test_mail_preview_repayment_no_entries_does_not_default_to_one`):

```
thread 'test_mail_preview_repayment_no_entries_does_not_default_to_one' (2716374) panicked at genossi_bin/tests/e2e_tests.rs:14628:44:
errors must be array
```

**Lauf NACH beiden Tasks** — Meldung und Fundstelle byte-identisch:

```
thread 'test_mail_preview_repayment_no_entries_does_not_default_to_one' (2729500) panicked at genossi_bin/tests/e2e_tests.rs:14628:44:
errors must be array
```

Damit ist die Acceptance-Criteria-Bedingung erfüllt: weder Meldung noch Ort haben sich
geändert, Task 1 hat den Repayment-Pfad nicht berührt. Die Zeilennummer bleibt stabil, weil
die vier neuen Tests hinter Zeile 15006 eingefügt wurden, also hinter der Fundstelle.

**Zweiter, im Plan nicht antizipierter vorbestehender Fehlschlag:**
`preview_body_html_round_trips_to_response` (`e2e_tests.rs:14961:5`) schlägt mit
`left: "Hallo **Max**"` / `right: "Hallo Max"` fehl. Der Plan listet diesen Test unter den
Acceptance Criteria als "bleibt grün" — er war es aber schon vorher nicht. Beweis: `rest.rs`
wurde temporär per `git show HEAD~1:genossi_mail/src/rest.rs` auf den Stand VOR dem
Phase-28-Commit zurückgesetzt; der Test schlug dort mit byte-identischer Meldung an derselben
Zeile `14961:5` fehl. Ursache ist Quick `260718-html-to-plain-derivation`, das
`PreviewResponse.body` per `plain_from_html` aus dem HTML ableitet
(`<p>Hallo <b>Max</b></p>` ⇒ `Hallo **Max**`), ohne die Phase-24-Assertion nachzuziehen.
Nicht gefixt, weil Task 2 jede Änderung an bestehenden Tests verbietet — siehe
`deferred-items.md` Punkt 1.

**Gesamtlauf:** `cargo test -p genossi_bin --test e2e_tests` ⇒ 314 passed, 2 failed
(exakt die beiden oben beschriebenen vorbestehenden Fehlschläge, keine weiteren).

## (c) Rendert die bestehende TemplatePreview sichtbar anders?

**Ja — und das ist der ausdrücklich erwünschte Nebeneffekt aus D-01/D-16.**

`genossi-frontend/src/component/mail_compose/template_preview.rs` rendert
`preview.body_html` per `dangerous_inner_html` (Zeile 193). Da der Backend-Handler jetzt die
sanitisierte Fassung zurückgibt, zeigt die bestehende `TemplatePreview` ab sofort ebenfalls
das gefilterte HTML. Für die UAT in Plan 28-05 relevant sind drei sichtbare Änderungen:

1. `<img>`-Elemente mit externer `src` erscheinen ohne Bild — bis Plan 28-02 die
   `/bytes`-URL frontend-seitig injiziert, bleibt an dieser Stelle ein leeres `<img>`.
2. Jinja-Platzhalter in ATTRIBUT-Position (`<a href="{{ link }}">`) verlieren das Attribut.
   Das ist seit Phase 24 out-of-contract; hier wird es erstmals sichtbar. Laut D-04 bewusst
   ohne Diff-Banner.
3. Inline-Styles und nicht-allowlistete Tags aus per Copy-Paste eingefügtem Fremd-Markup
   verschwinden aus der Vorschau — sie verschwinden in der versendeten Mail ohnehin, die
   Vorschau war bisher nur zu optimistisch.

Kein Datenverlust: sanitisiert wird ausschließlich die Preview-Response, der gespeicherte
Template-Inhalt bleibt unangetastet (Phase 28 schreibt nichts, T-28-05).

## Decisions Made

- **Sanitize-vor-Render statt Render-vor-Sanitize (D-02).** Bindende Reihenfolge aus dem Plan; sie spiegelt die Produktion, wo ammonia am Store-Boundary greift und das Jinja-Rendering erst im Send-Worker passiert. Die umgekehrte Reihenfolge würde Member-Werte sanitisieren, die in Produktion nur autoescaped werden.
- **Kein `if let Some(...)`.** `sanitize_body_html_opt` garantiert `None` in ⇒ `None` out; eine eigene Verzweigung hätte den `Some("")`-Sentinel-Pitfall wieder aufgemacht.
- **`sanitize.rs`-Formatierungsänderung zurückgenommen.** `cargo fmt -p genossi_mail` brach einen `rm_tag_attributes`-Aufruf in `sanitize.rs` um. Der Plan fordert `sanitize.rs` explizit als unverändert, deshalb per `git checkout HEAD -- genossi_mail/src/sanitize.rs` revertiert und nach `deferred-items.md` ausgelagert.

## Deviations from Plan

Keine Rule-1/2/3-Autofixes an Produktivcode. Zwei Abweichungen von den Acceptance Criteria,
beide durch Plan-Constraints erzwungen:

**1. [Scope Boundary] Acceptance Criterion "`preview_body_html_round_trips_to_response` bleibt grün" nicht erfüllbar**

- **Gefunden bei:** Task 2
- **Problem:** Der Test war bereits vor Phase 28 rot (`"Hallo **Max**"` statt `"Hallo Max"`), verursacht durch Quick `260718-html-to-plain-derivation`. Der Plan ging fälschlich von einem grünen Ausgangszustand aus.
- **Vorgehen:** Nicht gefixt. Task 2 verbietet ausdrücklich jede Änderung an bestehenden Tests (`git diff --numstat` muss 0 gelöschte Zeilen zeigen). Nicht-Regression stattdessen per temporärem `HEAD~1`-Rücksetzer von `rest.rs` bewiesen und in `deferred-items.md` Punkt 1 mit Reproduktion dokumentiert.
- **Verifikation:** `git diff --numstat genossi_bin/tests/e2e_tests.rs` ⇒ `263  0` (0 gelöschte Zeilen).

**2. [Scope Boundary] `cargo fmt`-Drift in unberührten Dateien nicht gefixt**

- **Gefunden bei:** Task 1 (`sanitize.rs`) und Task 2 (`membership_adjust_e2e.rs`, 16 Fundstellen)
- **Vorgehen:** `sanitize.rs`-Änderung revertiert (Plan-Constraint), `membership_adjust_e2e.rs` gar nicht angefasst. Beides in `deferred-items.md` Punkte 3 und 4.
- **Verifikation:** `git diff --name-only` listet ausschließlich `genossi_mail/src/rest.rs` und `genossi_bin/tests/e2e_tests.rs`. `cargo fmt -p genossi_bin -- --check` meldet 0 Fundstellen in `e2e_tests.rs`.

---

**Total deviations:** 0 Autofixes, 2 dokumentierte Scope-Boundary-Entscheidungen
**Impact on plan:** Kein Scope Creep. PREV-02 ist backend-seitig vollständig erfüllt; die
beiden Abweichungen betreffen ausschließlich vorbestehende, plan-fremde Zustände.

## Issues Encountered

- **`cargo fmt` als Kollateralschaden.** `cargo fmt -p genossi_mail` formatiert die ganze Crate, nicht nur die geänderte Datei, und riss `sanitize.rs` mit hinein — was das Acceptance Criterion "`sanitize.rs` unverändert" verletzt hätte. Gelöst per gezieltem `git checkout HEAD -- genossi_mail/src/sanitize.rs`. Für Folgeplans: nach `cargo fmt` immer `git diff --stat` prüfen.
- **Verdacht auf Selbstverschuldung bei `preview_body_html_round_trips_to_response`.** Statt zu behaupten, es sei vorbestehend, wurde `rest.rs` temporär auf `HEAD~1` zurückgesetzt und der Test dort laufen gelassen — byte-identische Panic-Meldung an derselben Zeile. Danach aus dem Scratchpad wiederhergestellt und per `git diff --stat` bestätigt, dass der Arbeitsstand exakt dem Commit entspricht.

## Threat Flags

Keine. Alle im Plan registrierten Threats (T-28-01 bis T-28-05, T-28-SC) sind adressiert;
es wurde keine neue Angriffsfläche eingeführt. Phase 28 führt keine schreibende Operation
aus, kein neues Paket wurde installiert, `Cargo.toml`/`Cargo.lock` sind unberührt.

## Known Stubs

Keine. Der geänderte Codepfad ist vollständig verdrahtet und durch vier Tests belegt.

## User Setup Required

Keine — kein externer Service, keine neue Umgebungsvariable, keine Migration.

## Next Phase Readiness

- **Plan 28-02** findet im `PreviewResponse.body_html` garantiert die Form
  `<img data-genossi-asset-id="…">` ohne `src` vor — genau der Input, den
  `inject_asset_src` erwartet. Durch `preview_body_html_img_keeps_asset_id_strips_src` festgenagelt.
- **Plan 28-03** (Device-Vorschau im sandboxed iframe) kann sich darauf verlassen, dass das
  gelieferte HTML bereits ammonia-gefiltert ist; das iframe-Sandboxing ist Defense-in-Depth,
  nicht die einzige Verteidigungslinie.
- **Plan 28-05 (UAT)** muss den unter (c) beschriebenen sichtbaren Nebeneffekt auf die
  bestehende `TemplatePreview` mit abdecken — insbesondere das bis 28-02 leere `<img>`.
- **Offen (nicht blockierend):** zwei vorbestehende e2e-Fehlschläge, siehe `deferred-items.md`.

## Self-Check: PASSED

- Dateien vorhanden: `genossi_mail/src/rest.rs`, `genossi_bin/tests/e2e_tests.rs`,
  `28-01-SUMMARY.md`, `deferred-items.md`
- Commits vorhanden: `f51ab45`, `2c8c143`

---
*Phase: 28-desktop-mobile-vorschau*
*Completed: 2026-07-28*
