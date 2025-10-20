use crate::api;
use crate::component::{ContainerList, TopBar};
use crate::i18n::{use_i18n, Key};
use crate::service::config::CONFIG;
use crate::service::container::CONTAINERS;
use dioxus::prelude::*;

#[component]
pub fn Containers() -> Element {
    let i18n = use_i18n();

    // Reload containers when the page mounts to ensure fresh data
    use_effect(move || {
        spawn(async move {
            let config = CONFIG.read().clone();
            if !config.backend.is_empty() {
                CONTAINERS.write().loading = true;
                match api::get_containers(&config).await {
                    Ok(containers) => {
                        CONTAINERS.write().items = containers;
                        CONTAINERS.write().error = None;
                    }
                    Err(e) => {
                        CONTAINERS.write().error =
                            Some(format!("Failed to load containers: {}", e));
                    }
                }
                CONTAINERS.write().loading = false;
            }
        });
    });

    rsx! {
        div { class: "flex flex-col min-h-screen",
            TopBar {}
            div { class: "flex-1 container mx-auto px-4 py-8",
                h1 { class: "text-3xl font-bold mb-6",
                    {i18n.t(Key::Containers)}
                }
                ContainerList {}
            }
        }
    }
}
