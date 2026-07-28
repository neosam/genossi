use dioxus::prelude::*;
use rest_types::{MemberDocumentTO, MemberTO};
use uuid::Uuid;

use crate::api::{self, BulkRecipient, MailJobDetailTO};
use crate::auth::RequirePrivilege;
use crate::component::mail_compose::{
    plain_to_html, MailAttachmentPicker, MailSubjectInput, TemplatePreview, TemplateSelector,
    TemplateVarButtons, WysiwygEditor,
};
// Quick 260614-ckn: status-Helper leben jetzt in der MailJobsList-Komponente
// (DRY) und werden von MailJobDetail weiterhin genutzt.
use crate::component::mail_jobs_list::{job_status_color, job_status_key};
use crate::component::member_search::filter_members;
use crate::component::{
    is_no_repayment_letter_failure, show_toast, ErrorAlert, MailRecipientRenderedContent,
    MailRecipientStatusBadge, NoRepaymentLetterAction, ToastContainer, TopBar,
};
use crate::i18n::{use_i18n, Key};
use crate::member_utils::{is_active, today};
use crate::page::AccessDeniedPage;
use crate::router::Route;
use crate::service::config::CONFIG;
use crate::service::member::{refresh_members, MEMBERS, SELECTED_MEMBER_IDS};

fn format_member(m: &MemberTO) -> String {
    format!("#{} {} {}", m.member_number, m.first_name, m.last_name)
}

