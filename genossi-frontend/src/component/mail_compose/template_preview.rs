use dioxus::prelude::*;
use uuid::Uuid;

use crate::api::{self, PreviewResponse};
use crate::i18n::{use_i18n, Key};
use crate::service::config::CONFIG;
use crate::service::member::MEMBERS;

fn format_member_for_preview(m: &rest_types::MemberTO) -> String {
    format!("#{} {} {}", m.member_number, m.first_name, m.last_name)
}

/// Shared preview trigger used by both onchange and refresh button.
fn trigger_preview(
    subject: &str,
    body: &str,
    member_id: Uuid,
    mut preview_loading: Signal<bool>,
    mut preview_result: Signal<Option<PreviewResponse>>,
) {
    let subj = subject.to_string();
    let b = body.to_string();
    let mid_str = member_id.to_string();
    spawn(async move {
        preview_loading.set(true);
        let config = CONFIG.read().clone();
        match api::preview_mail(&config, &subj, &b, &mid_str).await {
            Ok(result) => preview_result.set(Some(result)),
            Err(e) => preview_result.set(Some(PreviewResponse {
                subject: String::new(),
                body: String::new(),
                errors: vec![e.to_string()],
            })),
        }
        preview_loading.set(false);
    });
}

#[component]
pub fn TemplatePreview(
    subject: ReadOnlySignal<String>,
    body: ReadOnlySignal<String>,
    member_ids: Vec<Uuid>,
) -> Element {
    let i18n = use_i18n();
    let mut preview_member_id = use_signal(|| None::<Uuid>);
    let mut preview_result = use_signal(|| None::<PreviewResponse>);
    let preview_loading = use_signal(|| false);

    rsx! {
        div { class: "bg-gray-50 rounded-lg p-4",
            h3 { class: "text-sm font-medium text-gray-700 mb-2",
                {i18n.t(Key::MailTemplatePreview)}
            }
            div { class: "mb-3",
                select {
                    class: "w-full border rounded px-3 py-2 text-sm",
                    onchange: move |e| {
                        let val = e.value();
                        if val.is_empty() {
                            preview_member_id.set(None);
                            preview_result.set(None);
                        } else if let Ok(id) = val.parse::<Uuid>() {
                            preview_member_id.set(Some(id));
                            trigger_preview(&subject.read(), &body.read(), id, preview_loading, preview_result);
                        }
                    },
                    option { value: "", {i18n.t(Key::MailTemplatePreviewSelect)} }
                    {
                        let members = MEMBERS.read();
                        rsx! {
                            for id in member_ids.iter() {
                                {
                                    let member = members.items.iter().find(|m| m.id == Some(*id));
                                    if let Some(m) = member {
                                        let display = format_member_for_preview(m);
                                        let mid = id.to_string();
                                        rsx! {
                                            option { value: "{mid}", "{display}" }
                                        }
                                    } else {
                                        rsx! {}
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if preview_member_id.read().is_some() {
                button {
                    class: "bg-gray-200 hover:bg-gray-300 text-gray-700 px-3 py-1 rounded text-sm mb-3",
                    r#type: "button",
                    disabled: *preview_loading.read(),
                    onclick: move |_| {
                        if let Some(mid) = *preview_member_id.read() {
                            trigger_preview(&subject.read(), &body.read(), mid, preview_loading, preview_result);
                        }
                    },
                    if *preview_loading.read() { "..." } else { {i18n.t(Key::MailTemplatePreview)} }
                }
            }
            if let Some(preview) = preview_result.read().as_ref() {
                if !preview.errors.is_empty() {
                    div { class: "bg-red-50 border border-red-200 rounded p-3 text-sm text-red-700",
                        p { class: "font-medium mb-1", {i18n.t(Key::MailTemplateError)} }
                        for err in preview.errors.iter() {
                            p { "{err}" }
                        }
                    }
                } else {
                    div { class: "bg-white border rounded p-3 text-sm",
                        p { class: "font-medium text-gray-700 mb-1",
                            "{i18n.t(Key::MailSubject)}: "
                            span { class: "font-normal", "{preview.subject}" }
                        }
                        pre { class: "whitespace-pre-wrap text-gray-600 mt-2",
                            "{preview.body}"
                        }
                    }
                }
            } else if preview_member_id.read().is_none() {
                p { class: "text-sm text-gray-400 italic",
                    {i18n.t(Key::MailTemplatePreviewSelect)}
                }
            }
        }
    }
}
