//! Phase 24 Plan 02 — WYSIWYG Toolbar.
//!
//! Button row rendered above the contenteditable div inside
//! `WysiwygEditor`. Every button emits an execCommand that produces a tag
//! from the ammonia allow-list at the store boundary (Phase 23 D-03):
//! `b, i, u, s, ol, ul, li, blockquote, h1, h2, h3, p, a[href]`.
//!
//! Ordering rule per Pitfall 5 of 24-RESEARCH.md: every non-Link button
//! MUST (a) focus the editor, (b) run execCommand, (c) fire `on_command`
//! so the parent re-reads innerHTML+innerText and updates its signals.
//! Skipping the parent-sync step means the DOM mutation is invisible to
//! the outside world until the next keystroke.
//!
//! The Link button is special: it fires `on_link_click` instead of running
//! `createLink` directly, because the parent needs to capture the current
//! Selection Range BEFORE the modal opens (Pitfall 6 — the modal steals
//! focus and destroys the caret position).

use dioxus::prelude::*;
use wasm_bindgen::JsCast;

use crate::api;
use crate::i18n::{use_i18n, Key};
use crate::service::config::CONFIG;

/// Stable DOM id for the hidden file input that backs the image toolbar
/// button. The button's onclick programmatically clicks this input to open
/// the OS file picker without rendering a visible `<input>`.
const IMAGE_INPUT_ID: &str = "wysiwyg-image-input";

/// Phase 28 (PREV-03, D-06): Single Source of Truth für die browser-sichtbare
/// Asset-Bytes-URL.
///
/// Zwei Stellen erzeugen dieselbe URL in unterschiedlichen Markup-Formen: der
/// Editor-Insert in [`image_insert_html`] (kompletter `<img>`-Tag) und die
/// iframe-Vorschau in
/// `crate::component::mail_compose::mail_preview_frame::inject_asset_src`
/// (nachträglich eingefügtes `src`-Attribut). Die *Markup*-Formen dürfen
/// auseinandergehen, die *URL* nicht — eine Route-Änderung darf nicht an zwei
/// Stellen gepflegt werden müssen, weil die zweite Stelle sonst stillschweigend
/// zurückbleibt und die Bilder nur in einem der beiden Kontexte laden.
pub(crate) fn asset_bytes_url(backend: &str, id: &str) -> String {
    format!("{backend}/api/mail/assets/{id}/bytes")
}

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
        r#"<img data-genossi-asset-id="{id}" src="{src}">"#,
        src = asset_bytes_url(backend, id)
    )
}

