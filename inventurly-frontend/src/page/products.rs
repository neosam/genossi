use crate::component::{ProductList, TopBar};
use crate::i18n::{use_i18n, Key};
use crate::router::Route;
use dioxus::prelude::*;
use dioxus_router::prelude::use_navigator;

#[component]
pub fn Products() -> Element {
    let i18n = use_i18n();
    let navigator = use_navigator();

    rsx! {
        div { class: "flex flex-col min-h-screen",
            TopBar {}
            div { class: "flex-1 container mx-auto px-4 py-8",
                div { class: "flex justify-between items-center mb-6",
                    h1 { class: "text-3xl font-bold",
                        {i18n.t(Key::Products)}
                    }
                    div { class: "flex space-x-3",
                        button {
                            class: "px-4 py-2 bg-orange-600 text-white rounded-md hover:bg-orange-700 text-sm font-medium",
                            onclick: move |_| {
                                navigator.push(Route::DuplicateDetection {});
                            },
                            {i18n.t(Key::CheckDuplicates)}
                        }
                    }
                }
                ProductList {}
            }
        }
    }
}
