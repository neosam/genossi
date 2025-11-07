use crate::component::TopBar;
use crate::i18n::{use_i18n, Key};
use dioxus::prelude::*;

#[component]
pub fn InventurMeasurements(id: String) -> Element {
    let i18n = use_i18n();

    rsx! {
        div { class: "flex flex-col min-h-screen",
            TopBar {}
            div { class: "flex-1 container mx-auto px-4 py-8",
                h1 { class: "text-3xl font-bold mb-6",
                    {i18n.t(Key::Measurements)}
                }
                div { class: "bg-white rounded-lg shadow p-6",
                    p { "Inventur Measurements - Inventur ID: {id}" }
                    p { class: "text-gray-500 mt-4",
                        "This page is under construction."
                    }
                }
            }
        }
    }
}