#[component]
pub fn WysiwygToolbar(
    editor_id: String,
    on_command: EventHandler<()>,
    on_link_click: EventHandler<()>,
) -> Element {
    let i18n = use_i18n();

    // Helper that all execCommand buttons share: focus the editor, run the
    // dispatch closure, notify parent to re-sync DOM->signals.
    // We inline this per-button as a closure factory because Dioxus event
    // handlers own their captures and cannot share a Fn across buttons.
    let editor_id_a = editor_id.clone();
    let editor_id_b = editor_id.clone();
    let editor_id_c = editor_id.clone();
    let editor_id_d = editor_id.clone();
    let editor_id_e = editor_id.clone();
    let editor_id_f = editor_id.clone();
    let editor_id_g = editor_id.clone();
    let editor_id_h = editor_id.clone();
    let editor_id_i = editor_id.clone();
    let editor_id_j = editor_id.clone();
    let editor_id_k = editor_id.clone();
    let editor_id_l = editor_id.clone();
    let editor_id_m = editor_id.clone();

    rsx! {
        div { class: "flex flex-wrap gap-1 border-b px-2 py-1 bg-gray-50",
            // Bold
            button {
                r#type: "button",
                // Selection-preserve: mousedown → blur destroys the editor's Range before onclick.
                onmousedown: move |evt| { evt.prevent_default(); },
                class: "px-2 py-1 text-sm font-bold hover:bg-gray-200 rounded",
                title: "{i18n.t(Key::MailEditorBold)}",
                onclick: move |evt| {
                    evt.prevent_default();
                    focus_editor(&editor_id_a);
                    if let Some(doc) = doc() {
                        let _ = crate::js::exec_command_simple(&doc, "bold");
                    }
                    on_command.call(());
                },
                "B"
            }
            // Italic
            button {
                r#type: "button",
                // Selection-preserve: mousedown → blur destroys the editor's Range before onclick.
                onmousedown: move |evt| { evt.prevent_default(); },
                class: "px-2 py-1 text-sm italic hover:bg-gray-200 rounded",
                title: "{i18n.t(Key::MailEditorItalic)}",
                onclick: move |evt| {
                    evt.prevent_default();
                    focus_editor(&editor_id_b);
                    if let Some(doc) = doc() {
                        let _ = crate::js::exec_command_simple(&doc, "italic");
                    }
                    on_command.call(());
                },
                "I"
            }
            // Underline
            button {
                r#type: "button",
                // Selection-preserve: mousedown → blur destroys the editor's Range before onclick.
                onmousedown: move |evt| { evt.prevent_default(); },
                class: "px-2 py-1 text-sm underline hover:bg-gray-200 rounded",
                title: "{i18n.t(Key::MailEditorUnderline)}",
                onclick: move |evt| {
                    evt.prevent_default();
                    focus_editor(&editor_id_c);
                    if let Some(doc) = doc() {
                        let _ = crate::js::exec_command_simple(&doc, "underline");
                    }
                    on_command.call(());
                },
                "U"
            }
            // Strikethrough
            button {
                r#type: "button",
                // Selection-preserve: mousedown → blur destroys the editor's Range before onclick.
                onmousedown: move |evt| { evt.prevent_default(); },
                class: "px-2 py-1 text-sm line-through hover:bg-gray-200 rounded",
                title: "{i18n.t(Key::MailEditorStrike)}",
                onclick: move |evt| {
                    evt.prevent_default();
                    focus_editor(&editor_id_d);
                    if let Some(doc) = doc() {
                        let _ = crate::js::exec_command_simple(&doc, "strikeThrough");
                    }
                    on_command.call(());
                },
                "S"
            }
            // Unordered list
            button {
                r#type: "button",
                // Selection-preserve: mousedown → blur destroys the editor's Range before onclick.
                onmousedown: move |evt| { evt.prevent_default(); },
                class: "px-2 py-1 text-sm hover:bg-gray-200 rounded",
                title: "{i18n.t(Key::MailEditorUnorderedList)}",
                onclick: move |evt| {
                    evt.prevent_default();
                    focus_editor(&editor_id_e);
                    if let Some(doc) = doc() {
                        let _ = crate::js::exec_command_simple(&doc, "insertUnorderedList");
                    }
                    on_command.call(());
                },
                "•"
            }
            // Ordered list
            button {
                r#type: "button",
                // Selection-preserve: mousedown → blur destroys the editor's Range before onclick.
                onmousedown: move |evt| { evt.prevent_default(); },
                class: "px-2 py-1 text-sm hover:bg-gray-200 rounded",
                title: "{i18n.t(Key::MailEditorOrderedList)}",
                onclick: move |evt| {
                    evt.prevent_default();
                    focus_editor(&editor_id_f);
                    if let Some(doc) = doc() {
                        let _ = crate::js::exec_command_simple(&doc, "insertOrderedList");
                    }
                    on_command.call(());
                },
                "1."
            }
            // Heading 1
            button {
                r#type: "button",
                // Selection-preserve: mousedown → blur destroys the editor's Range before onclick.
                onmousedown: move |evt| { evt.prevent_default(); },
                class: "px-2 py-1 text-sm font-semibold hover:bg-gray-200 rounded",
                title: "{i18n.t(Key::MailEditorHeading1)}",
                onclick: move |evt| {
                    evt.prevent_default();
                    focus_editor(&editor_id_g);
                    if let Some(doc) = doc() {
                        let _ = crate::js::exec_command_str(&doc, "formatBlock", "<h1>");
                    }
                    on_command.call(());
                },
                "H1"
            }
            // Heading 2
            button {
                r#type: "button",
                // Selection-preserve: mousedown → blur destroys the editor's Range before onclick.
                onmousedown: move |evt| { evt.prevent_default(); },
                class: "px-2 py-1 text-sm font-semibold hover:bg-gray-200 rounded",
                title: "{i18n.t(Key::MailEditorHeading2)}",
                onclick: move |evt| {
                    evt.prevent_default();
                    focus_editor(&editor_id_h);
                    if let Some(doc) = doc() {
                        let _ = crate::js::exec_command_str(&doc, "formatBlock", "<h2>");
                    }
                    on_command.call(());
                },
                "H2"
            }
            // Heading 3
            button {
                r#type: "button",
                // Selection-preserve: mousedown → blur destroys the editor's Range before onclick.
                onmousedown: move |evt| { evt.prevent_default(); },
                class: "px-2 py-1 text-sm font-semibold hover:bg-gray-200 rounded",
                title: "{i18n.t(Key::MailEditorHeading3)}",
                onclick: move |evt| {
                    evt.prevent_default();
                    focus_editor(&editor_id_i);
                    if let Some(doc) = doc() {
                        let _ = crate::js::exec_command_str(&doc, "formatBlock", "<h3>");
                    }
                    on_command.call(());
                },
                "H3"
            }
            // Paragraph
            button {
                r#type: "button",
                // Selection-preserve: mousedown → blur destroys the editor's Range before onclick.
                onmousedown: move |evt| { evt.prevent_default(); },
                class: "px-2 py-1 text-sm hover:bg-gray-200 rounded",
                title: "{i18n.t(Key::MailEditorParagraph)}",
                onclick: move |evt| {
                    evt.prevent_default();
                    focus_editor(&editor_id_j);
                    if let Some(doc) = doc() {
                        let _ = crate::js::exec_command_str(&doc, "formatBlock", "<p>");
                    }
                    on_command.call(());
                },
                "¶"
            }
            // Blockquote
            button {
                r#type: "button",
                // Selection-preserve: mousedown → blur destroys the editor's Range before onclick.
                onmousedown: move |evt| { evt.prevent_default(); },
                class: "px-2 py-1 text-sm hover:bg-gray-200 rounded",
                title: "{i18n.t(Key::MailEditorBlockquote)}",
                onclick: move |evt| {
                    evt.prevent_default();
                    focus_editor(&editor_id_k);
                    if let Some(doc) = doc() {
                        let _ = crate::js::exec_command_str(&doc, "formatBlock", "<blockquote>");
                    }
                    on_command.call(());
                },
                "❝"
            }
            // Link — DIFFERENT: defer to parent so it can preserve Selection Range.
            button {
                r#type: "button",
                // Selection-preserve: mousedown → blur destroys the editor's Range before onclick.
                onmousedown: move |evt| { evt.prevent_default(); },
                class: "px-2 py-1 text-sm hover:bg-gray-200 rounded",
                title: "{i18n.t(Key::MailEditorLink)}",
                onclick: move |evt| {
                    evt.prevent_default();
                    // NOTE: no focus_editor call and no execCommand here — the
                    // parent captures the Selection Range before opening the
                    // link dialog (Pitfall 6). Focusing the editor at this
                    // point would move the caret and lose the selection.
                    on_link_click.call(());
                },
                "🔗"
            }
            // Unlink
            button {
                r#type: "button",
                // Selection-preserve: mousedown → blur destroys the editor's Range before onclick.
                onmousedown: move |evt| { evt.prevent_default(); },
                class: "px-2 py-1 text-sm hover:bg-gray-200 rounded",
                title: "{i18n.t(Key::MailEditorUnlink)}",
                onclick: move |evt| {
                    evt.prevent_default();
                    focus_editor(&editor_id_l);
                    if let Some(doc) = doc() {
                        let _ = crate::js::exec_command_simple(&doc, "unlink");
                    }
                    on_command.call(());
                },
                "⊘"
            }
            // Image (Phase 27, IMG-03) — opens a hidden PNG/JPEG/GIF file
            // picker, uploads via upload_mail_asset, then inserts the
            // data-genossi-asset-id <img> at the caret. Follows the button
            // pattern verbatim incl. the mandatory onmousedown+prevent_default
            // selection-preserve invariant (grep-gate).
            button {
                r#type: "button",
                // Selection-preserve: mousedown → blur destroys the editor's Range before onclick.
                onmousedown: move |evt| { evt.prevent_default(); },
                class: "px-2 py-1 text-sm hover:bg-gray-200 rounded",
                title: "{i18n.t(Key::MailEditorImage)}",
                onclick: move |evt| {
                    evt.prevent_default();
                    // Focus the editor first so the caret is inside the
                    // contenteditable when insertHTML runs after upload.
                    focus_editor(&editor_id_m);
                    // Programmatically open the OS file picker by clicking the
                    // hidden input; the actual upload+insert happens in the
                    // input's onchange handler below.
                    if let Some(doc) = doc() {
                        if let Some(el) = doc.get_element_by_id(IMAGE_INPUT_ID) {
                            if let Some(input) = el.dyn_ref::<web_sys::HtmlElement>() {
                                input.click();
                            }
                        }
                    }
                },
                "🖼"
            }
            // Hidden file input backing the image button. accept= is a UX hint
            // only — authoritative PNG/JPEG/GIF + 5 MB validation is server-side
            // (T-27-16). onchange reads the first File, uploads it, and inserts
            // the same shape as the drag&drop path via image_insert_html.
            input {
                id: IMAGE_INPUT_ID,
                r#type: "file",
                accept: "image/png,image/jpeg,image/gif",
                class: "hidden",
                onchange: move |evt| {
                    let Some(web_event) = evt.downcast::<web_sys::Event>().cloned() else { return; };
                    let Some(target) = web_event.target() else { return; };
                    let Ok(input) = target.dyn_into::<web_sys::HtmlInputElement>() else { return; };
                    let Some(files) = input.files() else { return; };
                    let Some(file) = files.get(0) else { return; };
                    spawn(async move {
                        let config = CONFIG.read().clone();
                        match api::upload_mail_asset(&config, file).await {
                            Ok(asset) => {
                                let img_html =
                                    image_insert_html(&config.backend, &asset.id.to_string());
                                if let Some(doc) = doc() {
                                    let _ = crate::js::exec_command_str(&doc, "insertHTML", &img_html);
                                }
                                on_command.call(());
                            }
                            Err(e) => {
                                tracing::error!("mail-asset image upload failed: {e}");
                            }
                        }
                    });
                    // Reset the input so selecting the same file again re-fires
                    // onchange.
                    input.set_value("");
                },
            }
        }
    }
}

