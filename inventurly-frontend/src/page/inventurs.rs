use crate::api;
use crate::component::{InventurList, TopBar};
use crate::i18n::{use_i18n, Key};
use crate::service::config::CONFIG;
use crate::service::inventur::INVENTURS;
use dioxus::prelude::*;

#[component]
pub fn Inventurs() -> Element {
    let i18n = use_i18n();

    // Reload inventurs when the page mounts to ensure fresh data
    use_effect(move || {
        spawn(async move {
            let config = CONFIG.read().clone();
            if !config.backend.is_empty() {
                INVENTURS.write().loading = true;
                match api::get_inventurs(&config).await {
                    Ok(inventurs) => {
                        INVENTURS.write().items = inventurs;
                        INVENTURS.write().error = None;
                    }
                    Err(e) => {
                        INVENTURS.write().error = Some(format!("Failed to load inventurs: {}", e));
                    }
                }
                INVENTURS.write().loading = false;
            }
        });
    });

    rsx! {
        div { class: "flex flex-col min-h-screen",
            TopBar {}
            div { class: "flex-1 container mx-auto px-4 py-8",
                h1 { class: "text-3xl font-bold mb-6",
                    {i18n.t(Key::Inventurs)}
                }
                InventurList {}
            }
        }
    }
}
