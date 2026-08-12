# Phase 28: Desktop/Mobile-Vorschau — Pattern Map

**Mapped:** 2026-07-27
**Files analyzed:** 10 (1 neu, 9 modifiziert)
**Analogs found:** 10 / 10

---

## File Classification

| Neue/Modifizierte Datei | Rolle | Data Flow | Nächster Analog | Match |
|---|---|---|---|---|
| `genossi-frontend/src/component/mail_compose/mail_preview_frame.rs` (NEU) | component + pure fns + grep-gate | request-response (indirekt) / transform | `template_preview.rs` (dangerous_inner_html + Grep-Gate) & `wysiwyg_editor.rs` (Grep-Gate) & `qr_scanner.rs` (Quoted-Attr) | exact (kombiniert) |
| `wysiwyg_editor.rs` | component | event-driven | `src/page/templates.rs` (`PreviewMode`-Enum + Segmented-Control) | role-match |
| `mail_compose/mod.rs` | module/barrel | — | sich selbst (Z. 1–19) | exact |
| `wysiwyg_toolbar.rs` | utility (pure fn) | transform | `image_insert_html` selbst (Z. 44–48) | exact |
| `template_preview.rs` | component | request-response | `template_tester.rs` (`#[props(default)]`-Migration) | exact |
| `page/mail_page.rs` | page (call-site) | request-response | eigener Block Z. 421–445 | exact |
| `page/mail_templates.rs` | page (call-site) | request-response | eigener Block Z. 326–355 | exact |
| `component/inbox/reply_form.rs` | component (call-site) | request-response | eigener Block Z. 236–270 | exact |
| `src/i18n/{mod,de,en}.rs` | config/i18n | — | `MailEditorPreviewHtml`-Key-Trio | exact |
| `genossi_mail/src/rest.rs` | REST-Handler | request-response | `service.rs:564` (`sanitize_body_html_opt`-Aufrufstelle) | exact |

---

## Pattern Assignments

### 1. `mail_preview_frame.rs` (NEU) — Grep-Gate-Testmodul

**Analog A:** `genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs:307–477`
**Analog B:** `genossi-frontend/src/component/mail_compose/template_preview.rs:219–266`

Die Self-Reference-Hazard-Abwehr ist zweischichtig und MUSS wörtlich reproduziert werden:
(a) Source vor dem Marker `mod grep_gate_tests` abschneiden, (b) Needles zur Laufzeit via `format!` zusammensetzen.

**Kompakte Vorlage (aus `template_preview.rs:219–266`, wörtlich):**

```rust
// Grep-gate tests below — module-level docstring intentionally omitted so no
// literal class-name strings live in `production_region()`. See test docs.
#[cfg(test)]
mod grep_gate_tests {
    const PREVIEW_SRC: &str = include_str!("template_preview.rs");
    const TEST_MODULE_MARKER: &str = "mod grep_gate_tests";

    fn production_region() -> &'static str {
        let cutoff = PREVIEW_SRC
            .find(TEST_MODULE_MARKER)
            .expect("BUG: grep-gate test module marker not found");
        &PREVIEW_SRC[..cutoff]
    }

    #[test]
    fn preview_uses_mail_html_render_scope() {
        let region = production_region();
        let scope_needle = format!("mail-html-rende{tail}", tail = "r");
        let prose_needle = format!("pros{tail}", tail = "e ");
        assert!(
            region.contains(&scope_needle),
            "Grep gate FAILED: expected `mail-html-render` class on the preview \
             div in template_preview.rs (production region). Without it the \
             preview flattens h1/ul/ol/blockquote to plain text."
        );
        assert!(
            !region.contains(&prose_needle),
            "Grep gate FAILED: the `prose ` class is back in template_preview.rs. \
             It is a no-op because Tailwind Typography is not installed."
        );
    }

    #[test]
    fn production_region_excludes_test_module() {
        let region = production_region();
        assert!(
            !region.contains(TEST_MODULE_MARKER),
            "BUG: production_region() slice still contains the test module marker"
        );
        assert!(region.len() < PREVIEW_SRC.len());
    }
}
```

