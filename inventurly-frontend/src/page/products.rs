use crate::auth::RequirePrivilege;
use crate::component::{ProductList, TopBar};
use crate::i18n::{use_i18n, Key};
use crate::page::AccessDeniedPage;
use crate::router::Route;
use crate::service::product::PRODUCTS;
use dioxus::prelude::*;
use dioxus_router::prelude::use_navigator;

#[component]
pub fn Products() -> Element {
    let i18n = use_i18n();
    let navigator = use_navigator();

    // Local state for the filter input
    let mut filter_input = use_signal(|| String::new());

    // Debounced filter update effect
    use_effect(move || {
        let query = filter_input();
        spawn(async move {
            // Debounce: wait 500ms before updating the filter
            gloo_timers::future::TimeoutFuture::new(500).await;
            // Only update if the input hasn't changed
            if query == filter_input() {
                PRODUCTS.write().filter_query = query;
            }
        });
    });

    // Function to clear the filter
    let clear_filter = move |_| {
        filter_input.set(String::new());
        PRODUCTS.write().filter_query = String::new();
    };

    rsx! {
        RequirePrivilege {
            privilege: "view_inventory",
            fallback: rsx! { AccessDeniedPage { required_privilege: "view_inventory".to_string() } },
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

                    // Filter input bar
                    div { class: "mb-4",
                        div { class: "relative",
                            input {
                                r#type: "text",
                                class: "w-full px-4 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500 focus:border-transparent",
                                placeholder: "{i18n.t(Key::FilterProducts)}",
                                value: "{filter_input}",
                                oninput: move |evt| filter_input.set(evt.value().clone()),
                            }
                            if !filter_input().is_empty() {
                                button {
                                    class: "absolute right-2 top-1/2 -translate-y-1/2 px-3 py-1 text-gray-500 hover:text-gray-700",
                                    onclick: clear_filter,
                                    title: "{i18n.t(Key::ClearFilter)}",
                                    "✕"
                                }
                            }
                        }
                    }

                    ProductList {}
                }
            }
        }
    }
}
