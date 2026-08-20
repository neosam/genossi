use dioxus::prelude::*;

use crate::api::{self, ApplicationStatusTO, ApplicationTO};
use crate::component::{ApplicationDocumentSlot, CommunicationTimeline, Modal};
use crate::i18n::{use_i18n, Key};
use crate::router::Route;
use crate::service::config::CONFIG;
use crate::util::email::is_email_empty;

/// Uebersetztes Status-Label fuer die "zuletzt gesendet"-Zeile. Spiegelt das
/// Mapping der CommunicationTimeline-Badges (sent/failed/pending).
fn outbound_status_label(i18n: &crate::i18n::I18n, status: Option<&str>) -> String {
    match status {
        Some("sent") => i18n.t(Key::CommunicationStatusSent).to_string(),
        Some("failed") => i18n.t(Key::CommunicationStatusFailed).to_string(),
        _ => i18n.t(Key::CommunicationStatusPending).to_string(),
    }
}

fn salutation_label(s: &rest_types::SalutationTO) -> &'static str {
    match s {
        rest_types::SalutationTO::Herr => "Herr",
        rest_types::SalutationTO::Frau => "Frau",
        rest_types::SalutationTO::Firma => "Firma",
    }
}

#[component]
pub fn ApplicationDetail(
    application: ApplicationTO,
    on_close: EventHandler<()>,
    on_changed: EventHandler<()>,
    on_edit: EventHandler<ApplicationTO>,
) -> Element {
    let i18n = use_i18n();
    let mut confirming = use_signal(|| false);
    let mut rejecting = use_signal(|| false);
    let mut show_confirm_dialog = use_signal(|| false);
    let mut show_reject_dialog = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    // Phase 32 (D-06/APUI-03): Kommunikations-Historie + Body-Detail-Panel.
    let mut communications = use_signal(Vec::<rest_types::CommunicationEntryTO>::new);
    let mut selected_entry = use_signal(|| None::<rest_types::CommunicationEntryTO>);

    let is_open = application.status == ApplicationStatusTO::Offen;
    let app_id = application.id;
    let nav = navigator();
    let email_empty = is_email_empty(application.email.as_deref());

    // Kommunikation beim Mount laden (Muster get_member_communications).
    use_effect(move || {
        spawn(async move {
            let config = CONFIG.read().clone();
            if let Ok(data) = api::get_application_communications(&config, app_id).await {
                communications.set(data);
            }
        });
    });

    rsx! {
        Modal {
            // Title + close button
            div { class: "flex justify-between items-center mb-4",
                h2 { class: "text-xl font-semibold", {i18n.t(Key::ApplicationDetails)} }
                button {
                    class: "text-gray-400 hover:text-gray-600 text-2xl leading-none",
                    onclick: move |_| on_close.call(()),
                    "×"
                }
            }

            // Error message
            if let Some(err) = error.read().as_ref() {
                div { class: "mb-4 p-3 bg-red-50 border border-red-200 rounded text-red-700 text-sm",
                    "{err}"
                }
            }

            // Detail fields
            div { class: "space-y-3",
                if let Some(sal) = &application.salutation {
                    div { class: "grid grid-cols-3 gap-2",
                        span { class: "text-sm text-gray-500", {i18n.t(Key::Salutation)} }
                        span { class: "col-span-2", {salutation_label(sal)} }
                    }
                }
                if let Some(title) = &application.title {
                    div { class: "grid grid-cols-3 gap-2",
                        span { class: "text-sm text-gray-500", {i18n.t(Key::Title)} }
                        span { class: "col-span-2", "{title}" }
                    }
                }
                div { class: "grid grid-cols-3 gap-2",
                    span { class: "text-sm text-gray-500", {i18n.t(Key::FirstName)} }
                    span { class: "col-span-2 font-medium", "{application.first_name}" }
                }
                div { class: "grid grid-cols-3 gap-2",
                    span { class: "text-sm text-gray-500", {i18n.t(Key::LastName)} }
                    span { class: "col-span-2 font-medium", "{application.last_name}" }
                }
                if let Some(email) = &application.email {
                    div { class: "grid grid-cols-3 gap-2",
                        span { class: "text-sm text-gray-500", {i18n.t(Key::Email)} }
                        span { class: "col-span-2", "{email}" }
                    }
                }
                if application.street.is_some() || application.house_number.is_some() {
                    div { class: "grid grid-cols-3 gap-2",
                        span { class: "text-sm text-gray-500", {i18n.t(Key::Street)} }
                        span { class: "col-span-2",
                            {application.street.as_deref().unwrap_or("")}
                            " "
                            {application.house_number.as_deref().unwrap_or("")}
                        }
                    }
                }
                if application.postal_code.is_some() || application.city.is_some() {
                    div { class: "grid grid-cols-3 gap-2",
                        span { class: "text-sm text-gray-500", {i18n.t(Key::City)} }
                        span { class: "col-span-2",
                            {application.postal_code.as_deref().unwrap_or("")}
                            " "
                            {application.city.as_deref().unwrap_or("")}
                        }
                    }
                }
                div { class: "grid grid-cols-3 gap-2",
                    span { class: "text-sm text-gray-500", {i18n.t(Key::Shares)} }
                    span { class: "col-span-2 font-medium", "{application.shares}" }
                }
                div { class: "grid grid-cols-3 gap-2",
                    span { class: "text-sm text-gray-500", {i18n.t(Key::SubmittedAt)} }
                    span { class: "col-span-2 text-sm",
                        {
                            application
                                .created
                                .as_deref()
                                .map(|s| i18n.format_datetime(s))
                                .unwrap_or_else(|| "-".to_string())
                        }
                    }
                }
            }

            // Phase 25 (APDOC-05): Single-slot Application document. Only rendered
            // for Offen applications — after confirm() the document is moved to
            // the newly-created Member and the row is soft-deleted.
            if is_open {
                ApplicationDocumentSlot {
                    application_id: app_id,
                    on_changed: move |_| on_changed.call(()),
                }
            }

            // Action buttons
            div { class: "flex space-x-3 mt-6 pt-4 border-t",
                button {
                    class: "bg-blue-500 hover:bg-blue-600 text-white px-4 py-2 rounded",
                    onclick: {
                        let app = application.clone();
                        move |_| on_edit.call(app.clone())
                    },
                    {i18n.t(Key::EditApplication)}
                }
                if is_open {
                    button {
                        class: "bg-green-500 hover:bg-green-600 text-white px-4 py-2 rounded disabled:opacity-50",
                        disabled: *confirming.read() || *rejecting.read(),
                        onclick: move |_| show_confirm_dialog.set(true),
                        if *confirming.read() {
                            {i18n.t(Key::Loading)}
                        } else {
                            {i18n.t(Key::ConfirmApplication)}
                        }
                    }
                    button {
                        class: "bg-red-500 hover:bg-red-600 text-white px-4 py-2 rounded disabled:opacity-50",
                        disabled: *confirming.read() || *rejecting.read(),
                        onclick: move |_| show_reject_dialog.set(true),
                        if *rejecting.read() {
                            {i18n.t(Key::Loading)}
                        } else {
                            {i18n.t(Key::RejectApplication)}
                        }
                    }
                }
            }

            // === Phase 32: E-Mail-Kommunikation (D-02/D-06, APMAIL-03/APUI-03) ===
            div { class: "mt-8 pt-4 border-t",
                // Header + Senden-Button (primaerer Anker, einziger blauer Akzent)
                div { class: "flex items-center justify-between mb-4",
                    h3 { class: "text-xl font-medium", {i18n.t(Key::Communication)} }
                    div { class: "flex items-center gap-2",
                        if email_empty {
                            span { class: "text-sm text-gray-500 italic",
                                {i18n.t(Key::NoEmailAddressHint)}
                            }
                        }
                        button {
                            class: "px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 text-sm font-medium disabled:opacity-50 disabled:cursor-not-allowed",
                            disabled: email_empty,
                            title: if email_empty { i18n.t(Key::NoEmailAddressHint).to_string() } else { String::new() },
                            onclick: move |_| {
                                if !email_empty {
                                    nav.push(Route::ApplicationCompose {
                                        id: app_id.to_string(),
                                    });
                                }
                            },
                            "✉ {i18n.t(Key::MailSendButton)}"
                        }
                    }
                }

                // "zuletzt gesendet"-Zeile (D-06, anti-double-send) — Betreff + Status + Datum
                {
                    let comms = communications.read();
                    match api::last_outbound_summary(&comms) {
                        Some((subject, status, date)) => {
                            let status_label = outbound_status_label(&i18n, status.as_deref());
                            let date_str = i18n.format_datetime(&date);
                            rsx! {
                                p { class: "text-sm text-gray-600 mb-4",
                                    span { class: "font-medium", {i18n.t(Key::LastSentSummary)} }
                                    ": {subject} — {status_label} am {date_str}"
                                }
                            }
                        }
                        None => rsx! {
                            p { class: "text-sm text-gray-500 italic mb-4",
                                {i18n.t(Key::NeverSent)}
                            }
                        },
                    }
                }

                // Kommunikations-Historie (unveraenderte, prop-getriebene Timeline)
                CommunicationTimeline {
                    entries: communications.read().clone(),
                    on_entry_click: move |entry: rest_types::CommunicationEntryTO| {
                        selected_entry.set(Some(entry));
                    },
                }

                // Body-Detail-Panel (D-06): Inline-Expand, KEIN Modal-in-Modal.
                // Zeigt den echten gespeicherten Body (kein Re-Render), HTML-escaped
                // in einem begrenzten Scroll-Container (T-32-03).
                if let Some(entry) = selected_entry.read().as_ref() {
                    {
                        let body = entry
                            .rendered_body
                            .clone()
                            .or_else(|| entry.rendered_html_body.clone())
                            .unwrap_or_default();
                        rsx! {
                            div { class: "mt-4 p-4 bg-gray-50 border border-gray-200 rounded",
                                div { class: "flex items-center justify-between mb-2",
                                    h4 { class: "text-xl font-medium", {i18n.t(Key::SentMailBody)} }
                                    button {
                                        class: "text-gray-400 hover:text-gray-600 text-2xl leading-none",
                                        onclick: move |_| selected_entry.set(None),
                                        "×"
                                    }
                                }
                                div { class: "text-sm whitespace-pre-wrap max-h-96 overflow-auto",
                                    "{body}"
                                }
                            }
                        }
                    }
                }
            }
        }

        // Confirm dialog
        if *show_confirm_dialog.read() {
            Modal {
                div { class: "space-y-4",
                    h3 { class: "text-lg font-semibold", {i18n.t(Key::ConfirmApplication)} }
                    p { {i18n.t(Key::ConfirmApplicationHint)} }
                    div { class: "flex space-x-3 justify-end",
                        button {
                            class: "px-4 py-2 border rounded hover:bg-gray-50",
                            onclick: move |_| show_confirm_dialog.set(false),
                            {i18n.t(Key::Cancel)}
                        }
                        button {
                            class: "bg-green-500 hover:bg-green-600 text-white px-4 py-2 rounded",
                            onclick: move |_| {
                                show_confirm_dialog.set(false);
                                spawn(async move {
                                    confirming.set(true);
                                    let config = CONFIG.read().clone();
                                    match api::confirm_application(&config, app_id).await {
                                        Ok(_) => on_changed.call(()),
                                        Err(e) => error.set(Some(format!("{}", e))),
                                    }
                                    confirming.set(false);
                                });
                            },
                            {i18n.t(Key::Confirm)}
                        }
                    }
                }
            }
        }

        // Reject dialog
        if *show_reject_dialog.read() {
            Modal {
                div { class: "space-y-4",
                    h3 { class: "text-lg font-semibold", {i18n.t(Key::RejectApplication)} }
                    p { {i18n.t(Key::RejectApplicationHint)} }
                    div { class: "flex space-x-3 justify-end",
                        button {
                            class: "px-4 py-2 border rounded hover:bg-gray-50",
                            onclick: move |_| show_reject_dialog.set(false),
                            {i18n.t(Key::Cancel)}
                        }
                        button {
                            class: "bg-red-500 hover:bg-red-600 text-white px-4 py-2 rounded",
                            onclick: move |_| {
                                show_reject_dialog.set(false);
                                spawn(async move {
                                    rejecting.set(true);
                                    let config = CONFIG.read().clone();
                                    match api::reject_application(&config, app_id).await {
                                        Ok(_) => on_changed.call(()),
                                        Err(e) => error.set(Some(format!("{}", e))),
                                    }
                                    rejecting.set(false);
                                });
                            },
                            {i18n.t(Key::Confirm)}
                        }
                    }
                }
            }
        }
    }
}
