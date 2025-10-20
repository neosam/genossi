use crate::api;
use crate::component::{BarcodeScanner, ScanResult, SearchableProductSelector};
use crate::i18n::{use_i18n, Key};
use crate::service::config::CONFIG;
use crate::service::product::{ProductService, PRODUCTS};
use crate::service::product_rack::add_product_to_rack_action;
use dioxus::prelude::*;
use rest_types::RackTO;
use uuid::Uuid;

#[component]
pub fn ProductRackForm(
    product_id: Option<Uuid>,
    rack_id: Option<Uuid>,
    on_saved: EventHandler<()>,
    on_cancel: EventHandler<()>,
) -> Element {
    let i18n = use_i18n();
    let product_service = use_coroutine_handle::<ProductService>();

    let mut selected_product = use_signal(|| product_id);
    let mut selected_rack = use_signal(|| rack_id);
    let racks = use_signal(|| Vec::<RackTO>::new());
    let loading = use_signal(|| false);
    let error = use_signal(|| None::<String>);
    let saving = use_signal(|| false);
    let mut show_scanner = use_signal(|| false);
    let mut scanner_message = use_signal(|| None::<String>);

    // Load racks and products on mount
    use_effect(move || {
        // Load products via ProductService (into global state)
        product_service.send(ProductService::LoadProducts);

        // Load racks
        spawn({
            let mut racks = racks.clone();
            let mut loading = loading.clone();
            let mut error = error.clone();

            async move {
                loading.set(true);
                let config = CONFIG.read().clone();

                match api::get_racks(&config).await {
                    Ok(rack_list) => {
                        racks.set(rack_list);
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to load racks: {}", e)));
                    }
                }

                loading.set(false);
            }
        });
    });

    let mut handle_product_selected = move |product_id: Option<Uuid>| {
        selected_product.set(product_id);
    };

    let handle_barcode_scan = move |scan_result: ScanResult| {
        let scanned_ean = scan_result.barcode;
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
            "ProductRackForm: Barcode scan result: {}",
            scanned_ean
        )));

        // Remove checksum digit from EAN (last digit)
        let ean = if scanned_ean.len() > 1 {
            &scanned_ean[..scanned_ean.len() - 1]
        } else {
            &scanned_ean
        };
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
            "ProductRackForm: EAN without checksum: {}",
            ean
        )));

        // Check if products are loaded
        let products_state = PRODUCTS.read();
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
            "ProductRackForm: Products loaded: {}",
            products_state.items.len()
        )));

        if products_state.items.is_empty() {
            scanner_message.set(Some(
                "⚠ Products not loaded yet, please try again".to_string(),
            ));
            show_scanner.set(false);
            return;
        }

        // Find product by EAN in global PRODUCTS state
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
            "ProductRackForm: Searching for EAN {} in {} products",
            ean,
            products_state.items.len()
        )));

        if let Some(product) = products_state.items.iter().find(|p| p.ean == ean) {
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
                "ProductRackForm: Found product {} (EAN: {}) with ID {:?}",
                product.name, product.ean, product.id
            )));

            if let Some(product_id) = product.id {
                web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
                    "ProductRackForm: Calling handle_product_selected with ID: {}",
                    product_id
                )));

                // Only call handle_product_selected - it will update the selected_product signal
                // and trigger the SearchableProductSelector to update
                handle_product_selected(Some(product_id));

                web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("ProductRackForm: After calling handle_product_selected, selected_product is: {:?}", selected_product())));

                scanner_message.set(Some(format!("✓ Product selected: {}", product.name)));
            } else {
                web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(
                    "ProductRackForm: Product found but has no ID",
                ));
                scanner_message.set(Some("⚠ Product found but has no ID".to_string()));
            }
        } else {
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
                "ProductRackForm: No product found with EAN: {} (scanned: {})",
                ean, scanned_ean
            )));
            scanner_message.set(Some(format!("❌ No product found with EAN: {}", ean)));
        }

        show_scanner.set(false);
    };

    let handle_scanner_close = move |_| {
        show_scanner.set(false);
        scanner_message.set(None);
    };

    let handle_save = move |_| {
        if let (Some(prod_id), Some(rack_id)) = (selected_product(), selected_rack()) {
            spawn({
                let mut saving = saving.clone();
                let mut error = error.clone();
                let on_saved = on_saved.clone();

                async move {
                    saving.set(true);
                    error.set(None);

                    match add_product_to_rack_action(prod_id, rack_id).await {
                        Ok(()) => {
                            on_saved.call(());
                        }
                        Err(e) => {
                            error.set(Some(e));
                        }
                    }

                    saving.set(false);
                }
            });
        }
    };

    let is_valid = selected_product().is_some() && selected_rack().is_some();

    rsx! {
        div { class: "space-y-4",
            h2 { class: "text-xl font-bold",
                {i18n.t(Key::AddProductToRack)}
            }

            if loading() {
                div { class: "text-center py-4",
                    {i18n.t(Key::Loading)}
                }
            } else {
                div { class: "space-y-4",

                    if let Some(error_msg) = error() {
                        div { class: "bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded",
                            {error_msg}
                        }
                    }

                    // Product selection with searchable selector and barcode scanner
                    div {
                        label { class: "block text-sm font-medium mb-1",
                            {i18n.t(Key::SelectProduct)}
                        }

                        // Scanner message
                        if let Some(message) = scanner_message.read().as_ref() {
                            div {
                                class: if message.contains("found:") {
                                    "bg-green-100 text-green-700 p-2 rounded mb-2 text-sm"
                                } else {
                                    "bg-yellow-100 text-yellow-700 p-2 rounded mb-2 text-sm"
                                },
                                {message.clone()}
                            }
                        }

                        div { class: "flex gap-2",
                            div { class: "flex-1",
                                SearchableProductSelector {
                                    selected_product_id: selected_product(),
                                    disabled: product_id.is_some() || saving(),
                                    on_product_selected: handle_product_selected
                                }
                            }
                            button {
                                r#type: "button",
                                class: "px-4 py-2 bg-gray-600 text-white rounded hover:bg-gray-700 disabled:opacity-50",
                                disabled: product_id.is_some() || saving() || loading(),
                                onclick: move |_| {
                                    scanner_message.set(None);
                                    show_scanner.set(true);
                                },
                                "📷 Scan"
                            }
                        }
                    }

                    // Rack selection (keep as dropdown since fewer items)
                    div {
                        label { class: "block text-sm font-medium mb-1",
                            {i18n.t(Key::SelectRack)}
                        }
                        select {
                            class: "w-full px-3 py-2 border rounded-md",
                            disabled: rack_id.is_some() || saving(),
                            value: selected_rack().map(|id| id.to_string()).unwrap_or_default(),
                            onchange: move |event| {
                                if let Ok(uuid) = Uuid::parse_str(&event.value()) {
                                    selected_rack.set(Some(uuid));
                                } else {
                                    selected_rack.set(None);
                                }
                            },

                            option { value: "", "-- {i18n.t(Key::SelectRack)} --" }

                            for rack in racks().iter() {
                                option {
                                    value: rack.id.unwrap().to_string(),
                                    selected: Some(rack.id.unwrap()) == selected_rack(),
                                    "{rack.name} - {rack.description}"
                                }
                            }
                        }
                    }

                    // Action buttons
                    div { class: "flex space-x-2 pt-4",
                        button {
                            r#type: "button",
                            class: "px-4 py-2 bg-blue-500 text-white rounded-md hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed",
                            disabled: !is_valid || saving(),
                            onclick: handle_save,

                            if saving() {
                                {i18n.t(Key::Loading)}
                            } else {
                                {i18n.t(Key::Save)}
                            }
                        }

                        button {
                            r#type: "button",
                            class: "px-4 py-2 bg-gray-500 text-white rounded-md hover:bg-gray-600",
                            disabled: saving(),
                            onclick: move |_| on_cancel.call(()),
                            {i18n.t(Key::Cancel)}
                        }
                    }
                }
            }

            // Barcode Scanner
            if show_scanner() {
                BarcodeScanner {
                    on_scan: handle_barcode_scan,
                    on_close: handle_scanner_close
                }
            }
        }
    }
}
