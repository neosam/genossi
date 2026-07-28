---
phase: 28-desktop-mobile-vorschau
plan: 03
subsystem: frontend
tags: [dioxus, wysiwyg, preview, segmented-control, offscreen, grep-gate, contenteditable]

# Dependency graph
requires:
  - phase: 24-wysiwyg-frontend-editor
    provides: "`WysiwygEditor` mit `sync_from_dom` (inner_text-basiert, D-02), `EDITOR_ID`-Konstante, Grep-Gate-Muster inkl. Self-Reference-Abwehr"
  - phase: 28-desktop-mobile-vorschau
    plan: 01
    provides: "sanitisiertes `body_html` in der `POST /api/mail/preview`-Response"
  - phase: 28-desktop-mobile-vorschau
    plan: 02
    provides: "`PreviewMode`, `preview_needs_fetch`, `inject_asset_src`, `preview_srcdoc`, `MailPreviewFrame`, sieben `MailEditorMode*`-i18n-Keys"
provides:
  - "`WysiwygEditor` mit zwei rückwärtskompatiblen Props `preview_member_id` und `repayment_phase_id` (D-03)"
  - "Drei-Modi-Segmented-Control im Editor selbst — wirkt ohne Verdrahtung in allen drei Call-Sites (D-13)"
  - "`editor_container_style(mode) -> &'static str` — Off-Screen statt Rendering-Unterdrückung (D-17, T-28-12)"
  - "`switch_preview_mode(...)` — Sync-vor-Wechsel, Fetch-Entscheidung, Request-Dispatch (D-05, T-28-13)"
  - "Toolbar-Ausblendung in beiden Vorschau-Modi (D-14)"
  - "5 neue Tests: 3 Verhaltenstests + 2 Source-Invarianten-Gates"
  - "Rückbau des Modul-Level `#![allow(dead_code)]` in `mail_preview_frame.rs`"
affects: [28-04-call-site-verkabelung, 28-05-uat]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Modus-State INNERHALB der wiederverwendeten Component statt als Prop: der Umschalter wirkt automatisch in allen Call-Sites, ohne dass eine Page ihn verdrahtet"
    - "Off-Screen-Positionierung statt Rendering-Unterdrückung, wenn ein DOM-Knoten weiterhin per `inner_text()` lesbar bleiben muss"
    - "Grep-Gate-Needle auf die AUFRUFFORM statt auf die Definition, damit der Gate die Verdrahtung beweist und nicht bloß die Existenz der Funktion"
    - "Zweischichtige Absicherung eines Pitfalls: Verhaltenstest auf dem Funktionsrückgabewert PLUS Source-Invarianten-Gate auf der Verdrahtung"

key-files:
  created: []
  modified:
    - "genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs"
    - "genossi-frontend/src/component/mail_compose/mail_preview_frame.rs"

key-decisions:
  - "Off-Screen-Style als Inline-`style` statt Tailwind-Klasse: arbiträre Tailwind-Werte müssten vom JIT-Purge im Quelltext gefunden werden, ein String-Rückgabewert ist robuster und nativ testbar"
  - "Needle des Gates `editor_uses_offscreen_style_helper` zielt auf `editor_container_style(*mode.read())` statt auf `editor_container_style(`: die Definition selbst enthält die kürzere Form, der Gate wäre sonst schon durch die bloße Existenz der Funktion erfüllt und würde die Verdrahtung NICHT beweisen"
  - "`#[allow(clippy::too_many_arguments)]` auf `switch_preview_mode` statt eines Parameter-Structs: die acht Parameter sind vom Plan vorgegeben, ein Struct hätte den 400-Zeichen-Abstand des Sync-Gates verschleiert und eine Indirektion ohne Mehrwert eingeführt"
  - "`#![allow(dead_code)]` in `mail_preview_frame.rs` entfernt statt behalten: nach der Verkabelung hat jedes Symbol einen Produktions-Konsumenten, der Build bleibt ohne das Allow warnungsfrei"

