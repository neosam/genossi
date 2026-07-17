//! Phase 24 Plan 02 — WYSIWYG Link Dialog.
//!
//! In-app modal for capturing (URL, display-text) pairs before the WYSIWYG
//! editor invokes `execCommand("createLink", url)`. Per D-06 of 24-CONTEXT.md
//! we do NOT use `window.prompt()` — it looks unprofessional, cannot be
//! styled, and blocks the WASM event loop.
//!
//! Selection preservation is the CALLER'S responsibility: the toolbar Link
//! button captures the current `web_sys::Selection` Range before opening
//! this dialog (per Pitfall 6 of 24-RESEARCH.md). This component never
//! touches `document.getSelection()`.

use dioxus::prelude::*;

use crate::component::modal::Modal;
use crate::i18n::{use_i18n, Key};

/// URL-validation helper for the Insert button.
///
/// Accepts only `http://` and `https://` URLs. Rejects `javascript:`,
/// `data:`, bare relative URLs, and empty/whitespace strings — matches the
/// ammonia allow-list at the store boundary (Phase 23 D-03) with an extra
/// UX gate so users are told upfront why their input is invalid instead of
/// silently having it stripped server-side.
pub fn is_valid_link_url(url: &str) -> bool {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return false;
    }
    trimmed.starts_with("http://") || trimmed.starts_with("https://")
}

#[component]
pub fn WysiwygLinkDialog(open: Signal<bool>, on_insert: EventHandler<(String, String)>) -> Element {
    let i18n = use_i18n();
    let mut url = use_signal(String::new);
    let mut display_text = use_signal(String::new);

    // Render nothing while closed.
    if !*open.read() {
        return rsx! {};
    }

    let insert_disabled = !is_valid_link_url(&url.read());

    rsx! {
        Modal {
            div { class: "space-y-4",
                h2 { class: "text-lg font-semibold text-gray-800",
                    {i18n.t(Key::MailEditorLinkDialogTitle)}
                }

                div {
                    label { class: "block text-sm font-medium text-gray-700 mb-1",
                        {i18n.t(Key::MailEditorLinkUrlLabel)}
                    }
                    input {
                        class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500 text-sm",
                        r#type: "url",
                        placeholder: "https://example.com",
                        value: "{url}",
                        oninput: move |e| url.set(e.value()),
                    }
                }

                div {
                    label { class: "block text-sm font-medium text-gray-700 mb-1",
                        {i18n.t(Key::MailEditorLinkTextLabel)}
                    }
                    input {
                        class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500 text-sm",
                        r#type: "text",
                        value: "{display_text}",
                        oninput: move |e| display_text.set(e.value()),
                    }
                }

                div { class: "flex justify-end gap-2 pt-2",
                    button {
                        // Memory feedback_dioxus_button_type.md: r#type="button"
                        // + prevent_default to avoid Page-Reload on click.
                        r#type: "button",
                        class: "px-4 py-2 bg-gray-200 hover:bg-gray-300 text-gray-800 rounded text-sm",
                        onclick: move |evt| {
                            evt.prevent_default();
                            url.set(String::new());
                            display_text.set(String::new());
                            open.set(false);
                        },
                        {i18n.t(Key::MailEditorLinkCancel)}
                    }
                    button {
                        r#type: "button",
                        class: "px-4 py-2 bg-blue-500 hover:bg-blue-600 text-white rounded text-sm disabled:opacity-50 disabled:cursor-not-allowed",
                        disabled: insert_disabled,
                        onclick: move |evt| {
                            evt.prevent_default();
                            let u = url.read().trim().to_string();
                            let t = display_text.read().trim().to_string();
                            if !is_valid_link_url(&u) {
                                return;
                            }
                            on_insert.call((u, t));
                            url.set(String::new());
                            display_text.set(String::new());
                            open.set(false);
                        },
                        {i18n.t(Key::MailEditorLinkInsert)}
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_valid_link_url_accepts_http_and_https() {
        assert!(is_valid_link_url("http://example.com"));
        assert!(is_valid_link_url("https://example.com/path?q=1"));
        assert!(is_valid_link_url("  https://example.com  "));
    }

    #[test]
    fn is_valid_link_url_rejects_javascript_and_data_scheme() {
        assert!(!is_valid_link_url("javascript:alert(1)"));
        assert!(!is_valid_link_url("data:text/html,<script>"));
        assert!(!is_valid_link_url("ftp://example.com"));
        assert!(!is_valid_link_url("/relative/path"));
        assert!(!is_valid_link_url("example.com"));
    }

    #[test]
    fn is_valid_link_url_rejects_empty_and_whitespace() {
        assert!(!is_valid_link_url(""));
        assert!(!is_valid_link_url("   "));
        assert!(!is_valid_link_url("\t\n"));
    }
}
