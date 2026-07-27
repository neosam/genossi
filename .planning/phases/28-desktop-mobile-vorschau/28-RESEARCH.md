# Phase 28: Desktop/Mobile-Vorschau — Research

**Researched:** 2026-07-27
**Domain:** Dioxus-0.6-WASM-Frontend (sandboxed iframe-Preview) + Axum/ammonia-Backend-Seam
**Confidence:** HIGH (alle architektur-kritischen Fragen im Repo bzw. an der Quelle verifiziert)

---

<user_constraints>
## User Constraints (aus 28-CONTEXT.md)

### Locked Decisions

**D-01:** `/api/mail/preview` wird um `sanitize_html` erweitert — KEIN neuer Endpoint. `preview_mail` (`genossi_mail/src/rest.rs:659`) rendert bereits Jinja gegen den vollen Member-/Repayment-Kontext; es fehlt nur der Sanitize-Schritt.

**D-02:** Reihenfolge ist `sanitize_html(body_html)` ZUERST, dann `render_html_template(…, ctx)`. Das spiegelt exakt die Produktion: ammonia greift am Store-Boundary (Phase 23 D-03), das Jinja-Rendering passiert erst beim Versand im Send-Worker. Jinja-Platzhalter in **Text-Content** überleben ammonia intakt; Platzhalter in **Attributen** sind seit Phase 24 explizit *out of contract* — ammonia strippt sie. Die Vorschau macht dieses vorbestehende Verhalten erstmals sichtbar. Das ist erwünscht.

