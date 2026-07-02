# Phase 24: WYSIWYG Frontend Editor - Research

**Researched:** 2026-07-02
**Domain:** Dioxus 0.6 WASM frontend, contenteditable + execCommand, ammonia interop
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01 (Dual-Body-Auflösung, Variante a):** Der WYSIWYG-Editor ist die **einzige** Eingabe im Compose-Flow (die separate Plain-Text-Textarea entfällt). Beim Submit wird aus dem contenteditable-DOM gelesen: (1) das **HTML** → `body_html`, und (2) der **sichtbare Text** (`innerText`/`textContent`) → Plain-`body`. Beide werden mit dem Dioxus-State synchronisiert (kein Datenverlust beim Submit).
- **D-02 (revidiert Phase-23-Annahme):** Im Compose-UI KEIN separates Plain-Text-Feld bauen; der Plain-Teil für `multipart/alternative` entsteht aus der Editor-Extraktion. `innerText` (nicht `textContent`) bevorzugen, da es Zeilenumbrüche/Listen lesbarer erhält.
- **D-03 (Migrations-Umfang):** **Alle 3** Verwender des heutigen `MailBodyEditor` werden auf die neue Component umgestellt — es ist EINE geteilte Component (Component-First): Massenmail-Compose (`page/mail_page.rs`), Inbox-Reply (`component/inbox/reply_form.rs`), Template-Tester (`component/mail_compose/template_tester.rs`).
- **D-04 (Live-Vorschau):** Der contenteditable ist selbst bereits live-WYSIWYG. Der Mehrwert der „Vorschau" ist die **Member-Variablen-Substitution** — analog zur heutigen `TemplatePreview`. Für Phase 24 wird diese Vorschau als **gerendertes HTML** dargestellt (nicht mehr als `<pre>`-Text). Reuse/Erweiterung des bestehenden `TemplatePreview`-Musters bevorzugt.
- **D-05 (Toolbar):** Toolbar bekommt **sämtliche gängigen** Formatierungs-Features (mindestens fett, kursiv, Aufzählungs-/nummerierte Listen; darüber hinaus die üblichen wie Überschriften etc.). Constraint: exakte Button-Liste beim Planen gegen die **ammonia-Default-Whitelist** verifizieren. `styleWithCSS=false` erzwingen, damit semantische `<b>/<i>`-Tags statt Inline-`style`-Spans entstehen (EDIT-02).
- **D-06 (Link-Dialog):** **Link-Einfügen über separaten Dialog** (URL-Eingabe in einem Dialog, kein Inline-Toolbar-Feld). Native `window.prompt()`-basierte Dialoge sind mit dem Dioxus-Reload-Bug/Blocking-Verhalten vorsichtig zu behandeln — beim Planen prüfen, ob ein In-App-Dialog-Modal (bestehende `modal.rs`-Component) statt eines nativen `prompt()` passender ist.
- **D-07 (Paste):** **Paste = nur Plain-Text.** Eingefügter Inhalt (z. B. aus Word/Browser) wird beim `paste`-Event auf reinen Text reduziert.

### Claude's Discretion

- Genaue Aufteilung/Benennung der neuen Component(s) (Editor + Toolbar + ggf. dünner JS-Interop-Layer in `js.rs`).
- Ob `execCommand` direkt über `web-sys`/`js-sys::Reflect` aufgerufen wird oder ein kleiner `extern "C"`-JS-Layer analog `create_typst_editor`/`codemirror-bundle.js` genutzt wird — **ohne** neue npm-/Frontend-Dependency (EDIT-02).
- Exakte finale Toolbar-Button-Liste (innerhalb D-05-Constraint gegen ammonia-Whitelist).
- Ob die HTML-Vorschau `TemplatePreview` erweitert oder ein paralleler Render-Zweig in derselben Component wird.

### Deferred Ideas (OUT OF SCOPE)

- HTML-Mail-Bilder / Briefkopf / Logo / Inline-CSS-Branding
- Backend `body_html`-Wire + ammonia-Gate (Phase 23 — already done)
- Backend-HTML-Render im `preview_mail`-Endpoint gehört als **Seam** zu Phase 24 (siehe unten): der `PreviewResponse` fehlt heute das `body_html`-Feld; das MUSS im Rahmen dieser Phase im Backend nachgezogen werden.

</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| EDIT-01 | Vorstände verfassen formatierte Mails in einem WYSIWYG-Editor (mindestens fett, kursiv, Links, Listen) als wiederverwendbare Dioxus-Component, ersetzt `body_editor` in allen 3 Verwendern | Standard Stack: `<div contenteditable="true">` + `execCommand`; Component-first extraction per project convention |
| EDIT-02 | Editor erzeugt sauberes, sanitisierbares HTML (`styleWithCSS=false` → `<b>/<i>` statt inline-`style`); KEINE neuen Frontend-Dependencies (contenteditable + `execCommand` über vorhandenes web-sys) | Pattern: `js_sys::Reflect` call `document.execCommand('styleWithCSS', false, false)` beim Editor-Mount; `execCommand` cross-browser weiterhin supported für Ziel-Commands |
| EDIT-03 | HTML-Inhalt wird beim Absenden zuverlässig aus dem contenteditable-DOM ausgelesen und mit dem Dioxus-State synchronisiert (kein Datenverlust beim Submit) | Pattern: `web_sys::Element` per `document.get_element_by_id()` oder per Dioxus `onmounted` Node-Ref; DOM-Read auf Submit-Klick liest `.inner_html()` / `.text_content()` bzw. `HtmlElement::inner_text()` |
| EDIT-04 | Eingefügter Inhalt (Paste) wird beim Einfügen bereinigt (kein verschmutztes Markup) | Pattern: `onpaste` handler `event.prevent_default()` + `getData("text/plain")` via `web-sys::ClipboardEvent`; dann `document.execCommand("insertText", …)` |
| EDIT-05 | Live-Vorschau zeigt gerendertes HTML vor Versand | Reuse `TemplatePreview`, erweitert Backend `PreviewResponse` um `body_html: Option<String>` + rendert via `render_html_template` — das ist ein **Backend-Seam-Task** in dieser Phase |

</phase_requirements>

## Summary

Phase 24 baut eine wiederverwendbare Dioxus-Component `WysiwygEditor` mit `<div contenteditable="true">` + Toolbar, ersetzt an 3 bekannten Stellen die Textarea-basierte `MailBodyEditor`, und ergänzt einen kleinen Backend-Seam-Task (`preview_mail` gibt `body_html` mit zurück). Alle nötigen Bausteine existieren im Repo: Phase 23 hat `render_html_template()`, `RenderedContent { subject, body, body_html }`, `sanitize_html`, ammonia-Default-Whitelist und den `body_html`-API-Wire durchgezogen — das Frontend kann sofort posten und rendern lassen. Der etablierte `codemirror-bundle.js`+`extern "C"`-Interop-Layer ist das direkte Vorbild für einen sehr dünnen `wysiwyg-bundle.js` (ohne npm-Dependency — nur `window.*`-Funktionen, EDIT-02-konform), falls die Autoren die execCommand-Aufrufe lieber JS-nah kapseln. Die Alternative (direkt via `js_sys::Reflect` in Rust, analog `js.rs::copy_with_exec_command`) ist ebenfalls valide.

