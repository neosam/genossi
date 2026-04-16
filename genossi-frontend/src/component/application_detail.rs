use dioxus::prelude::*;

use crate::api::{self, ApplicationStatusTO, ApplicationTO};
use crate::component::Modal;
use crate::i18n::{use_i18n, Key};
use crate::service::config::CONFIG;

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

    let is_open = application.status == ApplicationStatusTO::Offen;
    let app_id = application.id;

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
