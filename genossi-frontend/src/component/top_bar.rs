use dioxus::prelude::*;

use crate::{
    component::{
        nav_group::{NavGroup, NavItem},
        RevokeSessionsButton,
    },
    i18n::{use_i18n, Key},
    router::Route,
    service::{auth::AUTH, config::CONFIG},
};

#[component]
pub fn TopBar() -> Element {
    let i18n = use_i18n();
    let auth_info = AUTH.read().auth_info.clone();
    let config = CONFIG.read().clone();
    let backend_url = config.backend.clone();
    let mut visible = use_signal(|| false);
    let mut open_group: Signal<Option<&'static str>> = use_signal(|| None);

    let show_members = auth_info
        .as_ref()
        .map(|a| a.has_privilege("view_members") || a.has_privilege("admin"))
        .unwrap_or(false);
    let show_permissions = auth_info
        .as_ref()
        .map(|a| a.has_privilege("admin"))
        .unwrap_or(false);
    let show_templates = auth_info
        .as_ref()
        .map(|a| a.has_privilege("manage_members") || a.has_privilege("admin"))
        .unwrap_or(false);
    let show_admin = auth_info
        .as_ref()
        .map(|a| a.has_privilege("admin"))
        .unwrap_or(false);
    let show_backup = auth_info
        .as_ref()
        .map(|a| a.has_privilege("export_backup") || a.has_privilege("admin"))
        .unwrap_or(false);

    // Build nav group items
    let mut mitglieder_items = Vec::new();
    if show_members {
        mitglieder_items.push(NavItem {
            label: i18n.t(Key::Members).to_string(),
            route: Route::Members {},
        });
        mitglieder_items.push(NavItem {
            label: i18n.t(Key::Validation).to_string(),
            route: Route::Validation {},
        });
    }
    if show_templates {
        mitglieder_items.push(NavItem {
            label: i18n.t(Key::Templates).to_string(),
            route: Route::Templates {},
        });
    }
    if show_admin {
        mitglieder_items.push(NavItem {
            label: i18n.t(Key::Applications).to_string(),
            route: Route::ApplicationsPage {},
        });
        mitglieder_items.push(NavItem {
            label: i18n.t(Key::Assemblies).to_string(),
            route: Route::Assemblies {},
        });
    }

    let mut kommunikation_items = Vec::new();
    if show_admin {
        kommunikation_items.push(NavItem {
            label: i18n.t(Key::Mail).to_string(),
            route: Route::MailPage {},
        });
        kommunikation_items.push(NavItem {
            label: i18n.t(Key::MailTemplates).to_string(),
            route: Route::MailTemplatesPage {},
        });
        kommunikation_items.push(NavItem {
            label: "Posteingang".to_string(),
            route: Route::InboxPage {},
        });
    }

    let mut verwaltung_items = Vec::new();
    if show_admin {
        verwaltung_items.push(NavItem {
            label: i18n.t(Key::Config).to_string(),
            route: Route::ConfigPage {},
        });
        verwaltung_items.push(NavItem {
            label: "Dokumente".to_string(),
            route: Route::StaticDocumentsPage {},
        });
    }
    if show_backup {
        verwaltung_items.push(NavItem {
            label: i18n.t(Key::Backup).to_string(),
            route: Route::BackupPage {},
        });
    }
    if show_admin {
        verwaltung_items.push(NavItem {
            label: i18n.t(Key::AuditLog).to_string(),
            route: Route::AuditLogPage {},
        });
    }
    if show_permissions {
        verwaltung_items.push(NavItem {
            label: i18n.t(Key::Permissions).to_string(),
            route: Route::Permissions {},
        });
    }

    rsx! {
        // Overlay to close dropdown on outside click
        if open_group.read().is_some() {
            div {
                class: "fixed inset-0 z-40",
                onclick: move |_| open_group.set(None),
            }
        }
        div { class: "flex bg-gray-800 text-white p-4 md:p-0 items-center print:hidden relative z-50",
            button {
                class: "md:hidden pr-6 pl-4 text-xl",
                onclick: move |_| {
                    let visibility = *visible.read();
                    visible.set(!visibility);
                    open_group.set(None);
                },
                "\u{2630}"
            }

            h1 { class: "text-2xl font-bold ml-2",
                "Genossi"
                if !config.is_prod {
                    span { class: "ml-2 text-sm", "{config.env_short_description}" }
                }
            }

            nav {
                class: "hidden bg-gray-800 md:pl-0 p-4 md:grow md:ml-4 md:justify-between md:flex",
                style: if *visible.read() { "display: flex; flex-direction: column; position: absolute; left: 0px; top: 64px;" } else { "" },
                ul { class: "flex flex-col md:flex-row space-y-2 md:space-y-0 md:space-x-2 ml-1",
                    if !mitglieder_items.is_empty() {
                        NavGroup {
                            label: i18n.t(Key::Members).to_string(),
                            items: mitglieder_items,
                            is_open: *open_group.read() == Some("mitglieder"),
                            on_toggle: move |_| {
                                if *open_group.read() == Some("mitglieder") {
                                    open_group.set(None);
                                } else {
                                    open_group.set(Some("mitglieder"));
                                }
                            },
                            on_navigate: move |_| {
                                open_group.set(None);
                                visible.set(false);
                            },
                        }
                    }
                    if !kommunikation_items.is_empty() {
                        NavGroup {
                            label: i18n.t(Key::Communication).to_string(),
                            items: kommunikation_items,
                            is_open: *open_group.read() == Some("kommunikation"),
                            on_toggle: move |_| {
                                if *open_group.read() == Some("kommunikation") {
                                    open_group.set(None);
                                } else {
                                    open_group.set(Some("kommunikation"));
                                }
                            },
                            on_navigate: move |_| {
                                open_group.set(None);
                                visible.set(false);
                            },
                        }
                    }
                    if !verwaltung_items.is_empty() {
                        NavGroup {
                            label: i18n.t(Key::NavAdministration).to_string(),
                            items: verwaltung_items,
                            is_open: *open_group.read() == Some("verwaltung"),
                            on_toggle: move |_| {
                                if *open_group.read() == Some("verwaltung") {
                                    open_group.set(None);
                                } else {
                                    open_group.set(Some("verwaltung"));
                                }
                            },
                            on_navigate: move |_| {
                                open_group.set(None);
                                visible.set(false);
                            },
                        }
                    }
                }
                ul { class: "flex flex-col md:flex-row space-y-4 md:space-y-0 md:space-x-4 mr-4",
                    if let Some(auth) = &auth_info {
                        li { class: "px-3 py-2 md:py-4 text-gray-300",
                            "{auth.user}"
                        }
                        RevokeSessionsButton {}
                        li {
                            a {
                                class: "hover:underline px-3 py-2 md:py-4",
                                href: format!("{}/logout", backend_url),
                                {i18n.t(Key::Logout)}
                            }
                        }
                    }
                }
            }
        }
    }
}
