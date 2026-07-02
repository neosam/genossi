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

use crate::i18n::{use_i18n, Key};

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

    rsx! {
        div { class: "flex flex-wrap gap-1 border-b px-2 py-1 bg-gray-50",
            // Bold
            button {
                r#type: "button",
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
