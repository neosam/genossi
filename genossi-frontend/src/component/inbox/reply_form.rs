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
    on_sent: EventHandler<()>,
    on_error: EventHandler<String>,
) -> Element {
    let mut reply_subject = use_signal(move || initial_subject.clone());
    let mut reply_body = use_signal(String::new);
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

    // Load footer on mount
    use_effect(move || {
        spawn(async move {
            let config = CONFIG.read().clone();
            if let Ok(footer) = api::get_mail_footer(&config).await {
                cached_footer.set(footer.clone());
                if !footer.is_empty() {
                    reply_body.set(format!("\n\n{}", footer));
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
                    if footer.is_empty() {
                        reply_body.set(template_body);
                    } else {
                        reply_body.set(format!("{}\n{}", template_body, footer));
                    }
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
