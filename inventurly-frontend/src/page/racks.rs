use dioxus::prelude::*;
use crate::component::{TopBar, RackList};
use crate::i18n::{use_i18n, Key};

#[component]
pub fn Racks() -> Element {
    let i18n = use_i18n();
    
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