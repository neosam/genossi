use dioxus::prelude::*;
use uuid::Uuid;

use crate::api;
use crate::component::mail_compose::{
    MailBodyEditor, MailSubjectInput, TemplatePreview, TemplateSelector, TemplateVarButtons,
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
            if assigned_member_id.is_some() {
                {
                    let member_ids: Vec<Uuid> = assigned_member_id.as_ref()
                        .and_then(|s| Uuid::parse_str(s).ok())
                        .into_iter()
                        .collect();
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
                    spawn(async move {
                        sending.set(true);
                        let cfg = CONFIG.read().clone();
                        match api::reply_inbox_mail(&cfg, &mid, &subj, &b).await {
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
