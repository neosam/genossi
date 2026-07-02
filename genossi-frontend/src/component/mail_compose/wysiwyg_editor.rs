//! Phase 24 Plan 02 — WYSIWYG Editor.
//!
//! Contenteditable-based rich-text editor hosting `WysiwygToolbar` and
//! `WysiwygLinkDialog`. This is the reusable component that Plan 24-03
//! drops into all three MailBodyEditor call sites.
//!
//! Contract (per D-01 and D-02 of 24-CONTEXT.md):
//! - Props: `value: String` (initial innerHTML), `on_change:
//!   EventHandler<(String, String)>` where the tuple is
//!   `(plain: innerText, html: innerHTML)`.
//! - On mount: exactly ONE call to
//!   `document.execCommand("styleWithCSS", false, false)` so bold/italic
//!   emit semantic <b>/<i> tags (Pitfall 1 of 24-RESEARCH.md).
//! - Paste handler: preventDefault() first, then read text/plain and
//!   insertText via execCommand — no HTML paste (D-07).
//! - Toolbar buttons: each command runs, then the parent re-reads
//!   innerHTML+innerText (Pitfall 5 — DOM-sync-race).
//! - Link dialog: captures Selection Range BEFORE opening the modal so
//!   createLink hits the correct caret position (Pitfall 6).
//!
//! No native prompt fallback. No `form` wrapper. No new JS bundle.

use dioxus::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::Range;

use crate::component::mail_compose::wysiwyg_link_dialog::WysiwygLinkDialog;
use crate::component::mail_compose::wysiwyg_toolbar::WysiwygToolbar;

/// The stable DOM id for the contenteditable div. Constant so
/// `WysiwygToolbar::focus_editor` and every read-from-DOM call can find
/// the node without prop-drilling a UUID.
const EDITOR_ID: &str = "wysiwyg-editor";

#[component]
pub fn WysiwygEditor(
    value: String,
    on_change: EventHandler<(String, String)>,
) -> Element {
    let mut link_dialog_open = use_signal(|| false);
    let mut saved_range = use_signal(|| None::<Range>);

    // Clone initial value for the onmounted closure (moves into the FnOnce).
    let initial_value = value.clone();

    rsx! {
        div { class: "border rounded",
            WysiwygToolbar {
                editor_id: EDITOR_ID.to_string(),
                on_command: move |_| {
                    sync_from_dom(&on_change);
                },
                on_link_click: move |_| {
                    // Pitfall 6: capture the current Selection Range before
                    // opening the modal. The modal steals focus and the
                    // browser drops the range from Selection when the caret
                    // leaves the contenteditable, so we cache it here.
                    if let Some(win) = web_sys::window() {
                        if let Ok(Some(sel)) = win.get_selection() {
                            if sel.range_count() > 0 {
                                if let Ok(r) = sel.get_range_at(0) {
                                    saved_range.set(Some(r));
                                }
                            }
                        }
                    }
                    link_dialog_open.set(true);
                },
            }

            div {
                id: EDITOR_ID,
                class: "w-full px-3 py-2 min-h-40 focus:outline-none prose prose-sm max-w-none",
                contenteditable: "true",
                role: "textbox",
                onmounted: move |_| {
                    // Pitfall 1: styleWithCSS=false persists for the document
                    // lifetime — bold/italic emit <b>/<i> not <span style=…>.
                    if let Some(doc) = doc() {
                        let _ = crate::js::exec_command_bool(&doc, "styleWithCSS", false);
                        if let Some(el) = doc.get_element_by_id(EDITOR_ID) {
                            el.set_inner_html(&initial_value);
                        }
                    }
                },
                oninput: move |_| {
                    sync_from_dom(&on_change);
                },
                onpaste: move |evt| {
                    // Pitfall 3: preventDefault() FIRST so the browser does
                    // not run its own paste before our insertText.
                    evt.prevent_default();
                    // dioxus-web (0.6.3) impls HasClipboardData for
                    // Synthetic<web_sys::Event>; downcast<web_sys::Event>
                    // is the platform-native path with no direct dioxus_web
                    // import needed.
                    let Some(web_event) = evt.downcast::<web_sys::Event>().cloned() else { return; };
                    let Ok(ce) = web_event.dyn_into::<web_sys::ClipboardEvent>() else { return; };
                    let Some(dt) = ce.clipboard_data() else { return; };
                    let text = dt.get_data("text/plain").unwrap_or_default();
                    if text.is_empty() {
                        return;
                    }
                    if let Some(doc) = doc() {
                        let _ = crate::js::exec_command_str(&doc, "insertText", &text);
                        sync_from_dom(&on_change);
                    }
                },
            }

            WysiwygLinkDialog {
                open: link_dialog_open,
                on_insert: move |(url, _display_text): (String, String)| {
                    // Restore focus + Selection Range so createLink hits the
                    // caret position the user had before the dialog opened.
                    if let Some(win) = web_sys::window() {
                        if let Some(doc) = win.document() {
                            if let Some(el) = doc.get_element_by_id(EDITOR_ID) {
                                if let Some(html_el) = el.dyn_ref::<web_sys::HtmlElement>() {
                                    let _ = html_el.focus();
                                }
                            }
                            // Pitfall 6 (restore): put the saved range back on
                            // the Selection before dispatching createLink.
                            if let (Ok(Some(sel)), Some(range)) =
                                (win.get_selection(), saved_range.read().clone())
                            {
                                let _ = sel.remove_all_ranges();
                                let _ = sel.add_range(&range);
                            }
                            let _ = crate::js::exec_command_str(&doc, "createLink", &url);
                            sync_from_dom(&on_change);
                        }
                    }
                    saved_range.set(None);
                },
            }
        }
    }
}

/// Grab the browser Document; returns None if the WASM runtime is not in
/// a browser context (test/build fallback).
fn doc() -> Option<web_sys::Document> {
    web_sys::window().and_then(|w| w.document())
}

/// Read innerHTML + innerText from the contenteditable and push the tuple
/// through `on_change`. Called after every DOM mutation the parent needs
/// to see (oninput, toolbar command, link insert, paste). Pitfall 5.
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