**D-03 (A2 = b):** Die Device-Vorschau rendert Template-Variablen gegen ein Beispiel-Mitglied — nicht roh. `preview_member_id` wird aus `TemplatePreview` in die drei Call-Sites hochgezogen und an **beide** Components gereicht (`WysiwygEditor` + `TemplatePreview`). Fallback: Ist kein Member gewählt (`None`), zeigt der Preview-Modus **nicht** den iframe, sondern eine Hinweiszeile („Mitglied für die Vorschau wählen"). Kein Request, kein leerer Rahmen.
*Planner-Ausstiegsklausel:* Falls sich der Hochzieh-Refactor an einer der drei Call-Sites als unverhältnismäßig erweist (Verdacht: `reply_form.rs` hat ohnehin nur genau einen Member), darf dort stattdessen der einzige/erste Member implizit genutzt werden. Die Entscheidung gilt call-site-weise, nicht global.
*Request-Form:* `subject: ""`, `body: ""`, `body_html: <editor-innerHTML>`, `member_id: <gewählt>`, `repayment_phase_id: <durchgereicht>`.

**D-04 (A3 = a):** Kein Diff-Banner. Wenn ammonia etwas entfernt, zeigt die Vorschau schlicht das Ergebnis.

**D-05 (A4 = a):** Sanitize+Render laufen nur beim Wechsel in einen Preview-Modus. Ein Request pro Umschaltung, kein Debounce-Live-Rendering.

**D-06 (B1 = a):** Das Frontend injiziert `src` — nicht das Backend. Rewrite `data-genossi-asset-id="X"` → zusätzliches `src="{config.backend}/api/mail/assets/X/bytes"`. Spiegelt `image_insert_html()` (`wysiwyg_toolbar.rs:44`).

**D-07 (B2 = b):** `sandbox="allow-same-origin"` — ohne `allow-scripts`. `allow-popups` wurde bewusst NICHT gewählt: Links in der Vorschau sind nicht klickbar (unterstützt PREV-04).

**D-08 (B3 = a):** Bricht ein Bild (404/401), bleibt es beim Browser-Default (kaputtes Bild-Icon). Kein eigener Placeholder.

**D-09 (C1 = a):** Befüllung via `srcdoc`. Der HTML-Inhalt muss fürs Attribut escaped werden.

**D-10 (C2 = c):** Eigenes kleines „Mail-Client-Baseline"-Stylesheet als Frontend-Konstante, in den `srcdoc`-`<head>` injiziert. Richtwert: Arial/Helvetica sans-serif, ~14 px, `img { max-width: 100% }`. Ausdrücklich NICHT `.mail-html-render` duplizieren.

**D-11 (C3 = a):** Grep-Gate-Test auf die `sandbox`-Invariante. `include_str!`-Muster analog `wysiwyg_editor.rs:392` und `template_preview.rs:236` — der Test nagelt fest, dass die Preview-Component ein `sandbox`-Attribut setzt und `allow-scripts` NICHT enthält.

**D-12 (C4 = a):** 640 px / 360 px sind zwei Code-Konstanten. Keine Settings-Konfiguration.

**D-13 (D1 = a):** Der Modus-Umschalter lebt in `WysiwygEditor` selbst. Der iframe selbst wird eine eigene Component `MailPreviewFrame`.

**D-14 (D2 = a):** Im Preview-Modus wird die Toolbar ausgeblendet (nicht ausgegraut).

**D-15 (D3 = a):** Schlichter Device-Rahmen. Rahmen + Label „Desktop-Vorschau (640 px)" / „Mobile-Vorschau (360 px)" über dem iframe, iframe zentriert auf grauem Backdrop. Kein stilisiertes Phone-Mockup.

**D-16 (D4 = a):** Die bestehende `TemplatePreview` bleibt unverändert bestehen.

**D-17 (D5 = a):** Beim Umschalten bleibt das `contenteditable` im DOM und wird nur per CSS versteckt. Kein Unmount, kein Remount, kein Re-Seeding. `EDITOR_ID` ist eine Konstante — ein zweiter Editor-Knoten im DOM würde die `get_element_by_id`-Lookups der Toolbar brechen.

### Claude's Discretion

- Exakte Werte des Baseline-Stylesheets (D-10) — Schriftgröße, Zeilenhöhe, Margins.
- Ob `image_insert_html()` und die Preview-`src`-Injektion (D-06) zu einer gemeinsamen Helper-Funktion extrahiert werden oder als zwei Stellen mit Grep-Gate bestehen bleiben.
- Konkrete Umschalt-UI: Segmented-Control vs. drei Buttons vs. Tabs.
- Call-site-weise Anwendung der Ausstiegsklausel aus D-03 (insbesondere `reply_form.rs`).
- Escaping-Strategie für den `srcdoc`-Attributwert.
- Ob der Sanitize-Aufruf in D-01 unconditional läuft oder nur bei gesetztem `body_html`.
- Genaue i18n-Key-Namen (Konvention: `MailEditorMode*`).

### Deferred Ideas (OUT OF SCOPE)

- Dark-Mode-Vorschau
- Echte Mail-Client-Simulation (Outlook-Quirks, Gmail-CSS-Stripping)
- Tablet-Breakpoint (~768 px)
- Screenshot-/PDF-Export der Vorschau
- Konfigurierbare Breakpoints in den Settings (D-12 dagegen entschieden)
- Sanitize-Diff-Banner (D-04 dagegen entschieden)
- `TemplatePreview` durch den iframe ersetzen (D-16 dagegen entschieden)
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Beschreibung | Research Support |
|----|--------------|------------------|
| PREV-01 | Umschalten zwischen Bearbeiten / Desktop-Vorschau (~640px) / Mobile-Vorschau (~360px) | § Pattern 1 (`PreviewMode`-Enum mit `width_px()`), § Pitfall 1 (Hide-Strategie), § Pattern 5 (Prop-Migration mit `#[props(default)]`) |
| PREV-02 | Vorschau rendert die ammonia-sanitisierte HTML-Fassung | § Pattern 2 (`sanitize_body_html_opt` vor `render_html_template`, exakte Einbaustelle `rest.rs:770`), § Test-Strategie (Backend-Tests), § Pitfall 5 (bestehende Tests) |
| PREV-03 | `data-genossi-asset-id` → `/api/mail/assets/{id}/bytes` | § Pattern 3 (`inject_asset_src` als Spiegel von `rewrite_img_cids`), § Pitfall 3 (UUID-Validierung), § Don't Hand-Roll |
| PREV-04 | Vorschau visuell klar vom Bearbeiten-Modus abgegrenzt | § Pattern 4 (`MailPreviewFrame`-Rahmen), D-07/D-14/D-15; kein `allow-popups`, keine Toolbar |
| PREV-05 | Sandboxed iframe fester Breite, kein CSS-Bleed in beide Richtungen | § Pattern 4 + § CSS-Isolation (Beweisführung + verifizierbarer Testweg), § Pitfall 2 (Höhe) |
</phase_requirements>

---

## Summary

Diese Phase ist **kein Neuland-Problem, sondern ein Verkabelungs-Problem**. Alle vier benötigten Bausteine existieren bereits im Repo und sind produktiv erprobt: (1) der Preview-Endpoint mit vollständiger Jinja-Render-Pipeline inkl. Member-/Repayment-Kontext (`rest.rs:659-807`), (2) der ammonia-Sanitizer mit gehärteter `<img>`-Regel (`sanitize.rs`), (3) der `/api/mail/assets/{id}/bytes`-Endpoint (Phase 27), und (4) das exakte String-Rewrite-Muster für `data-genossi-asset-id` (`render.rs::rewrite_img_cids`). Phase 28 fügt **keine einzige neue Dependency** hinzu — weder Cargo-Crate noch web-sys-Feature.

Die drei technisch riskanten Annahmen der CONTEXT.md wurden verifiziert und halten: `srcdoc` **ist** in `dioxus-html 0.6.3` als Attribut deklariert (`elements.rs:1168`); `sandbox` **ist es nicht** und muss als quoted Custom-Attribute geschrieben werden (`"sandbox": "allow-same-origin"`) — ein Muster, das das Projekt bereits verwendet (`qr_scanner.rs:306`). Und `sandbox="allow-same-origin"` ist tatsächlich zwingend: ohne dieses Token serialisiert der Browser den Origin als `null`, alle Subresource-Requests gelten als cross-site, und das Genossi-Session-Cookie ist **`SameSite=Strict`** (`genossi_rest/src/lib.rs:759`) — die Bilder wären garantiert tot.

Zwei Punkte weichen von den Annahmen der CONTEXT.md ab und müssen in den Plan: **(a) Escaping ist NICHT nötig und wäre ein Bug.** Dioxus setzt Attribute über `node.setAttribute(field, value)` (`dioxus-interpreter-js-0.6.2/src/ts/set_attribute.ts:63`), also als reinen DOM-String ohne HTML-Parsing. Zusätzliches `&quot;`-Escaping würde im iframe sichtbaren Escape-Text erzeugen. **(b) Das Dev-Setup ist NICHT cross-origin.** `assets/config.json` setzt `backend = "http://localhost:8080"` — die Frontend-Origin selbst — und `dx serve` proxied `/api` nach `:3000` (`Dioxus.toml`). Die Bilder laden in Dev also über dieselbe Origin wie im Editor heute schon; D-07 bleibt trotzdem richtig und notwendig (der Sandbox-Origin ist unabhängig vom Deployment-Layout `null`, sobald `allow-same-origin` fehlt).

**Primary recommendation:** Baue die gesamte Preview-Pipeline als **vier pure Funktionen** im Frontend (`asset_bytes_url`, `inject_asset_src`, `preview_srcdoc`, `PreviewMode::width_px`) plus eine dünne RSX-Hülle. `cd genossi-frontend && cargo test` läuft nativ (heute 301 Tests grün, verifiziert) — damit sind Asset-Rewrite, srcdoc-Aufbau und CSS-Isolation *automatisiert* prüfbar, ohne wasm32-Target und ohne Browser-E2E. Der einzige unvermeidbar manuelle Teil ist der visuelle Vorstands-Smoke-Test.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| HTML-Sanitizing (ammonia) | API / Backend | — | Sicherheits-Boundary; ammonia ist server-side only, nie im WASM-Bundle (v1.4/v1.5-Constraint) |
| Jinja-Variablen-Rendering | API / Backend | — | `render_html_template` braucht Member-/Repayment-Kontext aus der DB |
| Plain-Text-Ableitung aus HTML | API / Backend | — | `plain_from_html`, bereits im Preview-Handler (`rest.rs:786`) |
| Asset-`src`-Injektion (`/bytes`-URL) | Browser / Client | — | Nur der Client kennt `config.backend`; das Backend kennt seine browser-sichtbare URL nicht (D-06) |
| iframe-Sandboxing + Device-Breite | Browser / Client | — | Reine Darstellungs-Isolation |
| Baseline-Stylesheet für die Vorschau | Browser / Client | — | Frontend-Konstante (D-10); gehört nicht in die Mail und nicht in die App-CSS |
| Modus-State (Edit/Desktop/Mobile) | Browser / Client | — | Ephemerer UI-State, keine Persistenz |
| Member-Auswahl für die Vorschau | Browser / Client (Page-Ebene) | — | D-03 zieht den State auf die Call-Site hoch, damit nur EINE Auswahl sichtbar ist |
| Bild-Bytes-Auslieferung + AuthZ | API / Backend | — | Phase 27, unverändert (`genossi_rest/src/mail_asset.rs:49`) |

---

## Project Constraints (aus CLAUDE.md)

Diese Direktiven haben dieselbe Autorität wie die Locked Decisions:

| Constraint | Quelle | Konsequenz für Phase 28 |
|---|---|---|
| **Component-First** — keine inline-RSX-Duplikate; identische UI wandert nach `src/component/` | Root-`CLAUDE.md`, `genossi-frontend/CLAUDE.md` | `MailPreviewFrame` wird eigene Component (D-13); die Modus-Buttons ebenfalls kandidatisch (siehe § Pattern 1) |
| **i18n zweisprachig** — jeder Key in `de.rs` UND `en.rs`, `Locale` hat nur `En`/`De` | `genossi-frontend/CLAUDE.md` | Alle `MailEditorMode*`-Keys in beide Dateien im selben Commit (Locale-Drift-Bug ist dokumentiert) |
| **Layered DAO/Service/REST** | Root-`CLAUDE.md` | Backend-Änderung bleibt strikt in der REST-Schicht (`rest.rs`); keine Service-/DAO-Änderung nötig |
| **Audit-Pflicht nur für Member/MemberAction/MemberDocument/Application** | Root-`CLAUDE.md` | Phase 28 schreibt nichts — **kein Audit-Log** |
| **Immer Enum statt Boolean** | Root-`CLAUDE.md` / Projekt-Regel | `PreviewMode`-Enum mit 3 Varianten, nie `preview: bool` + `mobile: bool` |
| **`r#type: "button"` an allen Buttons** | Memory `feedback_dioxus_button_type` | Gilt für die drei Modus-Buttons |
| **Tests für alle Änderungen** | globale User-Instruktion | Siehe § Test-Strategie — jede pure Funktion bekommt Unit-Tests |
| **jj statt git** | 28-CONTEXT `<code_context>` | Commits via `jj`; Achtung Memory `jj-git-index-desync-gsd-executors` (`git read-tree HEAD` vor dem Stagen) |
| **GSD-Workflow-Enforcement** | Root-`CLAUDE.md` | Keine direkten Repo-Edits außerhalb eines GSD-Commands |

---

## Standard Stack

### Core — alles bereits vorhanden, nichts Neues

| Baustein | Version | Zweck | Warum Standard |
|---|---|---|---|
| `dioxus` (`web`, `router`) | 0.6.3 | RSX, Signals, Component-Model | Bereits das einzige Frontend-Framework; `srcdoc` ist als iframe-Attribut deklariert `[VERIFIED: dioxus-html-0.6.3/src/elements.rs:1168]` |
| `web-sys` | 0.3 (bestehende Feature-Liste) | `get_element_by_id`, `inner_html()`, `inner_text()` | Für Phase 28 wird **kein neues Feature** benötigt — siehe § Pitfall 2 zur Höhe |
| `ammonia` | (Backend, Phase 23/27) | HTML-Sanitizing | Bereits der Store-Boundary-Sanitizer; wird nur *aufgerufen* (D-01) |
| `minijinja` | 2.0 (Backend) | `render_html_template` mit Autoescape-Env | Unverändert |
| Tailwind CSS | bestehend | Rahmen/Backdrop/Buttons **außerhalb** des iframes | Innerhalb des iframes bewusst NICHT (D-10) |

### Installation

```bash
# Keine. Phase 28 fügt weder eine Cargo-Dependency noch ein web-sys-Feature hinzu.
```

**Verifikation:** `genossi-frontend/Cargo.toml` `[dependencies.web-sys] features = [...]` enthält bereits `Window`, `Document`, `Element`, `HtmlElement`, `Node`, `EventTarget`. `HtmlIFrameElement` fehlt — wird aber nur für Auto-Höhen-Messung gebraucht, die § Pitfall 2 bewusst vermeidet. `[VERIFIED: genossi-frontend/Cargo.toml:39-79]`

### Alternatives Considered

| Statt | Möglich wäre | Tradeoff |
|---|---|---|
| `srcdoc` | `iframe.contentDocument.write(...)` per web-sys | Braucht neue web-sys-Features, ist imperativ, hat Timing-Fallen bei Remount/Signal-Lag (24-RESEARCH). D-09 hat bewusst dagegen entschieden. |
| Pure-Rust-String-Rewrite für `src`-Injektion | DOM-Parsing per `createElement("div") + query_selector_all` | Braucht `NodeList`+`HtmlImageElement`-Features, ist **nicht nativ unit-testbar**. Das Backend hat für exakt dasselbe Problem den String-Weg gewählt (`render.rs::rewrite_img_cids`) — Symmetrie schlägt Eleganz. |
| Auto-Höhe des iframes | Parent liest `contentDocument.body.scrollHeight` (mit `allow-same-origin` möglich) | Async-Timing (srcdoc parst asynchron), braucht `HtmlIFrameElement`+`load`-Listener. Feste Device-Höhe ist für eine *Device*-Vorschau ohnehin die semantisch richtigere Wahl (§ Pitfall 2). |
| `dangerous_inner_html` in einem `div` | — | Erfüllt PREV-05 nicht (kein CSS-Isolations-Boundary) und ist genau das, was `TemplatePreview` schon tut (D-16). |

---

## Package Legitimacy Audit

**Phase 28 installiert keine externen Pakete.** Es werden ausschließlich bereits im Repo vorhandene, produktiv genutzte Dependencies verwendet (`dioxus 0.6.3`, `web-sys 0.3`, `ammonia`, `minijinja 2.0`) — alle in `Cargo.toml` bzw. `Cargo.lock` festgeschrieben und seit Phase 23–27 im Einsatz.

| Package | Registry | Verdict | Disposition |
|---|---|---|---|
| — | — | — | Keine neuen Pakete |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

---

## Architecture Patterns

### System Architecture Diagram

```
 ┌──────────────────────────── Browser (Dioxus WASM) ────────────────────────────┐
 │                                                                               │
 │  Page (mail_page.rs / mail_templates.rs / reply_form.rs)                      │
 │    │  preview_member_id: Signal<Option<Uuid>>   ← D-03 hochgezogen            │
 │    │  repayment_phase_id: Option<Uuid>                                        │
 │    ├──────────────────────────────┬────────────────────────────────┐          │
 │    ▼                              ▼                                ▼          │
 │  WysiwygEditor                 TemplatePreview                Member-Select    │
 │    │  mode: Signal<PreviewMode>   (unverändert, D-16)          (Page-Ebene)    │
 │    │                                                                          │
 │    ├─ mode == Edit ──▶ Toolbar sichtbar + contenteditable sichtbar            │
 │    │                                                                          │
 │    └─ mode != Edit ──▶ Toolbar ausgeblendet (D-14)                            │
 │            │            contenteditable OFF-SCREEN, NICHT unmounted (D-17)     │
 │            │                                                                  │
 │            │  (1) innerHTML aus #wysiwyg-editor lesen (Submit-Guard-Muster)    │
 │            ▼                                                                  │
 │       preview_html: Signal<Option<String>>                                    │
 │            │                                                                  │
 │            │  (2) POST /api/mail/preview  { subject:"", body:"",             │
 │            │        body_html, member_id, repayment_phase_id }   ← D-05: 1×    │
 │            ▼                                                                  │
 └────────────┼──────────────────────────────────────────────────────────────────┘
              │
 ┌────────────▼──────────── Backend (Axum, genossi_mail) ────────────────────────┐
 │  preview_mail (rest.rs:659)                                                   │
 │    resolve_member → base_ctx → (repayment merge / dummy fallback)              │
 │    ┌──────────────────────────────────────────────────────────┐               │
 │    │ NEU (D-01/D-02):                                         │               │
 │    │   sanitize_body_html_opt(body.body_html)   ← ammonia     │               │
 │    │              ↓                                            │               │
 │    │   render_html_template(sanitized, ctx)     ← minijinja   │               │
 │    └──────────────────────────────────────────────────────────┘               │
 │    plain_from_html(rendered) → response.body                                  │
 │    → PreviewResponse { subject, body, body_html, errors, used_dummy_… }        │
 └────────────┬──────────────────────────────────────────────────────────────────┘
              │  (3) 200 JSON
 ┌────────────▼──────────── Browser (Dioxus WASM) ───────────────────────────────┐
 │  (4) inject_asset_src(body_html, config.backend)    ← D-06, pure fn           │
 │        <img data-genossi-asset-id="U">                                        │
 │          → <img data-genossi-asset-id="U" src="{backend}/api/mail/…/bytes">   │
 │  (5) preview_srcdoc(html) → "<!DOCTYPE html>…<style>BASELINE</style>…"  ← D-10 │
 │  (6) MailPreviewFrame { mode, srcdoc }                                        │
 │        <div Rahmen+Label>  ← D-15                                             │
 │          <iframe "sandbox"="allow-same-origin" srcdoc=… width=640|360 />      │
 │                    │                                                          │
 └────────────────────┼──────────────────────────────────────────────────────────┘
                      │  (7) GET /api/mail/assets/{id}/bytes  (Cookie geht mit,
                      ▼      weil allow-same-origin gesetzt ist)
                Backend mail_asset::download_mail_asset_bytes
```

### Recommended Project Structure

```
genossi-frontend/src/component/mail_compose/
├── mail_preview_frame.rs     # NEU — PreviewMode, MailPreviewFrame, preview_srcdoc,
│                             #        MAIL_PREVIEW_BASELINE_CSS + Grep-Gate
├── wysiwyg_editor.rs         # ERWEITERT — mode-Signal, Umschalt-UI, Off-Screen-Hide,
│                             #             neue Props (D-03), Fetch-on-Switch
├── wysiwyg_toolbar.rs        # ERWEITERT — asset_bytes_url() extrahiert (D-06 Discretion)
├── template_preview.rs       # ERWEITERT — preview_member_id wird Prop statt use_signal
├── template_tester.rs        # ERWEITERT — reicht seine selected_member_id durch
└── mod.rs                    # ERWEITERT — pub use mail_preview_frame::MailPreviewFrame

genossi_mail/src/rest.rs      # ERWEITERT — 1 Zeile Sanitize im preview_mail-Handler
genossi-frontend/src/i18n/{mod,de,en}.rs   # ERWEITERT — MailEditorMode*-Keys
```

---

### Pattern 1: `PreviewMode` als Enum mit reiner Geometrie-Funktion

**Was:** Drei-wertiges Enum statt Boolean-Paar (Projekt-Regel „Immer Enum statt Boolean"). Breite und Label-Key hängen als Methoden dran — damit sind D-12 (Code-Konstanten) und PREV-01 in einer nativ testbaren Einheit gebündelt.

**Wann:** Immer. Der Modus lebt als `use_signal(|| PreviewMode::Edit)` **innerhalb** `WysiwygEditor` (D-13).

```rust
// genossi-frontend/src/component/mail_compose/mail_preview_frame.rs
// Phase 28 (PREV-01, D-12): Breiten sind Code-Konstanten, keine Settings.

/// Desktop-Vorschau-Breite in CSS-px (Roadmap: „~640 px").
pub const PREVIEW_WIDTH_DESKTOP_PX: u32 = 640;
/// Mobile-Vorschau-Breite in CSS-px (Roadmap: „~360 px").
pub const PREVIEW_WIDTH_MOBILE_PX: u32 = 360;
/// Feste Viewport-Höhe des iframes — siehe Pitfall 2 (keine Auto-Messung).
pub const PREVIEW_HEIGHT_PX: u32 = 640;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PreviewMode {
    Edit,
    Desktop,
    Mobile,
}

impl PreviewMode {
    /// `None` für `Edit` — es gibt keinen iframe im Bearbeiten-Modus.
    pub fn width_px(self) -> Option<u32> {
        match self {
            PreviewMode::Edit => None,
            PreviewMode::Desktop => Some(PREVIEW_WIDTH_DESKTOP_PX),
            PreviewMode::Mobile => Some(PREVIEW_WIDTH_MOBILE_PX),
        }
    }

    pub fn is_preview(self) -> bool {
        !matches!(self, PreviewMode::Edit)
    }
}
```

**Anmerkung zur Discretion „Umschalt-UI":** Empfehlung ist ein **Segmented-Control aus drei `button`-Elementen** in einem `div.inline-flex.rounded.border` — die aktive Variante bekommt `bg-blue-500 text-white`, die anderen `bg-white text-gray-700`. Begründung: Tabs implizieren wechselnde Inhalts-*Bereiche* (semantisch falsch, es ist derselbe Inhalt), ein Dropdown versteckt den aktuellen Zustand. Das Segmented-Control zeigt permanent alle drei Zustände und welcher aktiv ist — genau das, was PREV-04 („Vorstand versteht sofort") verlangt. Alle drei Buttons brauchen `r#type: "button"` und `evt.prevent_default()` als erste Zeile.

---

### Pattern 2: Sanitize-vor-Render im Preview-Handler (D-01/D-02)

**Was:** Genau ein zusätzlicher Funktionsaufruf im bestehenden `preview_mail`-Handler. Die exakte Einbaustelle ist `genossi_mail/src/rest.rs:766-779` (der `rendered_body_html`-Block).

**Wichtig:** Der Kommentar in Zeile 768-769 (`// Read-only preview — no sanitization here; ammonia guards the store boundary`) wird durch D-02 **falsch** und MUSS mit ersetzt werden — sonst dokumentiert der Code das Gegenteil dessen, was er tut.

**Discretion-Entscheidung (unconditional vs. conditional):** Es existiert bereits ein `pub(crate)`-Helper mit genau der richtigen Semantik: `crate::service::sanitize_body_html_opt(Option<&str>) -> Option<String>` (`genossi_mail/src/service.rs:287`, Doc-Kommentar: „`None` in ⇒ `None` out"). Ihn zu benutzen macht die Frage gegenstandslos und hält die Symmetrie zu den vier bestehenden D-03-Entry-Points. **Empfehlung: diesen Helper verwenden, keine neue Verzweigung schreiben.** `[VERIFIED: genossi_mail/src/service.rs:280-289]`

```rust
// genossi_mail/src/rest.rs — ersetzt den Block ab Zeile 766
// Phase 24 (EDIT-05, D-04): if the caller supplied an HTML sibling, render it
// through the autoescape env (member values escaped, author markup preserved).
//
// Phase 28 (PREV-02, D-01/D-02): SANITIZE ZUERST, dann rendern. Reihenfolge
// spiegelt die Produktion: ammonia greift am Store-Boundary (Phase 23 D-03),
// das Jinja-Rendering passiert erst im Send-Worker. Damit zeigt die Vorschau
// exakt das, was der Empfänger bekommt — inklusive der Attribute, die ammonia
// entfernt. Jinja-Platzhalter in Text-Content überleben ammonia (sanitize.rs:30-34);
// Platzhalter in Attributen sind seit Phase 24 out-of-contract und werden
// hier erstmals sichtbar gestrippt — das ist gewollt (D-04: kein Diff-Banner).
let sanitized_body_html = crate::service::sanitize_body_html_opt(body.body_html.as_deref());
let rendered_body_html: Option<String> = match sanitized_body_html.as_deref() {
    Some(html_src) => match render_html_template(html_src, &ctx) {
        Ok(s) => Some(s),
        Err(e) => {
            errors.push(format!("HTML: {}", e.message));
            None
        }
    },
    None => None,
};
```

**Nebeneffekt (erwünscht, aus D-01):** Die bestehende `TemplatePreview` zeigt ab jetzt ebenfalls die sanitisierte Fassung. Das ist die Behebung eines vorbestehenden Defekts gegenüber dem Geist von PREV-02, kein Kollateralschaden.

---

### Pattern 3: `src`-Injektion als pure Funktion, gespiegelt aus dem Backend (D-06)

**Was:** Das Backend löst *dasselbe* Problem in `genossi_mail/src/render.rs:293-345` (`rewrite_img_cids` + `extract_asset_id`): es scannt nach `<img`, isoliert den Tag bis zum nächsten `>`, liest `data-genossi-asset-id` und ersetzt. Das Frontend braucht die Spiegelvariante — **behalten statt ersetzen** (`data-genossi-asset-id` bleibt stehen, `src` kommt dazu).

**Warum String-Scan und nicht DOM-Parsing:** Der Input ist ammonia-Output. Auf `<img>` überlebt laut `sanitize.rs:47-53` **ausschließlich** `data-genossi-asset-id` (`src`, `srcset`, `alt`, `width`, `height`, `loading` werden explizit entfernt). Die Tag-Form ist damit vollständig vorhersagbar. Ein gezielter, begrenzter Attribut-Rewrite ist sicher — genau die Begründung, die 27-RESEARCH für den Backend-Pfad festgehalten hat.

**Discretion-Entscheidung (gemeinsamer Helper ja/nein): JA, aber nur für die URL.** `image_insert_html()` und die Preview-Injektion erzeugen unterschiedliche *Markup*-Formen (kompletter Tag vs. Attribut-Einschub), aber dieselbe *URL*. Extrahiere nur die URL — das ist der Teil, der bei einer Route-Änderung stillschweigend auseinanderlaufen würde, und die Extraktion ist non-invasiv für `image_insert_html`.

```rust
// genossi-frontend/src/component/mail_compose/wysiwyg_toolbar.rs
/// Phase 28 (PREV-03, D-06): Single Source of Truth für die browser-sichtbare
/// Asset-Bytes-URL. Benutzt von `image_insert_html()` (Editor-Insert) UND von
/// `mail_preview_frame::inject_asset_src()` (iframe-Vorschau). Eine
/// Route-Änderung darf nicht an zwei Stellen gepflegt werden müssen.
pub(crate) fn asset_bytes_url(backend: &str, id: &str) -> String {
    format!("{backend}/api/mail/assets/{id}/bytes")
}

pub(crate) fn image_insert_html(backend: &str, id: &str) -> String {
    format!(
        r#"<img data-genossi-asset-id="{id}" src="{src}">"#,
        src = asset_bytes_url(backend, id)
    )
}
```

```rust
// genossi-frontend/src/component/mail_compose/mail_preview_frame.rs
use uuid::Uuid;
use crate::component::mail_compose::wysiwyg_toolbar::asset_bytes_url;

/// Phase 28 (PREV-03, D-06) — reine Funktion: fügt jedem
/// `<img data-genossi-asset-id="{uuid}">` ein `src="{backend}/api/…/bytes"`
/// hinzu. Spiegel von `genossi_mail::render::rewrite_img_cids`, nur dass das
/// Asset-Attribut erhalten bleibt statt ersetzt zu werden.
///
/// SICHERHEIT (Pitfall 3): der Attributwert wird als UUID geparst, bevor er in
/// die URL interpoliert wird. Ein nicht-UUID-Wert lässt den Tag unverändert —
/// damit ist eine Attribut-Injektion über einen präparierten Asset-Id-Wert
/// strukturell ausgeschlossen, unabhängig von der iframe-Sandbox.
pub(crate) fn inject_asset_src(html: &str, backend: &str) -> String {
    const ATTR: &str = "data-genossi-asset-id";
    let mut out = String::with_capacity(html.len() + 64);
    let mut rest = html;

    while let Some(start) = rest.find("<img") {
        out.push_str(&rest[..start]);
        let tail = &rest[start..];
        let Some(end) = tail.find('>') else {
            out.push_str(tail);
            return out;
        };
        let tag = &tail[..=end];

        match extract_asset_uuid(tag, ATTR) {
            // Insert vor dem schließenden '>' — Tag-Inhalt bleibt sonst 1:1.
            Some(id) => {
                out.push_str(&tag[..tag.len() - 1]);
                out.push_str(&format!(r#" src="{}">"#, asset_bytes_url(backend, &id.to_string())));
            }
            None => out.push_str(tag),
        }
        rest = &tail[end + 1..];
    }
    out.push_str(rest);
    out
}

fn extract_asset_uuid(tag: &str, attr: &str) -> Option<Uuid> {
    let after = &tag[tag.find(attr)? + attr.len()..];
    let after = after.trim_start();
    let after = after.strip_prefix('=')?.trim_start();
    let (quote, after) = match after.chars().next()? {
        q @ ('"' | '\'') => (q, &after[1..]),
        _ => return None,
    };
    let value = &after[..after.find(quote)?];
    Uuid::parse_str(value).ok()
}
```

---

### Pattern 4: `srcdoc` deklarativ setzen — ohne Escaping (D-09, Discretion aufgelöst)

**Was:** Der `srcdoc`-Wert wird als **roher** HTML-String gesetzt. Kein `&quot;`, kein `&amp;`.

**Beleg:** Dioxus' Web-Interpreter routet jedes nicht-spezialbehandelte Attribut in den `default`-Zweig von `setAttributeInner` und ruft dort `node.setAttribute(field, value)` auf `[VERIFIED: dioxus-interpreter-js-0.6.2/src/ts/set_attribute.ts:58-64]`. `setAttribute` nimmt einen DOM-String entgegen; es findet **kein HTML-Quelltext-Parsing** statt. Die von MDN dokumentierten srcdoc-Escaping-Regeln gelten ausschließlich für den Fall, dass das Attribut im HTML-Quelltext geschrieben wird `[CITED: developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/iframe]`. Zusätzliches Escaping würde im iframe sichtbares `&lt;p&gt;` erzeugen — es wäre ein Bug, keine Härtung.

**Zweiter Beleg für die Anwendbarkeit:** Die App läuft rein clientseitig (`Dioxus.toml: default_platform = "web"`, kein Fullstack-/SSR-Feature in `Cargo.toml`). Es gibt also keinen Pfad, auf dem `srcdoc` jemals als HTML-Quelltext serialisiert würde.

**`sandbox` braucht Quoted-Syntax:** In `dioxus-html 0.6.3` ist `sandbox` beim `iframe`-Element **auskommentiert** (`// sandbox: SpacedSet<Sandbox>,`, `elements.rs:1178`). Ein unquoted `sandbox: "…"` kompiliert nicht. Das Projekt nutzt die Quoted-Custom-Attribute-Syntax bereits (`qr_scanner.rs:306: "aria-label": "Schließen"`), also: `"sandbox": "allow-same-origin"`. `[VERIFIED: Repo-Grep + dioxus-html-0.6.3/src/elements.rs:1160-1179]`

```rust
// genossi-frontend/src/component/mail_compose/mail_preview_frame.rs

/// Phase 28 (PREV-05, D-10) — „Mail-Client-Baseline". BEWUSST NICHT identisch
/// mit `.mail-html-render` (input.css:12-35): sähe die Vorschau exakt wie der
/// Editor aus, wäre der Sinn der Phase (Diskrepanzen sichtbar machen, PREV-02)
/// unterlaufen. Nackte Browser-Defaults wären andererseits Times New Roman
/// 16 px — das zeigt kein realer Mail-Client. Werte orientieren sich an dem,
/// was Thunderbird/Outlook/Gmail für HTML-Mails ohne eigene Styles rendern.
const MAIL_PREVIEW_BASELINE_CSS: &str = "\
html,body{margin:0;padding:0}\
body{font-family:Arial,Helvetica,sans-serif;font-size:14px;line-height:1.45;color:#222;padding:12px;word-wrap:break-word}\
p{margin:0 0 1em}\
h1{font-size:22px;margin:.5em 0}h2{font-size:18px;margin:.6em 0}h3{font-size:16px;margin:.7em 0}\
ul,ol{margin:0 0 1em;padding-left:24px}li{margin:.15em 0}\
blockquote{margin:0 0 1em;padding-left:12px;border-left:3px solid #ccc;color:#555}\
a{color:#1155cc}\
img{max-width:100%;height:auto}\
table{border-collapse:collapse}";

/// Baut das vollständige, in sich geschlossene Vorschau-Dokument.
///
/// PREV-05-Invariante: das Ergebnis referenziert KEIN externes Stylesheet
/// (kein `<link>`, kein `tailwind.css`, keine `mail-html-render`-Klasse).
/// Damit ist die CSS-Isolation nicht nur durch den iframe-Browsing-Context
/// gegeben, sondern auch am String selbst überprüfbar (siehe Tests).
pub(crate) fn preview_srcdoc(body_html: &str) -> String {
    format!(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\">\
         <style>{css}</style></head><body>{body}</body></html>",
        css = MAIL_PREVIEW_BASELINE_CSS,
        body = body_html
    )
}

#[component]
pub fn MailPreviewFrame(mode: PreviewMode, srcdoc: String) -> Element {
    let i18n = use_i18n();
    let Some(width) = mode.width_px() else {
        return rsx! {};
    };
    let label = match mode {
        PreviewMode::Desktop => i18n.t(Key::MailEditorModeDesktopFrameLabel),
        _ => i18n.t(Key::MailEditorModeMobileFrameLabel),
    };

    rsx! {
        // D-15: grauer Backdrop, zentrierter Device-Rahmen, Label darüber.
        div { class: "bg-gray-200 p-4 flex flex-col items-center",
            p { class: "text-xs font-medium text-gray-600 mb-2", {label} }
            div {
                class: "border-4 border-gray-500 rounded-lg overflow-hidden bg-white shadow-lg",
                iframe {
                    // D-07 / PREV-05: allow-same-origin OHNE allow-scripts.
                    // NIEMALS allow-scripts ergänzen — die Kombination erlaubt
                    // dem Dokument, sich selbst aus der Sandbox zu nehmen.
                    // `sandbox` ist in dioxus-html 0.6.3 auskommentiert, daher
                    // Quoted-Custom-Attribute-Syntax.
                    "sandbox": "allow-same-origin",
                    // D-09: roher HTML-String. setAttribute() parst NICHT als
                    // HTML-Quelltext — Escaping wäre ein Bug (doppelt escaped).
                    srcdoc: "{srcdoc}",
                    width: "{width}",
                    height: "{PREVIEW_HEIGHT_PX}",
                    // Sichtbar machen, dass der Rahmen der Device-Rahmen ist.
                    style: "display:block;border:0;background:#fff",
                    title: "{label}",
                }
            }
        }
    }
}
```

---

### Pattern 5: Rückwärtskompatible Prop-Migration mit `#[props(default)]` (D-03)

**Was:** `WysiwygEditor` bekommt zwei neue Props. Beide `#[props(default)]`, exakt wie Phase 24 es bei `TemplatePreview.body_html` / `TemplateTester.body_html` gemacht hat (`template_preview.rs:75`, `template_tester.rs:53`). Damit kompiliert jede Call-Site einzeln umstellbar — kein Big-Bang-Commit.

```rust
#[component]
pub fn WysiwygEditor(
    value: String,
    on_change: EventHandler<(String, String)>,
    /// Phase 28 (PREV-02, D-03): Member, gegen den die Device-Vorschau die
    /// Template-Variablen rendert. `None` ⇒ Preview-Modus zeigt statt des
    /// iframes eine Hinweiszeile, kein Request.
    #[props(default)] preview_member_id: Option<Uuid>,
    /// Phase 28 (D-03): Repayment-Kontext, 1:1 durchgereicht an
    /// `/api/mail/preview` — damit `{{ payout_amount }}` etc. auch in der
    /// Device-Vorschau auflösen (gleiche Semantik wie TemplatePreview).
    #[props(default)] repayment_phase_id: Option<Uuid>,
) -> Element
```

**Warum `Option<Uuid>` und nicht `ReadOnlySignal<Option<Uuid>>`:** Ein Plain-Value-Prop löst bei Änderung ein Re-Render von `WysiwygEditor` aus — aber **keinen Remount**, solange der `key` gleich bleibt. Der `contenteditable`-`div` hat in RSX keine Kinder; Dioxus diffed „keine Kinder vorher, keine Kinder nachher" und rührt `innerHTML` nicht an. Der getippte Inhalt überlebt. Dieses Verhalten ist bereits heute produktiv bewiesen: jeder `oninput` schreibt in Parent-Signale, der Parent re-rendert, und der Editor-Inhalt bleibt stehen.

**Aufwands-Einschätzung des D-03-Refactors (verifiziert am Code):**

| Call-Site | Ist-Zustand | Aufwand | Empfehlung |
|---|---|---|---|
| `mail_page.rs:401ff` | `TemplatePreview { member_ids: selected_member_ids.read().clone(), repayment_phase_id }`; `WysiwygEditor` direkt darüber | **Gering.** Ein `use_signal(|| None::<Uuid>)` in der Page, als Prop an beide. `TemplatePreview` bekommt `preview_member_id` als Prop statt `use_signal`. | Voll hochziehen |
| `mail_templates.rs:333ff` | Nutzt **`TemplateTester`**, nicht `TemplatePreview` direkt. `TemplateTester` hat bereits ein eigenes `selected_member_id` (`use_signal`, gespeist von `MemberSearch`, `template_tester.rs:56`) und rendert `TemplatePreview` mit `member_ids: vec![mid]` | **Mittel** (eine Ebene mehr). **Aber der Gewinn ist hier am größten:** es existieren heute *zwei verschachtelte Member-Auswahlen* auf derselben Seite (MemberSearch in TemplateTester + `<select>` in TemplatePreview) — genau das Problem, das D-03 beheben will | Voll hochziehen; `TemplateTester.selected_member_id` wird zum Prop oder zusätzlich per `on_member_change`-EventHandler nach oben gemeldet |
| `reply_form.rs:239ff` | `TemplatePreview` wird nur gerendert `if assigned_member_id.is_some()`, mit `member_ids: vec![member_uuid_opt]` — also **genau ein Member, bereits bekannt** | **Ausstiegsklausel greift.** Es gibt keine Auswahl, die konkurrieren könnte | `preview_member_id: member_uuid_opt` direkt durchreichen, `TemplatePreview` dort unverändert lassen |

---

### Anti-Patterns to Avoid

- **`display:none` / Tailwind `hidden` auf dem contenteditable** — siehe Pitfall 1. Zerstört stillschweigend die Zeilenumbrüche im Plain-Text-Body.
- **`allow-scripts` zur Sandbox hinzufügen** — MDN: „strongly discouraged […] lets the embedded document remove the `sandbox` attribute". Der Grep-Gate aus D-11 existiert genau dafür.
- **Escaping des `srcdoc`-Werts** — erzeugt sichtbaren Escape-Text. Siehe Pattern 4.
- **Einen zweiten `contenteditable`-Knoten mit `EDITOR_ID` rendern** — `EDITOR_ID` ist eine Konstante; `get_element_by_id` liefert den ersten Treffer, und alle Toolbar-Kommandos + Submit-Guards greifen dann ins falsche Element (D-17).
- **`.mail-html-render` in das Baseline-CSS kopieren** — unterläuft den Phasenzweck (D-10).
- **Den Asset-Id-Wert ungeprüft in die URL interpolieren** — siehe Pitfall 3.
- **Den `key`-Bump-Remount als Modus-Wechsel-Mechanismus nutzen** — D-17 verbietet das explizit; der Seed-Lag-Pitfall aus Phase 24 ist dokumentiert.

---

## Don't Hand-Roll

| Problem | Nicht selbst bauen | Stattdessen | Warum |
|---|---|---|---|
| HTML-Sanitizing | Eigene Tag-/Attribut-Allowlist | `crate::sanitize::sanitize_html` (ammonia, Phase 23/27) | Sicherheits-Boundary mit 12 Regressionstests; jede Zweitimplementierung driftet |
| `Option`-Sanitize-Wrapper | Eigenes `if let Some(...)` im Handler | `crate::service::sanitize_body_html_opt` (`service.rs:287`) | Existiert, ist `pub(crate)`, dokumentiert „None in ⇒ None out", spiegelt die vier bestehenden Entry-Points |
| HTML→Plain-Text für die Vorschau | Eigene Tag-Strip-Schleife | `crate::render::plain_from_html` — läuft bereits (`rest.rs:786`) | Quick-260718 hat das gezielt eingebaut, damit Vorschau und Send-Worker identisch ableiten |
| CSS-Isolation zwischen Editor und Vorschau | Shadow-DOM, CSS-Namespacing, `all: initial`-Reset | `<iframe>` mit self-contained `srcdoc` | Der iframe erzeugt einen eigenen Browsing-Context mit eigenem Stylesheet-Set — die Isolation ist eine Browser-Garantie, kein Selbstbau |
| Jinja-Rendering im Frontend | Variablen im WASM ersetzen | `POST /api/mail/preview` (bestehend) | minijinja läuft server-side; Member-/Repayment-Kontext liegt in der DB |
| Asset-URL-Bau | Zweite `format!`-Stelle | `asset_bytes_url()` (Pattern 3) | Eine Route-Änderung darf nicht an zwei Stellen gepflegt werden müssen |
| UUID-Validierung des Asset-Attributs | Regex / Zeichen-Whitelist | `Uuid::parse_str(...).ok()` | Exakt das Muster aus `render.rs::extract_asset_id`; strukturell injektionsfrei |

**Key insight:** Phase 28 hat null echte Neuentwicklung im Problemraum „HTML sicher darstellen". Jede Zeile, die versucht, Sanitizing/Escaping/Isolation selbst nachzubauen, ist eine Regression gegenüber dem, was Phase 23 und 27 bereits abgesichert haben. Der einzige *neue* Code ist Geometrie (Breiten, Rahmen) und Verkabelung.

---

## Common Pitfalls

### Pitfall 1: `display:none` auf dem contenteditable zerstört `innerText` — der Plain-Text-Body wird zur Wall-of-Text

**Was schiefgeht:** D-17 sagt „nur per CSS versteckt". Der naheliegende Weg ist Tailwind `hidden` (= `display:none`). Dann liefert `HtmlElement::inner_text()` aber nicht mehr den layout-bewussten Text mit Zeilenumbrüchen, sondern denselben Wert wie `textContent` — also **ohne** die Umbrüche aus `<p>`, `<li>`, `<br>`.

**Warum das hier beißt:** `sync_from_dom` (`wysiwyg_editor.rs:159-173`) benutzt `inner_text()` **explizit** „so intentional line breaks survive" (Phase 24 D-02). Zusätzlich lesen **alle drei Call-Sites** in ihrem Submit-Guard direkt `#wysiwyg-editor` per `inner_text()` unmittelbar vor dem Absenden (z. B. `reply_form.rs:281-292`). Schaltet der Vorstand in die Vorschau und klickt dann Senden, überschreibt der Submit-Guard den Plain-Body mit der umbruchlosen Variante — die HTML-Mail ist korrekt, der `text/plain`-Teil ist eine einzige Zeile.

**Beleg:** „If the element itself is not being rendered […] the returned value is the same as `Node.textContent`." `[CITED: developer.mozilla.org/en-US/docs/Web/API/HTMLElement/innerText]`

**Wie vermeiden — zwei Schichten:**
1. **Off-Screen statt `display:none`.** Der Container des contenteditable bekommt im Preview-Modus `class: "absolute -left-[10000px] top-0 w-[640px]"` (oder inline `style: "position:absolute;left:-10000px;top:0;width:640px"`). Das Element bleibt *rendered*, `inner_text()` verhält sich unverändert, und es ist visuell vollständig weg. Der Elternknoten braucht `relative` **nicht** — `absolute` gegen den nächsten positionierten Vorfahren oder den Viewport ist beides unschädlich, weil das Element off-screen liegt.
2. **Sync vor dem Wechsel.** Der Modus-Umschalt-Handler ruft **zuerst** `sync_from_dom(&on_change)` (Signale sind damit garantiert aktuell), **dann** liest er `inner_html()` für den Preview-Request, **dann** setzt er den Modus.

**Warnzeichen:** Test-Mail nach einem Vorschau-Wechsel kommt als eine lange Zeile Fließtext an; `preview.body` im TemplatePreview-Block zeigt keine Absätze mehr.

---

### Pitfall 2: Der iframe ist standardmäßig 150 px hoch — und ohne `allow-scripts` kann er sich nicht selbst messen

**Was schiefgeht:** Ein `<iframe>` ohne `height` ist per CSS-Default 150 px hoch. Eine dreiseitige Mail zeigt dann einen winzigen Scroll-Ausschnitt, und der erste UAT-Eindruck ist „kaputt".

**Warum das hier besonders greift:** Die übliche Lösung — ein Script im iframe postMessage't seine Höhe — ist durch D-07 (kein `allow-scripts`) ausgeschlossen. Die Alternative, dass der Parent `iframe.contentDocument.body.scrollHeight` liest, ist mit `allow-same-origin` **technisch möglich** (MDN: „a same-origin parent document can still access and interact with the iframe's DOM even if `allow-scripts` is not set"), kostet aber: neues web-sys-Feature `HtmlIFrameElement`, einen nativen `load`-Listener (`srcdoc` parst asynchron), und einen Re-Measure bei jedem `srcdoc`-Update.

**Wie vermeiden:** **Feste Höhen-Konstante** (`PREVIEW_HEIGHT_PX = 640`) mit iframe-internem Scrolling. Für eine *Device*-Vorschau ist ein fester Viewport ohnehin die semantisch richtigere Simulation als „so hoch wie der Inhalt" — echte Mail-Clients haben auch einen festen Viewport. Kein neues Feature, kein Async-Timing, keine Remount-Falle.

**Wenn im UAT doch Auto-Höhe gewünscht wird:** als eigener Quick-Task nachrüsten (`HtmlIFrameElement`-Feature + `load`-Listener nach dem Muster aus `attach_image_drop_target`, `wysiwyg_editor.rs:184-243`) — nicht in Phase 28.

---

### Pitfall 3: Attribut-Injektion über einen präparierten Asset-Id-Wert

**Was schiefgeht:** `format!(r#" src="{backend}/api/mail/assets/{id}/bytes""#)` mit einem `id`, der ein `"` enthält, bricht aus dem Attribut aus und kann z. B. `onerror=` anhängen.

**Warum das trotz zweier Schutzschichten adressiert werden muss:** ammonia escaped `"` in Attributwerten bei der Serialisierung, und ohne `allow-scripts` würde `onerror` nicht feuern. Aber beides sind *Umgebungs*-Annahmen; wenn jemand später `allow-scripts` ergänzt oder der Sanitize-Schritt umgangen wird, ist die Injektion sofort scharf.

**Wie vermeiden:** `Uuid::parse_str(value).ok()` vor der Interpolation — genau das Muster aus `render.rs::extract_asset_id` („Malformed IDs (keine gültige UUID) werden unangetastet gelassen"). Ein Unit-Test mit `data-genossi-asset-id="a&quot; onerror=&quot;alert(1)"` gehört in die Suite.

**Warnzeichen:** Ein Test, der eine Nicht-UUID durchlässt und trotzdem `src=` einfügt.

---

### Pitfall 4: `srcdoc` in Dev funktioniert, in Prod brechen die Bilder — oder umgekehrt

**Was schiefgeht:** Es ist verlockend, für die Bilder relative URLs (`/api/mail/assets/…`) zu benutzen, weil das im `about:srcdoc`-Dokument gegen die Parent-Base-URL auflöst und in Dev funktioniert.

**Warum das bricht:** `config.backend` ist *nicht* immer die Page-Origin. In Dev ist es `http://localhost:8080` (= Frontend-Origin, `dx serve` proxied `/api` nach `:3000`), aber quick-260724 hat `image_insert_html` genau deshalb auf `config.backend` umgestellt: auf Deployments (z. B. beta) trägt `config.backend` bereits ein `/api`-Segment, das ein Proxy konsumiert. Eine relative URL würde `config.backend` umgehen und 404en. `[VERIFIED: genossi-frontend/assets/config.json, Dioxus.toml [[web.proxy]], wysiwyg_toolbar.rs:33-43]`

**Wie vermeiden:** Immer `asset_bytes_url(&config.backend, id)` — nie eine relative URL. Der Grep-Gate kann das mit absichern.

**Warnzeichen:** Bilder erscheinen in Dev, aber nicht auf beta.

---

### Pitfall 5: Der Sanitize-Schritt lässt einen bereits roten Test noch röter aussehen

**Status verifiziert (2026-07-27, dieser Research-Lauf):**

```
cargo test -p genossi_bin --test e2e_tests test_mail_preview
  test_mail_preview_repayment_share_count_aggregates_real_value ....... ok
  test_mail_preview_repayment_no_entries_does_not_default_to_one ...... FAILED
    panicked at e2e_tests.rs:14628: "errors must be array"
```

**Analyse:** Der Test bricht an `json["errors"].as_array()` — der Key fehlt auf der Wire, weil `errors` leer ist (`#[serde(skip_serializing_if = "Vec::is_empty")]`). Der Render war also *erfolgreich*, d. h. der Dummy-Repayment-Fallback aus Quick-260603-kon (`rest.rs:704-719`) hat gegriffen und die c19-D-05-Symmetrie überschrieben. Das ist ein **Konflikt zwischen zwei Quick-Tasks**, dokumentiert in STATE.md, und existiert seit Phase 22.

**Warum Phase 28 ihn nicht berührt:** Der Testrequest enthält **kein** `body_html`. `sanitize_body_html_opt(None)` gibt `None` zurück; der neue Codepfad wird nicht einmal betreten. `[VERIFIED: e2e_tests.rs:14604-14612 + service.rs:287-289]`

**Für den Verifier festhalten:** (a) Der Test ist ein Pre-existing Failure und darf Phase 28 **nicht** zugeschrieben werden. (b) Die Fehlermeldung muss nach Phase 28 unverändert `"errors must be array"` an Zeile 14628 lauten — ändert sich die Zeile oder die Meldung, hat Phase 28 den Pfad doch berührt und es ist eine echte Regression. Diese Assertion gehört als expliziter Prüfschritt in die Verifikation.

---

### Pitfall 6: Der Modus-State überlebt den `key`-Bump-Remount nicht

**Was schiefgeht:** Alle drei Call-Sites bumpen bewusst einen `key` auf `WysiwygEditor`, um beim Template-Wechsel einen Remount und damit ein Re-Seeding zu erzwingen (`mail_page.rs:422-436`, `mail_templates.rs:327-343`, `reply_form.rs:234-247`). Da `mode` ein `use_signal` **innerhalb** der Component ist (D-13), wird es bei jedem solchen Remount auf `Edit` zurückgesetzt.

**Bewertung:** Das ist **akzeptables, sogar erwünschtes** Verhalten — nach einem Template-Wechsel ist der Vorschau-Inhalt ohnehin veraltet, und ein Rücksprung in den Bearbeiten-Modus ist die richtige Reaktion. **Aber es muss bewusst entschieden und im Plan festgehalten sein**, sonst wird es später als Bug gemeldet („Vorschau springt beim Template-Wechsel zurück").

**Wie vermeiden (falls doch unerwünscht):** Modus-State in die Page hochziehen. Nicht empfohlen — widerspricht D-13 und macht drei Call-Sites komplizierter für einen Randfall.

---

### Pitfall 7: Leere Pflichtfelder im Preview-Request

**Was schiefgeht:** D-03 schickt `subject: ""` und `body: ""`. Beides sind Pflichtfelder von `PreviewRequest` (`rest.rs:258-262`, kein `Option`). Der Handler rendert sie durch `render_template` — mit leerem Template gibt minijinja `Ok("")` zurück, kein Fehler. `errors` bleibt leer. Und `rendered_body` wird ohnehin durch `plain_from_html(html)` überschrieben, sobald `body_html` gesetzt ist (`rest.rs:786-789`).

**Zusätzlicher Effekt, der bekannt sein muss:** `template_uses_repayment_vars(&body.subject, &body.body)` (`rest.rs:730`) prüft **nur** Subject und Plain-Body — nicht `body_html`. Enthält also **nur** das HTML `{{ payout_amount }}` und wird keine `repayment_phase_id` mitgeschickt, greift der Dummy-Fallback **nicht**, und minijinja's strict-env liefert einen Render-Fehler („undefined variable"). Der landet als `HTML: …` in `errors`.

**Wie vermeiden:** `repayment_phase_id` konsequent von der Call-Site durchreichen (`mail_page.rs` hat es bereits als Signal). Zusätzlich: der Preview-Modus muss `PreviewResponse.errors` **anzeigen** statt zu schlucken — sonst sieht der Vorstand nur einen leeren Rahmen und weiß nicht warum. Empfehlung: bei nicht-leerem `errors` statt des iframes den bestehenden roten Fehler-Block rendern (gleiche Optik wie `template_preview.rs:157-162`).

**Warnzeichen:** Vorschau bleibt leer, obwohl der Editor Inhalt hat.

---

### Pitfall 8: Umlaute werden zu Mojibake

**Was schiefgeht:** Ein `srcdoc`-Dokument ohne `<meta charset>` erbt zwar üblicherweise die Encoding-Einstellung des Parent-Dokuments, aber das ist Browser-Verhalten, auf das man sich bei deutschsprachigen Mails („Grüße", „Mitgliedschaftserklärung") nicht verlassen sollte.

**Wie vermeiden:** `<meta charset="utf-8">` als erstes Element im `<head>` von `preview_srcdoc` — kostenlos, und der Unit-Test kann es festnageln. `[ASSUMED — Encoding-Vererbung nicht empirisch geprüft; die Gegenmaßnahme ist trivial und risikofrei]`

---

## CSS-Isolation: Beweisführung und verifizierbarer Testweg (Erfolgskriterium 4 / PREV-05)

**Warum die Isolation strukturell gegeben ist:** Ein `<iframe>` erzeugt einen eigenen *nested browsing context* mit eigenem `Document` und eigener Stylesheet-Liste. Ein Stylesheet des Parent-Dokuments ist im Kind-Dokument schlicht nicht registriert; Selektoren können Dokumentgrenzen nicht überschreiten. Umgekehrt gilt dasselbe. Das ist eine Browser-Garantie und nichts, was man testen müsste — testen muss man, dass **wir sie nicht selbst unterlaufen**.

**Wie man das ohne Browser-E2E verifizierbar macht — drei Ebenen:**

1. **String-Invariante auf dem `srcdoc` (nativer Unit-Test).** Der einzige Weg, wie App-CSS in den iframe gelangen könnte, ist ein `<link>` oder ein inline-`@import` im srcdoc. Also:
   ```rust
   #[test]
   fn srcdoc_is_self_contained_no_external_css() {
       let doc = preview_srcdoc("<p>x</p>");
       assert!(!doc.contains("<link"), "srcdoc darf kein externes Stylesheet laden: {doc}");
       assert!(!doc.contains("@import"), "srcdoc darf kein @import enthalten: {doc}");
       assert!(!doc.contains("tailwind"), "srcdoc darf Tailwind nicht referenzieren: {doc}");
       assert!(!doc.contains("mail-html-render"),
           "srcdoc darf die Editor-CSS-Klasse nicht verwenden (D-10): {doc}");
       assert!(doc.contains("<style>"), "srcdoc muss das Baseline-Stylesheet inline tragen");
       assert!(doc.contains("<meta charset=\"utf-8\">"));
   }
   ```
2. **Grep-Gate auf die Sandbox-Invariante (D-11, nativer Unit-Test).** Nagelt fest, dass das `sandbox`-Attribut gesetzt ist, dass `allow-scripts` **nicht** vorkommt, und dass die Component tatsächlich ein `iframe` mit `srcdoc` rendert (und nicht heimlich auf `dangerous_inner_html` zurückfällt — was die Isolation aufheben würde).
3. **Ein UAT-Schritt für die Gegenrichtung.** Ein Konflikt-Selektor im Editor-Umfeld (z. B. temporär `body { font-family: cursive }` global) darf die Vorschau nicht verändern; ein Konflikt-Selektor im Baseline-CSS darf die App nicht verändern. Das ist ein 30-Sekunden-DevTools-Check und gehört in die 28-UAT-CHECKLIST, nicht in einen Browser-Automations-Stack.

---

## Test-Strategie

> `workflow.nyquist_validation` ist in `.planning/config.json` auf `false` gesetzt — der formale „Validation Architecture"-Abschnitt entfällt. Stattdessen hier die konkret verifizierten Test-Ebenen.

### Verifizierte Test-Kommandos und Baselines

| Ebene | Kommando | Baseline (verifiziert 2026-07-27) | Deckt ab |
|---|---|---|---|
| **Frontend nativ** | `cd genossi-frontend && cargo test` | **301 passed, 0 failed** | `PreviewMode`, `inject_asset_src`, `preview_srcdoc`, `asset_bytes_url`, Grep-Gates |
| Backend Sanitize | `cargo test -p genossi_mail --lib` | grün | Sanitize-Regeln (unverändert, nur Regression) |
| Backend Preview-Wire | `cargo test -p genossi_bin --test e2e_tests preview` | 1 Pre-existing Failure (s. Pitfall 5) | PREV-02 Sanitize-vor-Render am echten HTTP-Endpoint |
| Workspace | `cargo test` (Root) | — | Gesamtregression |

**Kritische Fallen bei den Kommandos:**
- `genossi-frontend` ist **kein Workspace-Member** (`Cargo.toml` `exclude = ["genossi-frontend", …]`) und hat ein **eigenes `Cargo.lock`**. `cargo test -p genossi-frontend` vom Root schlägt fehl mit *„package ID specification did not match any packages"*. `[VERIFIED: eigener Testlauf]`
- Der Frontend-Crate hat **kein Lib-Target** (nur `src/main.rs`). `cargo test --lib` schlägt fehl mit *„no library targets found"*. Richtig ist `cd genossi-frontend && cargo test`. `[VERIFIED: eigener Testlauf]`
- Es wird **kein wasm32-Target** benötigt — die Tests kompilieren das Binary für den Host. Alle 301 bestehenden Tests laufen so.

### Empfohlene neue Tests

**Frontend (nativ, `mail_preview_frame.rs` + `wysiwyg_toolbar.rs`):**

| Test | Prüft | Req |
|---|---|---|
| `preview_mode_widths_are_640_and_360` | `Desktop.width_px() == Some(640)`, `Mobile == Some(360)`, `Edit == None` | PREV-01 |
| `inject_asset_src_adds_src_and_keeps_asset_id` | `<img data-genossi-asset-id="{uuid}">` → beides vorhanden | PREV-03 |
| `inject_asset_src_uses_backend_base_not_relative` | Ergebnis enthält den `backend`-Präfix | PREV-03 / Pitfall 4 |
| `inject_asset_src_ignores_non_uuid_value` | Nicht-UUID → Tag unverändert, **kein** `src=` | Pitfall 3 |
| `inject_asset_src_rejects_quote_injection_payload` | Präparierter Wert mit `"`/`onerror` fügt nichts ein | Pitfall 3 |
| `inject_asset_src_handles_multiple_and_duplicate_images` | Mehrere `<img>`, auch dieselbe ID zweimal | PREV-03 |
| `inject_asset_src_leaves_html_without_images_untouched` | Byte-identisch — v1.4-Backward-Compat | PREV-05 (Erfolgskriterium 5) |
| `inject_asset_src_handles_unterminated_tag_without_panic` | `<img data-…` ohne `>` → kein Panic | Robustheit |
| `srcdoc_is_self_contained_no_external_css` | siehe § CSS-Isolation | PREV-05 |
| `srcdoc_embeds_body_html_verbatim` | Kein Escaping, Body 1:1 enthalten | D-09 |
| `asset_bytes_url_matches_image_insert_html` | Beide Stellen erzeugen dieselbe URL | D-06 |

**Frontend Grep-Gates (`mail_preview_frame.rs`, Muster aus `wysiwyg_editor.rs:333-477`):**

| Gate | Invariante | Quelle |
|---|---|---|
| `preview_frame_sets_sandbox_attribute` | `"sandbox"` kommt in der Production-Region vor | D-11 |
| `preview_frame_never_allows_scripts` | `allow-scripts` kommt **nicht** vor | D-11 / Sicherheit |
| `preview_frame_uses_iframe_srcdoc_not_inner_html` | `srcdoc` vorhanden, `dangerous_inner_html` **nicht** | PREV-05 |
| `production_region_excludes_test_module` | Meta-Test gegen die Self-Reference-Falle | Muster |

> **Self-Reference-Hazard beachten:** Die etablierte Abwehr ist zweischichtig — (a) `EDITOR_SRC` vor dem Marker `mod grep_gate_tests` abschneiden, (b) Needles zur Laufzeit via `format!`/`concat` zusammensetzen. Beide sind zwingend, sonst sind die Gates False-Positives (`wysiwyg_editor.rs:321-332`).

**Frontend Grep-Gate (`wysiwyg_editor.rs`, Ergänzung):**

| Gate | Invariante |
|---|---|
| `editor_hidden_offscreen_not_display_none` | Die Preview-Hide-Klasse enthält **nicht** `hidden`/`display:none` (Pitfall 1) |

**Backend (`genossi_bin/tests/e2e_tests.rs`, Muster: Two-Pass Some/None aus Phase 24):**

| Test | Prüft | Req |
|---|---|---|
| `preview_body_html_is_sanitized_before_render` | `body_html: "<p onclick=\"x\">Hallo {{ first_name }}</p><script>alert(1)</script>"` → Response ohne `onclick`, ohne `<script>`, mit interpoliertem Vornamen | PREV-02 |
| `preview_body_html_img_keeps_asset_id_strips_src` | `<img src="https://evil…" data-genossi-asset-id="{uuid}">` → nur das data-Attribut überlebt | PREV-02/03 |
| `preview_body_html_jinja_in_text_survives_sanitize` | `{{ first_name }}` im Text-Content wird interpoliert (D-02-Contract) | PREV-02 |
| `preview_without_body_html_unchanged` | Kein `body_html`-Key auf der Wire — Regression von `preview_body_html_round_trips_to_response` | Backward-Compat |

**Nicht automatisierbar (→ 28-UAT-CHECKLIST):** visuelle Abgrenzung des Device-Rahmens (PREV-04), tatsächliches Laden der Bilder im iframe inkl. Cookie-Verhalten (PREV-03), CSS-Bleed-Gegenprobe mit Konflikt-Klassen (Erfolgskriterium 4), Darstellung mit v1.4-Alt-Templates ohne Bilder (Erfolgskriterium 5).

**UAT-Setup-Warnung (aus 28-CONTEXT):** Backend `cargo run --features mock_auth --bin genossi` (:3000), Frontend `dx serve` (:8080), Skill `run-rust-backend-and-frontend`. Die Dev-DB enthält **echte Mitglieder-E-Mail-Adressen** — Send-Button im Smoke-Test NICHT klicken.

---

## Environment Availability

| Dependency | Benötigt von | Verfügbar | Version | Fallback |
|---|---|---|---|---|
| Rust/Cargo Host-Toolchain | Alle Tests | ✓ | Workspace baut | — |
| `dioxus 0.6.3` (`srcdoc`) | PREV-05 | ✓ | `elements.rs:1168` | — |
| `dx serve` / Dioxus CLI | UAT | ✓ (Skill `run-rust-backend-and-frontend`) | — | — |
| `wasm32-unknown-unknown` | **NICHT** für Tests | n/a | — | Nicht benötigt — alle neuen Tests laufen nativ |
| Browser mit iframe-`sandbox` | UAT | ✓ | Baseline seit ~2013 | — |
| `/api/mail/assets/{id}/bytes` | PREV-03 | ✓ (Phase 27 COMPLETE) | `genossi_rest/src/mail_asset.rs:49` | — |

**Missing dependencies with no fallback:** keine.
**Missing dependencies with fallback:** keine.

---

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---|---|---|
| V2 Authentication | ja (indirekt) | Bestehende Session-Middleware; `/bytes` ist bereits admin-gated (Phase 27, IMG-04) |
| V3 Session Management | ja (indirekt) | `app_session`-Cookie, `HttpOnly` + `SameSite=Strict` (`genossi_rest/src/lib.rs:752-759`). Phase 28 ändert nichts, **hängt aber davon ab** (siehe unten) |
| V4 Access Control | ja | Bilder in der Vorschau nur für authentifizierte Vorstands-Sessions — durch die Sandbox-Wahl (D-07) und den bestehenden Endpoint gewährleistet |
| V5 Input Validation | **ja, zentral** | ammonia (`sanitize_html`) + `Uuid::parse_str` für den Asset-Id-Wert |
| V6 Cryptography | nein | Phase 28 rührt keine Krypto an |
| V14 Configuration | ja | iframe-`sandbox`-Attribut als Konfigurations-Invariante, per Grep-Gate festgenagelt (D-11) |

### Threat Patterns für diesen Stack

| Pattern | STRIDE | Mitigation |
|---|---|---|
| Gespeichertes XSS über `body_html` → Ausführung in der Vorschau | Elevation of Privilege | Zwei unabhängige Schichten: ammonia strippt `<script>`/`on*`/`javascript:` (Phase 23, 3 Regressionstests) **und** die Sandbox hat kein `allow-scripts` (D-07) |
| Sandbox-Escape via `allow-scripts` + `allow-same-origin` | Elevation of Privilege | MDN: die Kombination erlaubt dem Dokument, das `sandbox`-Attribut selbst zu entfernen. Grep-Gate D-11 verhindert die Kombination dauerhaft `[CITED: MDN iframe]` |
| Attribut-Injektion über präparierten `data-genossi-asset-id`-Wert | Tampering | UUID-Parsing vor der Interpolation (Pitfall 3) |
| Tracking-Pixel / SSRF über externe `<img src>` | Information Disclosure | ammonia entfernt `src`/`srcset` von `<img>` vollständig (`sanitize.rs:49`); nur `data-genossi-asset-id` überlebt. Nach D-02 gilt das **auch für die Vorschau**, weil sanitisiert wird, bevor gerendert wird |
| `data:`-URI-Exfiltration | Information Disclosure | `rm_url_schemes(&["data"])` (`sanitize.rs:55`), Regressionstest vorhanden |
| Ungewollte Navigation aus der Vorschau heraus | Tampering | Kein `allow-popups`, kein `allow-top-navigation` (D-07) — Links sind nicht klickbar, das stützt zugleich PREV-04 |
| Bild-Requests ohne Session → 401, alle Bilder tot | Denial of Service (UX) | `allow-same-origin` ist **funktional zwingend**: ohne das Token wird der Origin als `null` serialisiert, alle Requests gelten als cross-site, und `SameSite=Strict`-Cookies gehen definitiv nicht mit `[VERIFIED: MDN + Privacy-Sandbox-Doku + genossi_rest/src/lib.rs:759]` |

**Kein Audit-Log:** Phase 28 führt keine schreibende Operation aus. Kein `Auditable`-Impl, keine Audit-Macros.

---

## State of the Art

| Alt | Aktuell | Wann geändert | Bedeutung für Phase 28 |
|---|---|---|---|
| `TemplatePreview` zeigt un-sanitisiertes HTML | Nach D-01 zeigt sie die sanitisierte Fassung | Phase 28 | Erwünschter Nebeneffekt; UAT sollte kurz gegenprüfen, dass die bestehende Preview nicht „plötzlich anders" wirkt |
| `image_insert_html` mit relativem `/api/...`-`src` | `config.backend`-basierter `src` | Quick 260724-8p1 | Die Preview-Injektion MUSS demselben Weg folgen (Pitfall 4) |
| Preview-Plain-Body aus `body`-Template | Aus `plain_from_html(body_html)` abgeleitet | Quick 260718 | `subject: ""`/`body: ""` im Preview-Request sind deshalb unschädlich (Pitfall 7) |
| Dioxus-Handler für DOM-Randfälle | Native `add_event_listener` | Quick 260724 | Falls `srcdoc` wider Erwarten Timing-Probleme macht, ist das das etablierte Ausweichmuster — D-09 wählt bewusst den deklarativen Weg, um es nicht zu brauchen |

**Deprecated/veraltet:**
- Der Kommentar `// Read-only preview — no sanitization here` (`rest.rs:768-769`) wird durch D-02 falsch und muss ersetzt werden.

---

## Assumptions Log

| # | Claim | Abschnitt | Risiko falls falsch |
|---|---|---|---|
| A1 | Ein `srcdoc`-Dokument erbt die Zeichenkodierung des Parent-Dokuments | Pitfall 8 | Umlaute als Mojibake. **Bereits mitigiert** durch das empfohlene `<meta charset="utf-8">` — Risiko praktisch null |
| A2 | Bei einem `allow-same-origin`-`srcdoc`-iframe wird das `SameSite=Strict`-Cookie tatsächlich an `/bytes` mitgesendet | Pattern 4, Security | Bilder zeigen ein Broken-Icon (D-08 fängt das UX-seitig ab). Die Quellen sind eindeutig („requests are treated as originating from the iframe's real origin, allowing cookies with any SameSite value"), aber es ist Browser-Verhalten, nicht Repo-Code. **UAT-Schritt einplanen** |
| A3 | Dioxus setzt `srcdoc` reaktiv per `setAttribute`, und ein `srcdoc`-Update lädt das iframe-Dokument neu | Pattern 4 | Vorschau zeigt beim zweiten Wechsel veralteten Inhalt. Der Mechanismus ist im Interpreter-Quelltext verifiziert; die *Neu-Navigation* bei `srcdoc`-Änderung ist Standard-Browser-Verhalten. **UAT: zweimal hin- und herschalten** |
| A4 | Ein Re-Render (ohne `key`-Änderung) von `WysiwygEditor` lässt den contenteditable-Inhalt unangetastet | Pattern 5 | Getippter Text verschwindet beim Member-Wechsel. Empirisch bereits bewiesen (jeder `oninput` löst denselben Zyklus aus), aber nicht formal getestet |
| A5 | ammonia serialisiert `<img>` als `<img data-genossi-asset-id="{uuid}">` mit doppelten Quotes und ohne Self-Closing-Slash | Pattern 3 | `inject_asset_src` findet den Tag nicht. **Mitigiert**: `extract_asset_uuid` toleriert Whitespace und beide Quote-Zeichen, genau wie `render.rs::extract_asset_id`, das in Produktion läuft |
| A6 | 640 px / 360 px sind die richtigen Näherungen für Desktop-/Mobile-Mail-Viewports | Pattern 1 | Vorschau ist unrealistisch. Kommt direkt aus der Roadmap (D-12); 640 px ist die klassische HTML-Mail-Content-Breite, 360 px die gängige Mobil-Viewport-Breite |

---

## Open Questions

1. **Soll der Preview-Modus `PreviewResponse.errors` anzeigen?**
   - Bekannt: Der Endpoint liefert Render-Fehler als `errors`-Array; `TemplatePreview` rendert dafür bereits einen roten Block (`template_preview.rs:157-162`).
   - Unklar: Ob der Vorstand in der Device-Vorschau denselben Block sehen soll oder einen kompakteren Hinweis.
   - **Entschieden (Discretion):** Ja — bei nicht-leerem `errors` **statt** des iframes den bestehenden roten Fehler-Block rendern. Ohne das ist ein Render-Fehler von einer leeren Mail nicht unterscheidbar (Pitfall 7). Optisch identisch zu `TemplatePreview`, damit Component-First gewahrt bleibt.

2. **Bleibt die Vorschau beim Wechsel Desktop ↔ Mobile ohne neuen Request?**
   - Bekannt: D-05 sagt „ein Request pro Umschaltung".
   - **Entschieden (Discretion):** Der Request läuft nur bei `Edit → Preview`. Der Wechsel Desktop ↔ Mobile ändert nur `width` und lässt `srcdoc` unangetastet — Dioxus' Attribut-Diff überspringt dann das `srcdoc`-`setAttribute`, das iframe-Dokument wird nicht neu geladen, und die Vorschau flackert nicht. Das ist wörtlich D-05 („Ein Request pro Umschaltung" = pro Wechsel *in* einen Preview-Modus) und spart einen Roundtrip.

3. **Wo lebt `preview_member_id` in `mail_templates.rs` konkret?**
   - Bekannt: `TemplateTester` hat bereits `selected_member_id` (`template_tester.rs:56`), gespeist von `MemberSearch`.
   - Unklar: Ob die Page das Signal besitzt oder `TemplateTester` es per `EventHandler` nach oben meldet.
   - **Empfehlung an den Planner:** Signal in der Page anlegen, `TemplateTester` bekommt es als Prop (`ReadOnlySignal<Option<Uuid>>` oder `Signal<Option<Uuid>>`) und schreibt aus `MemberSearch::on_select` hinein. Das entfernt zugleich die heute doppelte Member-Auswahl (MemberSearch + `<select>` in `TemplatePreview`) — ein eigenständiger UX-Gewinn, der ohnehin aus D-03 folgt.

---

## Sources

### Primary (HIGH confidence — im Repo verifiziert)

- `genossi_mail/src/rest.rs:258-281, 659-807` — `PreviewRequest`/`PreviewResponse`, `preview_mail`-Handler; exakte Einbaustelle für D-01/D-02
- `genossi_mail/src/service.rs:280-289` — `sanitize_body_html_opt`, `pub(crate)`, „None in ⇒ None out"
- `genossi_mail/src/sanitize.rs:36-70` — Builder-Policy, `<img>`-Härtung, Jinja-Contract (Zeilen 30-34)
- `genossi_mail/src/render.rs:293-345` — `rewrite_img_cids` + `extract_asset_id`, Vorlage für Pattern 3
- `genossi_rest/src/mail_asset.rs:42-49, 155-183` — `/bytes`-Route, Content-Type aus `asset.mime_type`
- `genossi_rest/src/lib.rs:752-759` — Session-Cookie `SameSite::Strict`
- `genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs` — `EDITOR_ID`, `sync_from_dom` (innerText!), Grep-Gate-Muster inkl. Self-Reference-Abwehr
- `genossi-frontend/src/component/mail_compose/wysiwyg_toolbar.rs:31-48` — `image_insert_html`, `config.backend`-Rationale
- `genossi-frontend/src/component/mail_compose/{template_preview,template_tester}.rs` — `preview_member_id`, `#[props(default)]`-Migrationsmuster, doppelte Member-Auswahl
- `genossi-frontend/src/page/{mail_page,mail_templates}.rs`, `src/component/inbox/reply_form.rs` — die drei Call-Sites, `key`-Bump-Remount, Submit-Guards
- `genossi-frontend/{Cargo.toml,Dioxus.toml,assets/config.json,input.css}` — web-sys-Features, Dev-Proxy, `config.backend`, `.mail-html-render`
- `dioxus-html-0.6.3/src/elements.rs:1160-1179` — `srcdoc: Uri DEFAULT` vorhanden, `sandbox` auskommentiert
- `dioxus-interpreter-js-0.6.2/src/ts/set_attribute.ts:4-64` — `default` → `node.setAttribute(field, value)`; kein HTML-Parsing
- Eigene Testläufe (2026-07-27): `cd genossi-frontend && cargo test` → 301/0; `cargo test -p genossi_bin --test e2e_tests test_mail_preview` → 1 pass / 1 pre-existing FAIL

### Secondary (MEDIUM confidence — offizielle Doku)

- MDN `<iframe>` — `allow-same-origin`-Semantik, `srcdoc`/`about:srcdoc`, Warnung vor `allow-scripts` + `allow-same-origin`, Escaping-Regeln für HTML-Quelltext
- MDN `HTMLElement.innerText` — Fallback auf `textContent` bei nicht gerendertem Element
- Google Privacy Sandbox, „New sandbox allow-same-site-none-cookies value from Chrome 135" — Cookie-Verhalten mit/ohne `allow-same-origin`

### Tertiary (LOW confidence)

- keine

---

## Metadata

**Confidence breakdown:**
- Standard Stack: **HIGH** — keine neue Dependency; alle verwendeten Bausteine im Repo verifiziert, `srcdoc`/`sandbox`-Verfügbarkeit im Dioxus-Quelltext nachgeschlagen
- Architecture: **HIGH** — Einbaustellen zeilengenau lokalisiert, alle drei Call-Sites gelesen, Refactor-Aufwand pro Call-Site belegt
- Pitfalls: **HIGH** — Pitfalls 1, 4, 5, 7 sind repo-spezifisch verifiziert (inkl. reproduziertem Testlauf); 2, 3, 6 folgen aus gelesenem Code; 8 ist die einzige Annahme, und ihre Gegenmaßnahme ist trivial
- Security: **HIGH** — Cookie-Policy im Repo verifiziert, Sandbox-Semantik an zwei unabhängigen Quellen bestätigt

**Research date:** 2026-07-27
**Valid until:** 2026-08-26 (30 Tage — Stack ist gepinnt, `Cargo.lock` committed, keine Fast-Moving-Abhängigkeit)

---

*Phase: 28-desktop-mobile-vorschau*