patterns-established:
  - "Zweistufiger Negativ-Nachweis: erst eine offensichtlich falsche Variante (`display:none` allein), dann eine subtile Variante, die alle anderen Asserts erfüllt und nur die eigentliche Invariante verletzt — beide Fehlschläge protokolliert"

requirements-completed: [PREV-01, PREV-02, PREV-04, PREV-05]

coverage:
  - id: D1
    description: "Im Bearbeiten-Modus trägt der contenteditable-Container keinen Inline-Style — das Element sieht exakt aus wie vor Phase 28"
    requirement: "PREV-04"
    verification:
      - kind: unit
        ref: "genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs#editor_container_style_is_empty_in_edit_mode"
        status: pass
    human_judgment: false
  - id: D2
    description: "In beiden Vorschau-Modi wird der contenteditable-Knoten off-screen positioniert und NICHT aus dem Rendering genommen — sonst fiele inner_text() auf textContent zurück und der Submit-Guard aller drei Call-Sites würde den text/plain-Teil der Mail umbruchlos überschreiben (T-28-12)"
    requirement: "PREV-05"
    verification:
      - kind: unit
        ref: "genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs#editor_is_hidden_offscreen_not_display_none"
        status: pass
      - kind: unit
        ref: "genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs#editor_uses_offscreen_style_helper"
        status: pass
    human_judgment: false
  - id: D3
    description: "Desktop und Mobile liefern denselben Container-Style — die Device-Breite lebt ausschließlich im iframe"
    requirement: "PREV-01"
    verification:
      - kind: unit
        ref: "genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs#editor_container_style_is_identical_for_both_preview_modes"
        status: pass
    human_judgment: false
  - id: D4
    description: "switch_preview_mode synchronisiert die Parent-Signale, BEVOR mode.set das Layout verändert (T-28-13, Pitfall 1 Schicht 2)"
    requirement: "PREV-02"
    verification:
      - kind: unit
        ref: "genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs#preview_mode_switch_syncs_dom_before_switching"
        status: pass
    human_judgment: false
  - id: D5
    description: "Die fünf bestehenden Grep-Gates in wysiwyg_editor.rs bleiben nach dem RSX-Umbau grün — insbesondere editor_uses_mail_html_render_scope, das eine versehentliche Änderung der class des contenteditable-Containers sofort meldet"
    requirement: "PREV-04"
    verification:
      - kind: unit
        ref: "genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs#editor_uses_mail_html_render_scope"
        status: pass
      - kind: unit
        ref: "genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs#style_with_css_false_guard_present"
        status: pass
      - kind: unit
        ref: "genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs#paste_handler_calls_prevent_default_before_read"
        status: pass
      - kind: unit
        ref: "genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs#native_drop_target_wired_and_prevents_default"
        status: pass
      - kind: unit
        ref: "genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs#production_region_excludes_test_module"
        status: pass
    human_judgment: false
  - id: D6
    description: "Der Vorstand erkennt den aktiven Modus sofort, die Toolbar ist im Vorschau-Modus sichtbar weg, und der Wechsel Desktop ↔ Mobile flackert nicht"
    requirement: "PREV-01, PREV-04"
    verification: []
    human_judgment: true
    rationale: "Die Modus-Logik, die Toolbar-Bedingung und die Fetch-Entscheidung sind am Quelltext bzw. am Funktionsrückgabewert nachgewiesen. Ob die optische Hervorhebung des aktiven Buttons ausreicht und ob der iframe beim Breitenwechsel tatsächlich nicht neu lädt (Assumption A3), ist eine visuelle Beurteilungsfrage im laufenden Browser. Gehört in die UAT in Plan 28-05 — sinnvoll erst NACH Plan 28-04, weil `preview_member_id` bis dahin überall `None` ist und ausschließlich die Hinweiszeile erscheint."

# Metrics
duration: 8min
completed: 2026-07-28
status: complete
---

# Phase 28 Plan 03: Editor-Verkabelung Summary

