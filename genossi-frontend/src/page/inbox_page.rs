use dioxus::prelude::*;
use rest_types::MemberTO;

use crate::api::{self, InboundMailDetailTO, InboundMailTO};
use crate::auth::RequirePrivilege;
use crate::component::TopBar;
use crate::component::inbox::{InboxMailListItem, InboxReplyForm, InboxStatusBadge};
use crate::page::AccessDeniedPage;
use crate::service::config::CONFIG;
use crate::service::member::{refresh_members, MEMBERS};

fn format_member_option(m: &MemberTO) -> String {
    format!("#{} {} {}", m.member_number, m.first_name, m.last_name)
}

#[component]
pub fn InboxPage() -> Element {
    rsx! {
        RequirePrivilege {
            privilege: "admin",
            fallback: rsx! { AccessDeniedPage { required_privilege: "admin".to_string() } },
            InboxPageInner {}
        }
    }
}

#[component]
fn InboxPageInner() -> Element {
    let mut mails = use_signal(Vec::<InboundMailTO>::new);
    let mut loading = use_signal(|| true);
    let mut error = use_signal(|| None::<String>);
    let mut info = use_signal(|| None::<String>);
    let mut selected_id = use_signal(|| None::<String>);
    let mut detail = use_signal(|| None::<InboundMailDetailTO>);
    let mut detail_loading = use_signal(|| false);
    let mut assign_search = use_signal(String::new);
    let mut show_reply = use_signal(|| false);

    use_effect(move || {
        spawn(async move {
            refresh_members().await;
        });
    });

    let reload = move || {
        spawn(async move {
            loading.set(true);
            let cfg = CONFIG.read().clone();
            match api::get_inbox(&cfg).await {
                Ok(data) => {
                    mails.set(data);
                    error.set(None);
                }
                Err(e) => error.set(Some(e)),
            }
            loading.set(false);
        });
    };

    use_effect(move || {
        reload();
    });

    let load_detail = move |id: String| {
        spawn(async move {
            detail_loading.set(true);
            let cfg = CONFIG.read().clone();
            match api::get_inbox_detail(&cfg, &id).await {
                Ok(d) => {
                    detail.set(Some(d));
                    error.set(None);
                }
                Err(e) => error.set(Some(e)),
            }
            detail_loading.set(false);
        });
    };

    let mut open_mail = move |id: String| {
        selected_id.set(Some(id.clone()));
        detail.set(None);
        assign_search.set(String::new());
        show_reply.set(false);
        load_detail(id.clone());
        spawn(async move {
            let cfg = CONFIG.read().clone();
            let _ = api::mark_inbox_mail_read(&cfg, &id).await;
        });
    };

    let assign_to = move |member_id: String| {
        let Some(mail_id) = selected_id.read().clone() else {
            return;
        };
        spawn(async move {
            let cfg = CONFIG.read().clone();
            match api::assign_inbox_mail(&cfg, &mail_id, &member_id).await {
                Ok(_) => {
                    info.set(Some("Zugeordnet".to_string()));
                    reload();
                    load_detail(mail_id);
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    let unassign = move |_| {
        let Some(mail_id) = selected_id.read().clone() else {
            return;
        };
        spawn(async move {
            let cfg = CONFIG.read().clone();
            match api::unassign_inbox_mail(&cfg, &mail_id).await {
                Ok(_) => {
                    reload();
                    load_detail(mail_id);
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    let archive = move |_| {
        let Some(mail_id) = selected_id.read().clone() else {
            return;
        };
        spawn(async move {
            let cfg = CONFIG.read().clone();
            match api::archive_inbox_mail(&cfg, &mail_id).await {
                Ok(_) => {
                    info.set(Some("Archiviert".to_string()));
                    selected_id.set(None);
                    detail.set(None);
                    reload();
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    let ignore_btn = move |_| {
        let Some(mail_id) = selected_id.read().clone() else {
            return;
        };
        spawn(async move {
            let cfg = CONFIG.read().clone();
            match api::ignore_inbox_mail(&cfg, &mail_id).await {
                Ok(_) => {
                    info.set(Some("Ignoriert".to_string()));
                    selected_id.set(None);
                    detail.set(None);
                    reload();
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    rsx! {
        TopBar {}
        div { class: "p-4 max-w-6xl mx-auto",
            h1 { class: "text-2xl font-bold mb-4", "Posteingang" }

            if let Some(e) = error.read().clone() {
                div { class: "bg-red-100 border border-red-400 text-red-700 px-3 py-2 mb-3 rounded",
                    "{e}"
                }
            }
            if let Some(m) = info.read().clone() {
                div { class: "bg-green-100 border border-green-400 text-green-700 px-3 py-2 mb-3 rounded",
                    "{m}"
                }
            }

            div { class: "flex gap-4",
                // List column
                div { class: "w-1/2 border rounded",
                    div { class: "flex justify-between items-center px-3 py-2 bg-gray-50 border-b",
                        span { class: "font-semibold", "Eingänge" }
                        button {
                            class: "text-sm text-blue-600 hover:underline",
                            onclick: move |_| reload(),
                            "Neu laden"
                        }
                    }
                    if *loading.read() {
                        div { class: "p-3 text-gray-500", "Lädt…" }
                    } else if mails.read().is_empty() {
                        div { class: "p-3 text-gray-500", "Keine Mails." }
                    } else {
                        ul { class: "divide-y",
                            for mail in mails.read().iter().cloned() {
                                {
                                    let mid = mail.id.clone();
                                    let selected = selected_id.read().as_deref() == Some(mid.as_str());
                                    let label = mail.assigned_member_name.clone()
                                        .unwrap_or_else(|| "nicht zugeordnet".to_string());
                                    let mid_click = mid.clone();
                                    rsx! {
                                        InboxMailListItem {
                                            subject: mail.subject.clone(),
                                            from_address: mail.from_address.clone(),
                                            received_at: mail.received_at.clone(),
                                            status: mail.status.clone(),
                                            has_attachments: mail.has_attachments,
                                            assigned_label: label,
                                            selected: selected,
                                            on_click: move |_| open_mail(mid_click.clone()),
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Detail column
                div { class: "w-1/2 border rounded p-3",
                    if *detail_loading.read() {
                        div { class: "text-gray-500", "Lädt Detail…" }
                    } else if let Some(d) = detail.read().clone() {
                        div { class: "flex flex-col gap-2",
                            div { class: "font-semibold text-lg", "{d.subject}" }
                            div { class: "text-sm text-gray-600",
                                "Von: {d.from_address}"
                            }
                            div { class: "text-xs text-gray-500",
                                "Empfangen: {d.received_at}"
                            }
                            div { class: "text-xs",
                                InboxStatusBadge { status: d.status.clone() }
                            }
                            if d.has_attachments {
                                div { class: "text-xs text-amber-700",
                                    "📎 Diese Mail enthält Anhänge (nicht anzeigbar im MVP)"
                                }
                            }
                            if d.has_html_body && d.body_text.is_empty() {
                                div { class: "text-xs text-gray-500 italic",
                                    "Nur HTML-Inhalt vorhanden — im MVP nicht gerendert."
                                }
                            }
                            pre { class: "bg-gray-50 p-2 border rounded text-sm whitespace-pre-wrap max-h-96 overflow-auto",
                                "{d.body_text}"
                            }

                            // Assignment section
                            div { class: "border-t pt-2 mt-2",
                                div { class: "text-sm mb-1",
                                    if let Some(name) = d.assigned_member_name.clone() {
                                        span { "Zugeordnet: {name}" }
                                    } else {
                                        span { class: "text-gray-500", "Nicht zugeordnet" }
                                    }
                                }
                                {
                                    let members = MEMBERS.read().items.clone();
                                    let from_lower = d.from_address.to_lowercase();
                                    let suggestion = members.iter()
                                        .find(|m| m.email.as_ref().map(|e| e.to_lowercase()) == Some(from_lower.clone()))
                                        .cloned();
                                    rsx! {
                                        if let Some(s) = suggestion {
                                            {
                                                let sid = s.id.map(|u| u.to_string()).unwrap_or_default();
                                                let label = format_member_option(&s);
                                                rsx! {
                                                    button {
                                                        class: "mb-2 bg-blue-600 hover:bg-blue-700 text-white text-sm px-3 py-1 rounded",
                                                        onclick: move |_| assign_to(sid.clone()),
                                                        "Vorschlag zuordnen: {label}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                input {
                                    r#type: "text",
                                    class: "w-full border rounded px-2 py-1 text-sm mb-1",
                                    placeholder: "Mitglied suchen (Name/Nr)",
                                    value: "{assign_search.read()}",
                                    oninput: move |e| assign_search.set(e.value()),
                                }
                                {
                                    let q = assign_search.read().to_lowercase();
                                    let members = MEMBERS.read().items.clone();
                                    let filtered: Vec<MemberTO> = if q.is_empty() {
                                        vec![]
                                    } else {
                                        members.into_iter()
                                            .filter(|m| {
                                                format!("{} {} {}", m.member_number, m.first_name, m.last_name)
                                                    .to_lowercase()
                                                    .contains(&q)
                                            })
                                            .take(10)
                                            .collect()
                                    };
                                    rsx! {
                                        if !filtered.is_empty() {
                                            ul { class: "border rounded max-h-40 overflow-auto text-sm",
                                                for m in filtered {
                                                    {
                                                        let mid = m.id.map(|u| u.to_string()).unwrap_or_default();
                                                        let label = format_member_option(&m);
                                                        rsx! {
                                                            li {
                                                                class: "px-2 py-1 hover:bg-gray-100 cursor-pointer",
                                                                onclick: move |_| assign_to(mid.clone()),
                                                                "{label}"
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                div { class: "flex gap-2 mt-3",
                                    button {
                                        class: "text-sm px-3 py-1 bg-blue-600 hover:bg-blue-700 text-white rounded",
                                        onclick: move |_| {
                                            let current = *show_reply.read();
                                            show_reply.set(!current);
                                        },
                                        if *show_reply.read() { "Antwort abbrechen" } else { "Antworten" }
                                    }
                                    button {
                                        class: "text-sm px-3 py-1 border rounded hover:bg-gray-100",
                                        onclick: unassign,
                                        "Zuordnung entfernen"
                                    }
                                    button {
                                        class: "text-sm px-3 py-1 border rounded hover:bg-gray-100",
                                        onclick: archive,
                                        "Archivieren"
                                    }
                                    button {
                                        class: "text-sm px-3 py-1 border rounded hover:bg-gray-100",
                                        onclick: ignore_btn,
                                        "Ignorieren"
                                    }
                                }

                                if *show_reply.read() {
                                    {
                                        let reply_subject = if d.subject.starts_with("Re:") {
                                            d.subject.clone()
                                        } else {
                                            format!("Re: {}", d.subject)
                                        };
                                        let mail_id = d.id.clone();
                                        let from_addr = d.from_address.clone();
                                        let member_id = d.assigned_member_id.clone();
                                        rsx! {
                                            InboxReplyForm {
                                                mail_id: mail_id,
                                                from_address: from_addr,
                                                initial_subject: reply_subject,
                                                assigned_member_id: member_id,
                                                on_sent: move |_| {
                                                    info.set(Some("Antwort gesendet".to_string()));
                                                    show_reply.set(false);
                                                    reload();
                                                    if let Some(mid) = selected_id.read().clone() {
                                                        load_detail(mid);
                                                    }
                                                },
                                                on_error: move |e: String| {
                                                    error.set(Some(e));
                                                },
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        div { class: "text-gray-500", "Wähle eine Mail aus der Liste." }
                    }
                }
            }
        }
    }
}
