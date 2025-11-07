use crate::component::{InventurForm, TopBar};
use crate::i18n::{use_i18n, Key};
use dioxus::prelude::*;
use uuid::Uuid;

#[component]
pub fn InventurDetails(id: String) -> Element {
    let i18n = use_i18n();

    // Parse the UUID or treat "new" as None
    let inventur_id = if id == "new" {
        None
    } else {
        Uuid::parse_str(&id).ok()
    };

    rsx! {
        div { class: "flex flex-col min-h-screen",
            TopBar {}
            div { class: "flex-1 container mx-auto px-4 py-8",
                h1 { class: "text-3xl font-bold mb-6",
                    if inventur_id.is_some() {
                        {i18n.t(Key::EditInventur)}
                    } else {
                        {i18n.t(Key::CreateInventur)}
                    }
                }
                InventurForm { inventur_id }
            }
        }
    }
}
