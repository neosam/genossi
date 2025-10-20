use crate::component::{ContainerForm, TopBar};
use crate::i18n::{use_i18n, Key};
use dioxus::prelude::*;
use uuid::Uuid;

#[component]
pub fn ContainerDetails(id: String) -> Element {
    let i18n = use_i18n();

    let container_id = if id == "new" {
        None
    } else {
        Uuid::parse_str(&id).ok()
    };

    rsx! {
        div { class: "flex flex-col min-h-screen",
            TopBar {}
            div { class: "flex-1 container mx-auto px-4 py-8",
                h1 { class: "text-3xl font-bold mb-6",
                    if container_id.is_some() {
                        {i18n.t(Key::Edit)}
                    } else {
                        {i18n.t(Key::Create)}
                    }
                    " "
                    {i18n.t(Key::Containers)}
                }
                ContainerForm { container_id: container_id }
            }
        }
    }
}
