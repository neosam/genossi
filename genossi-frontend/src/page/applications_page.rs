use dioxus::prelude::*;

use crate::api::{self, ApplicationTO};
use crate::auth::RequirePrivilege;
use crate::component::{ApplicationDetail, ApplicationList, TopBar};
use crate::i18n::{use_i18n, Key};
use crate::page::AccessDeniedPage;
use crate::service::config::CONFIG;
use uuid::Uuid;

#[component]
pub fn ApplicationsPage() -> Element {
    let i18n = use_i18n();
    let mut applications = use_signal(|| Vec::<ApplicationTO>::new());
    let mut loading = use_signal(|| true);
    let mut error = use_signal(|| None::<String>);
    let mut active_tab = use_signal(|| "Offen".to_string());
    let mut selected_app = use_signal(|| None::<ApplicationTO>);

    let load = move || {
        let tab = active_tab.read().clone();
        spawn(async move {
            loading.set(true);
            error.set(None);
            let config = CONFIG.read().clone();
            let filter = if tab == "all" { None } else { Some(tab.as_str()) };
            match api::get_applications(&config, filter).await {
                Ok(data) => {
                    applications.set(data);
                }
                Err(e) => {
                    error.set(Some(format!("{}", e)));
                }
            }
            loading.set(false);
        });
    };

    use_effect(move || {
        load();
    });

    let tabs = [
        ("Offen", Key::StatusOffen),
        ("Bestaetigt", Key::StatusBestaetigt),
        ("Abgelehnt", Key::StatusAbgelehnt),
        ("all", Key::StatusAll),
    ];

    rsx! {
        RequirePrivilege {
            privilege: "admin",
            fallback: rsx! { AccessDeniedPage { required_privilege: "admin".to_string() } },
            TopBar {}
            div { class: "container mx-auto px-4 py-6",
                h1 { class: "text-2xl font-bold mb-1", {i18n.t(Key::Applications)} }
                p { class: "text-sm text-gray-500 mb-4", {i18n.t(Key::ApplicationsDesc)} }

                // Status tabs
                div { class: "flex space-x-1 mb-6 border-b",
                    for (value, label_key) in tabs.iter() {
                        {
                            let value = value.to_string();
                            let is_active = *active_tab.read() == value;
                            let tab_class = if is_active {
                                "px-4 py-2 border-b-2 border-blue-500 text-blue-600 font-medium text-sm"
                            } else {
                                "px-4 py-2 text-gray-500 hover:text-gray-700 text-sm"
                            };
                            rsx! {
                                button {
                                    class: "{tab_class}",
                                    onclick: {
                                        let value = value.clone();
                                        move |_| {
                                            active_tab.set(value.clone());
                                            load();
                                        }
                                    },
                                    {i18n.t(label_key.clone())}
                                }
                            }
                        }
                    }
                }

                // Error
                if let Some(err) = error.read().as_ref() {
                    div { class: "mb-4 p-3 bg-red-50 border border-red-200 rounded text-red-700 text-sm",
                        "{err}"
                    }
                }

                // Content
                if *loading.read() {
                    p { class: "text-gray-500 text-center py-8", {i18n.t(Key::Loading)} }
                } else {
                    div { class: "bg-white rounded-lg shadow",
                        ApplicationList {
                            applications: applications.read().clone(),
                            on_select: move |id: Uuid| {
                                if let Some(app) = applications.read().iter().find(|a| a.id == id) {
                                    selected_app.set(Some(app.clone()));
                                }
                            },
                        }
                    }
                }

                // Detail modal
                if let Some(app) = selected_app.read().clone() {
                    ApplicationDetail {
                        application: app,
                        on_close: move |_| selected_app.set(None),
                        on_changed: move |_| {
                            selected_app.set(None);
                            load();
                        },
                    }
                }
            }
        }
    }
}
