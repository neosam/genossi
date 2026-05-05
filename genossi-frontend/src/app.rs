use crate::auth::Auth;
use crate::component::dropdown_base::DropdownBase;
use crate::component::{Footer, TopBar};
use crate::page::NotAuthenticated;
use crate::router::Route;
use crate::service;
use crate::service::config::CONFIG;
use dioxus::prelude::*;

pub fn App() -> Element {
    use_coroutine(service::config::config_service);
    use_coroutine(service::dropdown::dropdown_service);
    use_coroutine(service::i18n::i18n_service);
    let config = CONFIG.read();

    if !config.backend.is_empty() {
        let title = config.application_title.clone();
        let is_prod = config.is_prod;
        let env_short_description = config.env_short_description.clone();

        use_effect(move || {
            let window = web_sys::window().unwrap();
            let document = window.document().unwrap();
            if is_prod {
                document.set_title(title.as_ref());
            } else {
                document.set_title(format!("{} ({})", title, env_short_description).as_str());
            }
        });

        // Phase 4 Plan 07 (D-05/D-07): Layout-Branch für /helper*-Routes.
        // Helfer haben keinen OIDC-Login → Auth-Wrapper umgehen.
        // Helfer dürfen kein TopBar/Footer mit Vorstand-Links sehen → Datenschutz.
        let pathname = web_sys::window()
            .and_then(|w| w.location().pathname().ok())
            .unwrap_or_default();
        let is_helper_route = pathname.starts_with("/helper");

        if is_helper_route {
            rsx! {
                document::Stylesheet { href: "/assets/tailwind.css" }
                div { class: "min-h-screen flex flex-col",
                    DropdownBase {}
                    // KEIN TopBar/Footer/Auth-Wrapper für /helper*
                    Router::<Route> {}
                }
            }
        } else {
            rsx! {
                document::Stylesheet { href: "/assets/tailwind.css" }
                div { class: "flex flex-col min-h-screen",
                    DropdownBase {}
                    div { class: "flex-1",
                        Auth {
                            authenticated: rsx! {
                                Router::<Route> {}
                            },
                            unauthenticated: rsx! {
                                TopBar {}
                                NotAuthenticated {}
                            },
                        }
                    }
                    Footer {}
                }
            }
        }
    } else {
        rsx! {
            div { "Loading application configuration." }
        }
    }
}