**Der `WysiwygEditor` trägt jetzt den Drei-Modi-Umschalter selbst: Toolbar weg im Vorschau-Modus, contenteditable-Knoten off-screen statt aus dem Rendering genommen, genau ein Preview-Request beim Übergang Bearbeiten → Vorschau, gerendert über `MailPreviewFrame` — zwei `#[props(default)]`-Props halten alle drei Call-Sites unverändert kompilierbar.**

## Performance

- **Duration:** ~8 min
- **Tasks:** 3/3
- **Commits:** 3 (`9731f97`, `a98c910`, `667c3ae`)
- **Dateien:** 0 neu, 2 modifiziert — kein Cargo-Manifest, keine neue Dependency

## Testergebnis

`cd genossi-frontend && cargo test` → **327 passed, 0 failed** (Baseline aus Plan 28-02: 322 + 5 neue).

| Filter | Ergebnis |
|---|---|
| `cargo test editor_container_style` | 2 passed |
| `cargo test editor_is_hidden_offscreen_not_display_none` | 1 passed |
| `cargo test editor_uses_offscreen_style_helper` | 1 passed |
| `cargo test preview_mode_switch_syncs_dom_before_switching` | 1 passed |
| `cargo test grep_gate` | 13 passed (die fünf bestehenden aus `wysiwyg_editor.rs` inklusive) |

`cargo build` exit 0. `cargo clippy --all-targets` exit 0 — **null** Treffer mit Bezug auf
`mail_compose`. `cargo fmt -- --check` meldet für die beiden berührten Dateien nichts
(einzige verbleibende Fundstelle im Crate ist die vorbestehende `api.rs`-Drift, siehe
Abweichung 3).

Die Warnungszahl des Builds ist von 46 (nach Task 1, mit noch unverdrahteten Symbolen) auf
39 gefallen — die vier Signal-, zwei Funktions- und die `mod.rs`-Import-Warnung sind mit der
Verkabelung verschwunden.

## (a) Exakter Rückgabewert von `editor_container_style`

```rust
pub(crate) fn editor_container_style(mode: PreviewMode) -> &'static str {
    if mode.is_preview() {
        "position:absolute;left:-10000px;top:0;width:640px"
    } else {
        ""
    }
}
```

| Modus | Rückgabewert |
|---|---|
| `PreviewMode::Edit` | `""` (leerer String) |
| `PreviewMode::Desktop` | `"position:absolute;left:-10000px;top:0;width:640px"` |
| `PreviewMode::Mobile` | `"position:absolute;left:-10000px;top:0;width:640px"` (identisch) |

Die feste `width:640px` hält den Umbruch im off-screen liegenden Element stabil, damit
`inner_text()` dieselben Umbrüche liefert wie im sichtbaren Zustand. Sie hat **nichts** mit
der Device-Breite zu tun — die lebt ausschließlich in `PreviewMode::width_px()` und damit im
iframe.

## (b) Negativ-Nachweis für die Hide-Strategie — Wortlaut

Der Nachweis wurde **zweistufig** geführt, weil eine einzelne offensichtlich falsche Variante
nur die erste Assertion getroffen hätte.

**Stufe 1** — Rückgabewert für die Vorschau-Modi testweise auf `"display:none"` geändert:

```
thread 'component::mail_compose::wysiwyg_editor::tests::editor_is_hidden_offscreen_not_display_none'
panicked at src/component/mail_compose/wysiwyg_editor.rs:583:13:
editor_container_style(Desktop) muss das Element absolut positionieren, um es off-screen
schieben zu können. Ist: "display:none"

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 326 filtered out
```

**Stufe 2** — subtile Variante, die alle anderen Asserts erfüllt und nur die eigentliche
Invariante verletzt: `"position:absolute;left:-10000px;top:0;width:640px;display:none"`:

