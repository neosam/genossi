use dioxus::prelude::*;
use rest_types::MemberTO;
use uuid::Uuid;

use crate::api::{self, BulkRecipient, MailJobTO, MailJobDetailTO};
use crate::auth::RequirePrivilege;
use crate::component::TopBar;
use crate::component::member_search::filter_members;
use crate::i18n::{use_i18n, Key};
use crate::member_utils::{is_active, today};
use crate::page::AccessDeniedPage;
use crate::service::config::CONFIG;
use crate::service::member::{refresh_members, MEMBERS, SELECTED_MEMBER_IDS};

fn format_member(m: &MemberTO) -> String {
    format!("#{} {} {}", m.member_number, m.first_name, m.last_name)
}

fn job_status_key(status: &str) -> Key {
    match status {
        "running" => Key::MailJobRunning,
        "done" => Key::MailJobDone,
        "failed" => Key::MailJobFailed,
        _ => Key::MailJobPending,
    }
}

fn job_status_color(status: &str) -> &'static str {
    match status {
        "running" => "text-blue-600",
        "done" => "text-green-600",
        "failed" => "text-red-600",
        _ => "text-gray-600",
    }
}

#[component]
pub fn MailPage() -> Element {
    let i18n = use_i18n();
    let mut jobs = use_signal(|| Vec::<MailJobTO>::new());
    let mut loading = use_signal(|| true);
    let mut error = use_signal(|| None::<String>);
    let mut success_msg = use_signal(|| None::<String>);
    let mut expanded_job_id = use_signal(|| None::<String>);
    let mut job_detail = use_signal(|| None::<MailJobDetailTO>);

    // Compose form state — initialize from global selection (member list checkboxes)
    let mut selected_member_ids = use_signal(|| {
        let global = SELECTED_MEMBER_IDS.read();
        global.selected_ids.clone()
    });
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

    let reload_jobs = move || {
        spawn(async move {
            loading.set(true);
            let config = CONFIG.read().clone();
            match api::get_mail_jobs(&config).await {
                Ok(data) => {
                    jobs.set(data);
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
        reload_jobs();
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
                                    // Collect recipients with member_id
                                    let recipients: Vec<BulkRecipient> = {
                                        let members = MEMBERS.read();
                                        let ids = selected_member_ids.read();
                                        ids.iter()
                                            .filter_map(|id| {
                                                members.items.iter()
                                                    .find(|m| m.id == Some(*id))
                                                    .and_then(|m| {
                                                        m.email.as_ref().map(|email| BulkRecipient {
                                                            address: email.clone(),
                                                            member_id: m.id.map(|id| id.to_string()),
                                                        })
                                                    })
                                            })
                                            .collect()
                                    };
                                    spawn(async move {
                                        sending.set(true);
                                        error.set(None);
                                        success_msg.set(None);
                                        let config = CONFIG.read().clone();
                                        match api::send_bulk_mail(&config, &recipients, &subj, &b).await {
                                            Ok(_job) => {
                                                success_msg.set(Some(i18n.t(Key::MailJobCreated).to_string()));
                                                selected_member_ids.set(Vec::new());
                                                subject.set(String::new());
                                                body.set(String::new());
                                                reload_jobs();
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

                    // Mail jobs history
                    div { class: "bg-white rounded-lg shadow p-6",
                        h2 { class: "text-xl font-semibold mb-4", {i18n.t(Key::MailJobs)} }
                        if *loading.read() {
                            p { class: "text-gray-600", {i18n.t(Key::Loading)} }
                        } else if jobs.read().is_empty() {
                            p { class: "text-gray-600", {i18n.t(Key::MailNoHistory)} }
                        } else {
                            div { class: "space-y-3",
                                for job in jobs.read().iter() {
                                    {
                                        let job_id = job.id.clone();
                                        let job_id_expand = job.id.clone();
                                        let job_id_retry = job.id.clone();
                                        let status_color = job_status_color(&job.status);
                                        let status_key = job_status_key(&job.status);
                                        let progress_pct = if job.total_count > 0 {
                                            ((job.sent_count + job.failed_count) as f64 / job.total_count as f64 * 100.0) as i64
                                        } else {
                                            0
                                        };
                                        let is_expanded = expanded_job_id.read().as_ref() == Some(&job.id);
                                        let has_failures = job.failed_count > 0;
                                        let is_retryable = has_failures && job.status != "running";
                                        let progress_bar_color = if has_failures { "#ef4444" } else { "#22c55e" };
                                        let progress_style = format!("width: {}%; background-color: {};", progress_pct, progress_bar_color);
                                        let failed_text = format!("{} {}", job.failed_count, i18n.t(Key::MailJobFailed));
                                        rsx! {
                                            div { class: "border rounded-lg p-4",
                                                // Job header
                                                div {
                                                    class: "flex items-center justify-between cursor-pointer",
                                                    onclick: move |_| {
                                                        let current = expanded_job_id.read().clone();
                                                        if current.as_ref() == Some(&job_id_expand) {
                                                            expanded_job_id.set(None);
                                                            job_detail.set(None);
                                                        } else {
                                                            expanded_job_id.set(Some(job_id_expand.clone()));
                                                            let id = job_id_expand.clone();
                                                            spawn(async move {
                                                                let config = CONFIG.read().clone();
                                                                if let Ok(detail) = api::get_mail_job_detail(&config, &id).await {
                                                                    job_detail.set(Some(detail));
                                                                }
                                                            });
                                                        }
                                                    },
                                                    div { class: "flex-1",
                                                        div { class: "flex items-center gap-3",
                                                            span { class: "font-medium", "{job.subject}" }
                                                            span { class: "{status_color} text-sm font-medium",
                                                                {i18n.t(status_key)}
                                                            }
                                                        }
                                                        // Progress bar
                                                        div { class: "mt-2 flex items-center gap-3",
                                                            div { class: "flex-1 bg-gray-200 rounded-full h-2",
                                                                div {
                                                                    class: "h-2 rounded-full transition-all",
                                                                    style: "{progress_style}",
                                                                }
                                                            }
                                                            span { class: "text-sm text-gray-600 whitespace-nowrap",
                                                                "{job.sent_count + job.failed_count}/{job.total_count}"
                                                            }
                                                            if has_failures {
                                                                span { class: "text-sm text-red-500",
                                                                    "{failed_text}"
                                                                }
                                                            }
                                                        }
                                                    }
                                                    div { class: "flex items-center gap-2 ml-4",
                                                        if is_retryable {
                                                            button {
                                                                class: "bg-amber-500 hover:bg-amber-600 text-white px-3 py-1 rounded text-sm",
                                                                onclick: move |e| {
                                                                    e.stop_propagation();
                                                                    let id = job_id_retry.clone();
                                                                    spawn(async move {
                                                                        let config = CONFIG.read().clone();
                                                                        match api::retry_mail_job(&config, &id).await {
                                                                            Ok(_) => reload_jobs(),
                                                                            Err(e) => error.set(Some(e)),
                                                                        }
                                                                    });
                                                                },
                                                                {i18n.t(Key::MailRetry)}
                                                            }
                                                        }
                                                        span { class: "text-gray-400 text-sm",
                                                            if is_expanded { "▲" } else { "▼" }
                                                        }
                                                    }
                                                }

                                                // Expanded recipients
                                                if is_expanded {
                                                    if let Some(detail) = job_detail.read().as_ref() {
                                                        if detail.job.id == job_id {
                                                            div { class: "mt-4 border-t pt-3",
                                                                h3 { class: "text-sm font-medium text-gray-700 mb-2",
                                                                    {i18n.t(Key::MailRecipients)}
                                                                }
                                                                div { class: "max-h-60 overflow-y-auto",
                                                                    table { class: "w-full text-sm",
                                                                        thead { tr { class: "border-b text-left text-gray-500",
                                                                            th { class: "py-1 px-2", {i18n.t(Key::MailTo)} }
                                                                            th { class: "py-1 px-2", {i18n.t(Key::MailStatus)} }
                                                                            th { class: "py-1 px-2", {i18n.t(Key::MailError)} }
                                                                        }}
                                                                        tbody {
                                                                            for r in detail.recipients.iter() {
                                                                                {
                                                                                    let r_status_color = match r.status.as_str() {
                                                                                        "sent" => "text-green-600",
                                                                                        "failed" => "text-red-600",
                                                                                        _ => "text-gray-400",
                                                                                    };
                                                                                    let r_status_text = match r.status.as_str() {
                                                                                        "sent" => i18n.t(Key::MailSent),
                                                                                        "failed" => i18n.t(Key::MailFailed),
                                                                                        _ => i18n.t(Key::MailJobPending),
                                                                                    };
                                                                                    let error_text = r.error.clone().unwrap_or_default();
                                                                                    rsx! {
                                                                                        tr { class: "border-b last:border-b-0",
                                                                                            td { class: "py-1 px-2", "{r.to_address}" }
                                                                                            td { class: "py-1 px-2 {r_status_color}", {r_status_text} }
                                                                                            td { class: "py-1 px-2 text-red-500 text-xs", "{error_text}" }
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    } else {
                                                        div { class: "mt-4 text-gray-500 text-sm",
                                                            {i18n.t(Key::Loading)}
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
    }
}
