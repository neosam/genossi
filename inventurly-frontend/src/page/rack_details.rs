use dioxus::prelude::*;
use uuid::Uuid;
use crate::component::{TopBar, RackForm};
use crate::i18n::{use_i18n, Key};

#[component]
pub fn RackDetails(id: String) -> Element {
    let i18n = use_i18n();
    
    let rack_id = if id == "new" {
        None
    } else {
        Uuid::parse_str(&id).ok()
    };
    
    rsx! {
        div { class: "flex flex-col min-h-screen",
            TopBar {}
            div { class: "flex-1 container mx-auto px-4 py-8",
                h1 { class: "text-3xl font-bold mb-6",
                    if rack_id.is_some() {
                        {i18n.t(Key::Edit)}
                    } else {
                        {i18n.t(Key::Create)}
                    }
                    " "
                    {i18n.t(Key::Racks)}
                }
                RackForm { rack_id }
            }
        }
    }
}