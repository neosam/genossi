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

use crate::api;
use crate::component::mail_compose::wysiwyg_link_dialog::WysiwygLinkDialog;
use crate::component::mail_compose::wysiwyg_toolbar::{image_insert_html, WysiwygToolbar};
use crate::service::config::CONFIG;

/// The stable DOM id for the contenteditable div. Constant so
/// `WysiwygToolbar::focus_editor` and every read-from-DOM call can find
/// the node without prop-drilling a UUID.
const EDITOR_ID: &str = "wysiwyg-editor";

#[component]
pub fn WysiwygEditor(value: String, on_change: EventHandler<(String, String)>) -> Element {
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
                class: "w-full px-3 py-2 min-h-40 focus:outline-none mail-html-render",
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
                // Phase 27 (IMG-03): make the div a drop target. Without a
                // dragover handler that calls prevent_default the browser
                // refuses to fire ondrop.
                ondragover: move |evt| {
                    evt.prevent_default();
                },
                // Phase 27 (IMG-03): drop an image file → upload → insert the
                // same data-genossi-asset-id <img> as the toolbar button.
                // Mirrors the onpaste structure: prevent_default FIRST, then
                // downcast to the platform event and read the dropped files.
                ondrop: move |evt| {
                    // prevent_default FIRST so the browser does not navigate to
                    // / open the dropped file (T-27-18).
                    evt.prevent_default();
                    let Some(web_event) = evt.downcast::<web_sys::Event>().cloned() else { return; };
                    let Ok(drag_event) = web_event.dyn_into::<web_sys::DragEvent>() else { return; };
                    let Some(dt) = drag_event.data_transfer() else { return; };
                    let Some(files) = dt.files() else { return; };
                    // No file in the drop (e.g. dragged text) → nothing to do.
                    let Some(file) = files.get(0) else { return; };
                    spawn(async move {
                        let config = CONFIG.read().clone();
                        match api::upload_mail_asset(&config, file).await {
                            Ok(asset) => {
                                let img_html =
                                    image_insert_html(&config.backend, &asset.id.to_string());
                                if let Some(doc) = doc() {
                                    let _ = crate::js::exec_command_str(&doc, "insertHTML", &img_html);
                                    sync_from_dom(&on_change);
                                }
                            }
                            Err(e) => {
                                tracing::error!("mail-asset image drop upload failed: {e}");
                            }
                        }
                    });
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
    let Some(doc) = doc() else {
        return;
    };
    let Some(el) = doc.get_element_by_id(EDITOR_ID) else {
        return;
    };
    let html = el.inner_html();
    // D-02: innerText not textContent so intentional line breaks survive.
    let plain = el
        .dyn_ref::<web_sys::HtmlElement>()
        .map(|he| he.inner_text())
        .unwrap_or_default();
    on_change.call((plain, html));
}

/// Convert plain text to HTML suitable for seeding the WysiwygEditor.
/// Escapes HTML entities and turns line breaks into `<br>`, so legacy
/// templates that were saved before HTML support (Phase 24) show up in the
/// editor instead of appearing empty.
pub fn plain_to_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            '\r' => {}
            '\n' => out.push_str("<br>"),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::plain_to_html;

    #[test]
    fn empty_input_stays_empty() {
        assert_eq!(plain_to_html(""), "");
    }

    #[test]
    fn escapes_html_entities() {
        assert_eq!(
            plain_to_html("<b>&\"'</b>"),
            "&lt;b&gt;&amp;&quot;&#39;&lt;/b&gt;"
        );
    }

    #[test]
    fn converts_lf_to_br() {
        assert_eq!(plain_to_html("a\nb"), "a<br>b");
    }

    #[test]
    fn converts_crlf_to_br() {
        assert_eq!(plain_to_html("a\r\nb"), "a<br>b");
    }

    #[test]
    fn mixed_content_escapes_and_breaks() {
        assert_eq!(
            plain_to_html("Hallo <Welt>\nZeile2 & Zeile3"),
            "Hallo &lt;Welt&gt;<br>Zeile2 &amp; Zeile3"
        );
    }

    #[test]
    fn trailing_newline_becomes_br() {
        assert_eq!(plain_to_html("foo\n"), "foo<br>");
    }
}