**Ausführliche Variante mit Doc-Kommentar zur Hazard-Abwehr (`wysiwyg_editor.rs:307–348`, wörtlich — den Wortlaut für den Doc-Block der neuen Datei adaptieren):**

```rust
/// SELF-REFERENCE HAZARD (Deviation Rule 1 fix during Plan 26-02 execution):
/// The naive pattern `EDITOR_SRC.contains("target-literal")` produces a
/// **false positive** because the literal in the test's own source becomes
/// part of `EDITOR_SRC` via `include_str!`. To avoid this, we:
///   (a) Slice `EDITOR_SRC` to only the region BEFORE the test module marker,
///       so the test module's own bytes are excluded from the search range.
///   (b) Assemble target substrings at runtime via `format!`/concat so no
///       single literal byte sequence in the test source could satisfy the
///       search even if (a) failed.
/// Both defences run together; removing the guard in production code
/// (line ~77 or the `evt.prevent_default()` on line ~89) reliably trips
/// the assertion. Verified via manual negative-proof — see 26-02-SUMMARY.md.
#[cfg(test)]
mod grep_gate_tests {
    const EDITOR_SRC: &str = include_str!("wysiwyg_editor.rs");

    /// Marker string that begins the test module itself. Everything from
    /// this point on is EXCLUDED from the grep-search region, so the
    /// literals embedded in the assertions below cannot satisfy their
    /// own contains() checks (self-reference hazard, see module doc).
    const TEST_MODULE_MARKER: &str = "mod grep_gate_tests";

    fn production_region() -> &'static str {
        let cutoff = EDITOR_SRC
            .find(TEST_MODULE_MARKER)
            .expect("BUG: grep-gate test module marker not found; the marker string must appear verbatim before `mod grep_gate_tests` opens");
        &EDITOR_SRC[..cutoff]
    }
```

**Muster für ein Multi-Assertion-Gate mit Fenster-Suche (`wysiwyg_editor.rs:422–455`, gekürzt):**

```rust
    #[test]
    fn native_drop_target_wired_and_prevents_default() {
        let region = production_region();

        // (a) onmounted must invoke the native attach helper.
        let attach_needle = format!("attach_image_drop_targe{tail}", tail = "t(");
        assert!(region.contains(&attach_needle), "Grep gate FAILED: ...");

        // (b) the native helper must register a `drop` listener ...
        let listen_needle = format!("add_event_listener_with_callbac{tail}", tail = "k(");
        let drop_literal = format!("{q}dro{tail}{q}", q = "\"", tail = "p");
        assert!(region.contains(&listen_needle) && region.contains(&drop_literal), "...");
    }
```

**Anwendung für D-11 (Planner):** `SRC` = `include_str!("mail_preview_frame.rs")`; Needles zur Laufzeit bauen, z. B.
`format!("{q}sandbo{tail}{q}", q = "\"", tail = "x")`, `format!("allow-script{tail}", tail = "s")` (negativ),
`format!("srcdo{tail}", tail = "c:")` (positiv), `format!("dangerous_inner_htm{tail}", tail = "l")` (negativ).
Meta-Test `production_region_excludes_test_module` mit übernehmen.

---

### 2. `mail_preview_frame.rs` (NEU) — Quoted-Custom-Attribute in RSX

**Analog:** `genossi-frontend/src/component/qr_scanner.rs:300–308`

```rust
    rsx! {
        div { class: "fixed inset-0 z-50 bg-black/80 flex items-center justify-center p-4",
            div { class: "relative bg-black aspect-square w-full max-w-md rounded-lg overflow-hidden",
                button {
                    class: "absolute top-2 right-2 text-white text-2xl z-10 px-3 py-1",
                    "aria-label": "Schließen",
                    onclick: move |_| on_cancel.call(()),
                    "\u{00D7}"
                }
```