```
thread 'component::mail_compose::wysiwyg_editor::tests::editor_is_hidden_offscreen_not_display_none'
panicked at src/component/mail_compose/wysiwyg_editor.rs:593:13:
editor_container_style(Desktop) darf das Element NICHT aus dem Rendering nehmen. Bei einem
nicht gerenderten Element fällt HtmlElement::inner_text() auf Node.textContent zurück (MDN),
wodurch die Zeilenumbrüche aus <p>, <li> und <br> verlorengehen. sync_from_dom benutzt
inner_text() ausdrücklich (Phase 24, D-02), und der Submit-Guard ALLER DREI Call-Sites
(reply_form.rs, mail_page.rs, mail_templates.rs) liest unmittelbar vor dem Absenden erneut
inner_text() direkt vom Element — der text/plain-Teil der versendeten Mail würde damit zu
einer einzigen Zeile Fließtext (T-28-12). Ist:
"position:absolute;left:-10000px;top:0;width:640px;display:none"
```

Beide Änderungen wurden unmittelbar zurückgenommen; `cd genossi-frontend && cargo test` läuft
seitdem wieder mit 327 passed / 0 failed. Der committete Stand enthält weder `display:none`
noch `visibility:hidden` im Produktionscode.

Damit ist T-28-12 nicht nur behauptet: Stufe 2 beweist, dass der Test genau die eine
sicherheitsrelevante Eigenschaft prüft und nicht bloß zufällig durch die Positionierungs-Asserts
mitläuft.

## (c) Endgültige Signatur von `WysiwygEditor` (für Plan 28-04)

Plan 28-04 kann die Call-Sites ohne erneutes Lesen der Datei verkabeln. Prop-Reihenfolge
exakt so:

```rust
#[component]
pub fn WysiwygEditor(
    value: String,
    on_change: EventHandler<(String, String)>,
    #[props(default)] preview_member_id: Option<Uuid>,
    #[props(default)] repayment_phase_id: Option<Uuid>,
) -> Element
```

| Prop | Typ | Default | Semantik |
|---|---|---|---|
| `value` | `String` | — | initiales `innerHTML` (unverändert seit Phase 24) |
| `on_change` | `EventHandler<(String, String)>` | — | `(plain: innerText, html: innerHTML)` (unverändert) |
| `preview_member_id` | `Option<Uuid>` | `None` | Member für die Template-Variablen der Vorschau. `None` ⇒ Hinweiszeile, **kein** Request |
| `repayment_phase_id` | `Option<Uuid>` | `None` | 1:1 an `/api/mail/preview` durchgereicht, gleiche Semantik wie bei `TemplatePreview` |

Da beide neuen Props `#[props(default)]` tragen, kompilieren die drei bestehenden Call-Sites
(`reply_form.rs`, `mail_page.rs`, `mail_templates.rs`) **unverändert** weiter. Bis 28-04 ist
`preview_member_id` überall `None` und der Vorschau-Modus zeigt konsequent die Hinweiszeile
`MailEditorModeSelectMember` — ein ehrlicher, kein kaputter Zwischenzustand.

**Signatur von `switch_preview_mode`** (privat, für den Fall dass 28-04 sie referenziert):

```rust
fn switch_preview_mode(
    target: PreviewMode,
    on_change: EventHandler<(String, String)>,
    mut mode: Signal<PreviewMode>,
    mut preview_doc: Signal<String>,
    mut preview_errors: Signal<Vec<String>>,
    mut preview_loading: Signal<bool>,
    preview_member_id: Option<Uuid>,
    repayment_phase_id: Option<Uuid>,
)
```

**Umgesetzte Aufrufkette:**
`Modus-Button → switch_preview_mode → sync_from_dom → innerHTML → api::preview_mail("", "", member_id, repayment_phase_id, Some(html)) → inject_asset_src(…, &config.backend) → preview_srcdoc → MailPreviewFrame.srcdoc`

## (d) Verhalten beim `key`-Bump-Remount (T-28-18)

Alle drei Call-Sites bumpen beim Template-Wechsel absichtlich einen `key` auf
`WysiwygEditor`, um einen Remount und damit ein Re-Seeding des `innerHTML` zu erzwingen
(Pitfall 6 der Research). Da `mode` ein **lokales** Signal der Component ist, fällt es bei
diesem Remount auf `PreviewMode::Edit` zurück.

