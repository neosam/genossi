use dioxus::prelude::*;
use rest_types::MemberTO;
use uuid::Uuid;

use crate::api::{self, SentMailTO};
use crate::auth::RequirePrivilege;
use crate::component::TopBar;
use crate::component::member_search::filter_members;
use crate::i18n::{use_i18n, Key};
use crate::member_utils::{is_active, today};
use crate::page::AccessDeniedPage;
use crate::service::config::CONFIG;
use crate::service::member::{refresh_members, MEMBERS};

fn format_member(m: &MemberTO) -> String {
    format!("#{} {} {}", m.member_number, m.first_name, m.last_name)
}

#[component]
pub fn MailPage() -> Element {
    let i18n = use_i18n();
    let mut sent_mails = use_signal(|| Vec::<SentMailTO>::new());
    let mut loading = use_signal(|| true);
    let mut error = use_signal(|| None::<String>);
    let mut success_msg = use_signal(|| None::<String>);

    // Compose form state
    let mut selected_member_ids = use_signal(|| Vec::<Uuid>::new());
    let mut subject = use_signal(|| String::new());
    let mut body = use_signal(|| String::new());
    let mut sending = use_signal(|| false);

    // Member search state
    let mut search_query = use_signal(|| String::new());
    let mut show_dropdown = use_signal(|| false);

    // Load members
    use_effect(move || {
        spawn(async move {
            refresh_members().await;
        });
    });

    let reload_history = move || {
        spawn(async move {
            loading.set(true);
            let config = CONFIG.read().clone();
            match api::get_sent_mails(&config).await {
                Ok(data) => {
                    sent_mails.set(data);
                    error.set(None);
                }
                Err(e) => {
                    error.set(Some(format!("{}", e)));
                }
            }
            loading.set(false);
        });
    };

    use_effect(move || {
        reload_history();
    });

    // Count active members with email addresses
    let today = today();
    let members_with_email_count = {
        let members = MEMBERS.read();
        members
            .items
            .iter()
            .filter(|m| m.deleted.is_none() && is_active(m, &today) && m.email.is_some())
            .count()
    };

    // Count selected members without email
    let selected_without_email: Vec<String> = {
        let members = MEMBERS.read();
        let ids = selected_member_ids.read();
        ids.iter()
            .filter_map(|id| {
                members.items.iter().find(|m| m.id == Some(*id)).and_then(|m| {
                    if m.email.is_none() {
                        Some(format_member(m))
                    } else {
                        None
                    }
                })
            })
            .collect()
    };

    // Collect email addresses of selected members (only those with email)
    let recipient_count = {
        let members = MEMBERS.read();
        let ids = selected_member_ids.read();
        ids.iter()
            .filter(|id| {
                members
                    .items
                    .iter()
                    .find(|m| m.id == Some(**id))
                    .and_then(|m| m.email.as_ref())
                    .is_some()
            })
            .count()
    };

    rsx! {
        RequirePrivilege {
            privilege: "admin",
            fallback: rsx! { AccessDeniedPage { required_privilege: "admin".to_string() } },
            div { class: "flex flex-col min-h-screen",
                TopBar {}
                div { class: "flex-1 container mx-auto px-4 py-8",
                    h1 { class: "text-3xl font-bold mb-6",
                        {i18n.t(Key::Mail)}
                    }

                    // Success message
                    if let Some(msg) = success_msg.read().as_ref() {
                        div { class: "bg-green-100 border border-green-400 text-green-700 px-4 py-3 rounded mb-4",
                            "{msg}"
                        }
                    }

                    // Error message
                    if let Some(err) = error.read().as_ref() {
                        div { class: "bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4",
                            "{err}"
                        }
                    }

                    // Compose form
                    div { class: "bg-white rounded-lg shadow p-6 mb-6",
                        h2 { class: "text-xl font-semibold mb-4", {i18n.t(Key::MailCompose)} }
                        div { class: "space-y-4",
                            // Recipient selection
                            div {
                                label { class: "block text-sm font-medium text-gray-700 mb-1",
                                    {i18n.t(Key::MailTo)}
                                }

                                // Selected members as chips
                                if !selected_member_ids.read().is_empty() {
                                    div { class: "flex flex-wrap gap-2 mb-2",
                                        {
                                            let members = MEMBERS.read();
                                            let ids = selected_member_ids.read();
                                            rsx! {
                                                for id in ids.iter() {
                                                    {
                                                        let member = members.items.iter().find(|m| m.id == Some(*id));
                                                        let member_id = *id;
                                                        if let Some(m) = member {
                                                            let display = format_member(m);
                                                            let has_email = m.email.is_some();
                                                            let chip_class = if has_email {
                                                                "inline-flex items-center gap-1 bg-blue-100 text-blue-800 px-3 py-1 rounded-full text-sm"
                                                            } else {
                                                                "inline-flex items-center gap-1 bg-amber-100 text-amber-800 px-3 py-1 rounded-full text-sm"
                                                            };
                                                            rsx! {
                                                                span { class: "{chip_class}",
                                                                    "{display}"
                                                                    if !has_email {
                                                                        span { class: "text-xs", " (keine E-Mail)" }
                                                                    }
                                                                    button {
                                                                        class: "ml-1 text-current hover:text-red-600 font-bold",
                                                                        onclick: move |_| {
                                                                            selected_member_ids.write().retain(|id| *id != member_id);
                                                                        },
                                                                        "x"
                                                                    }
                                                                }
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

                                // Search input and select all button
                                div { class: "flex gap-2",
                                    div { class: "flex-1 relative",
                                        onfocusout: move |_| {
                                            spawn(async move {
                                                gloo_timers::future::TimeoutFuture::new(150).await;
                                                show_dropdown.set(false);
                                            });
                                        },
                                        input {
                                            class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500",
                                            r#type: "text",
                                            placeholder: "Name oder Nummer suchen...",
                                            value: "{search_query}",
                                            oninput: move |e| {
                                                search_query.set(e.value().clone());
                                                show_dropdown.set(!e.value().is_empty());
                                            },
                                            onfocus: move |_| {
                                                if !search_query.read().is_empty() {
                                                    show_dropdown.set(true);
                                                }
                                            },
                                        }

                                        // Dropdown results
                                        if *show_dropdown.read() {
                                            {
                                                let members = MEMBERS.read();
                                                let filtered = filter_members(&members.items, &search_query.read(), None);
                                                let selected_ids = selected_member_ids.read().clone();
                                                // Exclude already-selected members
                                                let available: Vec<_> = filtered.into_iter()
                                                    .filter(|m| !m.id.map(|id| selected_ids.contains(&id)).unwrap_or(false))
                                                    .collect();
                                                if !available.is_empty() {
                                                    rsx! {
                                                        div { class: "absolute z-20 w-full mt-1 bg-white border border-gray-300 rounded-md shadow-lg max-h-60 overflow-y-auto",
                                                            for member in available.iter() {
                                                                {
                                                                    let member_id = member.id;
                                                                    let display = format_member(member);
                                                                    let has_email = member.email.is_some();
                                                                    rsx! {
                                                                        button {
                                                                            class: "w-full text-left px-3 py-2 hover:bg-blue-50 cursor-pointer border-b border-gray-100 last:border-b-0",
                                                                            r#type: "button",
                                                                            onmousedown: move |e| {
                                                                                e.stop_propagation();
                                                                                if let Some(id) = member_id {
                                                                                    selected_member_ids.write().push(id);
                                                                                }
                                                                                search_query.set(String::new());
                                                                                show_dropdown.set(false);
                                                                            },
                                                                            span { "{display}" }
                                                                            if !has_email {
                                                                                span { class: "ml-2 text-xs text-amber-600", "(keine E-Mail)" }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                } else {
                                                    rsx! {}
                                                }
                                            }
                                        }
                                    }

                                    // Select all button
                                    button {
                                        class: "bg-gray-200 hover:bg-gray-300 text-gray-700 px-4 py-2 rounded whitespace-nowrap text-sm",
                                        onclick: move |_| {
                                            let members = MEMBERS.read();
                                            let all_ids: Vec<Uuid> = members.items.iter()
                                                .filter(|m| m.deleted.is_none() && is_active(m, &today))
                                                .filter_map(|m| m.id)
                                                .collect();
                                            selected_member_ids.set(all_ids);
                                        },
                                        "Alle ({members_with_email_count})"
                                    }

                                    // Clear button
                                    if !selected_member_ids.read().is_empty() {
                                        button {
                                            class: "bg-gray-200 hover:bg-gray-300 text-gray-700 px-4 py-2 rounded whitespace-nowrap text-sm",
                                            onclick: move |_| {
                                                selected_member_ids.set(Vec::new());
                                            },
                                            {i18n.t(Key::Cancel)}
                                        }
                                    }
                                }

                                // Warning for members without email
                                if !selected_without_email.is_empty() {
                                    p { class: "text-sm text-amber-600 mt-1",
                                        "{selected_without_email.len()} Mitglied(er) ohne E-Mail-Adresse werden übersprungen."
                                    }
                                }
                            }

                            div {
                                label { class: "block text-sm font-medium text-gray-700 mb-1", {i18n.t(Key::MailSubject)} }
                                input {
                                    class: "w-full border rounded px-3 py-2",
                                    r#type: "text",
                                    value: "{subject}",
                                    oninput: move |e| subject.set(e.value()),
                                }
                            }
                            div {
                                label { class: "block text-sm font-medium text-gray-700 mb-1", {i18n.t(Key::MailBody)} }
                                textarea {
                                    class: "w-full border rounded px-3 py-2 h-40",
                                    value: "{body}",
                                    oninput: move |e| body.set(e.value()),
                                }
                            }
                            button {
                                class: "bg-blue-500 hover:bg-blue-600 text-white px-6 py-2 rounded disabled:opacity-50",
                                disabled: *sending.read() || recipient_count == 0 || subject.read().is_empty(),
                                onclick: move |_| {
                                    let subj = subject.read().clone();
                                    let b = body.read().clone();
                                    let i18n = i18n.clone();
                                    // Collect email addresses
                                    let emails: Vec<String> = {
                                        let members = MEMBERS.read();
                                        let ids = selected_member_ids.read();
                                        ids.iter()
                                            .filter_map(|id| {
                                                members.items.iter()
                                                    .find(|m| m.id == Some(*id))
                                                    .and_then(|m| m.email.clone())
                                            })
                                            .collect()
                                    };
                                    spawn(async move {
                                        sending.set(true);
                                        error.set(None);
                                        success_msg.set(None);
                                        let config = CONFIG.read().clone();
                                        match api::send_bulk_mail(&config, &emails, &subj, &b).await {
                                            Ok(results) => {
                                                let sent_count = results.iter().filter(|r| r.status == "sent").count();
                                                let failed_count = results.iter().filter(|r| r.status == "failed").count();
                                                if failed_count == 0 {
                                                    success_msg.set(Some(
                                                        format!("{} - {} Empfänger", i18n.t(Key::MailSentSuccess), sent_count)
                                                    ));
                                                    selected_member_ids.set(Vec::new());
                                                    subject.set(String::new());
                                                    body.set(String::new());
                                                } else {
                                                    error.set(Some(
                                                        format!("{} gesendet, {} fehlgeschlagen", sent_count, failed_count)
                                                    ));
                                                }
                                                reload_history();
                                            }
                                            Err(e) => {
                                                error.set(Some(e));
                                            }
                                        }
                                        sending.set(false);
                                    });
                                },
                                if *sending.read() {
                                    {i18n.t(Key::MailSending)}
                                } else {
                                    "{i18n.t(Key::MailSend)} ({recipient_count})"
                                }
                            }
                        }
                    }

                    // Sent mails history
                    div { class: "bg-white rounded-lg shadow p-6",
                        h2 { class: "text-xl font-semibold mb-4", {i18n.t(Key::MailHistory)} }
                        if *loading.read() {
                            p { class: "text-gray-600", {i18n.t(Key::Loading)} }
                        } else if sent_mails.read().is_empty() {
                            p { class: "text-gray-600", {i18n.t(Key::MailNoHistory)} }
                        } else {
                            table { class: "w-full",
                                thead { tr { class: "border-b text-left",
                                    th { class: "py-2 px-3", {i18n.t(Key::MailTo)} }
                                    th { class: "py-2 px-3", {i18n.t(Key::MailSubject)} }
                                    th { class: "py-2 px-3", {i18n.t(Key::MailStatus)} }
                                    th { class: "py-2 px-3", {i18n.t(Key::MailSentAt)} }
                                    th { class: "py-2 px-3", {i18n.t(Key::MailError)} }
                                }}
                                tbody {
                                    for mail in sent_mails.read().iter() {
                                        {
                                            let status_class = if mail.status == "sent" {
                                                "text-green-600"
                                            } else {
                                                "text-red-600"
                                            };
                                            let status_text = if mail.status == "sent" {
                                                i18n.t(Key::MailSent)
                                            } else {
                                                i18n.t(Key::MailFailed)
                                            };
                                            let sent_at = mail.sent_at.clone().unwrap_or_default();
                                            let error_text = mail.error.clone().unwrap_or_default();
                                            rsx! {
                                                tr { class: "border-b hover:bg-gray-50",
                                                    td { class: "py-2 px-3", "{mail.to_address}" }
                                                    td { class: "py-2 px-3", "{mail.subject}" }
                                                    td { class: "py-2 px-3 {status_class} font-medium", {status_text} }
                                                    td { class: "py-2 px-3 text-sm text-gray-500", "{sent_at}" }
                                                    td { class: "py-2 px-3 text-sm text-red-500", "{error_text}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