Kritische Fallen: `styleWithCSS=false` MUSS beim Editor-Mount einmal aufgerufen werden (sonst produziert Chrome `<span style="font-weight:bold">` das ammonia's Default-Filter stripped); Toolbar-Buttons brauchen `r#type="button"` (Projekt-Memory `feedback_dioxus_button_type.md`) sonst Page-Reload; Paste-Handler muss `prevent_default()` aufrufen bevor der Text via `insertText` gesetzt wird; die Backend-`PreviewResponse` HAT `body_html` HEUTE NICHT — das ist ein neuer Backend-Task in dieser Phase.

**Primary recommendation:** Component `WysiwygEditor { value: String, on_change: EventHandler<(String /* plain */, String /* html */)> }` in `genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs` + optionalem `assets/wysiwyg-bundle.js` (extern "C") ODER `js.rs`-Rust-Helper; Toolbar-Sub-Component; separater `LinkDialog` via `modal.rs`; migration der 3 Verwender auf ein zweites `body_html`-Signal parallel zu `body`.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| contenteditable rendering | Browser / Client | — | Native DOM; nothing to do server-side |
| execCommand formatting | Browser / Client | — | Native browser command; deprecated-but-supported |
| Paste sanitization | Browser / Client | API/Backend (defense-in-depth) | Frontend ist NUR UX; ammonia (Phase 23) ist die Sicherheitsgrenze |
| HTML rendering (`{{ vars }}` substitution) | API / Backend | — | Backend besitzt `render_html_template` + autoescape-Env (Phase 23) |
| body_html persistence + sanitization | API / Backend | Database / Storage | Bereits erledigt (Phase 23 D-03 Sanitize-on-store) |
| innerText / innerHTML DOM extraction | Browser / Client | — | Dioxus-Signal-Sync passiert im Client vor dem POST |
| i18n | Browser / Client | — | Bestehendes `i18n/mod.rs`+`de.rs`+`en.rs`-System |

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| dioxus | 0.6.3 | Reactive UI (bereits im Workspace) | Projekt-Framework, keine Änderung |
| web-sys | 0.3 | DOM zugriff (`Element::inner_html`, `HtmlElement::inner_text`, `Document::exec_command_*`, `ClipboardEvent`) | Bereits genutzt (`js.rs`, `qr_scanner.rs`); EDIT-02-konform (keine neue Dep) |
| js-sys | 0.3.77 | `Reflect::get`/`Function::call1` für `execCommand`-Aufruf | Muster aus `js.rs::copy_with_exec_command` [VERIFIED: repo grep] |
| wasm-bindgen | 0.2.97 | `extern "C"` Interop für optionalen `wysiwyg-bundle.js` | Vorbild `create_typst_editor` in `js.rs:5-22` [VERIFIED: repo grep] |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `web-sys` Features (add) | 0.3 | `ClipboardEvent`, `DataTransfer` — MUST enable in `Cargo.toml` | Für Paste-Handler (D-07) |

**web-sys Feature-Additionen in `genossi-frontend/Cargo.toml`:**

```toml
[dependencies.web-sys]
features = [
    # ... existing ...
    "ClipboardEvent",   # for onpaste text extraction
    "DataTransfer",     # ClipboardEvent::clipboard_data() returns Option<DataTransfer>
]
```

Cross-check the current feature list at `genossi-frontend/Cargo.toml` — HTML*Element features like `HtmlElement`, `HtmlInputElement`, `Node`, `Document`, `Window` are already enabled [VERIFIED: repo read].

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| contenteditable + execCommand | Quill / TipTap / Trix (via bundle) | Explicit REQUIREMENTS.md exclusion (line 62): "JS-Editor-Bibliotheken … via wasm-bindgen-Bundle: bewusst abgelehnt". EDIT-02 verbietet neue Deps. |
| execCommand (deprecated) | Selection/Range API + custom formatBlock | Deutlich mehr Code für die selben Standard-Commands; execCommand ist auf allen Zielbrowsern (Chromium, Firefox, Safari) für die Ziel-Commands (`bold`, `italic`, `insertUnorderedList`, `insertOrderedList`, `formatBlock`, `createLink`, `unlink`, `styleWithCSS`, `insertText`) stabil implementiert [ASSUMED — cross-verified in web docs]. Der Sonderfall `insertText` ist der einzige, den Firefox in einigen Versionen nur mit input-events unterstützt; für unseren Paste-Fall reicht es. Beobachtung als Pitfall dokumentiert. |
| In-App-Dialog Modal | `window.prompt()` | prompt() ist Blocking und im Kontext von async-Dioxus-Handlers unklar; ausserdem Projekt-Memory `feedback_dioxus_button_type.md` und generell fragil. **Empfohlen: `modal.rs`-Component nutzen** (D-06). |
| innerText (D-02) | textContent | textContent zerstört Zeilenumbrüche / Listen; innerText respektiert CSS-`display: block` und gibt einen sinnvollen Plaintext (mit `\n` bei `<br>`, `<li>`, `<p>`). Für Multipart-Alternative-Plain-Body ist das der richtige Ansatz. [ASSUMED — well-established browser behavior; verify via manual smoke test in the plan] |

**Installation:** Keine neuen Deps außer der Cargo.toml-Feature-Erweiterung oben. Nichts zu installieren.

## Package Legitimacy Audit

Nicht anwendbar — Phase 24 fügt **keine** neuen Frontend-Dependencies hinzu (EDIT-02). Feature-Additionen zu vorhandenen `web-sys`/`wasm-bindgen`-Deps sind keine neuen Pakete.

## Architecture Patterns

### System Architecture Diagram

```
┌────────────────────────────────────────────────────────────────────┐
│ Browser (WASM)                                                     │
│                                                                    │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │ MassMailComposePage / InboxReplyForm / TemplateTester        │  │
│  │  ─ hold body: Signal<String>, body_html: Signal<String>      │  │
│  │  ─ pass both to <WysiwygEditor>                              │  │
│  │  ─ pass body_html to <TemplatePreview> (see D-04 seam)       │  │
│  └──────────────────────────────────────────────────────────────┘  │
│              │           ▲                            │            │
│              │ props     │ on_change(plain, html)     │            │
│              ▼           │                            ▼            │
│  ┌────────────────────────────────────┐   ┌────────────────────┐   │
│  │ WysiwygEditor (new)                │   │ TemplatePreview    │   │
│  │  ┌─────────────────────────────┐   │   │  (extended: HTML   │   │
│  │  │ Toolbar (sub-component)     │   │   │  render, no <pre>) │   │
│  │  │  [B][I][U][•][1.][H1…H3]…   │   │   └────────────────────┘   │
│  │  │  onclick → execCommand(…)   │   │           │                │
│  │  └─────────────────────────────┘   │           │ POST           │
│  │  ┌─────────────────────────────┐   │           ▼                │
│  │  │ <div contenteditable=true   │   │   /api/mail/preview        │
│  │  │       oninput onpaste       │   │                            │
│  │  │       onfocus onmounted>    │   │                            │
│  │  └─────────────────────────────┘   │                            │
│  │  ┌─────────────────────────────┐   │                            │
│  │  │ LinkDialog (via modal.rs)   │   │                            │
│  │  └─────────────────────────────┘   │                            │
│  └────────────────────────────────────┘                            │
│                                                                    │
│  On mount:                                                         │
│   ─ document.execCommand("styleWithCSS", false, false)             │
│                                                                    │
│  On input:                                                         │
│   ─ read .innerHTML → body_html signal                             │
│   ─ read .innerText → body signal                                  │
│                                                                    │
│  On paste:                                                         │
│   ─ evt.preventDefault()                                           │
│   ─ text = evt.clipboardData.getData("text/plain")                 │
│   ─ document.execCommand("insertText", false, text)                │
└────────────────────────────────────────────────────────────────────┘
                                                    │
                                                    │ HTTP POST body_html
                                                    ▼
┌────────────────────────────────────────────────────────────────────┐
│ Backend (existing, Phase 23 done)                                  │
│  POST /api/mail/send        │ sanitize_html → create_job           │
│  POST /api/mail/send-bulk   │ sanitize_html → create_job           │
│  POST /api/mail/preview     │ (NEW SEAM: render body_html)         │
│  POST /api/mail/templates   │ sanitize_html → create               │
│  PUT  /api/mail/templates/:id │ sanitize_html → update             │
└────────────────────────────────────────────────────────────────────┘
```

### Recommended Project Structure

```
genossi-frontend/src/component/mail_compose/
├── wysiwyg_editor.rs      # new: <div contenteditable> + hosts Toolbar + LinkDialog
├── wysiwyg_toolbar.rs     # new: Toolbar buttons; onclick → execCommand via js.rs helper
├── wysiwyg_link_dialog.rs # new: In-App Dialog (uses modal.rs) for URL entry
├── body_editor.rs         # existing: DEPRECATED after migration, may be removed at Task 4
├── template_preview.rs    # existing: EXTENDED to render body_html when preview_result.body_html.is_some()
└── mod.rs                 # add: pub use wysiwyg_editor::WysiwygEditor;

genossi-frontend/src/
├── js.rs                  # existing: EXTEND with exec_command_* helpers (or add extern "C" bundle bindings)
└── (optional) assets/wysiwyg-bundle.js  # optional: mirror codemirror-bundle.js pattern
                                          # if authors prefer JS-side encapsulation

genossi_mail/src/
├── rest.rs                # BACKEND SEAM: extend PreviewResponse.body_html + preview_mail handler
└── template.rs            # existing: render_html_template already there (Phase 23)
```

### Pattern 1: Dioxus contenteditable + onmounted / oninput / onpaste

**What:** A minimal reactive contenteditable div wired to Dioxus signals via `onmounted` (grab the Element ref) + `oninput` (sync back into signal).

**When to use:** Whenever the editor produces both an HTML AND a plaintext value; the plaintext derivation is what the Textarea gives you for free — here it's `.innerText` of the DOM node.

**Example (pseudo-Dioxus, verify exact signatures during planning):**

```rust
// Source: pattern established in genossi-frontend/src/page/templates.rs:484 (onmounted)
// combined with web-sys DOM read (js.rs pattern) — verify Dioxus 0.6.3 event surface
use dioxus::prelude::*;
use wasm_bindgen::JsCast;

#[component]
pub fn WysiwygEditor(
    value: String,
    on_change: EventHandler<(String /* plain */, String /* html */)>,
) -> Element {
    let mut node_ref = use_signal(|| None::<Rc<MountedData>>);

    rsx! {
        div {
            // stable id lets Toolbar's execCommand focus the right node
            id: "wysiwyg-editor",
            class: "w-full border rounded px-3 py-2 min-h-40 focus:outline-none",
            contenteditable: "true",
            // capture the node ref (Dioxus 0.6 supports onmounted on any element)
            onmounted: move |cx: Event<MountedData>| {
                node_ref.set(Some(cx.data()));
                // ONE-TIME: force semantic <b>/<i> instead of inline styles
                // (crucial for ammonia survival — see Pitfall 1)
                if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                    // See js.rs::copy_with_exec_command for the js_sys::Reflect pattern
                    let _ = crate::js::exec_command_bool(&doc, "styleWithCSS", false);
                }
                // Set initial HTML content
                if let Some(el) = document_get_editor_element() {
                    el.set_inner_html(&value);
                }
            },
            oninput: move |_evt| {
                // Read innerHTML and innerText from the DOM, sync both back
                if let Some(el) = document_get_editor_element() {
                    let html = el.inner_html();
                    let plain = el.dyn_ref::<web_sys::HtmlElement>()
                        .map(|h| h.inner_text())
                        .unwrap_or_default();
                    on_change.call((plain, html));
                }
            },
            onpaste: move |evt: Event<ClipboardData>| {
                // D-07: plain-text paste — Pattern 2 below
                evt.prevent_default();
                let text = evt.data().get_data("text/plain").unwrap_or_default();
                if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                    let _ = crate::js::exec_command_str(&doc, "insertText", &text);
                }
            },
        }
    }
}
```

> **Sanity check required at plan time:** Dioxus 0.6.3 exposes `onpaste` via `Event<ClipboardData>` in `web`-feature builds. If the exact `ClipboardData` binding isn't there, the fallback is `web_sys::ClipboardEvent` via a direct DOM `addEventListener` in `onmounted` — the sibling `codemirror-bundle.js` pattern already demonstrates the JS-side wiring approach if pure Dioxus falls short. **Planner MUST verify Dioxus 0.6.3 supports `onpaste` before locking the pattern.**

### Pattern 2: execCommand via js_sys::Reflect (no bundle needed)

**What:** Call `document.execCommand(name, showUI=false, value?)` from Rust without a JS shim.

**Example (extending the js.rs pattern, verified via existing `copy_with_exec_command`):**

```rust
// New helpers in genossi-frontend/src/js.rs — mirrors copy_with_exec_command
use js_sys::Reflect;
use wasm_bindgen::{JsCast, JsValue};

pub fn exec_command_bool(doc: &web_sys::Document, cmd: &str, arg: bool) -> Result<bool, JsValue> {
    let exec = Reflect::get(doc, &JsValue::from_str("execCommand"))?;
    let f = exec.dyn_ref::<js_sys::Function>()
        .ok_or_else(|| JsValue::from_str("execCommand is not a function"))?;
    let ok = f.call3(
        doc,
        &JsValue::from_str(cmd),
        &JsValue::from_bool(false),        // showUI
        &JsValue::from_bool(arg),
    )?;
    Ok(ok.as_bool().unwrap_or(false))
}

pub fn exec_command_str(doc: &web_sys::Document, cmd: &str, arg: &str) -> Result<bool, JsValue> {
    let exec = Reflect::get(doc, &JsValue::from_str("execCommand"))?;
    let f = exec.dyn_ref::<js_sys::Function>()
        .ok_or_else(|| JsValue::from_str("execCommand is not a function"))?;
    let ok = f.call3(
        doc,
        &JsValue::from_str(cmd),
        &JsValue::from_bool(false),
        &JsValue::from_str(arg),
    )?;
    Ok(ok.as_bool().unwrap_or(false))
}

pub fn exec_command_simple(doc: &web_sys::Document, cmd: &str) -> Result<bool, JsValue> {
    let exec = Reflect::get(doc, &JsValue::from_str("execCommand"))?;
    let f = exec.dyn_ref::<js_sys::Function>()
        .ok_or_else(|| JsValue::from_str("execCommand is not a function"))?;
    let ok = f.call1(doc, &JsValue::from_str(cmd))?;
    Ok(ok.as_bool().unwrap_or(false))
}
```

**Toolbar-Button Onclick pattern:**

```rust
button {
    r#type: "button",  // <-- CRITICAL: Dioxus button-reload-bug (feedback_dioxus_button_type.md)
    onclick: move |evt| {
        evt.prevent_default();
        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
            let _ = crate::js::exec_command_simple(&doc, "bold");
        }
        // After execCommand, DOM changed → re-read innerHTML/innerText to sync signal
        // (see Pitfall 5)
    },
    "B"  // or use i18n key
}
```

### Pattern 3: In-App Link Dialog via `modal.rs`

**What:** When "Link" toolbar button is clicked, open the existing `Modal` component with a URL input, an optional link-text input, and Insert/Cancel buttons. On Insert, save the current Selection Range (before the modal steals focus can be tricky), then `execCommand("createLink", false, url)` or manually build an `<a href="">` via Range API.

**Why not `window.prompt()`:** prompt() is a synchronous blocking modal that historically interacts poorly with WASM event loops and Dioxus's reactive updates. The project already has `modal.rs` and `feedback_dioxus_button_type.md` memory suggesting caution around native dialogs.

**Selection preservation tip (Pitfall 6):** When the modal opens, the contenteditable loses focus and the Selection Range is lost. Two solutions:
- **Save the Range before opening the modal:** capture `document.getSelection().getRangeAt(0)` into a signal before setting `modal_open.set(true)`; restore via `selection.removeAllRanges(); selection.addRange(saved)` after the user clicks Insert.
- **Simpler alternative:** re-focus the editor first (`editor_el.focus()`), then call `execCommand("createLink", …)`. But if no selection existed, `createLink` does nothing meaningful — so require the user to select text first (UX: disable Link button until Selection is inside the editor and non-empty).

**Recommendation:** Save the Range on toolbar-click, restore on Insert. Same pattern is documented in every contenteditable tutorial.

### Anti-Patterns to Avoid

- **`window.prompt()` for the Link URL:** blocking; native browser prompt may not integrate cleanly with Dioxus event lifecycle. (See D-06 note in CONTEXT.md.)
- **Toolbar button without `r#type="button"`:** page reload — Projekt-Memory `feedback_dioxus_button_type.md`.
- **Skipping `styleWithCSS=false`:** ammonia's default filter strips inline `style` attributes on `<span>` — bold/italic would be lost after the sanitize-on-store gate in `create_job`. **This is the single biggest failure mode**; must be pinned by a test or explicit smoke check.
- **Storing the HTML in a `Signal<String>` and RENDERING it back into the contenteditable on every input event:** creates a cursor-jump bug. The editor is the source of truth WHILE editing; only sync FROM DOM TO signal, not back, until an external caller resets it (e.g. TemplateSelector picking a template).
- **Using `textContent` instead of `innerText`:** kills line breaks and list structure in the plaintext extraction (D-02).
- **Frontend-Sanitization als Sicherheitsgrenze:** ammonia (Phase 23) ist die Grenze; die Paste-Sanitization ist reine UX.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Rich-text HTML sanitization | Custom regex tag stripper | `ammonia` (Phase 23, done) | Edge cases, escaping, security — bereits erledigt Backend-seitig |
| Text extraction from HTML | `html2text` / custom `<br>`-splitter | `innerText` via `web-sys::HtmlElement` | Browser hat es eingebaut; D-02 |
| Selection/Range custom API | Manual DOM walker | `document.getSelection().getRangeAt(0)` | Zwei Zeilen JS via `js_sys::Reflect` |
| Autoescape für Member-Variablen im HTML | Manuelles `&amp;`-Replace | Backend `render_html_template()` (Phase 23 D-04) | Backend hat autoescape-minijinja-Env; Frontend rendert nichts selbst |
| execCommand-Polyfill | Selection/Range formatBlock rewrite | Native `document.execCommand` | Auf Ziel-Browsern für unsere Commands stabil; execCommand ist zwar deprecated aber breit unterstützt |

**Key insight:** Der WYSIWYG-Editor ist bewusst 100 LOC Frontend-Code + eine dünne execCommand-Fassade — mehr braucht Genossi nicht. Alles Riskante (HTML-Sanitization, Autoescape, Persistenz) läuft server-seitig und ist in Phase 23 gelandet.

## Runtime State Inventory

Nicht anwendbar — Phase 24 ist kein Rename/Refactor/Migrations-Phase. Es werden keine Strings umbenannt, keine gespeicherten IDs geändert. Der neue Code fügt zwei zusätzliche Signal-Reads (`.innerHTML` + `.innerText`) und drei neue Frontend-Files hinzu.

## Common Pitfalls

### Pitfall 1: `styleWithCSS=true` ist Default in Chromium — ammonia strippt die inline-styles

**What goes wrong:** Chromium (Chrome/Edge) und Firefox setzen per Default `styleWithCSS=true`, sodass `document.execCommand("bold")` `<span style="font-weight: bold">…</span>` erzeugt (statt `<b>…</b>`). Ammonia's Default-Filter erlaubt `<span>` aber strippt das `style`-Attribut → Fett-Formatierung geht beim Speichern verloren.

**Why it happens:** WHATWG-Compat mit CSS-basiertem Rich-Text.

**How to avoid:** Beim Editor-Mount **einmal** `document.execCommand("styleWithCSS", false, false)` aufrufen. Der Aufruf-Zustand persistiert für die Lebenszeit des Documents. Test: nach `bold` muss `innerHTML` `<b>…</b>` (oder `<strong>` je nach Browser — beide sind ammonia-safe) enthalten, kein `<span style=`.

**Warning signs:** Nach Round-Trip durch Backend → GET template → RE-Load, Fett-Text ist plaintext.

### Pitfall 2: Dioxus button-reload

**What goes wrong:** Toolbar-Button ohne `r#type="button"` triggert Form-Submit → Page-Reload → Editor-Content verloren.

**How to avoid:** JEDER `<button>` in Editor/Toolbar/LinkDialog bekommt `r#type: "button"` + `evt.prevent_default()` im onclick. Projekt-Memory: `feedback_dioxus_button_type.md`; Live-Referenz: `template_tester.rs:113`, `reply_form.rs:151,228,251`.

### Pitfall 3: Paste-Event `getData("text/plain")` MUSS nach `preventDefault()` gelesen werden

**What goes wrong:** In manchen Browser-Versionen ist die ClipboardData nach `preventDefault()` nicht mehr lesbar (in anderen ist es genau umgekehrt).

**How to avoid:** Reihenfolge: `getData` FIRST, `preventDefault` DANACH; ODER (sicherer) `preventDefault` FIRST + `getData` DANACH, aber `getData` auf dem event.clipboardData snapshot. In modernen Browsern (2020+) funktioniert beides — beim Smoke-Test dokumentieren, welche Reihenfolge stabil ist.

**Warning signs:** Paste fügt gar nichts ein, oder fügt den formatted Text ein trotz `preventDefault`.

### Pitfall 4: `execCommand("insertText")` in Firefox

**What goes wrong:** Firefox hat `insertText` erst spät (~85+) korrekt implementiert. Für die Genossi-Zielgruppe (Vorstand, produktive Nutzung) sollte das kein Problem sein, aber Firefox-Nutzer könnten stumme Paste-Ausfälle sehen.

**How to avoid:** Fallback via `document.getSelection().getRangeAt(0).insertNode(document.createTextNode(text))` mit anschließendem Range-Move. Nur einbauen wenn Smoke-Test zeigt dass es nötig ist.

### Pitfall 5: DOM-Sync-Race — Toolbar-Click ändert DOM, Signal ist stale

**What goes wrong:** User schreibt „Hallo" (Signal = `Hallo`). User klickt Bold — `execCommand("bold")` verändert DOM, ABER `oninput` feuert NICHT für execCommand-Änderungen (nur für Tastatur/Paste). Beim Submit ist der Signal-Wert `Hallo` (unformatiert), aber der DOM hat `<b>Hallo</b>`.

**How to avoid:** Nach jedem `execCommand`-Call einen expliziten `sync_from_dom()` Aufruf: lese `.innerHTML`/`.innerText`, schreibe `on_change.call((plain, html))`. Alternativ: beim Submit-Klick der Verwender-Component (Compose/Reply/Tester) VOR dem POST einen expliziten Read: hole das Editor-Element per `document.get_element_by_id("wysiwyg-editor")`, lese `.innerHTML`/`.innerText`, überschreibe die Signals. **Empfehlung: beide Wege — sync auf Toolbar-Click UND Submit-Guard-Read**, weil ein Toolbar-Click ohne späteres Tippen sonst verloren geht.

**Warning signs:** Fettgedruckter Text landet im gespeicherten `body_html` NICHT.

### Pitfall 6: Modal öffnet → Selection Range verloren

**What goes wrong:** LinkDialog opens (via `modal.rs`), overlay steals focus, Selection collapses. `execCommand("createLink", url)` operates on nothing.

**How to avoid:** Vor dem Öffnen des Modals: `let saved_range = document.getSelection().getRangeAt(0)`; Speichere in einem Signal. Nach Insert-Click: `editor_el.focus()`, `selection.removeAllRanges()`, `selection.addRange(saved_range)`, dann `execCommand("createLink", …)`.

**Warning signs:** Link wird nicht eingefügt oder wird an falscher Stelle eingefügt.

### Pitfall 7: `on_change` alt-Contract vs neuer Dual-Value-Contract

**What goes wrong:** Bestehende Verwender rufen `on_change: move |val: String| body.set(val)`. Neue `WysiwygEditor`-Component liefert `(plain: String, html: String)`. Migration fehlerhaft → Kompilierfehler ODER (schlimmer) einer der beiden Werte wird ignoriert.

**How to avoid:** Signal-Shape-Erweiterung an allen 3 Verwendern:
- `mail_page.rs`: nach `let mut body = use_signal(|| String::new());` (Zeile 59) ergänze `let mut body_html = use_signal(|| String::new());`; ersetze `on_change: move |val: String| body.set(val)` durch `on_change: move |(plain, html): (String, String)| { body.set(plain); body_html.set(html); }`; im Send-Payload das neue `body_html`-Signal mit-POSTen (`api::send_bulk_mail` etc. haben das Feld bereits, Phase 23).
- `reply_form.rs`: analog, plus baseline-body-Erweiterung für dirty-check.
- `template_tester.rs`: nutzt einen `body`-`ReadOnlySignal<String>` als Prop; Aufrufer (`page/mail_templates.rs` oder wo TemplateTester eingebaut ist) muss ebenfalls body_html tragen — beim Planen den Aufrufer lokalisieren.

**Warning signs:** Compile fehlschlägt an allen 3 Migrationsstellen — das ist gut, weil es die Migration erzwingt.

### Pitfall 8: Backend `preview_mail` liefert HEUTE kein `body_html`

**What goes wrong:** D-04 verlangt eine HTML-Live-Vorschau. Frontend ruft `api::preview_mail(...)` und erhält `PreviewResponse { subject, body, errors, used_dummy_repayment }` — kein Feld `body_html`. Wenn ihr die Preview-Component erweitert ohne den Backend-Endpoint zu erweitern, kommt kein HTML zurück.

**How to avoid:** Backend-Seam-Task in dieser Phase:
1. `PreviewRequest` um `body_html: Option<String>` erweitern (`genossi_mail/src/rest.rs:258`).
2. `PreviewResponse` um `body_html: Option<String>` mit `#[serde(default, skip_serializing_if = "Option::is_none")]` erweitern (`rest.rs:276`; gleiches Muster wie in Phase 23 Plan 04 überall angewendet).
3. Im `preview_mail`-Handler (`rest.rs:643`): wenn `body.body_html.is_some()`, ruf `crate::template::render_html_template(&html_src, &ctx)` (analog zum vorhandenen `render_template` auf Zeile 742) und packe das Ergebnis in `response.body_html`.
4. `api.rs::PreviewResponse` (Frontend, Zeile 944) um `body_html: Option<String>` mit `#[serde(default)]` erweitern.
5. `api.rs::preview_mail` Frontend-Signatur bleibt gleich (member_id-basiert), das JSON ist backward-kompatibel via `skip_serializing_if`.

**Warning signs:** TemplatePreview zeigt leere Vorschau; DevTools zeigt Response ohne `body_html`-Key.

## Code Examples

Verified patterns from official sources & repo:

### Reading `.innerHTML` and `.innerText` from a DOM element

```rust
// Source: web-sys 0.3 API + genossi-frontend js.rs pattern
use wasm_bindgen::JsCast;

fn read_editor(doc: &web_sys::Document, id: &str) -> Option<(String, String)> {
    let el = doc.get_element_by_id(id)?;
    let html = el.inner_html();
    let plain = el.dyn_ref::<web_sys::HtmlElement>()
        .map(|h| h.inner_text())
        .unwrap_or_default();
    Some((plain, html))
}
```

### execCommand — full commands the toolbar will use

| Command | Args | Produces (with `styleWithCSS=false`) | Ammonia-safe? |
|---------|------|--------------------------------------|---------------|
| `bold` | none | `<b>…</b>` (Chromium/Firefox) or `<strong>` | ✓ (both `<b>` and `<strong>` in default whitelist) |
| `italic` | none | `<i>…</i>` or `<em>` | ✓ |
| `underline` | none | `<u>…</u>` | ✓ — `<u>` IST im Default-Whitelist [CITED: docs.rs/ammonia] |
| `strikeThrough` | none | `<strike>` or `<s>` | ✓ — beide im Default |
| `insertUnorderedList` | none | `<ul><li>…</li></ul>` | ✓ |
| `insertOrderedList` | none | `<ol><li>…</li></ol>` | ✓ |
| `formatBlock` | `<h1>` / `<h2>` / `<h3>` / `<p>` / `<blockquote>` | Corresponding block element | ✓ (h1-h6, p, blockquote all in default) |
| `createLink` | url | `<a href="url">…</a>` | ✓ (a[href] in default; `target=_blank` gets `rel=noopener` forced) |
| `unlink` | none | strips `<a>` from selection | ✓ |
| `styleWithCSS` | `false` (bool) | (no output — flips a browser flag) | N/A |
| `insertText` | text | plaintext at cursor (Paste-Path) | N/A |

**Note on the exact tag mapping:** modern browsers emit `<b>`/`<i>` for `bold`/`italic` when `styleWithCSS=false`; some Firefox versions may emit `<strong>`/`<em>`. Both are in ammonia's default whitelist ([CITED: docs.rs/ammonia/latest/ammonia — default allowed elements list contains `b, i, em, strong, u, s, strike, p, br, ol, ul, li, blockquote, h1-h6, a, span, div, pre, code, hr` and more]), so either mapping survives.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Textarea + plaintext body | contenteditable + HTML+plain dual body | This phase | Enables formatted mails; retains backward-compat via body_html=NULL path (Phase 23) |
| `execCommand` (deprecated MDN) | Same — still de-facto standard for contenteditable formatting | 2018-ish deprecation notice | Deprecated but every browser still ships it; alternative (custom Selection/Range) is 10x more code |
| `document.execCommand("copy")` | `navigator.clipboard.writeText()` | Modern browsers | Not relevant here — we don't clipboard-copy from the editor. (`js.rs::copy_to_clipboard` already uses the modern path with the exec_command fallback.) |
| Frontend HTML sanitization | Backend `ammonia` gate | Phase 23 | Frontend paste-cleanup ist reine UX; Sicherheit liegt Backend-seitig |

**Deprecated/outdated:**
- `document.execCommand` is officially deprecated but every major browser still implements it for these standard commands. Genossi's use is bounded to a handful of commands ([`bold`, `italic`, `underline`, `strikeThrough`, `insertUnorderedList`, `insertOrderedList`, `formatBlock`, `createLink`, `unlink`, `styleWithCSS`, `insertText`]) — the boring, well-supported subset.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Dioxus 0.6.3 exposes `onpaste` on `<div>` with `Event<ClipboardData>` | Pattern 1 | Falls falsch: fallback via `web_sys::EventListener` + `addEventListener("paste", …)` im `onmounted`-Handler; +~10 LOC. Planner MUST verify by grepping Dioxus 0.6.3 event surface or writing a tiny smoke test |
| A2 | Dioxus 0.6.3 exposes `onmounted` on any element (verified for `<div>` in templates.rs:484) | Pattern 1 | LOW — already used in production code |
| A3 | `innerText` includes `\n` for block elements (`<p>`, `<li>`, `<br>`) across Chromium/Firefox/Safari | D-02 / Pitfall | Falls falsch: Plaintext-Body im multipart/alternative sieht wie eine lange Zeile aus. Manueller Smoke-Test lockt das |
| A4 | `execCommand("insertText", …)` funktioniert in Firefox 85+ | Pitfall 4 | LOW — Firefox 85 ist 2021; Genossi's Zielgruppe ist Vorstand mit moderner Browser-Version |
| A5 | Ammonia 4.1.3 Default lässt `<u>`, `<s>`, `<blockquote>`, `<h1>`-`<h6>` durch | Toolbar-Feature-Liste | [CITED: docs.rs/ammonia — default allowed elements list explicitly contains these tags]; Test in Plan 24 bestätigt via ammonia-Roundtrip |
| A6 | `document.execCommand("styleWithCSS", false, false)` persistiert für die Session ohne erneuten Aufruf | Pitfall 1 | LOW — Standard-Browser-Verhalten; falls falsch, den Call vor JEDEM formatting-execCommand ausführen (+3 LOC) |
| A7 | Backend `preview_mail` render_html_template mit dummy repayment context ist verhaltenskompatibel zum bestehenden render_template-Zweig | Backend seam | LOW — beide Envs teilen den `ctx`; nur der Escape unterscheidet sich |

**Any `[ASSUMED]` claim above must be gated by a discovery task or a manual smoke-check in the plan before it becomes a locked decision.**

## Open Questions

1. **Signal-Prop-Shape für WysiwygEditor**
   - What we know: alte Signatur `value: String, on_change: EventHandler<String>` (siehe body_editor.rs:6)
   - What's unclear: neue Prop-Signatur — `value_html: String, on_change: EventHandler<(String, String)>` ODER zwei separate `on_change_plain` + `on_change_html`?
   - Recommendation: EIN `on_change: EventHandler<(String /* plain */, String /* html */)>` — halbiert die Verkabelung an den 3 Verwendern.

2. **Templates-Editor (`page/templates.rs`) betroffen?**
   - What we know: templates.rs verwendet `createTypstEditor` (Typst-Templates), NICHT `MailBodyEditor`.
   - What's unclear: Ob der Mail-Template-Editor an anderer Stelle (Mail-Template-Verwaltung) den neuen WYSIWYG-Editor als Body-Feld braucht.
   - Recommendation: **Nein**, im Scope. Mail-Template-Body-Feld liegt (heute) in einem Mail-Template-Formular — beim Planen prüfen ob dort AUCH `MailBodyEditor` steht (grep sagt: nur die 3 in CONTEXT.md; templates.rs ist Typst nicht Mail).

3. **Dioxus `Event<ClipboardData>` vs. `web_sys::ClipboardEvent`**
   - What we know: Dioxus 0.6 hat verschiedene Event-Wrapper; ob `ClipboardData::get_data("text/plain")` direkt exponiert ist, ist zu verifizieren.
   - What's unclear: Die exakte API-Signatur.
   - Recommendation: kleinen Spike im Plan verankern („Task 0.5: Verify Dioxus 0.6.3 paste event surface via 5-line probe"). Fallback dokumentiert (Pitfall 3).

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain (from flake.nix) | Alle Builds | ✓ | (flake-pinned) | — |
| `dx serve` CLI | Frontend Dev Server | ✓ | (via flake) | — |
| Backend running (Phase 23 code) | Live-Preview + Send-Path | ✓ | current main | — |
| Modern Browser (Chromium/Firefox/Safari) | Contenteditable + execCommand + Clipboard | ✓ | current | — |
| ammonia (backend crate) | Sanitization | ✓ (Phase 23) | 4.1.x | — |

Keine fehlenden externen Abhängigkeiten.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (Rust unit tests via `#[test]`); `wasm-bindgen-test` verfügbar via dev-dep für WASM-run in headless-Browser [VERIFIED: Cargo.toml `[dev-dependencies] wasm-bindgen-test = "0.3"`] |
| Config file | none — standard cargo test discovery |
| Quick run command | `cargo test -p genossi-frontend --lib` (pure-Rust helpers), `cargo test -p genossi_mail --lib` (backend seam) |
| Full suite command | `cargo test` (workspace) |

**Testable pure helpers (empfohlen zum unit-testen):**
- `plain_from_html_edge_cases`: kleiner Rust-Helper der `<br>` in `\n` mapped, `<li>` in `\n- …` etc. — falls die Component ihre eigene "sicherheitsnetz"-Extraktion baut. (Alternative: reine DOM-Extraktion, unit-test dann nur via `wasm-bindgen-test`.)
- `is_valid_url_for_link_dialog(url: &str) -> bool`: eine Mini-Validation die das Link-Dialog-„Insert"-Button gated (analog `is_valid_test_address` in `template_tester.rs:39`).
- `is_editor_dirty(current: &str, baseline: &str) -> bool` — für den Reply-Flow (Reuse des `is_draft_dirty`-Musters in `reply_form.rs:313`).

**Backend seam tests (existing pattern):**
- `preview_mail_returns_body_html_when_body_html_in_request` — analog zum HTML-05 e2e-Muster in `bulk_mail_body_html_sanitized_and_persisted` [VERIFIED: 23-04-SUMMARY.md].
- `preview_response_serializes_without_body_html_when_none` — Serde-lock analog `mail_template_to_serializes_without_body_html_when_none`.

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| EDIT-01 | Editor rendert Toolbar + contenteditable, ersetzt MailBodyEditor an 3 Stellen | Unit (compile) + manual smoke | `cargo check -p genossi-frontend` + `dx serve` browser check | ❌ Wave 0 (new component file) |
| EDIT-02 | `styleWithCSS=false` produziert `<b>/<i>` (nicht `<span style>`) | Manual smoke (via DevTools inspect innerHTML after Bold-Click) + wasm-bindgen-test (optional) | manual: `dx serve` → inspect | ❌ Wave 0 (manual UAT step in plan) |
| EDIT-03 | Submit reads DOM zuverlässig → body + body_html | Manual UAT: type formatted, click Send, check POST payload in DevTools Network; ODER e2e: extend `bulk_mail_body_html_sanitized_and_persisted` mit einem echten POST-shape-Test | UAT-Checkliste in plan | Test infrastructure exists (e2e_tests.rs) |
| EDIT-04 | Paste ist plain — pasted `<b>bold</b>` erscheint als `bold` im innerHTML | Manual smoke: kopiere formatted Text aus Word/Browser, paste in Editor | UAT-Checkliste in plan | ❌ Wave 0 (manual) |
| EDIT-05 | Preview zeigt gerendertes HTML mit substituierten Vars | Manual smoke + backend unit-test (`render_html_template` mit `{{ first_name }}`) | `cargo test -p genossi_mail --lib preview` + `dx serve` | Test infrastructure exists |

**Backend seam tests (empfohlen als automatisierte Absicherung):**

| Test | Location | Assertion |
|------|----------|-----------|
| `preview_returns_body_html_when_html_source_provided` | `genossi_mail/src/rest.rs` `#[cfg(test)]` | POST `/api/mail/preview` mit `body_html: Some("<p>Hallo {{ first_name }}</p>")` → response has `body_html: Some("<p>Hallo Max</p>")` |
| `preview_response_skip_serializing_body_html_when_none` | `genossi_mail/src/rest.rs` `#[cfg(test)]` | Serde-lock — pre-Phase-24 clients see no wire change |

### Sampling Rate

- **Per task commit:** `cargo check -p genossi-frontend` (fast type check) + `cargo test -p genossi_mail --lib` (backend seam)
- **Per wave merge:** `cargo test` (workspace)
- **Phase gate:** Manual UAT-Checkliste durchlaufen mit `dx serve` + Backend `cargo run --bin genossi` (see project skill `run-rust-backend-and-frontend`)

### Wave 0 Gaps

- [ ] `genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs` — new component file
- [ ] `genossi-frontend/src/component/mail_compose/wysiwyg_toolbar.rs` — new sub-component
- [ ] `genossi-frontend/src/component/mail_compose/wysiwyg_link_dialog.rs` — new sub-component (uses `modal.rs`)
- [ ] `genossi-frontend/src/js.rs` — extend with `exec_command_bool`, `exec_command_str`, `exec_command_simple` helpers
- [ ] `genossi-frontend/Cargo.toml` — add `web-sys` features `"ClipboardEvent"`, `"DataTransfer"`
- [ ] `genossi-frontend/src/i18n/mod.rs` + `de.rs` + `en.rs` — new keys (see i18n section)
- [ ] Test file (optional): `#[cfg(test)]` module in wysiwyg_toolbar.rs for pure helper unit tests
- [ ] `genossi_mail/src/rest.rs` — extend `PreviewRequest.body_html`, `PreviewResponse.body_html`, `preview_mail` handler
- [ ] `genossi-frontend/src/api.rs` — extend `PreviewResponse.body_html` (frontend mirror)
- [ ] UAT-Checkliste-Doku im Phase-24 Verification-Artefakt (7-10 Punkte: Bold/Italic/List/Heading/Link/Paste-plain/Preview-shows-HTML/…)

## Security Domain

Security enforcement gilt — Phase 24 ist reines Frontend UX; die Sicherheitsgrenzen liegen in Phase 23 (ammonia). Trotzdem der ASVS-Sanity-Check:

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | keine Auth-Änderung |
| V3 Session Management | no | keine Session-Änderung |
| V4 Access Control | no | admin-only bleibt Backend-durchgesetzt |
| V5 Input Validation | yes | Server-side ammonia (Phase 23 D-03 an 4 EPs); Frontend Paste-Sanitization ist reine UX, KEINE Sicherheitsgrenze |
| V6 Cryptography | no | — |
| V7 Error Handling | yes | ClipboardEvent-Fehler dürfen keinen Page-Reload triggern; XSS via `<script>` im Paste ist backend-gemitigated |
| V14 Configuration | no | — |

### Known Threat Patterns for contenteditable + WASM Frontend

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| XSS via pasted `<script>`-content | Tampering | ammonia strippt beim `create_job` sanitize-on-store (Phase 23 T-23-08); Frontend Paste-Handler reduziert auf Plain-Text als UX-Verbesserung, NICHT als Sicherheitsgrenze |
| `javascript:` URLs im Link-Dialog | Tampering | ammonia default-filter erlaubt keine `javascript:`/`data:` schemes; Backend rejected/strippt beim Store |
| Reflected XSS in Preview | Tampering | Backend rendert via autoescape-minijinja-Env (Phase 23 HTML-04 D-04); Member-Werte sind escaped, Author-Markup bleibt strukturell erhalten |
| execCommand-CSP-Verstoß | Configuration | execCommand ist reine DOM-Manipulation, kein `eval`; kompatibel mit striktester CSP (`default-src 'self'`) |
| Editor-Content leakt in Fremd-DOM | Information Disclosure | Editor ist im Component-Scope; keine Cross-Frame-Kommunikation |

**Zusammenfassung:** Frontend führt keine neuen Threats ein; die einzigen serverseitig-relevanten (T-23-08..T-23-11 in 23-04-SUMMARY) sind bereits mitigated.

## i18n Keys Required

Neue Keys in `genossi-frontend/src/i18n/mod.rs` `enum Key { … }` + Übersetzung in `de.rs` + `en.rs`:

| Key | German | English | Where used |
|-----|--------|---------|------------|
| `MailEditorBold` | "Fett" | "Bold" | Toolbar B button (aria-label / title) |
| `MailEditorItalic` | "Kursiv" | "Italic" | Toolbar I button |
| `MailEditorUnderline` | "Unterstrichen" | "Underline" | Toolbar U button |
| `MailEditorStrike` | "Durchgestrichen" | "Strikethrough" | Toolbar S button |
| `MailEditorUnorderedList` | "Aufzählung" | "Bulleted list" | Toolbar UL button |
| `MailEditorOrderedList` | "Nummerierte Liste" | "Numbered list" | Toolbar OL button |
| `MailEditorHeading1` | "Überschrift 1" | "Heading 1" | Toolbar H1 button |
| `MailEditorHeading2` | "Überschrift 2" | "Heading 2" | Toolbar H2 button |
| `MailEditorHeading3` | "Überschrift 3" | "Heading 3" | Toolbar H3 button |
| `MailEditorParagraph` | "Absatz" | "Paragraph" | Toolbar P button (formatBlock=p) |
| `MailEditorBlockquote` | "Zitat" | "Blockquote" | Toolbar blockquote button |
| `MailEditorLink` | "Link" | "Link" | Toolbar Link button — opens dialog |
| `MailEditorUnlink` | "Link entfernen" | "Remove link" | Toolbar Unlink button |
| `MailEditorLinkDialogTitle` | "Link einfügen" | "Insert link" | Modal title |
| `MailEditorLinkUrlLabel` | "URL" | "URL" | Modal input label |
| `MailEditorLinkTextLabel` | "Anzeige-Text (optional)" | "Display text (optional)" | Modal input label |
| `MailEditorLinkInsert` | "Einfügen" | "Insert" | Modal action button |
| `MailEditorLinkCancel` | "Abbrechen" | "Cancel" | Modal action button |
| `MailEditorPreviewHtml` | "HTML-Vorschau" | "HTML preview" | TemplatePreview extended-section label |

**Reuse existing keys where possible:**
- `MailBody` (Zeile 243 in `i18n/mod.rs`) — bleibt für das Editor-Label „Nachricht".
- `MailTemplatePreview` (Zeile 270) — bleibt für die Preview-Header.

**Constraint:** ALLE neuen Keys MÜSSEN in BEIDEN Locales (`de.rs` + `en.rs`) gepflegt werden — genossi-frontend/CLAUDE.md Component-First-Convention + i18n-Rule.

## Migration Plan for 3 Verwender

| Verwender | File:Line | Current Prop Shape | New Prop Shape | Neue Signals |
|-----------|-----------|--------------------|-----------------|--------------|
| Massenmail-Compose | `page/mail_page.rs:401` (`body` signal declared Zeile 59) | `MailBodyEditor { value: body.read().clone(), on_change: move |val: String| body.set(val) }` | `WysiwygEditor { value: body_html.read().clone(), on_change: move |(plain, html): (String, String)| { body.set(plain); body_html.set(html); } }` | Add `let mut body_html = use_signal(String::new);` next to `body`; Send-POST greift beide ab und schickt in `SendBulkMailRequest.body` + `body_html` (bereits in DTO seit Phase 23 Plan 04) |
| Inbox-Reply | `component/inbox/reply_form.rs:201` (`reply_body` signal declared Zeile 50) | same | same | Add `reply_body_html` signal; dirty-check kann bei body_html mit-verglichen werden, ODER weiter nur auf body (plain) — beim Planen entscheiden. Empfehlung: nur `body` (plain) für dirty-check, weil der Editor der Author-Sicht folgt |
| Template-Tester | `component/mail_compose/template_tester.rs:45` (nur consumer, kein eigener Editor gebunden; nutzt Preview) | template_tester nimmt `subject: ReadOnlySignal<String>, body: ReadOnlySignal<String>` — es RENDERT keine Editor, aber sein CALLER (der Mail-Template-Editor-Page in `page/mail_templates.rs` o.ä.) hat das Editor-Feld | CALLER muss auch `body_html` tragen und optional an TemplateTester weiterreichen für die HTML-Preview | Beim Planen den Caller lokalisieren: grep `TemplateTester\s*{` |

**Backwards-compat guard:** solange `body_html` leer bleibt (User schreibt Plain-Text ohne Toolbar zu klicken), sollte der POST `body_html: None` senden — sonst schickt Phase 23's create_job ein leer-sanitisiertes String für alle Empfänger. Regel: `if body_html.is_empty() { None } else { Some(body_html) }` in der `SendMailRequest`-Konstruktion.

## Sources

### Primary (HIGH confidence)
- Repo grep `genossi_mail/src/rest.rs:258,276,643-755` — PreviewRequest/Response current shape [VERIFIED: repo read]
- Repo grep `genossi_mail/src/template.rs:101` — `render_html_template` function exists [VERIFIED: repo grep]
- Repo grep `genossi_mail/src/render.rs:54,63` — `RenderedContent { subject, body, body_html }` exists [VERIFIED: repo grep]
- Repo file `genossi-frontend/src/js.rs:100-155` — `copy_with_exec_command` js_sys::Reflect pattern [VERIFIED: repo read]
- Repo file `genossi-frontend/src/js.rs:5-22` + `assets/codemirror-bundle.js` + `index.html:25` + `flake.nix:151` — extern "C" JS interop pattern [VERIFIED: repo read]
- Repo file `genossi-frontend/src/page/templates.rs:484` — `onmounted` prop supported on `<div>` in Dioxus 0.6.3 [VERIFIED: repo read]
- Repo file `genossi-frontend/src/component/mail_compose/template_preview.rs` — existing preview pattern to extend for HTML render [VERIFIED: repo read]
- Repo file `genossi-frontend/src/component/modal.rs` — In-App-Modal component available [VERIFIED: repo read]
- Repo file `.planning/phases/23-html-mail-backend/23-04-SUMMARY.md` — Phase 23 wire complete: `body_html: Option<String>` in all 8 DTOs, sanitize at 4 EPs, autoescape env [VERIFIED: repo read]
- Repo file `genossi-frontend/Cargo.toml` — current web-sys feature list (missing `ClipboardEvent`, `DataTransfer`) [VERIFIED: repo read]
- Repo file `genossi-frontend/src/component/mail_compose/template_tester.rs:113` — `r#type: "button"` pattern with memory-note [VERIFIED: repo read]

### Secondary (MEDIUM confidence)
- [ammonia Builder default allowed elements](https://docs.rs/ammonia/latest/ammonia/struct.Builder.html) — the default tag whitelist including `<b>`, `<i>`, `<u>`, `<s>`, `<strike>`, `<blockquote>`, `<h1>`-`<h6>`, `<ol>`, `<ul>`, `<li>`, `<p>`, `<a>` [CITED]
- [ammonia crate](https://crates.io/crates/ammonia) — sanitization overview [CITED]
- [ammonia clean() function](https://docs.rs/ammonia/latest/ammonia/fn.clean.html) — default-filter entry point [CITED]

### Tertiary (LOW confidence)
- Dioxus 0.6.3 exact `onpaste`/`ClipboardData` API — assumed present, requires 5-line spike at plan time
- `execCommand` exact tag output across Firefox versions — well-documented but assumed for our subset

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — bereits vollständig im Repo verfügbar; keine neuen Abhängigkeiten
- Architecture: HIGH — Component-first + Migration-map ist mechanisch aus dem Repo ableitbar
- Pitfalls: HIGH — Pitfalls 1 (styleWithCSS), 2 (button-reload), 5 (DOM-sync-race), 6 (Selection lost on modal) sind bekannte Fallstricke; 3 (paste-order), 4 (Firefox insertText), 7 (signal-shape), 8 (backend preview seam) sind repo-spezifisch verifiziert

**Research date:** 2026-07-02
**Valid until:** 2026-08-02 (30 Tage; contenteditable/execCommand-Landschaft ändert sich langsam; ammonia-Version-Bumps müssten Whitelist-Änderungen mitbringen um relevant zu werden)