**Das ist bewusst akzeptiertes und sogar erwünschtes Verhalten, kein Bug.** Nach einem
Template-Wechsel ist der Vorschau-Inhalt ohnehin veraltet; ein Rücksprung in den
Bearbeiten-Modus ist die richtige Reaktion. Die Alternative — den Modus in der Page zu halten
und hereinzureichen — würde D-13 aufgeben und den Umschalter in jeder Call-Site einzeln
verdrahtungspflichtig machen.

Die Begründung steht als Kommentar direkt über der `mode`-Deklaration in
`wysiwyg_editor.rs`, damit sie nicht später als Bug gemeldet wird.

## Weitere Umsetzungsdetails

**Segmented-Control (PREV-01, D-13).** Wörtlich nach dem Muster aus `page/templates.rs`
Zeilen 485-515: `div { class: "flex rounded-md overflow-hidden border border-gray-300" }` mit
drei Buttons, deren `class` ein `if`-Ausdruck auf `*mode.read() == <Zielmodus>` ist
(aktiv `px-3 py-1 text-sm bg-blue-600 text-white`, inaktiv
`px-3 py-1 text-sm bg-white text-gray-700 hover:bg-gray-100`). Jeder Button trägt
`r#type: "button"`, jeder `onclick` ruft als erste Anweisung `evt.prevent_default()`.

Bewusst **nicht** in eine eigene Component extrahiert: genau ein Verwender, und D-13 legt ihn
ausdrücklich in den `WysiwygEditor`. Der iframe hingegen **ist** eine eigene Component
(`MailPreviewFrame`) — dort greift Component-First. Die Begründung steht als Kommentar über
dem Block, analog zum Einzelverwender-Kommentar in `template_preview.rs`.

**Vorschau-Bereich.** Drei sich ausschließende Fälle: ohne Member die Hinweiszeile
(`MailEditorModeSelectMember`), während des Requests die Ladezeile
(`MailEditorModeLoading`), sonst `MailPreviewFrame { mode, srcdoc, errors }`. Kein
Diff-Banner (D-04) — die Darstellung des sanitisierten Ergebnisses ist der Beweis.

**Unverändert geblieben:** `EDITOR_ID`, `sync_from_dom` (weiterhin `inner_text()`),
`attach_image_drop_target`, `plain_to_html`, die `class` des contenteditable-Containers
(`w-full px-3 py-2 min-h-40 focus:outline-none mail-html-render`) und alle fünf bestehenden
Grep-Gates. Der `WysiwygLinkDialog` bleibt in jedem Modus gemountet, damit das gecachte
Selection-Range aus Pitfall 6 der Phase 24 nicht gefährdet wird.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blockierend] `#[allow(clippy::too_many_arguments)]` auf `switch_preview_mode`**

- **Gefunden während:** Task 1
- **Problem:** Die vom Plan vorgegebene Parameterliste hat acht Einträge; Clippys
  `too_many_arguments`-Schwelle liegt bei sieben. Das Akzeptanzkriterium fordert
  `cargo clippy --all-targets` ohne neue Warnungen.
- **Fix:** Ein gezieltes `#[allow(clippy::too_many_arguments)]` an der Funktion. Bewusst
  gewählt gegenüber einem Parameter-Struct: die Parameterliste ist vom Plan explizit
  vorgegeben, und ein Struct hätte den 400-Zeichen-Abstand zwischen `fn switch_preview_mode(`
  und `sync_from_dom(` — die Grundlage des Gates `preview_mode_switch_syncs_dom_before_switching`
  — verschleiert.
- **Datei:** `genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs`
- **Commit:** `9731f97`

**2. [Rule 3 - Blockierend] Rückbau des `#![allow(dead_code)]` in `mail_preview_frame.rs`**

- **Gefunden während:** Task 2
- **Problem:** Plan 28-02 hat ein Modul-Level `#![allow(dead_code)]` mit explizitem
  `TODO (Plan 28-03): nach der Verkabelung wieder entfernen` hinterlassen. Der Plan-Text von
  28-03 nennt nur `wysiwyg_editor.rs` als zu ändernde Datei, das Akzeptanzkriterium
  `git diff --name-only` listet genau eine Datei.
