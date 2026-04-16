use dioxus::prelude::*;
use rest_types::MemberTO;

use crate::api::{self, InboundMailDetailTO, InboundMailTO};
use crate::auth::RequirePrivilege;
use crate::component::inbox::{InboxMailListItem, InboxReplyForm, InboxStatusBadge};
use crate::component::TopBar;
use crate::i18n::use_i18n;
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
            InboxPageInner { initial_id: None }
        }
    }
}

#[component]
pub fn InboxDetail(id: String) -> Element {
    rsx! {
        RequirePrivilege {
            privilege: "admin",
            fallback: rsx! { AccessDeniedPage { required_privilege: "admin".to_string() } },
            InboxPageInner { initial_id: Some(id) }
        }
    }
}

#[component]
fn InboxPageInner(initial_id: Option<String>) -> Element {
    let i18n = use_i18n();
    let mut mails = use_signal(Vec::<InboundMailTO>::new);
    let mut loading = use_signal(|| true);
    let mut error = use_signal(|| None::<String>);
    let mut info = use_signal(|| None::<String>);
    let mut selected_id = use_signal(|| None::<String>);
    let mut detail = use_signal(|| None::<InboundMailDetailTO>);
    let mut detail_loading = use_signal(|| false);
    let mut assign_search = use_signal(String::new);
    let mut show_reply = use_signal(|| false);
    let mut filter = use_signal(|| "open".to_string()); // "open", "done", "all"

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

    // Auto-select mail when opened via deep link
    let initial_id_clone = initial_id.clone();
    use_effect(move || {
        if let Some(ref id) = initial_id_clone {
            let id = id.clone();
            spawn(async move {
                selected_id.set(Some(id.clone()));
                detail_loading.set(true);
                let cfg = CONFIG.read().clone();
                match api::get_inbox_detail(&cfg, &id).await {
                    Ok(d) => {
                        detail.set(Some(d));
                    }
                    Err(e) => error.set(Some(e)),
                }
                detail_loading.set(false);
            });
        }
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

    let done_btn = move |_| {
        let Some(mail_id) = selected_id.read().clone() else {
            return;
        };
        spawn(async move {
            let cfg = CONFIG.read().clone();
            match api::done_inbox_mail(&cfg, &mail_id).await {
                Ok(_) => {
                    info.set(Some("Erledigt".to_string()));
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
        div { class: "p-4 max-w-6xl mx-auto flex flex-col h-[calc(100vh-4rem)]",
            div { class: "flex-none",
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
            }

            div { class: "flex gap-4 flex-1 min-h-0",
                // List column
                div { class: if selected_id.read().is_some() { "w-full md:w-1/2 border rounded flex flex-col overflow-hidden hidden md:flex" } else { "w-full md:w-1/2 border rounded flex flex-col overflow-hidden" },
                    div { class: "flex justify-between items-center px-3 py-2 bg-gray-50 border-b",
                        span { class: "font-semibold", "Eingänge" }
                        div { class: "flex gap-2 items-center",
                            {
                                let current = filter.read().clone();
                                let btn_class = |f: &str| {
                                    if current == f {
                                        "text-xs px-2 py-0.5 rounded bg-blue-600 text-white"
                                    } else {
                                        "text-xs px-2 py-0.5 rounded border hover:bg-gray-100"
                                    }
                                };
                                rsx! {
                                    button {
                                        class: btn_class("open"),
                                        onclick: move |_| filter.set("open".to_string()),
                                        "Offen"
                                    }
                                    button {
                                        class: btn_class("done"),
                                        onclick: move |_| filter.set("done".to_string()),
                                        "Erledigt"
                                    }
                                    button {
                                        class: btn_class("all"),
                                        onclick: move |_| filter.set("all".to_string()),
                                        "Alle"
                                    }
                                }
                            }
                            button {
                                class: "text-sm text-blue-600 hover:underline",
                                onclick: move |_| reload(),
                                "Neu laden"
                            }
                        }
                    }
                    if *loading.read() {
                        div { class: "p-3 text-gray-500", "Lädt…" }
                    } else {
                        {
                            let current_filter = filter.read().clone();
                            let filtered: Vec<_> = mails.read().iter().filter(|m| {
                                match current_filter.as_str() {
                                    "open" => !m.done,
                                    "done" => m.done,
                                    _ => true,
                                }
                            }).cloned().collect();
                            if filtered.is_empty() {
                                rsx! {
                                    div { class: "p-3 text-gray-500", "Keine Mails." }
                                }
                            } else {
                                rsx! {
                                    ul { class: "divide-y overflow-y-auto flex-1",
                                        for mail in filtered {
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
                                                        received_at: i18n.format_datetime(&mail.received_at),
                                                        replied: mail.replied,
                                                        done: mail.done,
                                                        archived: mail.archived,
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
                        }
                    }
                }

                // Detail column
                div { class: if selected_id.read().is_none() { "w-full md:w-1/2 border rounded p-3 flex flex-col overflow-hidden hidden md:flex" } else { "w-full md:w-1/2 border rounded p-3 flex flex-col overflow-hidden" },
                    // Back button (mobile only)
                    if selected_id.read().is_some() {
                        button {
                            class: "md:hidden text-sm text-blue-600 hover:underline mb-2",
                            onclick: move |_| {
                                selected_id.set(None);
                                detail.set(None);
                            },
                            "← Zurück zur Liste"
                        }
                    }

                    if *detail_loading.read() {
                        div { class: "text-gray-500", "Lädt Detail…" }
                    } else if let Some(d) = detail.read().clone() {
                        // Detail header (fixed)
                        div { class: "flex-none flex flex-col gap-2",
                            div { class: "font-semibold text-lg", "{d.subject}" }
                            div { class: "text-sm text-gray-600",
                                "Von: {d.from_address}"
                            }
                            div { class: "text-xs text-gray-500",
                                "Empfangen: {i18n.format_datetime(&d.received_at)}"
                            }
                            div { class: "text-xs",
                                InboxStatusBadge { replied: d.replied, done: d.done, archived: d.archived }
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
                        }

                        // Scrollable body + actions
                        div { class: "flex-1 overflow-y-auto flex flex-col gap-2 mt-2",
                            pre { class: "bg-gray-50 p-2 border rounded text-sm whitespace-pre-wrap",
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
                                div { class: "flex flex-wrap gap-2 mt-3",
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
                                        class: "text-sm px-3 py-1 bg-green-600 hover:bg-green-700 text-white rounded",
                                        onclick: done_btn,
                                        "Erledigt"
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
