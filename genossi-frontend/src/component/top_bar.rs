use dioxus::prelude::*;

use crate::{
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

    rsx! {
        div { class: "flex bg-gray-800 text-white p-4 md:p-0 items-center print:hidden",
            button {
                class: "md:hidden pr-6 pl-4 text-xl",
                onclick: move |_| {
                    let visibility = *visible.read();
                    visible.set(!visibility)
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
                ul { class: "flex flex-col md:flex-row space-y-4 md:space-y-0 md:space-x-4 ml-1",
                    if show_members {
                        li {
                            Link { class: "hover:underline px-3 py-2 md:py-4", to: Route::Members {}, {i18n.t(Key::Members)} }
                        }
                    }
                    if show_members {
                        li {
                            Link { class: "hover:underline px-3 py-2 md:py-4", to: Route::Validation {}, {i18n.t(Key::Validation)} }
                        }
                    }
                    if show_templates {
                        li {
                            Link { class: "hover:underline px-3 py-2 md:py-4", to: Route::Templates {}, {i18n.t(Key::Templates)} }
                        }
                    }
                    if show_admin {
                        li {
                            Link { class: "hover:underline px-3 py-2 md:py-4", to: Route::ConfigPage {}, {i18n.t(Key::Config)} }
                        }
                    }
                    if show_admin {
                        li {
                            Link { class: "hover:underline px-3 py-2 md:py-4", to: Route::MailPage {}, {i18n.t(Key::Mail)} }
                        }
                    }
                    if show_permissions {
                        li {
                            Link { class: "hover:underline px-3 py-2 md:py-4", to: Route::Permissions {}, {i18n.t(Key::Permissions)} }
                        }
                    }
                }
                ul { class: "flex flex-col md:flex-row space-y-4 md:space-y-0 md:space-x-4 mr-4",
                    if let Some(auth) = &auth_info {
                        li { class: "px-3 py-2 md:py-4 text-gray-300",
                            "{auth.user}"
                        }
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