- **Fix:** Das Allow entfernt und durch einen kurzen Kommentar ersetzt, der den Rückbau
  dokumentiert. `cargo build` und `cargo clippy --all-targets` melden danach **null**
  Treffer mit Bezug auf `mail_compose` — jedes Symbol der Datei hat jetzt einen
  Produktions-Konsumenten. Damit ist auch der in `deferred-items.md` Punkt 6 vermerkte
  `unused import: mail_preview_frame::MailPreviewFrame` in `mod.rs` erledigt, weil
  `wysiwyg_editor.rs` die Component über die Re-Export-Route importiert.
- **Abweichung vom Kriterium:** `git diff --name-only` listet dadurch zwei statt einer Datei.
  Die zweite Datei ist der vom Vorgängerplan ausdrücklich beauftragte Rückbau, kein
  Scope-Creep.
- **Datei:** `genossi-frontend/src/component/mail_compose/mail_preview_frame.rs`
- **Commit:** `a98c910`

**3. [Rule 3 - Blockierend] Gezieltes `rustfmt` statt crate-weitem `cargo fmt`**

- **Gefunden während:** Task 1, 2 und 3
- **Problem:** Ein `cargo fmt` über den Crate hätte zugleich die vorbestehende Drift in
  `src/api.rs:405` mitverändert und damit eine unbeteiligte Datei in den Plan-Diff gezogen —
  was das repo-spezifische Git-Protokoll ausdrücklich verbietet. Identische Abweichung wie in
  Plan 28-02 (dort Abweichung 2).
- **Fix:** `rustfmt --edition 2021 <datei>` auf den beiden berührten Dateien. Danach ist
  `cargo fmt -- --check` für beide sauber; die einzige verbleibende Fundstelle im Crate ist
  die vorbestehende `api.rs`-Drift.
- **Nachkontrolle:** Suite nach jedem Formatieren erneut grün — insbesondere die Grep-Gates,
  deren Needles auf exakte Byte-Sequenzen zielen.

**4. [Rule 1 - Bug] Grep-Gate-Needle auf die Aufrufform statt auf die Definition**

- **Gefunden während:** Task 3
- **Problem:** Der Plan gibt für `editor_uses_offscreen_style_helper` die Needle aus
  `editor_container_styl` und `e(` vor. Diese Byte-Sequenz kommt aber bereits in der
  **Definitionszeile** `pub(crate) fn editor_container_style(mode: PreviewMode)` vor, die
  ebenfalls in der Produktionsregion liegt. Der Gate wäre damit schon durch die bloße
  Existenz der Funktion erfüllt und hätte die vom Plan geforderte Aussage — „der Helper ist
  tatsächlich im RSX verdrahtet" — **nicht** getroffen. Ein Entfernen des `style`-Attributs
  am contenteditable-`div` wäre unbemerkt durchgegangen und hätte den Verhaltenstest aus
  Teil A wertlos gemacht.
- **Fix:** Die Needle zielt jetzt auf die RSX-Aufrufform
  `editor_container_style(*mode.read())`, die in der Definition nicht vorkommt. Zur Laufzeit
  aus zwei Fragmenten zusammengesetzt, wie bei allen anderen Gates der Datei.
- **Datei:** `genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs`
- **Commit:** `667c3ae`

### Bewusst NICHT gefixt (Scope Boundary)

- **`cargo fmt`-Drift in `genossi-frontend/src/api.rs:405`** — vorbestehend, unberührte
  Datei, kein Phase-28-Bezug. Bereits als `deferred-items.md` Punkt 5 ausgelagert.
- **`genossi-frontend/Cargo.lock`** — wird bei jedem Build wegen des datumsbasierten
  Dev-Version-Strings neu geschrieben. Diff geprüft (reiner Version-String, **keine**
  Dependency-Änderung), in keinem Commit gestaged, im Arbeitsverzeichnis dirty gelassen.
  Es wurde kein Paket installiert (T-28-SC).
