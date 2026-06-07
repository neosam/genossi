use dioxus::prelude::*;
use rest_types::MemberDocumentTO;
use uuid::Uuid;

use crate::api::{self, StaticDocumentTO};
use crate::component::mail_compose::{
    MailAttachmentPicker, MailBodyEditor, MailSubjectInput, TemplatePreview, TemplateSelector,
    TemplateVarButtons,
};
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
) -> Element {
    let mut reply_subject = use_signal(move || initial_subject.clone());

    // Build the quote block once; it's static for the lifetime of the form.
    let quote_block = build_original_quote(&original_body, &original_from, &original_date);
    let mut reply_body = use_signal({
        let q = quote_block.clone();
        move || {
            if q.is_empty() {
                String::new()
            } else {
                format!("\n\n{}", q)
            }
        }
    });
    let cached_quote = use_signal({
        let q = quote_block.clone();
        move || q
    });
    let mut sending = use_signal(|| false);
    let mut cached_footer = use_signal(|| String::new());

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
            if let Ok(footer) = api::get_mail_footer(&config).await {
                cached_footer.set(footer.clone());
                let quote = cached_quote.read().clone();
                let initial = compose_initial_body(&footer, &quote);
                if !initial.is_empty() {
                    reply_body.set(initial);
                }
            }
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
        div { class: "border-t pt-3 mt-3 space-y-3",
            div { class: "text-sm text-gray-600",
                "An: {from_address}"
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
                    reply_body.set(body);
                },
            }
            TemplateVarButtons {
                on_insert: move |var_text: String| {
                    reply_body.write().push_str(&var_text);
                },
            }
            MailBodyEditor {
                value: reply_body.read().clone(),
                on_change: move |val: String| reply_body.set(val),
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
                            member_ids: member_ids,
                        }
                    }
                }
            }
            button {
                class: "bg-blue-500 hover:bg-blue-600 text-white px-4 py-2 rounded disabled:opacity-50",
                disabled: *sending.read() || reply_subject.read().is_empty(),
                onclick: move |_| {
                    let mid = mail_id.clone();
                    let subj = reply_subject.read().clone();
                    let b = reply_body.read().clone();
                    let att_ids: Vec<Uuid> = selected_attachment_ids.read().clone();
                    let static_ids: Vec<String> = selected_static_document_ids.read().clone();
                    spawn(async move {
                        sending.set(true);
                        let cfg = CONFIG.read().clone();
                        match api::reply_inbox_mail(&cfg, &mid, &subj, &b, &att_ids, &static_ids).await {
                            Ok(_) => on_sent.call(()),
                            Err(e) => on_error.call(e.to_string()),
                        }
                        sending.set(false);
                    });
                },
                if *sending.read() { "Sende..." } else { "Antwort senden" }
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
}
