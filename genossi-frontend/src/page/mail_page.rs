use dioxus::prelude::*;
use rest_types::{MemberDocumentTO, MemberTO};
use uuid::Uuid;

use crate::api::{self, BulkRecipient, MailJobTO, MailJobDetailTO, PreviewResponse};
use crate::auth::RequirePrivilege;
use crate::component::TopBar;
use crate::component::member_search::filter_members;
use crate::i18n::{use_i18n, Key};
use crate::member_utils::{is_active, today};
use crate::page::AccessDeniedPage;
use crate::service::config::CONFIG;
use crate::service::member::{refresh_members, MEMBERS, SELECTED_MEMBER_IDS};

const TEMPLATE_FORMAL: &str = r#"Sehr geehrte{% if salutation == "Herr" %}r Herr{% elif salutation == "Frau" %} Frau{% else %}s Mitglied{% endif %}{% if title %} {{ title }}{% endif %} {{ last_name }},



Mit freundlichen Grüßen"#;

const TEMPLATE_INFORMAL: &str = r#"{% if salutation == "Herr" %}Lieber{% elif salutation == "Frau" %}Liebe{% else %}Hallo{% endif %}{% if title %} {{ title }}{% endif %} {{ first_name }},



Viele Grüße"#;

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

    // Attachment state
    let mut available_documents = use_signal(|| Vec::<MemberDocumentTO>::new());
    let mut selected_attachment_ids = use_signal(|| Vec::<Uuid>::new());

    // Template variable buttons
    let primary_vars = [
        ("first_name", "Vorname"),
        ("last_name", "Nachname"),
        ("salutation", "Anrede"),
        ("title", "Titel"),
        ("member_number", "Nr."),
        ("company", "Firma"),
    ];
    let secondary_vars = [
        ("street", "Straße"),
        ("house_number", "Hausnr."),
        ("postal_code", "PLZ"),
        ("city", "Stadt"),
        ("join_date", "Beitrittsdatum"),
        ("shares_at_joining", "Anteile (Beitritt)"),
        ("current_shares", "Anteile (aktuell)"),
        ("current_balance", "Guthaben"),
        ("exit_date", "Austrittsdatum"),
        ("bank_account", "Bankverbindung"),
        ("email", "E-Mail"),
    ];
    let mut show_more_vars = use_signal(|| false);

    // Preview state
    let mut preview_member_id = use_signal(|| None::<Uuid>);
    let mut preview_result = use_signal(|| None::<PreviewResponse>);
    let mut preview_loading = use_signal(|| false);

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

    // Fetch documents when exactly one member is selected
    use_effect(move || {
        let ids = selected_member_ids.read().clone();
        if ids.len() == 1 {
            let member_id = ids[0];
            spawn(async move {
                let config = CONFIG.read().clone();
                match api::get_member_documents(&config, member_id).await {
                    Ok(docs) => available_documents.set(docs),
                    Err(_) => available_documents.set(vec![]),
                }
            });
        } else {
            available_documents.set(vec![]);
            selected_attachment_ids.set(vec![]);
        }
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

                            // Template variable buttons
                            div { class: "bg-gray-50 rounded-lg p-3",
                                label { class: "block text-xs font-medium text-gray-500 mb-2",
                                    {i18n.t(Key::MailTemplateVariables)}
                                }
                                div { class: "flex flex-wrap gap-1",
                                    for (var_name, label) in primary_vars.iter() {
                                        {
                                            let vn = var_name.to_string();
                                            let vn2 = var_name.to_string();
                                            let lbl = label.to_string();
                                            rsx! {
                                                button {
                                                    class: "bg-blue-100 hover:bg-blue-200 text-blue-800 px-2 py-1 rounded text-xs font-mono",
                                                    r#type: "button",
                                                    title: "{vn}",
                                                    onclick: move |_| {
                                                        let insert = format!("{{{{ {} }}}}", vn2);
                                                        body.write().push_str(&insert);
                                                    },
                                                    "{lbl}"
                                                }
                                            }
                                        }
                                    }
                                    if *show_more_vars.read() {
                                        for (var_name, label) in secondary_vars.iter() {
                                            {
                                                let vn2 = var_name.to_string();
                                                let lbl = label.to_string();
                                                rsx! {
                                                    button {
                                                        class: "bg-gray-100 hover:bg-gray-200 text-gray-700 px-2 py-1 rounded text-xs font-mono",
                                                        r#type: "button",
                                                        onclick: move |_| {
                                                            let insert = format!("{{{{ {} }}}}", vn2);
                                                            body.write().push_str(&insert);
                                                        },
                                                        "{lbl}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    button {
                                        class: "text-gray-500 hover:text-gray-700 px-2 py-1 text-xs underline",
                                        r#type: "button",
                                        onclick: move |_| {
                                            let current = *show_more_vars.read();
                                            show_more_vars.set(!current);
                                        },
                                        if *show_more_vars.read() {
                                            "Weniger"
                                        } else {
                                            {i18n.t(Key::MailTemplateMore)}
                                        }
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
                            // Template selector dropdown
                            div {
                                label { class: "block text-sm font-medium text-gray-700 mb-1", "Vorlage" }
                                select {
                                    class: "w-full border rounded px-3 py-2 text-sm",
                                    onchange: move |e| {
                                        let val = e.value();
                                        match val.as_str() {
                                            "formal" => body.set(TEMPLATE_FORMAL.to_string()),
                                            "informal" => body.set(TEMPLATE_INFORMAL.to_string()),
                                            _ => {}
                                        }
                                    },
                                    option { value: "", {i18n.t(Key::MailTemplateSelect)} }
                                    option { value: "formal", {i18n.t(Key::MailTemplateFormal)} }
                                    option { value: "informal", {i18n.t(Key::MailTemplateInformal)} }
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
                            // Template Preview
                            div { class: "bg-gray-50 rounded-lg p-4",
                                h3 { class: "text-sm font-medium text-gray-700 mb-2",
                                    {i18n.t(Key::MailTemplatePreview)}
                                }
                                // Preview member selector
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
                                                // Trigger preview
                                                let subj = subject.read().clone();
                                                let b = body.read().clone();
                                                let mid = id.to_string();
                                                spawn(async move {
                                                    preview_loading.set(true);
                                                    let config = CONFIG.read().clone();
                                                    match api::preview_mail(&config, &subj, &b, &mid).await {
                                                        Ok(result) => preview_result.set(Some(result)),
                                                        Err(e) => preview_result.set(Some(PreviewResponse {
                                                            subject: String::new(),
                                                            body: String::new(),
                                                            errors: vec![e],
                                                        })),
                                                    }
                                                    preview_loading.set(false);
                                                });
                                            }
                                        },
                                        option { value: "", {i18n.t(Key::MailTemplatePreviewSelect)} }
                                        {
                                            let members = MEMBERS.read();
                                            let ids = selected_member_ids.read();
                                            rsx! {
                                                for id in ids.iter() {
                                                    {
                                                        let member = members.items.iter().find(|m| m.id == Some(*id));
                                                        if let Some(m) = member {
                                                            let display = format_member(m);
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
                                // Preview refresh button
                                if preview_member_id.read().is_some() {
                                    button {
                                        class: "bg-gray-200 hover:bg-gray-300 text-gray-700 px-3 py-1 rounded text-sm mb-3",
                                        r#type: "button",
                                        disabled: *preview_loading.read(),
                                        onclick: move |_| {
                                            if let Some(mid) = *preview_member_id.read() {
                                                let subj = subject.read().clone();
                                                let b = body.read().clone();
                                                let mid_str = mid.to_string();
                                                spawn(async move {
                                                    preview_loading.set(true);
                                                    let config = CONFIG.read().clone();
                                                    match api::preview_mail(&config, &subj, &b, &mid_str).await {
                                                        Ok(result) => preview_result.set(Some(result)),
                                                        Err(e) => preview_result.set(Some(PreviewResponse {
                                                            subject: String::new(),
                                                            body: String::new(),
                                                            errors: vec![e],
                                                        })),
                                                    }
                                                    preview_loading.set(false);
                                                });
                                            }
                                        },
                                        if *preview_loading.read() { "..." } else { {i18n.t(Key::MailTemplatePreview)} }
                                    }
                                }
                                // Preview result
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

                            // Attachment selector — visible only for single recipient
                            if selected_member_ids.read().len() == 1 {
                                div {
                                    label { class: "block text-sm font-medium text-gray-700 mb-1",
                                        "Anhänge"
                                    }
                                    if available_documents.read().is_empty() {
                                        p { class: "text-sm text-gray-400 italic",
                                            "Keine Dokumente vorhanden"
                                        }
                                    } else {
                                        div { class: "border rounded-md p-2 max-h-40 overflow-y-auto space-y-1",
                                            for doc in available_documents.read().iter() {
                                                {
                                                    let doc_id = doc.id;
                                                    let doc_type = doc.document_type.clone();
                                                    let file_name = doc.file_name.clone();
                                                    let is_checked = doc_id.map(|id| selected_attachment_ids.read().contains(&id)).unwrap_or(false);
                                                    rsx! {
                                                        label { class: "flex items-center gap-2 px-2 py-1 hover:bg-gray-50 rounded cursor-pointer text-sm",
                                                            input {
                                                                r#type: "checkbox",
                                                                checked: is_checked,
                                                                onchange: move |_| {
                                                                    if let Some(id) = doc_id {
                                                                        let mut ids = selected_attachment_ids.write();
                                                                        if ids.contains(&id) {
                                                                            ids.retain(|i| *i != id);
                                                                        } else {
                                                                            ids.push(id);
                                                                        }
                                                                    }
                                                                },
                                                            }
                                                            span { class: "text-gray-600", "{doc_type}" }
                                                            span { class: "text-gray-800 font-medium", "{file_name}" }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            button {
                                class: "bg-blue-500 hover:bg-blue-600 text-white px-6 py-2 rounded disabled:opacity-50",
                                disabled: *sending.read() || recipient_count == 0 || subject.read().is_empty(),
                                onclick: move |_| {
                                    let subj = subject.read().clone();
                                    let b = body.read().clone();
                                    let att_ids: Vec<String> = selected_attachment_ids.read().iter().map(|id| id.to_string()).collect();
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
                                        match api::send_bulk_mail(&config, &recipients, &subj, &b, &att_ids).await {
                                            Ok(_job) => {
                                                success_msg.set(Some(i18n.t(Key::MailJobCreated).to_string()));
                                                selected_member_ids.set(Vec::new());
                                                selected_attachment_ids.set(Vec::new());
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