fn doc() -> Option<web_sys::Document> {
    web_sys::window().and_then(|w| w.document())
}

fn focus_editor(editor_id: &str) {
    if let Some(doc) = doc() {
        if let Some(el) = doc.get_element_by_id(editor_id) {
            if let Some(html_el) = el.dyn_ref::<web_sys::HtmlElement>() {
                let _ = html_el.focus();
            }
        }
    }
}

/// Source-Invariant Grep-Gate — every toolbar button MUST have an onmousedown
/// handler with `prevent_default()`. Without it, the mousedown → blur sequence
/// destroys the contenteditable's Selection Range before onclick can read it,
/// which makes block-level execCommands (formatBlock, insertUnorderedList,
/// insertOrderedList) silently no-op. See `.planning/quick/20260718-wysiwyg-toolbar-onmousedown-fix/PLAN.md`.
///
/// Uses the same self-reference-hazard defence as `wysiwyg_editor::grep_gate_tests`:
/// slice the source before the test module and assemble needles at runtime.
#[cfg(test)]
mod grep_gate_tests {
    const TOOLBAR_SRC: &str = include_str!("wysiwyg_toolbar.rs");
    const TEST_MODULE_MARKER: &str = "mod grep_gate_tests";

    fn production_region() -> &'static str {
        let cutoff = TOOLBAR_SRC
            .find(TEST_MODULE_MARKER)
            .expect("BUG: grep-gate test module marker not found");
        &TOOLBAR_SRC[..cutoff]
    }

    #[test]
    fn every_button_has_onmousedown_prevent_default() {
        let region = production_region();
        let button_needle = format!("r#type: {q}button{q}", q = "\"");
        let mousedown_needle = format!("onmousedow{tail}", tail = "n:");
        let prevent_needle = format!("evt.prevent_defaul{tail}", tail = "t()");

        let button_count = region.matches(&button_needle).count();
        let mousedown_count = region.matches(&mousedown_needle).count();
        assert!(
            button_count >= 13,
            "Grep gate FAILED: expected >=13 toolbar buttons, found {button_count}. \
             If a button was removed intentionally, update this assertion."
        );
        assert_eq!(
            button_count, mousedown_count,
            "Grep gate FAILED: {button_count} toolbar buttons declared but only \
             {mousedown_count} have `onmousedown` handlers. Every button MUST \
             call `evt.prevent_default()` in onmousedown to preserve the editor's \
             Selection Range — without it, block-level execCommands (formatBlock, \
             insertUnorderedList, insertOrderedList) silently no-op."
        );

        // For every onmousedown occurrence, prevent_default() must appear within
        // the next 80 chars (i.e. inside the closure body, not somewhere else).
        for (idx, _) in region.match_indices(&mousedown_needle) {
            let window = &region[idx..idx.saturating_add(80).min(region.len())];
            assert!(
                window.contains(&prevent_needle),
                "Grep gate FAILED: onmousedown at byte {idx} does not call \
                 prevent_default() within 80 chars. Window:\n{window}"
            );
        }
    }

    #[test]
    fn production_region_excludes_test_module() {
        let region = production_region();
        assert!(
            !region.contains(TEST_MODULE_MARKER),
            "BUG: production_region() slice still contains the test module marker"
        );
        assert!(region.len() < TOOLBAR_SRC.len());
    }
}