#[component]
pub fn MailPage() -> Element {
    let i18n = use_i18n();
    let mut error: Signal<Option<api::AppError>> = use_signal(|| None);
    let mut success_msg = use_signal(|| None::<String>);

    // Phase 12 D-18 (UAT-Defekt #3 Fix): Query-Params SYNCHRON im use_signal-Initializer
    // parsen — vor dem ersten Render. Der vorherige use_effect-basierte Ansatz
    // litt unter einem Race: chip-Render-Block las MEMBERS.read() = empty bei
    // mount, die nachfolgende selected_member_ids.set() trat dann zwar ein, aber
    // visuell hat der Vorstand "kein Mitglied ausgewählt" gesehen.
    let url_params: ParsedMailContext = web_sys::window()
        .and_then(|w| w.location().search().ok())
        .map(|s| parse_mail_query(&s))
        .unwrap_or(ParsedMailContext {
            phase_id: None,
            member_ids: Vec::new(),
        });

    // Compose form state — URL-Params haben Vorrang, sonst global selection
    let mut selected_member_ids = use_signal(|| {
        if !url_params.member_ids.is_empty() {
            url_params.member_ids.clone()
        } else {
            let global = SELECTED_MEMBER_IDS.read();
            global.selected_ids.clone()
        }
    });
    let mut subject = use_signal(|| String::new());
    let mut body = use_signal(|| String::new());
    // Phase 24 (EDIT-01, D-01): companion HTML body pushed from the
    // WysiwygEditor's DOM (innerHTML) alongside the plain-text body from
    // innerText. Empty-string sentinel → send/reply path posts None so
    // Phase 23 backend stores NULL and the mail stays legacy text-only.
    let mut body_html = use_signal(|| String::new());
    let mut sending = use_signal(|| false);
    let mut cached_footer = use_signal(|| String::new());
    // Phase 28 (PREV-02, D-03): GENAU EINE Mitglieds-Auswahl für die Vorschau
    // auf dieser Seite. Das Signal speist beide Verbraucher — die Device-
    // Vorschau im WysiwygEditor und die TemplatePreview darunter (D-16). Es
    // wird ausschließlich vom Auswahlfeld der TemplatePreview geschrieben.
    let preview_member_id = use_signal(|| None::<Uuid>);

    // Phase 12 D-18: Repayment-Kontext aus URL-Params synchron initialisiert
    let mut repayment_phase_id = use_signal(|| url_params.phase_id);
    // Phase 12 D-18 + Issue #2 BLOCKER-Fix: Template-Auswahl wird ueber on_select_id
    // gesetzt und als template_id an send_bulk_mail durchgereicht; KEIN hardcoded None mehr.
    let mut selected_template_id = use_signal(|| Option::<String>::None);
    // Quick 260603-e6p: opt-in flag for the backend to auto-attach
    // the per-recipient DocumentType::RepaymentLetter PDF. Defaults to
    // false on every page load — Vorstand decides per send. Backend rejects
    // `attach_repayment_letter=true` without `repayment_phase_id` with 400,
    // so the checkbox itself is hidden when no phase_id is set (mirror of
    // backend gate genossi_mail/src/rest.rs:478-481).
    let mut attach_repayment_letter = use_signal(|| false);

    // Attachment state
    let mut available_documents = use_signal(|| Vec::<MemberDocumentTO>::new());
    let mut selected_attachment_ids = use_signal(|| Vec::<Uuid>::new());

    // Static documents (global, available for every bulk send)
    let mut available_static_documents = use_signal(|| Vec::<crate::api::StaticDocumentTO>::new());
    let mut selected_static_document_ids = use_signal(|| Vec::<String>::new());

    // Member search state
    let mut search_query = use_signal(|| String::new());
    let mut show_dropdown = use_signal(|| false);

    // Load members
    use_effect(move || {
        spawn(async move {
            refresh_members().await;
        });
    });

    // Phase 12 D-18: Query-Params werden bereits synchron im use_signal-Initializer
    // oben geparst (UAT-Defekt #3 Fix). use_effect entfernt — war Race-anfällig.

    // Load footer on mount
    use_effect(move || {
        spawn(async move {
            let config = CONFIG.read().clone();
            if let Ok(footer) = api::get_mail_footer(&config).await {
                cached_footer.set(footer.clone());
                if !footer.is_empty() {
                    body.set(format!("\n\n{}", footer));
                }
            }
        });
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

    // Load available static documents once on mount
    use_effect(move || {
        spawn(async move {
            let config = CONFIG.read().clone();
            if let Ok(docs) = api::list_static_documents(&config).await {
                available_static_documents.set(docs);
            }
        });
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
                members
                    .items
                    .iter()
                    .find(|m| m.id == Some(*id))
                    .and_then(|m| {
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
            privilege: crate::auth::PRIVILEGE_ADMIN,
            fallback: rsx! { AccessDeniedPage { required_privilege: crate::auth::PRIVILEGE_ADMIN.to_string() } },
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
                    if let Some(ref err) = *error.read() {
                        ErrorAlert {
                            error: err.clone(),
                            on_dismiss: move |_| error.set(None),
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

                            TemplateVarButtons {
                                on_insert: move |var_text: String| {
                                    body.write().push_str(&var_text);
                                },
                                // Phase 12 D-19: zeige Repayment-Var-Buttons nur,
                                // wenn der Mail-Compose-Flow aus dem Repayment-Kontext kommt.
                                show_repayment_vars: repayment_phase_id.read().is_some(),
                            }

                            MailSubjectInput {
                                value: subject.read().clone(),
                                on_change: move |val: String| subject.set(val),
                            }
                            TemplateSelector {
                                on_select: move |template_body: String| {
                                    let footer = cached_footer.read().clone();
                                    let combined = if footer.is_empty() {
                                        template_body
                                    } else {
                                        format!("{}\n{}", template_body, footer)
                                    };
                                    body.set(combined.clone());
                                    // TemplateSelector liefert Plain-Text; für
                                    // den WysiwygEditor in HTML konvertieren
                                    // (escape + \n→<br>), sonst bleibt der
                                    // Editor beim Template-Wechsel leer, weil
                                    // die neue Seed-Version niemandem als HTML
                                    // gerendert wird.
                                    body_html.set(plain_to_html(&combined));
                                },
                                // Phase 12 D-18 / Issue #2 BLOCKER-Fix:
                                // store selected template id so send_bulk_mail can use it.
                                on_select_id: move |id: Option<String>| {
                                    selected_template_id.set(id);
                                },
                            }
                            // Phase 24 (EDIT-01, D-01): WysiwygEditor is the
                            // SINGLE input source. Its on_change tuple pushes
                            // (innerText, innerHTML) into (body, body_html)
                            // signals after every DOM mutation.
                            // `key` erzwingt Remount, wenn ein anderes Template
                            // gewählt wird — der Editor seedet innerHTML nur
                            // beim Mount, ohne Remount würde der neue Inhalt
                            // nie ins DOM übernommen.
                            {
                                let editor_key = selected_template_id
                                    .read()
                                    .clone()
                                    .unwrap_or_else(|| "__no_template__".to_string());
                                rsx! {
                                    // Phase 28 (PREV-02, D-03): Device-Vorschau
                                    // im Editor — sie liest dieselbe, einzige
                                    // Mitglieds-Auswahl der Seite wie die
                                    // TemplatePreview darunter, damit für den
                                    // Vorstand eindeutig ist, welche Auswahl
                                    // gilt. repayment_phase_id ist derselbe
                                    // Wert, den TemplatePreview schon bekommt —
                                    // sonst blieben Repayment-Variablen in der
                                    // Device-Vorschau unaufgelöst.
                                    WysiwygEditor {
                                        key: "{editor_key}",
                                        value: body_html.read().clone(),
                                        on_change: move |(plain, html): (String, String)| {
                                            body.set(plain);
                                            body_html.set(html);
                                        },
                                        preview_member_id: *preview_member_id.read(),
                                        repayment_phase_id: *repayment_phase_id.read(),
                                    }
                                }
                            }
                            // Phase 28 (PREV-02, D-03): dasselbe Signal wie am
                            // Editor oben — das Auswahlfeld dieser Component
                            // ist die EINE Auswahl der Seite und speist beide
                            // Vorschauen.
                            TemplatePreview {
                                subject: subject,
                                body: body,
                                body_html: body_html,
                                member_ids: selected_member_ids.read().clone(),
                                // UAT-Defekt #6: Live-Preview soll Repayment-Vars rendern
                                repayment_phase_id: *repayment_phase_id.read(),
                                preview_member_id: preview_member_id,
                            }

                            // Quick 260603-e6p: Vorstand opt-in to attach the per-recipient
                            // RepaymentLetter PDF. Visible only when repayment_phase_id is set,
                            // because the backend rejects the combination otherwise (400).
                            if repayment_phase_id.read().is_some() {
                                div { class: "mt-2 p-3 border border-gray-200 rounded bg-gray-50",
                                    label { class: "flex items-center space-x-2 text-sm",
                                        input {
                                            r#type: "checkbox",
                                            checked: *attach_repayment_letter.read(),
                                            onchange: move |evt| {
                                                attach_repayment_letter.set(evt.checked());
                                            },
                                        }
                                        span { class: "font-medium text-gray-700",
                                            {i18n.t(Key::MailAttachRepaymentLetter)}
                                        }
                                    }
                                    p { class: "text-xs text-gray-500 mt-1 ml-6",
                                        {i18n.t(Key::MailAttachRepaymentLetterHint)}
                                    }
                                }
                            }

                            // Quick 260607-s0s: Picker-RSX wandert in die shared
                            // MailAttachmentPicker-Component (Component-First).
                            // Verhalten 1:1 zum vorherigen Inline-Block:
                            // Member-Doc-Block nur bei genau einem Empfänger,
                            // Static-Doc-Block immer wenn welche vorhanden.
                            {
                                let single_recipient_id: Option<Uuid> = {
                                    let ids = selected_member_ids.read();
                                    if ids.len() == 1 { Some(ids[0]) } else { None }
                                };
                                rsx! {
                                    MailAttachmentPicker {
                                        member_id: single_recipient_id,
                                        available_documents,
                                        available_static_documents,
                                        selected_member_doc_ids: selected_attachment_ids,
                                        selected_static_doc_ids: selected_static_document_ids,
                                    }
                                }
                            }

                            button {
                                class: "bg-blue-500 hover:bg-blue-600 text-white px-6 py-2 rounded disabled:opacity-50",
                                disabled: *sending.read() || recipient_count == 0 || subject.read().is_empty(),
                                onclick: move |_| {
                                    // Phase 24 Plan 03 Task 2 Submit-Guard
                                    // (Pitfall 5 of 24-RESEARCH.md, D-01
                                    // belt-and-suspenders): re-read the
                                    // contenteditable's innerHTML+innerText
                                    // from the DOM BEFORE building the send
                                    // request so any late toolbar-click that
                                    // did not fire on_command is captured.
                                    if let Some(doc) = web_sys::window()
                                        .and_then(|w| w.document())
                                    {
                                        if let Some(el) = doc.get_element_by_id("wysiwyg-editor") {
                                            let html = el.inner_html();
                                            let plain = wasm_bindgen::JsCast::dyn_ref::<web_sys::HtmlElement>(&el)
                                                .map(|he| he.inner_text())
                                                .unwrap_or_default();
                                            body.set(plain);
                                            body_html.set(html);
                                        }
                                    }
                                    let subj = subject.read().clone();
                                    let b = body.read().clone();
                                    // Phase 24 (EDIT-01, D-01): capture the
                                    // body_html value + apply the empty→None
                                    // backwards-compat rule (Phase 23 HTML-03):
                                    // empty innerHTML means the user typed only
                                    // plain text with zero formatting, so the
                                    // send should stay legacy text-only.
                                    let bh_value = body_html.read().clone();
                                    let att_ids: Vec<String> = selected_attachment_ids.read().iter().map(|id| id.to_string()).collect();
                                    let static_ids: Vec<String> = selected_static_document_ids.read().clone();
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
                                    // Phase 12 D-18 + Issue #2 BLOCKER-Fix:
                                    // resolve template_id from selected_template_id signal
                                    // (fed by TemplateSelector::on_select_id) and
                                    // repayment_phase_id from query-param-parsed signal.
                                    // If no template selected, template_id stays None — that's
                                    // legitimate for a plain bulk-mail without template-resolution.
                                    let template_id_owned: Option<String> =
                                        selected_template_id.read().clone();
                                    let phase_id = *repayment_phase_id.read();
                                    // Quick 260603-e6p: capture the opt-in flag BEFORE the spawn
                                    // so the move-closure doesn't need to capture the Signal itself.
                                    let attach_letter_flag: bool = *attach_repayment_letter.read();
                                    spawn(async move {
                                        sending.set(true);
                                        error.set(None);
                                        success_msg.set(None);
                                        let config = CONFIG.read().clone();
                                        let template_id: Option<&str> =
                                            template_id_owned.as_deref();
                                        // Phase 24 Plan 03 Task 2 (D-01):
                                        // apply the empty→None backward-compat
                                        // rule — if the WYSIWYG editor emitted
                                        // no HTML (user typed only plain text),
                                        // treat as legacy plaintext send.
                                        let body_html_opt: Option<&str> =
                                            if bh_value.trim().is_empty() {
                                                None
                                            } else {
                                                Some(bh_value.as_str())
                                            };
                                        match api::send_bulk_mail(
                                            &config,
                                            &recipients,
                                            &subj,
                                            &b,
                                            body_html_opt,
                                            &att_ids,
                                            &static_ids,
                                            template_id,
                                            phase_id,
                                            attach_letter_flag,
                                        )
                                        .await
                                        {
                                            Ok(_job) => {
                                                success_msg.set(Some(i18n.t(Key::MailJobCreated).to_string()));
                                                selected_member_ids.set(Vec::new());
                                                selected_attachment_ids.set(Vec::new());
                                                selected_static_document_ids.set(Vec::new());
                                                subject.set(String::new());
                                                body.set(String::new());
                                                // Phase 24 Plan 03 Task 2:
                                                // reset the WysiwygEditor's
                                                // companion body_html signal.
                                                body_html.set(String::new());
                                                attach_repayment_letter.set(false);
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

                    // Quick 260614-ckn: Job-Liste ist auf /mail/jobs ausgelagert.
                    // Die Versand-Seite verlinkt nur noch dorthin (User-Entscheidung:
                    // "Komplett entfernen + Link").
                    div { class: "bg-white rounded-lg shadow p-6",
                        Link {
                            to: Route::MailJobsPage {},
                            class: "text-blue-600 hover:underline font-medium",
                            {i18n.t(Key::MailHistory)}
                        }
                    }
                }
            }
        }
    }
}

// ── Mail Job Detail page (deep link from communication timeline) ─────

#[component]
pub fn MailJobDetail(id: String) -> Element {
    let i18n = use_i18n();
    let mut detail = use_signal(|| None::<MailJobDetailTO>);
    let mut loading = use_signal(|| true);
    let mut error: Signal<Option<api::AppError>> = use_signal(|| None);

    // Quick 260603-evf: toast state for the NoRepaymentLetterAction recovery flow.
    let mut toast_messages = use_signal(|| Vec::<(u64, String)>::new());
    let mut toast_counter = use_signal(|| 0u64);

    // Quick 260603-evf: id is moved into use_effect for the initial fetch; we
    // need a separate stored copy so multiple on_done callbacks (one per
    // recipient row) can each re-fetch the detail after recovery succeeds.
    // `use_signal(|| id.clone())` produces a `Copy` Signal<String> we can
    // capture by-value in the per-row closures.
    let id_signal = use_signal(|| id.clone());

    use_effect(move || {
        let id = id_signal.read().clone();
        spawn(async move {
            let config = CONFIG.read().clone();
            match api::get_mail_job_detail(&config, &id).await {
                Ok(d) => detail.set(Some(d)),
                Err(e) => error.set(Some(e)),
            }
            loading.set(false);
        });
    });

    rsx! {
        TopBar {}
        div { class: "max-w-4xl mx-auto p-6",
            Link {
                to: crate::router::Route::MailPage {},
                class: "text-blue-600 hover:underline text-sm mb-4 inline-block",
                "\u{2190} {i18n.t(Key::Back)}"
            }

            if *loading.read() {
                div { class: "text-gray-500", {i18n.t(Key::Loading)} }
            } else if let Some(ref err) = *error.read() {
                ErrorAlert {
                    error: err.clone(),
                    on_dismiss: move |_| error.set(None),
                }
            } else if let Some(d) = detail.read().as_ref() {
                div { class: "space-y-4",
                    h1 { class: "text-2xl font-bold", "{d.job.subject}" }
                    div { class: "flex items-center gap-4 text-sm",
                        span { class: "{job_status_color(&d.job.status)} font-medium",
                            {i18n.t(job_status_key(&d.job.status))}
                        }
                        span { class: "text-gray-500", {i18n.format_datetime(&d.job.created)} }
                        span { class: "text-gray-500",
                            "{d.job.sent_count}/{d.job.total_count} {i18n.t(Key::MailSent)}"
                        }
                    }

                    // Mail body
                    pre { class: "bg-gray-50 p-4 border rounded text-sm whitespace-pre-wrap max-h-96 overflow-auto",
                        "{d.job.body}"
                    }

                    // Recipients table
                    div { class: "border rounded-lg overflow-hidden",
                        h3 { class: "text-sm font-medium text-gray-700 p-3 bg-gray-50",
                            {i18n.t(Key::MailRecipients)}
                        }
                        table { class: "w-full text-sm",
                            thead { tr { class: "border-b text-left text-gray-500 bg-gray-50",
                                th { class: "py-2 px-3", {i18n.t(Key::MailTo)} }
                                th { class: "py-2 px-3", {i18n.t(Key::MailStatus)} }
                                th { class: "py-2 px-3", {i18n.t(Key::MailError)} }
                                // Quick 260603-evf: action column (empty header).
                                th { class: "py-2 px-3", "" }
                            }}
                            tbody {
                                for r in d.recipients.iter() {
                                    {
                                        // Quick 260603-evf: Badge-Rendering wandert in
                                        // `MailRecipientStatusBadge` (Component-First).
                                        let error_text = r.error.clone().unwrap_or_default();
                                        // Quick 260603-evf: resolve (member_id, phase_id) tuple.
                                        let mid: Option<Uuid> = r.member_id.as_deref().and_then(|s| Uuid::parse_str(s).ok());
                                        let pid: Option<Uuid> = d.job.repayment_phase_id.as_deref().and_then(|s| Uuid::parse_str(s).ok());
                                        let show_action = is_no_repayment_letter_failure(&r.status, r.error.as_deref());
                                        let job_id_for_action = d.job.id.clone();
                                        let recipient_id_for_action = r.id.clone();
                                        let i18n_for_action = i18n.clone();
                                        rsx! {
                                            tr { class: "border-b last:border-b-0",
                                                td { class: "py-2 px-3", "{r.to_address}" }
                                                td { class: "py-2 px-3",
                                                    MailRecipientStatusBadge {
                                                        status: r.status.clone(),
                                                        error: r.error.clone(),
                                                    }
                                                }
                                                td { class: "py-2 px-3 text-red-500 text-xs", "{error_text}" }
                                                td { class: "py-2 px-3",
                                                    if show_action {
                                                        if let (Some(mid), Some(pid)) = (mid, pid) {
                                                            NoRepaymentLetterAction {
                                                                job_id: job_id_for_action,
                                                                recipient_id: recipient_id_for_action,
                                                                member_id: mid,
                                                                phase_id: pid,
                                                                on_done: move |_| {
                                                                    show_toast(
                                                                        &mut toast_messages,
                                                                        &mut toast_counter,
                                                                        i18n_for_action.t(Key::MailGenerateLetterAndRetrySuccess).to_string(),
                                                                    );
                                                                    // Quick 260603-evf: re-fetch the detail so the
                                                                    // recipient table reflects the retry result.
                                                                    let id = id_signal.read().clone();
                                                                    spawn(async move {
                                                                        let config = CONFIG.read().clone();
                                                                        if let Ok(d) = api::get_mail_job_detail(&config, &id).await {
                                                                            detail.set(Some(d));
                                                                        }
                                                                    });
                                                                },
                                                                on_error: move |msg: String| {
                                                                    show_toast(
                                                                        &mut toast_messages,
                                                                        &mut toast_counter,
                                                                        msg,
                                                                    );
                                                                },
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            // Quick 260614-9zf: per-recipient rendered
                                            // subject/body (Component-First; renders
                                            // nothing when both are None).
                                            tr {
                                                td { colspan: 4, class: "px-3",
                                                    MailRecipientRenderedContent {
                                                        rendered_subject: r.rendered_subject.clone(),
                                                        rendered_body: r.rendered_body.clone(),
                                                        rendered_reconstructed: r.rendered_reconstructed,
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
            // Quick 260603-evf: toasts for the NoRepaymentLetterAction recovery flow.
            ToastContainer { messages: toast_messages }
        }
    }
}

// ── Phase 12 D-18 — Query-Param-Parsing for Repayment Mail Redirect ──

/// Phase 12 D-18: Result of parsing the `/mail`-page URL search-string.
///
/// `phase_id` is populated only when `?phase_id=<valid-uuid>` is present.
/// `member_ids` is populated from `?members=<uuid>,<uuid>,...` filtering out invalid UUIDs.
pub struct ParsedMailContext {
    pub phase_id: Option<Uuid>,
    pub member_ids: Vec<Uuid>,
}

/// Phase 12 D-18: Parse the URL query-string into a `ParsedMailContext`.
///
/// `search` is the raw search-string (with or without leading `?`).
/// Tested directly via cargo test against native targets; does not depend on web_sys.
pub fn parse_mail_query(search: &str) -> ParsedMailContext {
    let mut phase_id: Option<Uuid> = None;
    let mut member_ids: Vec<Uuid> = Vec::new();

    let s = search.trim_start_matches('?');
    for pair in s.split('&') {
        if pair.is_empty() {
            continue;
        }
        let mut kv = pair.splitn(2, '=');
        let key = kv.next().unwrap_or("");
        let value = kv.next().unwrap_or("");
        match key {
            "phase_id" => {
                if let Ok(u) = Uuid::parse_str(value) {
                    phase_id = Some(u);
                }
            }
            "members" => {
                member_ids = value
                    .split(',')
                    .filter_map(|s| Uuid::parse_str(s.trim()).ok())
                    .collect();
            }
            _ => {}
        }
    }
    ParsedMailContext {
        phase_id,
        member_ids,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty() {
        let r = parse_mail_query("");
        assert!(r.phase_id.is_none());
        assert!(r.member_ids.is_empty());
    }

    #[test]
    fn parse_invalid_phase_id() {
        let r = parse_mail_query("?phase_id=abc");
        assert!(r.phase_id.is_none());
    }

    #[test]
    fn parse_valid_phase_id() {
        let r = parse_mail_query("?phase_id=550e8400-e29b-41d4-a716-446655440000");
        assert!(r.phase_id.is_some());
    }

    #[test]
    fn parse_valid_members() {
        let r = parse_mail_query(
            "?members=550e8400-e29b-41d4-a716-446655440000,550e8400-e29b-41d4-a716-446655440001",
        );
        assert_eq!(r.member_ids.len(), 2);
    }

    #[test]
    fn parse_members_filters_invalid() {
        let r = parse_mail_query(
            "?members=550e8400-e29b-41d4-a716-446655440000,invalid,550e8400-e29b-41d4-a716-446655440001",
        );
        assert_eq!(r.member_ids.len(), 2);
    }

    #[test]
    fn parse_combined() {
        let r = parse_mail_query(
            "?from=repayment&phase_id=550e8400-e29b-41d4-a716-446655440000&members=550e8400-e29b-41d4-a716-446655440001",
        );
        assert!(r.phase_id.is_some());
        assert_eq!(r.member_ids.len(), 1);
    }

    #[test]
    fn parse_without_leading_question_mark() {
        let r = parse_mail_query("phase_id=550e8400-e29b-41d4-a716-446655440000");
        assert!(r.phase_id.is_some());
    }
}