Die Zeile `"aria-label": "Schließen",` ist die Syntax, die für `"sandbox": "allow-same-origin",` zu verwenden ist —
`sandbox` ist in `dioxus-html 0.6.3` auskommentiert, ein unquoted `sandbox:` kompiliert nicht.

---

### 3. `mail_preview_frame.rs` (NEU) — `dangerous_inner_html`-Gegenstück / HTML-Einbettung

**Analog:** `template_preview.rs:178–195` — zeigt, wie das Projekt heute backend-gerendertes HTML einbettet **und wie es kommentiert wird** (Rationale-Kommentar über der Zeile). Die neue Component ersetzt `dangerous_inner_html` durch `srcdoc` (PREV-05); der Kommentarstil bleibt.

```rust
                        // Phase 24 Plan 03 Task 5 (EDIT-05, D-04): HTML preview
                        // block. Renders the backend-rendered body_html via
                        // Dioxus's `dangerous_inner_html`. This is safe because
                        // the backend rendered via the autoescape env (Phase 23
                        // D-04, member-supplied values HTML-escaped) AND passes
                        // through the ammonia allow-list at the store boundary
                        // when persisted (Phase 23 D-03).
                        if let Some(html) = preview.body_html.as_ref() {
                            p { class: "text-xs font-medium text-gray-500 mt-3 mb-1",
                                {i18n.t(Key::MailEditorPreviewHtml)}
                            }
                            div {
                                class: "mail-html-render border rounded p-3 text-sm bg-gray-50",
                                dangerous_inner_html: "{html}",
                            }
                        }
```

