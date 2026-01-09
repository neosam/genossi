use crate::component::{RackContainers, RackForm, RackProducts, TopBar};
use crate::i18n::{use_i18n, Key};
use dioxus::prelude::*;
use uuid::Uuid;

#[component]
pub fn RackDetails(id: String) -> Element {
    let i18n = use_i18n();
    let mut active_tab = use_signal(|| "products");

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

                // Show product/container management tabs for existing racks
                if let Some(id) = rack_id {
                    div { class: "mt-8",
                        // Tab bar
                        div { class: "border-b border-gray-200",
                            nav { class: "-mb-px flex space-x-8",
                                button {
                                    class: if *active_tab.read() == "products" {
                                        "border-blue-500 text-blue-600 whitespace-nowrap py-4 px-1 border-b-2 font-medium text-sm"
                                    } else {
                                        "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300 whitespace-nowrap py-4 px-1 border-b-2 font-medium text-sm"
                                    },
                                    onclick: move |_| active_tab.set("products"),
                                    {i18n.t(Key::ProductsTab)}
                                }
                                button {
                                    class: if *active_tab.read() == "containers" {
                                        "border-blue-500 text-blue-600 whitespace-nowrap py-4 px-1 border-b-2 font-medium text-sm"
                                    } else {
                                        "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300 whitespace-nowrap py-4 px-1 border-b-2 font-medium text-sm"
                                    },
                                    onclick: move |_| active_tab.set("containers"),
                                    {i18n.t(Key::ContainersTab)}
                                }
                            }
                        }

                        // Tab content
                        div { class: "mt-4",
                            match *active_tab.read() {
                                "products" => rsx! { RackProducts { rack_id: id } },
                                "containers" => rsx! { RackContainers { rack_id: id } },
                                _ => rsx! { RackProducts { rack_id: id } }
                            }
                        }
                    }
                }
            }
        }
    }
}