- **Die beiden vorbestehenden Backend-Testfehlschläge**
  (`test_mail_preview_repayment_no_entries_does_not_default_to_one`,
  `preview_body_html_round_trips_to_response`) — dieser Plan fasst kein Backend-File an.
  Bereits als `deferred-items.md` Punkt 1 und 2 erfasst.

## Authentication Gates

Keine.

## Known Stubs

Keine. Dass `preview_member_id` bis zum Abschluss von Plan 28-04 an allen drei Call-Sites
`None` ist, ist der ausdrückliche Plan-Zuschnitt (D-03: rückwärtskompatible Props, damit jede
Call-Site einzeln umstellbar bleibt und kein Big-Bang-Commit entsteht) und kein Stub: Der
`None`-Fall ist ein vollständig implementierter, getesteter Pfad mit eigener
Benutzerführung (`MailEditorModeSelectMember`), nicht ein hartkodierter Leerwert und kein
Platzhaltertext.

## Threat Flags

Keine neue Angriffsfläche außerhalb des Threat Models. Die sechs `mitigate`-Dispositionen
dieses Plans sind umgesetzt und belegt:

| Threat | Umsetzung | Beweis |
|---|---|---|
| T-28-12 (Hide-Strategie) | Off-Screen-Positionierung statt Rendering-Unterdrückung | `editor_is_hidden_offscreen_not_display_none` + zweistufiger Negativ-Nachweis, Verdrahtung durch `editor_uses_offscreen_style_helper` |
| T-28-13 (Sync-Reihenfolge) | `sync_from_dom` bedingungslos als erste Anweisung in `switch_preview_mode` | `preview_mode_switch_syncs_dom_before_switching` (Abstand gemessen: 335 Zeichen, Fenster 400) |
| T-28-14 (zweiter `EDITOR_ID`-Knoten) | contenteditable-`div` in keinem `if`-Zweig, `id`/`class` unverändert | `editor_uses_mail_html_render_scope` grün, `grep -c 'const EDITOR_ID: &str = "wysiwyg-editor";'` = 1 |
| T-28-15 (Member-Id im Request) | bestehender `/api/mail/preview`, kein neuer Endpoint, kein neues Feld | keine Änderung an `api.rs` |
| T-28-16 (verschluckte Render-Fehler) | `PreviewResponse.errors` → `preview_errors` → roter Block in `MailPreviewFrame` | Zweig `Ok(resp) if !resp.errors.is_empty()` in `switch_preview_mode` |
| T-28-17 (Request-Sturm) | `preview_needs_fetch` als erste Fetch-Entscheidung, leerer Editor ⇒ kein Request | `preview_needs_fetch_false_between_device_modes` (Plan 28-02) |
| T-28-18 (Modus-Rücksprung) | `accept` — als Kommentar über der `mode`-Deklaration festgehalten | siehe Abschnitt (d) |
| T-28-SC (Package-Legitimacy) | kein Install, kein Manifest im Diff | `git diff --name-only` listet nur die zwei Quelldateien |

## Offene Punkte für die UAT (Plan 28-05)

- Sinnvoll erst **nach** Plan 28-04: bis dahin ist `preview_member_id` überall `None` und der
  Vorschau-Modus zeigt ausschließlich die Hinweiszeile.
- Optische Hervorhebung des aktiven Modus-Buttons (PREV-01).
- Kein Flackern beim Wechsel Desktop ↔ Mobile (Assumption A3: das Dokument-Attribut bleibt
  unangetastet, Dioxus' Attribut-Diff überspringt es).
- Zeilenumbrüche im `text/plain`-Teil nach Vorschau → Senden (die praktische Gegenprobe zu
  T-28-12 im laufenden Browser).
- Zweimaliges Hin- und Herschalten Bearbeiten ↔ Vorschau ohne Inhaltsverlust (T-28-13
  praktisch).

## Self-Check: PASSED

Beide behaupteten Dateien existieren auf der Platte, alle drei Commit-Hashes (`9731f97`,
`a98c910`, `667c3ae`) sind in `git log` auffindbar.