**Fehler-Block, den der Preview-Modus wiederverwenden soll** (Open Question 1 → „ja"), `template_preview.rs:155–162`:

```rust
            if let Some(preview) = preview_result.read().as_ref() {
                if !preview.errors.is_empty() {
                    div { class: "bg-red-50 border border-red-200 rounded p-3 text-sm text-red-700",
                        p { class: "font-medium mb-1", {i18n.t(Key::MailTemplateError)} }
                        for err in preview.errors.iter() {
                            p { "{err}" }
                        }
                    }
```

---

### 4. `wysiwyg_editor.rs` — Enum-State + Segmented-Control (Projekt-Regel „Enum statt Boolean")

**Analog (bester Treffer):** `genossi-frontend/src/page/templates.rs` — 2-wertiges Modus-Enum mit Button-Gruppe.
Zweiter Beleg für Enum-State mit Payload: `src/page/mail_templates.rs:13–18` (`EditorMode { None, Create, Edit(String) }`).

**Definition** (`templates.rs:13–17`):
```rust
#[derive(Clone, Copy, PartialEq, Eq)]
enum PreviewMode {
    Member,
    Application,
}
```

**Signal** (`templates.rs:124`):
```rust
    let mut preview_mode = use_signal(|| PreviewMode::Member);
```

**Segmented-Control im RSX** (`templates.rs:489–511`, wörtlich — Vorlage für die drei Modus-Buttons; beachte `r#type: "button"` und den `class: if …`-Ausdruck):
```rust
                                // Toggle tabs
                                div { class: "flex rounded-md overflow-hidden border border-gray-300",
                                    button {
                                        class: if *preview_mode.read() == PreviewMode::Member { "px-3 py-1 text-sm bg-blue-600 text-white" } else { "px-3 py-1 text-sm bg-white text-gray-700 hover:bg-gray-100" },
                                        r#type: "button",
                                        onclick: move |_| {
                                            preview_mode.set(PreviewMode::Member);
                                            preview_application_id.set(None);
                                        },
                                        {i18n.t(Key::PreviewMember)}
                                    }
                                    button {
                                        class: if *preview_mode.read() == PreviewMode::Application { "px-3 py-1 text-sm bg-blue-600 text-white" } else { "px-3 py-1 text-sm bg-white text-gray-700 hover:bg-gray-100" },
                                        r#type: "button",
                                        onclick: move |_| {
                                            preview_mode.set(PreviewMode::Application);
                                            preview_member_id.set(None);
                                        },
                                        {i18n.t(Key::PreviewApplication)}
                                    }
                                }
```

**Match-im-RSX-Muster** (`templates.rs:529–530`):
```rust
                                        PreviewMode::Member => preview_member_id.read().is_some(),
                                        PreviewMode::Application => preview_application_id.read().is_some(),
```

> **Namenskollision beachten:** `PreviewMode` existiert bereits privat in `src/page/templates.rs`. Das neue
> `mail_preview_frame::PreviewMode` ist ein anderer Typ in einem anderen Modul — kein Konflikt, aber der
> Planner sollte den vollqualifizierten Pfad in Plan-Texten nennen.

---

### 5. `wysiwyg_editor.rs` — bestehende Signatur + Off-Screen-Hide-Ziel

**Aktuelle Signatur** (`wysiwyg_editor.rs:38–41`) — hier kommen die zwei `#[props(default)]`-Felder aus D-03 dazu:
```rust
#[component]
pub fn WysiwygEditor(value: String, on_change: EventHandler<(String, String)>) -> Element {
    let mut link_dialog_open = use_signal(|| false);
    let mut saved_range = use_signal(|| None::<Range>);
```

**Der contenteditable-Container, der im Preview-Modus off-screen gehen muss** (`wysiwyg_editor.rs:71–74`) — `class` wird zum bedingten Ausdruck, `id`/`contenteditable` bleiben unverändert (D-17):
```rust
            div {
                id: EDITOR_ID,
                class: "w-full px-3 py-2 min-h-40 focus:outline-none mail-html-render",
                contenteditable: "true",
```

**`sync_from_dom` — vor dem Moduswechsel aufrufen (Pitfall 1, Schicht 2)** (`wysiwyg_editor.rs:159–173`):
```rust
fn sync_from_dom(on_change: &EventHandler<(String, String)>) {
    let Some(doc) = doc() else { return; };
    let Some(el) = doc.get_element_by_id(EDITOR_ID) else { return; };
    let html = el.inner_html();
    // D-02: innerText not textContent so intentional line breaks survive.
    let plain = el
        .dyn_ref::<web_sys::HtmlElement>()
        .map(|he| he.inner_text())
        .unwrap_or_default();
    on_change.call((plain, html));
}
```

**Async-Request aus einer Component heraus** (`template_preview.rs:35–64`) — Vorlage für den Preview-Fetch beim Moduswechsel (`spawn` + `CONFIG.read().clone()` + `api::preview_mail` + Error-in-Signal):
```rust
    spawn(async move {
        preview_loading.set(true);
        let config = CONFIG.read().clone();
        match api::preview_mail(
            &config, &subj, &b, &mid_str, repayment_phase_id, body_html_opt.as_deref(),
        ).await {
            Ok(result) => preview_result.set(Some(result)),
            Err(e) => preview_result.set(Some(PreviewResponse {
                subject: String::new(),
                body: String::new(),
                body_html: None,
                errors: vec![e.to_string()],
                used_dummy_repayment: false,
            })),
        }
        preview_loading.set(false);
    });
```

---

### 6. `wysiwyg_toolbar.rs` — `image_insert_html` als Vorlage / Helper-Extraktion

**Analog = die Funktion selbst** (`wysiwyg_toolbar.rs:31–48`, wörtlich). Der Doc-Kommentar enthält die Pitfall-4-Rationale und ist beim Refactor zu erhalten:

```rust
/// Pure helper producing the inline-image markup inserted at the caret.
///
/// Emits `<img data-genossi-asset-id="{id}" src="{backend}/api/mail/assets/{id}/bytes">`.
/// The `src` is built from `config.backend` — exactly like every other API call
/// (`format!("{}/api/...", config.backend)`) — so the live preview resolves to
/// the same base the working requests use in every environment. A relative
/// `/api/...` src would bypass `config.backend` and 404 on deployments where the
/// browser-visible API base is not the page origin (e.g. beta, where
/// `config.backend` already carries an `/api` segment consumed by the proxy).
/// The `src` is a convenience for the live editor only — 27-02's sanitizer
/// strips it on store, so only `data-genossi-asset-id` persists (T-27-17).
/// Both the toolbar button and the editor drag&drop handler reuse this
/// helper so the inserted shape is identical.
pub(crate) fn image_insert_html(backend: &str, id: &str) -> String {
    format!(
        r#"<img data-genossi-asset-id="{id}" src="{backend}/api/mail/assets/{id}/bytes">"#
    )
}
```

Sichtbarkeitskonvention: `pub(crate)` für pure Helper, `pub` nur für Components (`mod.rs`-Re-Export).
Aufrufstelle, die nach der Extraktion unverändert bleiben muss: `wysiwyg_editor.rs:231`
(`let img_html = image_insert_html(&config.backend, &asset.id.to_string());`).

---

### 7. `template_preview.rs` — Prop-Hochziehen eines `use_signal`

**Es gibt kein exaktes Repo-Vorbild** für „Signal von Child zu Page hochgezogen". Das nächste Muster ist die
rückwärtskompatible Prop-Ergänzung mit `#[props(default)]` aus Phase 24 — daran orientiert sich der Refactor.

**Vorbild `#[props(default)]`-Migration** (`template_tester.rs:44–56`):
```rust
#[component]
pub fn TemplateTester(
    subject: ReadOnlySignal<String>,
    body: ReadOnlySignal<String>,
    // Phase 24 Plan 03 Task 4 (EDIT-01, D-01): HTML sibling of `body` —
    // forwarded to TemplatePreview so the Live-Preview renders the
    // backend's HTML sibling (Phase 24 Plan 01 Task 1 extended preview_mail).
    // Defaults to an empty ReadOnlySignal via #[props(default)] so existing
    // callers stay source-compat.
    #[props(default)] body_html: ReadOnlySignal<String>,
) -> Element {
    let i18n = use_i18n();
    let mut selected_member_id = use_signal(|| None::<Uuid>);
```

**Ist-Signatur `TemplatePreview`** (`template_preview.rs:67–84`) — Zeile 82 ist das hochzuziehende Signal:
```rust
#[component]
pub fn TemplatePreview(
    subject: ReadOnlySignal<String>,
    body: ReadOnlySignal<String>,
    #[props(default)] body_html: ReadOnlySignal<String>,
    member_ids: Vec<Uuid>,
    // UAT-Defekt #6: optional Repayment-Kontext, damit Live-Preview im
    // Phase-12-Flow `{{ payout_amount }}` etc. korrekt rendert.
    #[props(default)] repayment_phase_id: Option<Uuid>,
) -> Element {
    let i18n = use_i18n();
    let mut preview_member_id = use_signal(|| None::<Uuid>);   // ← D-03: wird Prop
    let mut preview_result = use_signal(|| None::<PreviewResponse>);
    let preview_loading = use_signal(|| false);
```

`preview_member_id` wird an fünf Stellen gelesen/geschrieben: `:98`, `:100` (im `select`-onchange),
`:134` (Refresh-Button-Sichtbarkeit), `:140` (Refresh-Klick), `:210` (Leer-Hinweis).

**Die drei Call-Sites — wörtliche Ist-Aufrufe:**

`page/mail_page.rs:421–445`:
```rust
                            {
                                let editor_key = selected_template_id
                                    .read()
                                    .clone()
                                    .unwrap_or_else(|| "__no_template__".to_string());
                                rsx! {
                                    WysiwygEditor {
                                        key: "{editor_key}",
                                        value: body_html.read().clone(),
                                        on_change: move |(plain, html): (String, String)| {
                                            body.set(plain);
                                            body_html.set(html);
                                        },
                                    }
                                }
                            }
                            TemplatePreview {
                                subject: subject,
                                body: body,
                                body_html: body_html,
                                member_ids: selected_member_ids.read().clone(),
                                // UAT-Defekt #6: Live-Preview soll Repayment-Vars rendern
                                repayment_phase_id: *repayment_phase_id.read(),
                            }
```

`page/mail_templates.rs:326–355` (nutzt `TemplateTester`, nicht `TemplatePreview` direkt):
```rust
                                            {
                                                let editor_key = match &*editor_mode.read() {
                                                    EditorMode::Edit(id) => id.clone(),
                                                    EditorMode::Create => "__create__".to_string(),
                                                    EditorMode::None => String::new(),
                                                };
                                                rsx! {
                                                    WysiwygEditor {
                                                        key: "{editor_key}",
                                                        value: edit_body_html.read().clone(),
                                                        on_change: move |(plain, html): (String, String)| {
                                                            edit_body.set(plain);
                                                            edit_body_html.set(html);
                                                        },
                                                    }
                                                }
                                            }
                                        }
                                        TemplateTester {
                                            subject: edit_subject,
                                            body: edit_body,
                                            body_html: edit_body_html,
                                        }
```
Innerhalb `TemplateTester` (`:56`, `:83`, `:87`, `:95–96`) existiert bereits `selected_member_id`,
gespeist von `MemberSearch { on_select … , selected_id: *selected_member_id.read() }`, und
`if let Some(mid) = *selected_member_id.read() { TemplatePreview { … member_ids: vec![mid] … } }`.

`component/inbox/reply_form.rs:236–270` (Ausstiegsklausel D-03 greift — genau ein Member):
```rust
            {
                let editor_key = format!("reply-{}", *editor_reset_counter.read());
                rsx! {
                    WysiwygEditor {
                        key: "{editor_key}",
                        value: reply_body_html.read().clone(),
                        on_change: move |(plain, html): (String, String)| {
                            reply_body.set(plain);
                            reply_body_html.set(html);
                        },
                    }
                }
            }
            …
            if assigned_member_id.is_some() {
                {
                    let member_ids: Vec<Uuid> = member_uuid_opt.into_iter().collect();
                    rsx! {
                        TemplatePreview {
                            subject: reply_subject,
                            body: reply_body,
                            body_html: reply_body_html,
                            member_ids: member_ids,
                        }
                    }
                }
            }
```
→ `preview_member_id: member_uuid_opt` direkt an `WysiwygEditor` durchreichen; `TemplatePreview` dort unverändert lassen.

**Submit-Guard, der von Pitfall 1 betroffen ist** (`reply_form.rs:281–292`) — liest `#wysiwyg-editor` direkt per `inner_text()`; identische Blöcke existieren in `mail_page.rs` und `mail_templates.rs`:
```rust
                        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                            if let Some(el) = doc.get_element_by_id("wysiwyg-editor") {
                                let html = el.inner_html();
                                let plain = wasm_bindgen::JsCast::dyn_ref::<web_sys::HtmlElement>(&el)
                                    .map(|he| he.inner_text())
                                    .unwrap_or_default();
                                reply_body.set(plain);
                                reply_body_html.set(html);
                            }
                        }
```

---

### 8. `mail_compose/mod.rs` — Re-Export

**Analog = die Datei selbst** (`mod.rs:1–19`). Zwei Einfügungen, alphabetisch:
```rust
pub mod attachment_picker;
pub mod subject_input;
pub mod template_preview;
…
pub mod wysiwyg_toolbar;

pub use attachment_picker::MailAttachmentPicker;
pub use subject_input::MailSubjectInput;
pub use template_preview::TemplatePreview;
…
pub use wysiwyg_editor::{plain_to_html, WysiwygEditor};
```
→ `pub mod mail_preview_frame;` (nach `attachment_picker`, vor `subject_input`) und
`pub use mail_preview_frame::MailPreviewFrame;` in denselben Blöcken.

---

### 9. `i18n/{mod,de,en}.rs` — Key-Trio

**Analog:** `MailEditorPreviewHtml` — der Key liegt im `MailEditor*`-Cluster, exakt dort gehören die neuen `MailEditorMode*`-Keys hin.

`src/i18n/mod.rs:296–300` (Enum-Varianten):
```rust
    MailEditorLinkUrlLabel,
    MailEditorLinkTextLabel,
    MailEditorLinkInsert,
    MailEditorLinkCancel,
    MailEditorPreviewHtml,
```

`src/i18n/de.rs:236–240` (Match-Arms):
```rust
        Key::MailEditorLinkUrlLabel => "URL".into(),
        Key::MailEditorLinkTextLabel => "Anzeige-Text (optional)".into(),
        Key::MailEditorLinkInsert => "Einfügen".into(),
        Key::MailEditorLinkCancel => "Abbrechen".into(),
        Key::MailEditorPreviewHtml => "HTML-Vorschau".into(),
```

`src/i18n/en.rs:236–240`:
```rust
        Key::MailEditorLinkUrlLabel => "URL".into(),
        Key::MailEditorLinkTextLabel => "Display text (optional)".into(),
        Key::MailEditorLinkInsert => "Insert".into(),
        Key::MailEditorLinkCancel => "Cancel".into(),
        Key::MailEditorPreviewHtml => "HTML preview".into(),
```

**Regel:** Jeder neue Key wird in **allen drei** Dateien an derselben relativen Position ergänzt
(`mod.rs` Variante → `de.rs` Arm → `en.rs` Arm), im selben Commit. Mehrzeilige Werte nutzen die
Block-Form `Key::X => { "…".into() }` (siehe `MailAttachRepaymentLetter`, de.rs:241–243).

---

### 10. `genossi_mail/src/rest.rs` — Sanitize-Aufruf im `preview_mail`-Handler

**Helper-Signatur** (`genossi_mail/src/service.rs:280–289`, wörtlich):
```rust
/// Phase 23 D-03 entry point 4 (helper for `send_test_mail_with_body`).
///
/// Extracted as a free function so the sanitize wire is testable in isolation
/// without spinning up SMTP mocks. Mirrors the inline sanitize step used at
/// the other three D-03 entry points (`create_job`, template create/update).
///
/// `None` in ⇒ `None` out (no `Some("")` sentinel; RESEARCH Pitfall 4).
pub(crate) fn sanitize_body_html_opt(body_html: Option<&str>) -> Option<String> {
    body_html.map(crate::sanitize::sanitize_html)
}
```

**Bestehende Aufrufstellen** (Stil-Vorlage):
- `genossi_mail/src/service.rs:564` — `let sanitized_html = sanitize_body_html_opt(body_html.as_deref());`
- `genossi_mail/src/inbox.rs:694` — `body_html: crate::service::sanitize_body_html_opt(body_html.as_deref()).map(Arc::from),`

**Der zu ersetzende Ist-Block** (`genossi_mail/src/rest.rs:766–779`, wörtlich — Kommentar Z. 768–769 wird durch D-02 falsch und MUSS mit ersetzt werden):
```rust
            // Phase 24 (EDIT-05, D-04): if the caller supplied an HTML sibling,
            // render it through the autoescape env (member values escaped, author
            // markup structurally preserved). Read-only preview — no sanitization
            // here; ammonia guards the store boundary (Phase 23 D-03).
            let rendered_body_html: Option<String> = match body.body_html.as_deref() {
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

**Unmittelbar folgender Block, der unverändert bleibt** (`rest.rs:781–789`) — belegt Pitfall 7 (`rendered_body` wird ohnehin überschrieben):
```rust
            let rendered_body = match rendered_body_html.as_deref() {
                Some(html) => crate::render::plain_from_html(html),
                None => rendered_body,
            };
```

---

## Shared Patterns

### Kommentar-Konvention: Phasen-/Decision-Referenz über jedem nicht-offensichtlichen Block
**Quelle:** durchgängig, z. B. `wysiwyg_editor.rs:77–78`, `template_preview.rs:178–186`, `rest.rs:766–769`
**Anwenden auf:** alle neuen/geänderten Codeblöcke dieser Phase.
Form: `// Phase 28 (PREV-0X, D-YY): <Was> — <Warum>`. Wenn ein bestehender Kommentar durch die Änderung
falsch wird, MUSS er mitgeändert werden (explizit für `rest.rs:768–769`).

### `r#type: "button"` an jedem Button
**Quelle:** `templates.rs:493`, `template_preview.rs:137`, `reply_form.rs:272`
**Anwenden auf:** die drei Modus-Buttons in `wysiwyg_editor.rs`.

### `#[props(default)]` für rückwärtskompatible Prop-Ergänzung
**Quelle:** `template_preview.rs:75`, `template_preview.rs:79`, `template_tester.rs:53`
**Anwenden auf:** `WysiwygEditor { preview_member_id, repayment_phase_id }` und `TemplatePreview { preview_member_id }` —
so bleibt jede Call-Site einzeln umstellbar, kein Big-Bang-Commit.

### Pure-Funktion + `#[cfg(test)] mod tests` in derselben Datei
**Quelle:** `wysiwyg_editor.rs:249–305` (`plain_to_html` + 6 Tests)
```rust
#[cfg(test)]
mod tests {
    use super::plain_to_html;

    #[test]
    fn empty_input_stays_empty() {
        assert_eq!(plain_to_html(""), "");
    }
    …
}
```
**Anwenden auf:** `inject_asset_src`, `preview_srcdoc`, `PreviewMode::width_px`, `asset_bytes_url`.
Beachte: zwei getrennte Testmodule pro Datei sind etabliert — `mod tests` (Verhalten) **und**
`mod grep_gate_tests` (Source-Invarianten). Der Marker `mod grep_gate_tests` muss der letzte sein.

### Frontend-Testkommando
`cd genossi-frontend && cargo test` (kein Workspace-Member, kein Lib-Target, kein wasm32-Target).

---

## No Analog Found

| Datei/Baustein | Rolle | Data Flow | Grund |
|---|---|---|---|
| `srcdoc`-basierter `<iframe>` in RSX | component | — | Kein `<iframe>` im gesamten Frontend. RESEARCH Pattern 4 (28-RESEARCH.md:427–498) ist die Vorlage; Attribut-Syntax aus `qr_scanner.rs` übernehmen. |
| „Signal von Child in Page hochziehen"-Refactor | component | — | Kein Präzedenzfall im Repo. Ersatz: `#[props(default)]`-Migration aus Phase 24 (siehe Pattern 7) + die drei wörtlich zitierten Call-Sites. |
| String-Rewrite `data-genossi-asset-id` → `src` im Frontend | utility | transform | Kein Frontend-Analog. Backend-Vorlage: `genossi_mail/src/render.rs:293–345` (`rewrite_img_cids` / `extract_asset_id`); Zielcode steht in 28-RESEARCH.md:342–413. |

---

## Metadata

**Analog search scope:** `genossi-frontend/src/{component,page,i18n}`, `genossi_mail/src/{rest,service,sanitize,render}.rs`
**Files scanned:** 14 gelesen, ~40 gegrept
**Pattern extraction date:** 2026-07-27

*Phase: 28-desktop-mobile-vorschau*
