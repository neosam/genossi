use dioxus::prelude::*;
use rest_types::MemberDocumentTO;
use uuid::Uuid;

use crate::api::{self, StaticDocumentTO};
use crate::component::mail_compose::{
    plain_to_html, MailAttachmentPicker, MailSubjectInput, TemplatePreview, TemplateSelector,
    TemplateVarButtons, WysiwygEditor,
};
use crate::i18n::{use_i18n, Key};
use crate::service::config::CONFIG;

#[component]
pub fn InboxReplyForm(
    mail_id: String,
    from_address: String,
    initial_subject: String,
    assigned_member_id: Option<String>,
    original_body: String,
    original_from: String,
    original_date: String,
    on_sent: EventHandler<()>,
    on_error: EventHandler<String>,
    on_close: EventHandler<()>,
) -> Element {
    let i18n = use_i18n();
    let header_title = i18n.t(Key::InboxReplyModalTitle).to_string();
    let cancel_label = i18n.t(Key::InboxReplyCancel).to_string();
    let confirm_msg = i18n.t(Key::InboxReplyDiscardConfirm).to_string();
    // WR-01: localize the recipient label and the send/sending button copy via
    // the existing reusable mail keys (Component-First — they exist in both locales).
    let mail_to_label = i18n.t(Key::MailTo).to_string();
    let send_label = i18n.t(Key::MailSend).to_string();
    let sending_label = i18n.t(Key::MailSending).to_string();

    let mut reply_subject = use_signal({
        let s = initial_subject.clone();
        move || s
    });

    // Build the quote block once; it's static for the lifetime of the form.
    let quote_block = build_original_quote(&original_body, &original_from, &original_date);
    // The synchronous initial body: empty typing space, then the quote. Computed
    // once so reply_body AND the dirty-check baseline start from the SAME value.
    let initial_body = if quote_block.is_empty() {
        String::new()
    } else {
        format!("\n\n{}", quote_block)
    };
    let mut reply_body = use_signal({
        let b = initial_body.clone();
        move || b
    });
    // Phase 24 Plan 03 Task 3 (EDIT-01, D-01): companion HTML body pushed
    // from the WysiwygEditor's DOM alongside reply_body (innerText). Empty
    // sentinel → reply_inbox_mail posts None → legacy plaintext reply.
    let mut reply_body_html = use_signal(|| String::new());
    // Zähler, der bei jedem Template-Select hochgezählt wird → als `key` auf
    // dem WysiwygEditor triggert er einen Remount, damit der neu geseedete
    // reply_body_html tatsächlich im contenteditable-DOM landet
    // (onmounted läuft nur beim Mount, nicht bei Prop-Änderungen).
    let mut editor_reset_counter = use_signal(|| 0u32);
    let cached_quote = use_signal({
        let q = quote_block.clone();
        move || q
    });
    let mut sending = use_signal(|| false);
    let mut cached_footer = use_signal(|| String::new());

    // D-05: dirty-check baseline. Seeded SYNCHRONOUSLY with the same initial
    // subject/body the editors start with (WR-02 — no spurious confirm during the
    // footer-load window), then refined to the composed footer+quote body in the
    // footer use_effect below once it resolves (CR-01).
    let mut baseline_subject = use_signal({
        let s = initial_subject.clone();
        move || s
    });
    let mut baseline_body = use_signal({
        let b = initial_body.clone();
        move || b
    });

    // Quick 260607-s0s: same attachment state as Compose — populated by
    // MailAttachmentPicker via shared signals.
    let mut available_documents = use_signal(Vec::<MemberDocumentTO>::new);
    let mut available_static_documents = use_signal(Vec::<StaticDocumentTO>::new);
    let selected_attachment_ids = use_signal(Vec::<Uuid>::new);
    let selected_static_document_ids = use_signal(Vec::<String>::new);

    // Parse the optional assigned_member_id into a Uuid once for downstream use.
    let member_uuid_opt: Option<Uuid> = assigned_member_id
        .as_ref()
        .and_then(|s| Uuid::parse_str(s).ok());

    // Load footer on mount. Once the footer arrives, recompose the initial body
    // so the footer sits between the (still empty) typing area and the quote.
    use_effect(move || {
        spawn(async move {
            let config = CONFIG.read().clone();
            // CR-01: capture the body BEFORE the await so we can tell whether the
            // user typed into the editor during the (possibly slow) footer load.
            let pre_footer = reply_body.read().clone();
            if let Ok(footer) = api::get_mail_footer(&config).await {
                cached_footer.set(footer.clone());
                let quote = cached_quote.read().clone();
                let initial = compose_initial_body(&footer, &quote);
                if !initial.is_empty() {
                    // CR-01: only seed the composed footer+quote body if the user
                    // has NOT typed in the load window — never clobber their text.
                    if reply_body.read().clone() == pre_footer {
                        reply_body.set(initial.clone());
                    }
                    // WR-02/CR-01: the baseline must reflect the INTENDED initial
                    // body (composed footer+quote), NOT the possibly-edited current
                    // body — so text typed during the load window stays dirty and
                    // an untouched draft closes without a confirm.
                    baseline_body.set(initial);
                }
            }
            // Err path / empty-footer path: leave the synchronous step-1 baselines
            // in place. The subject baseline is never modified here (the footer
            // effect does not touch the subject), preserving any subject edits.
        });
    });

    // Quick 260607-s0s: load the assigned member's documents (if any) —
    // analog mail_page.rs:154-179. No member → empty list.
    use_effect(move || {
        if let Some(mid) = member_uuid_opt {
            spawn(async move {
                let config = CONFIG.read().clone();
                match api::get_member_documents(&config, mid).await {
                    Ok(docs) => available_documents.set(docs),
                    Err(_) => available_documents.set(vec![]),
                }
            });
        } else {
            available_documents.set(vec![]);
        }
    });

    // Quick 260607-s0s: static documents are global — load once on mount.
    use_effect(move || {
        spawn(async move {
            let config = CONFIG.read().clone();
            if let Ok(docs) = api::list_static_documents(&config).await {
                available_static_documents.set(docs);
            }
        });
    });

    rsx! {
        div { class: "space-y-3",
            // ── Modal header (D-01, D-03): title + X-close affordance ──
            div { class: "flex items-center justify-between border-b border-gray-200 pb-3",
                h2 { class: "text-xl font-semibold text-gray-900",
                    "{header_title}"
                }
                button {
                    r#type: "button",
                    class: "text-gray-500 hover:text-gray-700 px-2 py-1",
                    onclick: {
                        let confirm_msg = confirm_msg.clone();
                        move |_| {
                            let subj = reply_subject.read().clone();
                            let body = reply_body.read().clone();
                            let bsubj = baseline_subject.read().clone();
                            let bbody = baseline_body.read().clone();
                            if !is_draft_dirty(&subj, &body, &bsubj, &bbody) {
                                on_close.call(());
                            } else if web_sys::window()
                                .and_then(|w| w.confirm_with_message(&confirm_msg).ok())
                                .unwrap_or(false)
                            {
                                on_close.call(());
                            }
                        }
                    },
                    "\u{2715}"
                }
            }
            div { class: "text-sm text-gray-600",
                "{mail_to_label}: {from_address}"
            }
            MailSubjectInput {
                value: reply_subject.read().clone(),
                on_change: move |val: String| reply_subject.set(val),
            }
            TemplateSelector {
                on_select: move |template_body: String| {
                    let footer = cached_footer.read().clone();
                    let quote = cached_quote.read().clone();
                    let mut body = template_body;
                    if !footer.is_empty() {
                        body.push('\n');
                        body.push_str(&footer);
                    }
                    if !quote.is_empty() {
                        body.push_str("\n\n");
                        body.push_str(&quote);
                    }
                    // TemplateSelector liefert Plain-Text; ohne HTML-Konversion
                    // wäre der WysiwygEditor beim Template-Wechsel leer, weil
                    // set_inner_html("") beim Remount nichts anzeigt.
                    reply_body_html.set(plain_to_html(&body));
                    reply_body.set(body);
                    // Remount des Editors erzwingen (via key-Bump), damit der
                    // neue Seed-HTML im DOM ankommt.
                    editor_reset_counter.with_mut(|c| *c = c.wrapping_add(1));
                },
            }
            TemplateVarButtons {
                on_insert: move |var_text: String| {
                    reply_body.write().push_str(&var_text);
                    // Phase 24 Plan 03 Task 3 (Pitfall 5 partial-fix):
                    // TemplateVarButtons injects text directly into the
                    // reply_body signal, bypassing the WysiwygEditor DOM.
                    // Mirror the same text — HTML-escaped — into
                    // reply_body_html so the two signals stay in sync
                    // until the next user keystroke re-syncs the DOM.
                    // TODO: contenteditable does not re-sync from `value`
                    // prop after mount; TemplateVarButtons inserts show up
                    // in innerText/innerHTML on the next user keystroke
                    // via oninput. UAT (Plan 24-04) will smoke-check this.
                    let escaped = var_text
                        .replace('&', "&amp;")
                        .replace('<', "&lt;")
                        .replace('>', "&gt;");
                    reply_body_html.write().push_str(&escaped);
                },
            }
            // Phase 24 (EDIT-01, D-01): WysiwygEditor is the SINGLE input
            // source. on_change tuple → (innerText, innerHTML) → signals.
            // `key` bumpt bei jedem Template-Select → Remount → set_inner_html
            // seedet den neuen Body.
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
            // Quick 260607-s0s: same picker the Compose-flow uses
            // (Component-First).
            MailAttachmentPicker {
                member_id: member_uuid_opt,
                available_documents,
                available_static_documents,
                selected_member_doc_ids: selected_attachment_ids,
                selected_static_doc_ids: selected_static_document_ids,
            }
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
            div { class: "flex gap-2 items-center",
                button {
                    r#type: "button",
                    class: "bg-blue-500 hover:bg-blue-600 text-white px-4 py-2 rounded disabled:opacity-50",
                    disabled: *sending.read() || reply_subject.read().is_empty(),
                    onclick: move |_| {
                        // Phase 24 Plan 03 Task 3 Submit-Guard (Pitfall 5,
                        // D-01 belt-and-suspenders): re-read the DOM's
                        // innerHTML+innerText before building the reply so
                        // any late toolbar-click that missed on_command is
                        // still captured.
                        if let Some(doc) = web_sys::window()
                            .and_then(|w| w.document())
                        {
                            if let Some(el) = doc.get_element_by_id("wysiwyg-editor") {
                                let html = el.inner_html();
                                let plain = wasm_bindgen::JsCast::dyn_ref::<web_sys::HtmlElement>(&el)
                                    .map(|he| he.inner_text())
                                    .unwrap_or_default();
                                reply_body.set(plain);
                                reply_body_html.set(html);
                            }
                        }
                        let mid = mail_id.clone();
                        let subj = reply_subject.read().clone();
                        let b = reply_body.read().clone();
                        // Phase 24 (EDIT-01, D-01): capture body_html + apply
                        // empty→None backwards-compat rule.
                        let bh_value = reply_body_html.read().clone();
                        let att_ids: Vec<Uuid> = selected_attachment_ids.read().clone();
                        let static_ids: Vec<String> = selected_static_document_ids.read().clone();
                        spawn(async move {
                            sending.set(true);
                            let cfg = CONFIG.read().clone();
                            let body_html_opt: Option<&str> =
                                if bh_value.trim().is_empty() {
                                    None
                                } else {
                                    Some(bh_value.as_str())
                                };
                            match api::reply_inbox_mail(&cfg, &mid, &subj, &b, &att_ids, &static_ids, body_html_opt).await {
                                Ok(_) => on_sent.call(()),
                                Err(e) => on_error.call(e.to_string()),
                            }
                            sending.set(false);
                        });
                    },
                    if *sending.read() { "{sending_label}" } else { "{send_label}" }
                }
                // «Abbrechen» (D-01): neutral, second close affordance.
                button {
                    r#type: "button",
                    class: "px-4 py-2 text-gray-700 hover:bg-gray-100 rounded",
                    onclick: {
                        let confirm_msg = confirm_msg.clone();
                        move |_| {
                            let subj = reply_subject.read().clone();
                            let body = reply_body.read().clone();
                            let bsubj = baseline_subject.read().clone();
                            let bbody = baseline_body.read().clone();
                            if !is_draft_dirty(&subj, &body, &bsubj, &bbody) {
                                on_close.call(());
                            } else if web_sys::window()
                                .and_then(|w| w.confirm_with_message(&confirm_msg).ok())
                                .unwrap_or(false)
                            {
                                on_close.call(());
                            }
                        }
                    },
                    "{cancel_label}"
                }
            }
        }
    }
}

