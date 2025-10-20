use dioxus::prelude::*;
use crate::component::{TopBar, RackList};
use crate::i18n::{use_i18n, Key};
use crate::service::rack::RACKS;
use crate::service::config::CONFIG;
use crate::api;

#[component]
pub fn Racks() -> Element {
    let i18n = use_i18n();
    
    // Reload racks when the page mounts to ensure fresh data
    use_effect(move || {
        spawn(async move {
            let config = CONFIG.read().clone();
            if !config.backend.is_empty() {
                RACKS.write().loading = true;
                match api::get_racks(&config).await {
                    Ok(racks) => {
                        RACKS.write().items = racks;
                        RACKS.write().error = None;
                    }
                    Err(e) => {
                        RACKS.write().error = Some(format!("Failed to load racks: {}", e));
                    }
                }
                RACKS.write().loading = false;
            }
        });
    });
    
    rsx! {
        div { class: "flex flex-col min-h-screen",
            TopBar {}
            div { class: "flex-1 container mx-auto px-4 py-8",
                h1 { class: "text-3xl font-bold mb-6",
                    {i18n.t(Key::Racks)}
                }
                RackList {}
            }
        }
    }
}