#[cfg(test)]
mod image_insert_html_tests {
    use super::image_insert_html;

    #[test]
    fn produces_exact_asset_img_shape() {
        let backend = "http://localhost:8080";
        let id = "123e4567-e89b-12d3-a456-426614174000";
        assert_eq!(
            image_insert_html(backend, id),
            format!(
                "<img data-genossi-asset-id=\"{id}\" src=\"{backend}/api/mail/assets/{id}/bytes\">"
            )
        );
    }

    /// Regression guard for the beta Preview-404 (quick 260724-8p1): the preview
    /// `src` MUST be built from `config.backend` — exactly like every other API
    /// call — not a relative `/api/...` path. A relative src bypasses the backend
    /// base and 404s where the browser-visible API base is not the page origin.
    #[test]
    fn preview_src_uses_backend_base_not_relative() {
        let id = "123e4567-e89b-12d3-a456-426614174000";

        // Beta-style backend that already carries an `/api` segment.
        let backend = "https://genossi-beta.nebenan-unverpackt.de/api";
        let html = image_insert_html(backend, id);
        assert!(
            html.contains(&format!("src=\"{backend}/api/mail/assets/{id}/bytes\"")),
            "preview src must start with config.backend, got: {html}"
        );
        // Must NOT emit the relative single-`/api` src that caused the 404.
        assert!(
            !html.contains("src=\"/api/mail/assets/"),
            "preview src must not be relative (bypasses config.backend), got: {html}"
        );
    }
}