/// Build a German-style quoted block of the original mail body.
/// Returns an empty string when `body` is empty so the initial reply stays clean.
fn build_original_quote(body: &str, from: &str, date: &str) -> String {
    if body.is_empty() {
        return String::new();
    }
    let header = format!("Am {} schrieb {}:", date, from);
    let quoted: String = body
        .lines()
        .map(|line| {
            if line.is_empty() {
                ">".to_string()
            } else {
                format!("> {}", line)
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("{}\n{}", header, quoted)
}

/// Compose the initial reply body from the optional footer and optional quote.
/// Order: empty typing space, then footer, then the quoted original.
fn compose_initial_body(footer: &str, quote: &str) -> String {
    match (footer.is_empty(), quote.is_empty()) {
        (true, true) => String::new(),
        (false, true) => format!("\n\n{}", footer),
        (true, false) => format!("\n\n{}", quote),
        (false, false) => format!("\n\n{}\n\n{}", footer, quote),
    }
}

/// Pure dirty-check (D-06): the draft differs from the post-footer-load baseline.
/// The baseline is captured AFTER the async footer load (D-05), so an untouched
/// draft — whose body equals the composed footer+quote string, NOT the first quote —
/// is correctly reported as not dirty.
fn is_draft_dirty(subject: &str, body: &str, baseline_subject: &str, baseline_body: &str) -> bool {
    subject != baseline_subject || body != baseline_body
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_original_quote_empty_body_returns_empty() {
        assert_eq!(build_original_quote("", "a@b.c", "07.06.2026"), "");
    }

    #[test]
    fn build_original_quote_single_line() {
        let q = build_original_quote("Hallo Welt", "a@b.c", "07.06.2026 14:30");
        assert_eq!(q, "Am 07.06.2026 14:30 schrieb a@b.c:\n> Hallo Welt");
    }

    #[test]
    fn build_original_quote_multiline_keeps_blanks() {
        let body = "Zeile 1\n\nZeile 3";
        let q = build_original_quote(body, "x@y.z", "01.01.2026");
        assert_eq!(q, "Am 01.01.2026 schrieb x@y.z:\n> Zeile 1\n>\n> Zeile 3");
    }

    #[test]
    fn build_original_quote_trailing_newline_dropped() {
        let body = "nur eine zeile\n";
        let q = build_original_quote(body, "x@y.z", "d");
        assert_eq!(q, "Am d schrieb x@y.z:\n> nur eine zeile");
    }

    #[test]
    fn compose_initial_body_all_empty() {
        assert_eq!(compose_initial_body("", ""), "");
    }

    #[test]
    fn compose_initial_body_footer_only() {
        assert_eq!(compose_initial_body("-- \nFoo", ""), "\n\n-- \nFoo");
    }

    #[test]
    fn compose_initial_body_quote_only() {
        assert_eq!(
            compose_initial_body("", "Am ... schrieb x:\n> hi"),
            "\n\nAm ... schrieb x:\n> hi"
        );
    }

    #[test]
    fn compose_initial_body_footer_then_quote() {
        assert_eq!(
            compose_initial_body("Foo", "Am d schrieb x:\n> hi"),
            "\n\nFoo\n\nAm d schrieb x:\n> hi"
        );
    }

    #[test]
    fn is_draft_dirty_unchanged_is_not_dirty() {
        // subject + body equal the baseline → not dirty.
        assert!(!is_draft_dirty(
            "Re: Anfrage",
            "\n\nFoo\n\nAm d schrieb x:\n> hi",
            "Re: Anfrage",
            "\n\nFoo\n\nAm d schrieb x:\n> hi",
        ));
    }

    #[test]
    fn is_draft_dirty_subject_changed_is_dirty() {
        assert!(is_draft_dirty(
            "Re: Anfrage (geändert)",
            "body",
            "Re: Anfrage",
            "body",
        ));
    }

    #[test]
    fn is_draft_dirty_body_changed_is_dirty() {
        assert!(is_draft_dirty(
            "Re: Anfrage",
            "body + getippter Text",
            "Re: Anfrage",
            "body",
        ));
    }

    #[test]
    fn is_draft_dirty_typed_during_footer_load_is_dirty() {
        // CR-01/WR-02 corrected semantics: while the footer loads, the user
        // top-posts into the quote-prefilled body. The baseline is the INTENDED
        // composed-initial body (footer+quote); the current body is quote+usertext.
        // They differ → dirty=true → the unsaved text is protected by a confirm.
        let quote = "Am d schrieb x:\n> hi";
        let composed_initial = compose_initial_body("Foo", quote); // footer+quote baseline
        let typed_during_load = format!("Meine Antwort\n\n{}", quote); // user top-posted
        assert_ne!(typed_during_load, composed_initial);
        assert!(is_draft_dirty(
            "Re: Anfrage",
            &typed_during_load,
            "Re: Anfrage",
            &composed_initial,
        ));
        // Untouched draft: body equals the composed-initial baseline → not dirty.
        assert!(!is_draft_dirty(
            "Re: Anfrage",
            &composed_initial,
            "Re: Anfrage",
            &composed_initial,
        ));
    }

    #[test]
    fn is_draft_dirty_baseline_is_post_footer_body_not_first_quote() {
        // D-05 trap: reply_body starts quote-only, then the footer load overwrites
        // it with the composed footer+quote string. The baseline equals that
        // POST-footer body. Comparing the post-footer body against the post-footer
        // baseline → not dirty, even though it differs from the first quote string.
        let first_quote = "\n\nAm d schrieb x:\n> hi";
        let post_footer_body = compose_initial_body("Foo", "Am d schrieb x:\n> hi");
        assert_ne!(post_footer_body, first_quote);
        // baseline == post-footer body → not dirty.
        assert!(!is_draft_dirty(
            "Re: Anfrage",
            &post_footer_body,
            "Re: Anfrage",
            &post_footer_body,
        ));
    }
}
