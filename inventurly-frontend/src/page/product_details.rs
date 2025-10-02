use dioxus::prelude::*;
use uuid::Uuid;
use crate::component::{TopBar, ProductForm};
use crate::i18n::{use_i18n, Key};

#[component]
pub fn ProductDetails(id: String) -> Element {
    let i18n = use_i18n();
    let product_id = id.parse::<Uuid>().ok();
    
    rsx! {
        div { class: "flex flex-col min-h-screen",
            TopBar {}
            div { class: "flex-1 container mx-auto px-4 py-8",
                div { class: "max-w-2xl mx-auto",
                    ProductForm { product_id }
                }
            }
        }
    }
}