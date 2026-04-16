use dioxus::prelude::*;
use uuid::Uuid;

use crate::api::{ApplicationStatusTO, ApplicationTO};
use crate::i18n::{use_i18n, Key};

fn status_label(i18n: &crate::i18n::I18n, status: &ApplicationStatusTO) -> String {
    match status {
        ApplicationStatusTO::Offen => i18n.t(Key::StatusOffen).to_string(),
        ApplicationStatusTO::Bestaetigt => i18n.t(Key::StatusBestaetigt).to_string(),
        ApplicationStatusTO::Abgelehnt => i18n.t(Key::StatusAbgelehnt).to_string(),
    }
}

fn status_badge_class(status: &ApplicationStatusTO) -> &'static str {
    match status {
        ApplicationStatusTO::Offen => {
            "bg-yellow-100 text-yellow-800 px-2 py-1 rounded text-xs font-medium"
        }
        ApplicationStatusTO::Bestaetigt => {
            "bg-green-100 text-green-800 px-2 py-1 rounded text-xs font-medium"
        }
        ApplicationStatusTO::Abgelehnt => {
            "bg-red-100 text-red-800 px-2 py-1 rounded text-xs font-medium"
        }
    }
}

#[component]
pub fn ApplicationList(applications: Vec<ApplicationTO>, on_select: EventHandler<Uuid>) -> Element {
    let i18n = use_i18n();

    if applications.is_empty() {
        return rsx! {
            p { class: "text-gray-500 text-center py-8", {i18n.t(Key::NoApplications)} }
        };
    }

    rsx! {
        div { class: "overflow-x-auto",
            table { class: "w-full",
                thead {
                    tr { class: "border-b bg-gray-50",
                        th { class: "text-left py-3 px-4 text-sm font-medium text-gray-600", {i18n.t(Key::FirstName)} }
                        th { class: "text-left py-3 px-4 text-sm font-medium text-gray-600", {i18n.t(Key::LastName)} }
                        th { class: "text-left py-3 px-4 text-sm font-medium text-gray-600", {i18n.t(Key::Email)} }
                        th { class: "text-left py-3 px-4 text-sm font-medium text-gray-600", {i18n.t(Key::Shares)} }
                        th { class: "text-left py-3 px-4 text-sm font-medium text-gray-600", "Status" }
                        th { class: "text-left py-3 px-4 text-sm font-medium text-gray-600", {i18n.t(Key::SubmittedAt)} }
                    }
                }
                tbody {
                    for app in applications.iter() {
                        {
                            let id = app.id;
                            let status_text = status_label(&i18n, &app.status);
                            let badge_class = status_badge_class(&app.status);
                            let created = app
                                .created
                                .as_deref()
                                .map(|s| i18n.format_datetime(s))
                                .unwrap_or_else(|| "-".to_string());
                            rsx! {
                                tr {
                                    class: "border-b hover:bg-gray-50 cursor-pointer",
                                    onclick: move |_| on_select.call(id),
                                    td { class: "py-3 px-4", "{app.first_name}" }
                                    td { class: "py-3 px-4", "{app.last_name}" }
                                    td { class: "py-3 px-4 text-sm", {app.email.as_deref().unwrap_or("-")} }
                                    td { class: "py-3 px-4", "{app.shares}" }
                                    td { class: "py-3 px-4",
                                        span { class: "{badge_class}", "{status_text}" }
                                    }
                                    td { class: "py-3 px-4 text-sm text-gray-500", "{created}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
