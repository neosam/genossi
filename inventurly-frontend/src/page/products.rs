use crate::auth::RequirePrivilege;
use crate::component::{BarcodeScanner, ProductList, ScanResult, TopBar};
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

    // Barcode scanner state
    let mut show_scanner = use_signal(|| false);
    let mut scanner_message = use_signal(|| None::<String>);

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

    // Handle barcode scan
    let i18n_clone = i18n.clone();
    let handle_barcode_scan = move |result: ScanResult| {
        // Strip checksum digit (scanner returns EAN with checksum, DB stores without)
        let ean = if result.barcode.len() > 1 {
            &result.barcode[..result.barcode.len() - 1]
        } else {
            &result.barcode
        };

        // Set filter input and update global filter immediately (no debounce for scans)
        filter_input.set(ean.to_string());
        PRODUCTS.write().filter_query = ean.to_string();

        // Show feedback message
        scanner_message.set(Some(format!("{}: {}", i18n_clone.t(Key::SearchingForEAN), ean)));

        // Close scanner
        show_scanner.set(false);
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
                        // Scanner feedback message
                        if let Some(msg) = scanner_message() {
                            div { class: "mb-2 px-4 py-2 bg-blue-50 text-blue-700 rounded-md text-sm",
                                {msg}
                            }
                        }

                        div { class: "flex flex-col sm:flex-row gap-2",
                            div { class: "relative flex-1",
                                input {
                                    r#type: "text",
                                    class: "w-full px-4 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500 focus:border-transparent",
                                    placeholder: "{i18n.t(Key::FilterProducts)}",
                                    value: "{filter_input}",
                                    oninput: move |evt| {
                                        filter_input.set(evt.value().clone());
                                        scanner_message.set(None); // Clear message when typing
                                    },
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
                            button {
                                class: "w-full sm:w-auto px-4 py-2 bg-gray-600 text-white rounded-md hover:bg-gray-700 text-sm font-medium flex items-center gap-2 justify-center",
                                onclick: move |_| {
                                    scanner_message.set(None);
                                    show_scanner.set(true);
                                },
                                "📷 {i18n.t(Key::ScanBarcode)}"
                            }
                        }
                    }

                    ProductList {}
                }
            }

            // Barcode scanner modal
            if show_scanner() {
                BarcodeScanner {
                    on_scan: handle_barcode_scan,
                    on_close: move |_| show_scanner.set(false)
                }
            }
        }
    }
}
