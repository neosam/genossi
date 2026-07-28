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
use uuid::Uuid;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::Range;

use crate::api;
use crate::component::mail_compose::mail_preview_frame::{
    inject_asset_src, preview_needs_fetch, preview_srcdoc, PreviewMode,
};
use crate::component::mail_compose::wysiwyg_link_dialog::WysiwygLinkDialog;
use crate::component::mail_compose::wysiwyg_toolbar::{image_insert_html, WysiwygToolbar};
use crate::component::mail_compose::MailPreviewFrame;
use crate::i18n::{use_i18n, Key};
use crate::service::config::CONFIG;

/// The stable DOM id for the contenteditable div. Constant so
/// `WysiwygToolbar::focus_editor` and every read-from-DOM call can find
/// the node without prop-drilling a UUID.
const EDITOR_ID: &str = "wysiwyg-editor";

#[component]
pub fn WysiwygEditor(
    value: String,
    on_change: EventHandler<(String, String)>,
    // Phase 28 (PREV-02, D-03): Member, gegen den die Device-Vorschau die
    // Template-Variablen rendert. `None` bedeutet Hinweiszeile statt iframe
    // und ausdrücklich KEIN Request. Das Default-Attribut unten hält jede der
    // drei Call-Sites einzeln umstellbar (dasselbe Migrationsmuster, mit dem
    // Phase 24 `TemplatePreview.body_html` eingeführt hat).
    #[props(default)] preview_member_id: Option<Uuid>,
    // Phase 28 (PREV-02, D-03): Repayment-Kontext, 1:1 an `/api/mail/preview`
    // durchgereicht, damit Repayment-Variablen auch in der Device-Vorschau
    // auflösen — gleiche Semantik wie bei `TemplatePreview`.
    #[props(default)] repayment_phase_id: Option<Uuid>,
) -> Element {
    let i18n = use_i18n();
    let mut link_dialog_open = use_signal(|| false);
    let mut saved_range = use_signal(|| None::<Range>);

    // Phase 28 (PREV-01, D-13): Der Modus lebt bewusst INNERHALB dieser
    // Component, damit der Umschalter automatisch in allen drei Call-Sites
    // wirkt, ohne dass irgendeine Page ihn verdrahten muss.
    //
    // Pitfall 6 (bewusst akzeptiert, T-28-18): Alle drei Call-Sites bumpen
    // beim Template-Wechsel absichtlich einen `key` auf `WysiwygEditor`, um
    // einen Remount und damit ein Re-Seeding zu erzwingen. Dabei fällt `mode`
    // auf `Edit` zurück. Das ist gewünschtes Verhalten und KEIN Bug: nach
    // einem Template-Wechsel ist der Vorschau-Inhalt ohnehin veraltet, und
    // der Rücksprung in den Bearbeiten-Modus ist die richtige Reaktion.
    let mode = use_signal(|| PreviewMode::Edit);
    let preview_doc = use_signal(String::new);
    let preview_errors = use_signal(Vec::<String>::new);
    let preview_loading = use_signal(|| false);

    // Clone initial value for the onmounted closure (moves into the FnOnce).
    let initial_value = value.clone();

    rsx! {
        div { class: "border rounded",
            // Phase 28 (PREV-01, D-13): Segmented-Control mit den drei Modi —
            // gegenüber Tabs und Dropdown die richtige Wahl, weil es permanent
            // ALLE drei Zustände UND den aktiven zeigt (genau das verlangt
            // PREV-04). Der Umschalter bleibt bewusst inline in dieser
            // Component und wird NICHT extrahiert: er hat genau einen
            // Verwender, und D-13 legt ihn ausdrücklich in den `WysiwygEditor`,
            // damit er ohne Verdrahtung pro Page in allen drei Call-Sites
            // wirkt. Der iframe hingegen IST eine eigene Component
            // (`MailPreviewFrame`) — dort gilt Component-First.
            div { class: "flex items-center gap-2 px-2 py-1 border-b bg-gray-50",
                div { class: "flex rounded-md overflow-hidden border border-gray-300",
                    button {
                        class: if *mode.read() == PreviewMode::Edit { "px-3 py-1 text-sm bg-blue-600 text-white" } else { "px-3 py-1 text-sm bg-white text-gray-700 hover:bg-gray-100" },
                        // `r#type: "button"` ist zwingend — ohne dieses Attribut
                        // lädt die Seite trotz prevent_default neu (dokumentierter,
                        // bereits zweimal aufgetretener Projektbug).
                        r#type: "button",
                        onclick: move |evt: Event<MouseData>| {
                            evt.prevent_default();
                            switch_preview_mode(
                                PreviewMode::Edit,
                                on_change,
                                mode,
                                preview_doc,
                                preview_errors,
                                preview_loading,
                                preview_member_id,
                                repayment_phase_id,
                            );
                        },
                        {i18n.t(Key::MailEditorModeEdit)}
                    }
                    button {
                        class: if *mode.read() == PreviewMode::Desktop { "px-3 py-1 text-sm bg-blue-600 text-white" } else { "px-3 py-1 text-sm bg-white text-gray-700 hover:bg-gray-100" },
                        r#type: "button",
                        onclick: move |evt: Event<MouseData>| {
                            evt.prevent_default();
                            switch_preview_mode(
                                PreviewMode::Desktop,
                                on_change,
                                mode,
                                preview_doc,
                                preview_errors,
                                preview_loading,
                                preview_member_id,
                                repayment_phase_id,
                            );
                        },
                        {i18n.t(Key::MailEditorModeDesktop)}
                    }
                    button {
                        class: if *mode.read() == PreviewMode::Mobile { "px-3 py-1 text-sm bg-blue-600 text-white" } else { "px-3 py-1 text-sm bg-white text-gray-700 hover:bg-gray-100" },
                        r#type: "button",
                        onclick: move |evt: Event<MouseData>| {
                            evt.prevent_default();
                            switch_preview_mode(
                                PreviewMode::Mobile,
                                on_change,
                                mode,
                                preview_doc,
                                preview_errors,
                                preview_loading,
                                preview_member_id,
                                repayment_phase_id,
                            );
                        },
                        {i18n.t(Key::MailEditorModeMobile)}
                    }
                }
                if *preview_loading.read() {
                    p { class: "text-xs text-gray-500", {i18n.t(Key::MailEditorModeLoading)} }
                }
            }

            // Phase 28 (PREV-04, D-14): Toolbar nur im Bearbeiten-Modus.
            // Ausblenden statt Ausgrauen — das klarste Signal, dass hier gerade
            // nichts editiert wird.
            if !mode.read().is_preview() {
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
            }

            // Phase 28 (PREV-05, D-17, Pitfall 1): der contenteditable-Knoten
            // bleibt in JEDEM Modus genau einmal im DOM und wird nur
            // off-screen geschoben — nicht aus dem Rendering genommen und
            // nicht in einen if-Zweig gehängt. Alles andere an diesem Element
            // (id, class, contenteditable, role, onmounted, oninput, onpaste)
            // ist buchstäblich unverändert.
            div {
                id: EDITOR_ID,
                style: editor_container_style(*mode.read()),
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
                    // Phase 27 (IMG-03) / quick 260724: wire drag&drop as NATIVE
                    // listeners on the element. Dioxus 0.6's ondragover/ondrop
                    // handlers did not fire in practice (two attempts, no drop,
                    // no log), so we attach dragenter/dragover/drop directly via
                    // web-sys — vanilla-JS semantics the framework cannot swallow.
                    attach_image_drop_target(on_change);
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
                // Drag&drop is wired natively in `onmounted` via
                // `attach_image_drop_target` — see the comment there.
            }

            // Phase 28 (PREV-02/PREV-05, D-04): Vorschau-Bereich mit drei sich
            // ausschließenden Fällen. Es gibt bewusst KEIN Banner der Form
            // „der Sanitizer hat N Elemente entfernt" — zeigt ammonia weniger
            // an, als der Editor enthielt, ist genau das die Information. Die
            // Darstellung selbst ist der Beweis, ein Element-Diff wäre eigene
            // Komplexität ohne Mehrwert.
            if mode.read().is_preview() {
                if preview_member_id.is_none() {
                    // D-03-Fallback: Hinweiszeile statt leerem Rahmen. In
                    // `switch_preview_mode` wurde bereits kein Request gestellt.
                    p { class: "text-sm text-gray-400 italic p-4",
                        {i18n.t(Key::MailEditorModeSelectMember)}
                    }
                } else if *preview_loading.read() {
                    p { class: "text-sm text-gray-400 italic p-4",
                        {i18n.t(Key::MailEditorModeLoading)}
                    }
                } else {
                    // Die Component entscheidet selbst, ob sie den roten
                    // Fehler-Block (T-28-16) oder den sandboxed iframe rendert.
                    MailPreviewFrame {
                        mode: *mode.read(),
                        srcdoc: preview_doc.read().clone(),
                        errors: preview_errors.read().clone(),
                    }
                }
            }

            // Phase 28: bleibt unverändert und in JEDEM Modus gemountet. Er ist
            // nur über die Toolbar erreichbar, die im Vorschau-Modus
            // ausgeblendet ist; ein bedingtes Rendern würde das gecachte
            // Selection-Range aus Pitfall 6 der Phase 24 unnötig gefährden.
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

/// Phase 28 (PREV-05, D-17) — Inline-Style des contenteditable-Containers je
/// Modus. Im Bearbeiten-Modus leer (das Element sieht exakt aus wie vor
/// Phase 28), in beiden Vorschau-Modi absolut positioniert und weit nach
/// links aus dem sichtbaren Bereich geschoben.
///
/// PITFALL 1 — warum off-screen und nicht aus dem Rendering genommen:
/// Der naheliegende Weg wäre, das Element per `display:none` bzw. die
/// entsprechende Tailwind-Utility unsichtbar zu machen. Genau das ist hier
/// VERBOTEN. Bei einem nicht gerenderten Element fällt
/// `HtmlElement::inner_text()` auf `Node.textContent` zurück und liefert den
/// Text OHNE die Umbrüche aus `<p>`, `<li>` und `<br>`. `sync_from_dom`
/// benutzt `inner_text()` ausdrücklich, damit gewollte Zeilenumbrüche
/// überleben (Phase 24, D-02) — und ALLE DREI Call-Sites (`reply_form.rs`,
/// `mail_page.rs`, `mail_templates.rs`) lesen in ihrem Submit-Guard
/// unmittelbar vor dem Absenden erneut `inner_text()` direkt vom Element.
/// Schaltet der Vorstand in die Vorschau und klickt dann Senden, würde der
/// Submit-Guard den Plain-Body mit der umbruchlosen Variante überschreiben:
/// die HTML-Mail wäre korrekt, der `text/plain`-Teil eine einzige Zeile
/// Fließtext (T-28-12). Off-Screen lässt das Element gerendert,
/// `inner_text()` verhält sich unverändert, und visuell ist es vollständig
/// weg.
///
/// Bewusst ein Inline-`style` statt einer Tailwind-Klasse: arbiträre
/// Tailwind-Werte müssten vom JIT-Purge im Quelltext gefunden werden, und ein
/// String-Rückgabewert ist robuster und obendrein nativ testbar.
///
/// `EDITOR_ID` bleibt eine Konstante und der Knoten bleibt genau einmal im
/// DOM (T-28-14) — ein zweiter Knoten mit derselben Id würde alle
/// `get_element_by_id`-Lookups der Toolbar und der drei Submit-Guards ins
/// falsche Element leiten.
pub(crate) fn editor_container_style(mode: PreviewMode) -> &'static str {
    if mode.is_preview() {
        // Feste Breite, damit der Umbruch im off-screen liegenden Element
        // stabil bleibt und `inner_text()` dieselben Umbrüche liefert wie im
        // sichtbaren Zustand.
        "position:absolute;left:-10000px;top:0;width:640px"
    } else {
        ""
    }
}

/// Phase 28 (PREV-01/PREV-02, D-05) — Moduswechsel: Sync vor Wechsel,
/// Fetch-Entscheidung, Request-Dispatch.
///
/// PITFALL 1, SCHICHT 2 (T-28-13): `sync_from_dom` läuft bedingungslos als
/// ERSTE Anweisung, unabhängig vom Zielmodus. Damit sind die Parent-Signale
/// garantiert aktuell, BEVOR sich am Layout irgendetwas ändert.
///
/// PITFALL 7 (T-28-16): `template_uses_repayment_vars` prüft serverseitig nur
/// Subject und Plain-Body, NICHT `body_html`. Enthält also nur das HTML einen
/// Repayment-Platzhalter und wird keine `repayment_phase_id` mitgeschickt,
/// greift der Dummy-Fallback nicht und minijinjas strict-env liefert einen
/// Render-Fehler, der mit `HTML:`-Präfix in `errors` landet. Genau deshalb
/// wird `repayment_phase_id` konsequent von der Call-Site durchgereicht und
/// `errors` angezeigt statt verworfen — sonst ist ein Render-Fehler von einer
/// leeren Mail nicht unterscheidbar.
#[allow(clippy::too_many_arguments)]
fn switch_preview_mode(
    target: PreviewMode,
    on_change: EventHandler<(String, String)>,
    mut mode: Signal<PreviewMode>,
    mut preview_doc: Signal<String>,
    mut preview_errors: Signal<Vec<String>>,
    mut preview_loading: Signal<bool>,
    preview_member_id: Option<Uuid>,
    repayment_phase_id: Option<Uuid>,
) {
    sync_from_dom(&on_change);
    let from = *mode.read();

    // D-05 / T-28-17: Desktop ↔ Mobile und jeder Rückweg nach Bearbeiten
    // lösen KEINEN Request aus. Es ändert sich ausschließlich die Breite, das
    // Dokument-Attribut bleibt unangetastet, Dioxus' Attribut-Diff überspringt
    // es, der iframe lädt nicht neu und die Vorschau flackert nicht.
    if !preview_needs_fetch(from, target) {
        mode.set(target);
        return;
    }

    // D-03-Fallback: ohne Member kein Request und kein leerer Rahmen — die
    // RSX-Seite zeigt stattdessen die Hinweiszeile.
    let Some(member_id) = preview_member_id else {
        preview_doc.set(String::new());
        preview_errors.set(Vec::new());
        mode.set(target);
        return;
    };

    let html = doc()
        .and_then(|d| d.get_element_by_id(EDITOR_ID))
        .map(|el| el.inner_html())
        .unwrap_or_default();

    // Kein Request für einen leeren Editor — entspricht der projektweit
    // etablierten empty→None-Regel an allen send/reply/save-Entry-Points.
    if html.trim().is_empty() {
        preview_doc.set(preview_srcdoc(""));
        preview_errors.set(Vec::new());
        mode.set(target);
        return;
    }

    mode.set(target);
    preview_loading.set(true);

    let member_id_str = member_id.to_string();
    spawn(async move {
        let config = CONFIG.read().clone();
        // `subject` und `body` sind leere Strings: beide sind Pflichtfelder
        // der `PreviewRequest`, leere Werte sind zulässig, minijinja liefert
        // für ein leeres Template `Ok("")`, und `rendered_body` wird bei
        // gesetztem `body_html` ohnehin serverseitig aus dem HTML abgeleitet.
        match api::preview_mail(
            &config,
            "",
            "",
            &member_id_str,
            repayment_phase_id,
            Some(&html),
        )
        .await
        {
            Ok(resp) if !resp.errors.is_empty() => {
                preview_errors.set(resp.errors.clone());
                preview_doc.set(String::new());
            }
            Ok(resp) => {
                preview_errors.set(Vec::new());
                let raw = resp.body_html.as_deref().unwrap_or("");
                preview_doc.set(preview_srcdoc(&inject_asset_src(raw, &config.backend)));
            }
            Err(e) => {
                preview_errors.set(vec![e.to_string()]);
                preview_doc.set(String::new());
            }
        }
        preview_loading.set(false);
    });
}

/// Phase 27 (IMG-03) / quick 260724 — wire image drag&drop as NATIVE DOM
/// listeners on the contenteditable element, bypassing Dioxus' drag-event
/// handling (which did not fire in practice: no drop, no upload, no console
/// log). Attaches `dragenter` + `dragover` (which only `preventDefault()` so
/// the element becomes a valid drop target and the browser fires `drop`) and
/// `drop` (preventDefault → read the first file → upload → insert the same
/// `data-genossi-asset-id` <img> as the toolbar button). Listeners are
/// `forget()`-leaked; they can only fire while the element is in the DOM, i.e.
/// while this component is mounted, so capturing `on_change` is sound.
fn attach_image_drop_target(on_change: EventHandler<(String, String)>) {
    let Some(document) = doc() else {
        return;
    };
    let Some(el) = document.get_element_by_id(EDITOR_ID) else {
        return;
    };

    // A drop target only fires `drop` if dragenter AND dragover are canceled.
    for name in ["dragenter", "dragover"] {
        let cb = Closure::<dyn FnMut(web_sys::Event)>::new(|e: web_sys::Event| {
            e.prevent_default();
        });
        let _ = el.add_event_listener_with_callback(name, cb.as_ref().unchecked_ref());
        cb.forget();
    }

    // drop: cancel the browser default (open the file), then upload + insert.
    let cb = Closure::<dyn FnMut(web_sys::Event)>::new(move |e: web_sys::Event| {
        e.prevent_default();
        let Ok(drag_event) = e.dyn_into::<web_sys::DragEvent>() else {
            return;
        };
        let Some(dt) = drag_event.data_transfer() else {
            return;
        };
        let Some(files) = dt.files() else {
            return;
        };
        // No file in the drop (e.g. dragged text) → nothing to do.
        let Some(file) = files.get(0) else {
            return;
        };
        let config = CONFIG.read().clone();
        wasm_bindgen_futures::spawn_local(async move {
            match api::upload_mail_asset(&config, file).await {
                Ok(asset) => {
                    let Some(doc) = doc() else {
                        return;
                    };
                    // Focus the editor so insertHTML lands inside it (the drop
                    // may have happened without the caret in the contenteditable).
                    if let Some(el) = doc.get_element_by_id(EDITOR_ID) {
                        if let Some(he) = el.dyn_ref::<web_sys::HtmlElement>() {
                            let _ = he.focus();
                        }
                    }
                    let img_html = image_insert_html(&config.backend, &asset.id.to_string());
                    let _ = crate::js::exec_command_str(&doc, "insertHTML", &img_html);
                    sync_from_dom(&on_change);
                }
                Err(err) => {
                    tracing::error!("mail-asset image drop upload failed: {err}");
                }
            }
        });
    });
    let _ = el.add_event_listener_with_callback("drop", cb.as_ref().unchecked_ref());
    cb.forget();
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
    use super::{editor_container_style, plain_to_html};
    use crate::component::mail_compose::mail_preview_frame::PreviewMode;

    /// Phase 28 (D-17): Im Bearbeiten-Modus soll das Element exakt so aussehen
    /// wie vor Phase 28. Ein leerer Style-String garantiert das.
    #[test]
    fn editor_container_style_is_empty_in_edit_mode() {
        assert_eq!(
            editor_container_style(PreviewMode::Edit),
            "",
            "Im Bearbeiten-Modus darf kein Inline-Style am contenteditable-Container \
             hängen — sonst verschiebt sich das Layout gegenüber dem Stand vor Phase 28."
        );
    }

    /// Phase 28 (T-28-12, Pitfall 1): Verhaltenstest auf dem Rückgabewert der
    /// Funktion, KEIN Quelltext-Grep — dadurch unabhängig von Formatierung und
    /// Kommentaren.
    #[test]
    fn editor_is_hidden_offscreen_not_display_none() {
        for mode in [PreviewMode::Desktop, PreviewMode::Mobile] {
            let style = editor_container_style(mode);
            assert!(
                style.contains("position:absolute"),
                "editor_container_style({mode:?}) muss das Element absolut positionieren, \
                 um es off-screen schieben zu können. Ist: {style:?}"
            );
            assert!(
                style.contains("left:-"),
                "editor_container_style({mode:?}) muss einen negativen left-Wert setzen, \
                 damit das Element aus dem sichtbaren Bereich wandert. Ist: {style:?}"
            );
            assert!(
                !style.contains("display:none") && !style.contains("visibility:hidden"),
                "editor_container_style({mode:?}) darf das Element NICHT aus dem Rendering \
                 nehmen. Bei einem nicht gerenderten Element fällt HtmlElement::inner_text() \
                 auf Node.textContent zurück (MDN), wodurch die Zeilenumbrüche aus <p>, <li> \
                 und <br> verlorengehen. sync_from_dom benutzt inner_text() ausdrücklich \
                 (Phase 24, D-02), und der Submit-Guard ALLER DREI Call-Sites (reply_form.rs, \
                 mail_page.rs, mail_templates.rs) liest unmittelbar vor dem Absenden erneut \
                 inner_text() direkt vom Element — der text/plain-Teil der versendeten Mail \
                 würde damit zu einer einzigen Zeile Fließtext (T-28-12). Ist: {style:?}"
            );
        }
    }

    /// Phase 28 (D-15): Die Off-Screen-Position hängt nicht vom Device-Modus
    /// ab; die Device-Breite lebt ausschließlich im iframe.
    #[test]
    fn editor_container_style_is_identical_for_both_preview_modes() {
        assert_eq!(
            editor_container_style(PreviewMode::Desktop),
            editor_container_style(PreviewMode::Mobile),
            "Desktop und Mobile müssen denselben Container-Style liefern — die Device-Breite \
             gehört in den iframe (PreviewMode::width_px), nicht an den versteckten Editor."
        );
    }

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

    /// Phase 27 (IMG-03) / quick 260724 — drag&drop is wired as NATIVE DOM
    /// listeners, not Dioxus handlers (which never fired: no drop, no log). The
    /// `onmounted` handler MUST call `attach_image_drop_target`, and that fn
    /// MUST register a native `drop` listener and `prevent_default()` the drag
    /// events. This guard bites if someone reverts to Dioxus ondrop/ondragover
    /// (which regressed twice) or drops the native wiring. Needles assembled at
    /// runtime + production-region slice defeat the include_str! self-match.
    #[test]
    fn native_drop_target_wired_and_prevents_default() {
        let region = production_region();

        // (a) onmounted must invoke the native attach helper.
        let attach_needle = format!("attach_image_drop_targe{tail}", tail = "t(");
        assert!(
            region.contains(&attach_needle),
            "Grep gate FAILED: expected {attach_needle} in wysiwyg_editor.rs \
             (production region) — drag&drop is no longer wired natively in \
             onmounted. Dioxus ondrop/ondragover handlers do NOT fire here \
             (quick 260724); reverting to them regresses drag&drop."
        );

        // (b) the native helper must register a `drop` listener ...
        let listen_needle = format!("add_event_listener_with_callbac{tail}", tail = "k(");
        let drop_literal = format!("{q}dro{tail}{q}", q = "\"", tail = "p");
        assert!(
            region.contains(&listen_needle) && region.contains(&drop_literal),
            "Grep gate FAILED: expected a native {listen_needle} registering a \
             {drop_literal} listener in wysiwyg_editor.rs (production region). \
             Without a native drop listener the browser opens the dropped file \
             instead of uploading it (T-27-18)."
        );

        // (c) ... and cancel the drag defaults so `drop` fires at all.
        let prevent_needle = format!("prevent_defaul{tail}", tail = "t()");
        assert!(
            region.contains(&prevent_needle),
            "Grep gate FAILED: expected {prevent_needle} in wysiwyg_editor.rs \
             (production region). A drop target only fires `drop` when dragenter \
             and dragover are canceled (quick 260724)."
        );
    }

    /// Phase 28 (T-28-12, Pitfall 1) — der Off-Screen-Helper muss tatsächlich
    /// im RSX verdrahtet sein und nicht bloß ungenutzt herumliegen. Ohne diese
    /// Verdrahtung wäre `editor_is_hidden_offscreen_not_display_none` wertlos:
    /// die Funktion könnte perfekt sein und der Container trotzdem per
    /// Rendering-Unterdrückung versteckt werden. Die Needle zielt deshalb auf
    /// die AUFRUFFORM im RSX, nicht auf die Definition — sonst wäre der Gate
    /// schon durch die bloße Existenz der Funktion erfüllt.
    #[test]
    fn editor_uses_offscreen_style_helper() {
        let region = production_region();
        let call_needle = format!("editor_container_styl{tail}", tail = "e(*mode.read())");
        assert!(
            region.contains(&call_needle),
            "Grep gate FAILED: expected {call_needle} in wysiwyg_editor.rs \
             (production region) — der contenteditable-Container bezieht seinen \
             Style nicht mehr aus dem Off-Screen-Helper. Wird er stattdessen aus \
             dem Rendering genommen, fällt inner_text() auf textContent zurück \
             und der Submit-Guard aller drei Call-Sites überschreibt den \
             text/plain-Teil der Mail umbruchlos (T-28-12, Pitfall 1)."
        );
    }

    /// Phase 28 (T-28-13, Pitfall 1 Schicht 2) — `switch_preview_mode` muss die
    /// Parent-Signale synchronisieren, BEVOR der Modus umgestellt und damit das
    /// Layout verändert wird. Fenster-Such-Muster analog
    /// `paste_handler_calls_prevent_default_before_read`.
    #[test]
    fn preview_mode_switch_syncs_dom_before_switching() {
        let region = production_region();
        let switch_needle = format!("fn switch_preview_mod{tail}", tail = "e(");
        let sync_needle = format!("sync_from_do{tail}", tail = "m(");
        let idx = region.find(&switch_needle).expect(
            "Grep gate FAILED: switch_preview_mode missing entirely in \
             wysiwyg_editor.rs (production region)",
        );
        let window = &region[idx..idx.saturating_add(400).min(region.len())];
        assert!(
            window.contains(&sync_needle),
            "Grep gate FAILED: expected {sync_needle} within 400 chars of \
             {switch_needle} in wysiwyg_editor.rs (production region). Das ist \
             Schicht 2 der Pitfall-1-Abwehr (T-28-13): der Editor-Inhalt muss in \
             die Parent-Signale wandern, BEVOR mode.set das Layout umbaut — sonst \
             gehen die letzten Tastenanschläge beim Wechsel in die Vorschau \
             verloren. Fenster ab switch_preview_mode (erste 400 Zeichen):\n{window}"
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
