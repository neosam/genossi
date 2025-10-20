use dioxus::prelude::*;

use crate::{
    i18n::{Key, use_i18n},
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
    
    let show_products = auth_info.is_some();

    rsx! {
        div { class: "flex bg-gray-800 text-white p-4 md:p-0 items-center print:hidden",
            button {
                class: "md:hidden pr-6 pl-4 text-xl",
                onclick: move |_| {
                    let visibility = *visible.read();
                    visible.set(!visibility)
                },
                "☰"
            }

            h1 { class: "text-2xl font-bold ml-2",
                "Inventurly"
                if !config.is_prod {
                    span { class: "ml-2 text-sm", "{config.env_short_description}" }
                }
            }

            nav {
                class: "hidden bg-gray-800 md:pl-0 p-4 md:grow md:ml-4 md:justify-between md:flex",
                style: if *visible.read() { "display: flex; flex-direction: column; position: absolute; left: 0px; top: 64px;" } else { "" },
                ul { class: "flex flex-col md:flex-row space-y-4 md:space-y-0 md:space-x-4 ml-1",
                    if show_products {
                        li {
                            Link { to: Route::Products {}, {i18n.t(Key::Products)} }
                        }
                        li {
                            Link { to: Route::Racks {}, {i18n.t(Key::Racks)} }
                        }
                    }
                    if auth_info.is_some() {
                        div { class: "mb-6 md:mb-0" }
                    }
                }
                ul { class: "ml-1",
                    li { class: "flex",
                        if let Some(auth_info) = auth_info {
                            a { href: "{backend_url}/logout", 
                                { i18n.t(Key::Logout).replace("{user}", &auth_info.user) }
                            }
                        } else {
                            a { href: "/authenticate", {i18n.t(Key::Login)} }
                        }
                    }
                }
            }
        }
        div {
        }
    }
}