/// Phase 26 EDIT-09 — Source-Invariant Grep-Gate for the WYSIWYG editor.
///
/// These two tests protect against silent regression of the two invariants
/// that keep the ammonia sanitize gate working:
/// (1) styleWithCSS=false is set exactly once at mount, so bold/italic emit
///     semantic <b>/<i> and not <span style=…> (Pitfall 1 of 24-RESEARCH.md).
/// (2) The onpaste handler calls prevent_default() FIRST, so the browser
///     does not paste rich-text markup before our insertText override
///     (Pitfall 3 of 24-RESEARCH.md).
///
/// The tests load THIS FILE via include_str! and assert the invariants
/// are present verbatim. A cargo fmt reformat that changes whitespace or
/// argument quoting breaks these tests — that is the point.
///
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

    #[test]
    fn style_with_css_false_guard_present() {
        // Assemble the target at runtime so its literal byte sequence does
        // NOT appear anywhere in this test source. Combined with
        // `production_region()` slicing, this makes the check bite only
        // when the actual production call is missing.
        let target = format!(
            "exec_command_bool(&doc, {q}styleWithCSS{q}, false)",
            q = "\""
        );
        assert!(
            production_region().contains(&target),
            "Grep gate FAILED: expected literal call {target} in wysiwyg_editor.rs \
             (production region, before the test module). This guard is Pitfall 1 \
             of 24-RESEARCH.md — removing it means Bold emits <span style=…> \
             instead of <b>, which ammonia strips silently."
        );
    }

    #[test]
    fn paste_handler_calls_prevent_default_before_read() {
        // Same defence as test 1: search only the production region, and
        // build the needle strings at runtime.
        let region = production_region();
        let paste_needle = format!("onpast{tail}", tail = "e:");
        let prevent_needle = format!("evt.prevent_defaul{tail}", tail = "t()");
        let idx = region.find(&paste_needle).expect(
            "Grep gate FAILED: onpaste handler missing entirely in wysiwyg_editor.rs \
             (production region)",
        );
        let window = &region[idx..idx.saturating_add(400).min(region.len())];
        assert!(
            window.contains(&prevent_needle),
            "Grep gate FAILED: expected {prevent_needle} within 400 chars of \
             {paste_needle} in wysiwyg_editor.rs (production region). This is \
             Pitfall 3 of 24-RESEARCH.md — without it, the browser pastes \
             formatted HTML before our insertText overrides it. Window around \
             the paste handler (first 400 chars):\n{window}"
        );
    }

    /// Quick 260718-wysiwyg-editor-preview-css-fix — the editor container must
    /// use `mail-html-render` scope so h1..h6 / ul / ol / blockquote render
    /// visibly. The old `prose prose-sm` is a no-op because Tailwind Typography
    /// is not installed; regressing to it silently plattes the toolbar output.
    #[test]
    fn editor_uses_mail_html_render_scope() {
        let region = production_region();
        let scope_needle = format!("mail-html-rende{tail}", tail = "r");
        let prose_needle = format!("pros{tail}", tail = "e ");
        assert!(
            region.contains(&scope_needle),
            "Grep gate FAILED: expected `mail-html-render` class on the editor \
             div in wysiwyg_editor.rs (production region). Without it the \
             semantic HTML from the toolbar (h1, ul, ol, blockquote) is \
             flattened by Tailwind Preflight and looks like plain text."
        );
        assert!(
            !region.contains(&prose_needle),
            "Grep gate FAILED: the `prose ` class is back in wysiwyg_editor.rs. \
             It is a no-op because Tailwind Typography is not installed and \
             leaves the editor visually broken. Use `mail-html-render` instead."
        );
    }

    /// Phase 27 (IMG-03) — the drop handler must exist and call
    /// prevent_default(), otherwise the browser opens/navigates to the dropped
    /// file instead of uploading it (T-27-18). Same self-reference-hazard
    /// defence as the paste test: search the production region only and build
    /// the needles at runtime.
    #[test]
    fn drop_handler_calls_prevent_default() {
        let region = production_region();
        let drop_needle = format!("ondro{tail}", tail = "p:");
        let prevent_needle = format!("evt.prevent_defaul{tail}", tail = "t()");
        let idx = region.find(&drop_needle).expect(
            "Grep gate FAILED: ondrop handler missing entirely in wysiwyg_editor.rs \
             (production region) — image drag&drop insert is gone",
        );
        let window = &region[idx..idx.saturating_add(400).min(region.len())];
        assert!(
            window.contains(&prevent_needle),
            "Grep gate FAILED: expected {prevent_needle} within 400 chars of \
             {drop_needle} in wysiwyg_editor.rs (production region). Without it \
             the browser opens the dropped file instead of uploading it \
             (T-27-18). Window around the drop handler (first 400 chars):\n{window}"
        );
    }

    /// Meta-test: prove that `production_region()` actually excludes the
    /// test module. If someone renames `mod grep_gate_tests` or moves the
    /// tests above the invariants, this test forces the fix.
    #[test]
    fn production_region_excludes_test_module() {
        let region = production_region();
        assert!(
            !region.contains(TEST_MODULE_MARKER),
            "BUG: production_region() slice still contains the test module \
             marker — the slice is wrong, and grep_gate tests would be false \
             positives. Fix production_region() before trusting this suite."
        );
        // And the excluded portion must be non-empty (i.e. tests DO live
        // in this file somewhere after the marker).
        assert!(
            region.len() < EDITOR_SRC.len(),
            "BUG: production_region() covers the whole file — test module \
             marker was not found via .find(), which should have panicked."
        );
    }
}
