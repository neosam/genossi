use crate::api;
use crate::component::{BarcodeScanner, ScanResult};
use crate::i18n::{use_i18n, Key};
use crate::router::Route;
use crate::service::config::CONFIG;
use dioxus::prelude::*;
use rest_types::{Price, ProductTO};
use uuid::Uuid;

#[component]
pub fn ProductForm(product_id: Option<Uuid>) -> Element {
    let i18n = use_i18n();
    let nav = navigator();
    let mut product = use_signal(|| ProductTO {
        id: product_id,
        ean: String::new(),
        name: String::new(),
        short_name: String::new(),
        sales_unit: String::new(),
        requires_weighing: false,
        price: Price::from_cents(0),
        deposit: Price::from_cents(0),
        rack_count: None,
        created: None,
        deleted: None,
        version: None,
    });

    let loading = use_signal(|| false);
    let error = use_signal(|| None::<String>);
    let mut show_scanner = use_signal(|| false);

    // Load existing product data if editing
    use_effect(move || {
        if let Some(id) = product_id {
            spawn({
                let mut product = product.clone();
                let mut loading = loading.clone();
                let mut error = error.clone();

                async move {
                    loading.set(true);
                    let config = CONFIG.read().clone();

                    match api::get_product(&config, id).await {
                        Ok(product_data) => {
                            *product.write() = product_data;
                            error.set(None);
                        }
                        Err(e) => {
                            error.set(Some(format!("Failed to load product: {}", e)));
                        }
                    }

                    loading.set(false);
                }
            });
        }
    });

    let save_product = move |_| {
        spawn({
            let product = product.clone();
            let mut loading = loading.clone();
            let mut error = error.clone();
            let nav = nav.clone();

            async move {
                loading.set(true);
                error.set(None);

                let config = CONFIG.read().clone();
                let product_data = product.read().clone();

                let result = if product_data.id.is_some() {
                    // Update existing product
                    api::update_product(&config, product_data).await
                } else {
                    // Create new product
                    api::create_product(&config, product_data).await
                };

                match result {
                    Ok(_) => {
                        // Navigate to products list on success
                        nav.push(Route::Products {});
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to save product: {}", e)));
                    }
                }

                loading.set(false);
            }
        });
    };

    rsx! {
        div { class: "bg-white rounded-lg shadow p-6",
            h2 { class: "text-2xl font-bold mb-6",
                if product_id.is_some() {
                    {i18n.t(Key::Edit)}
                } else {
                    {i18n.t(Key::Create)}
                }
                " "
                {i18n.t(Key::Products)}
            }

            if let Some(err) = error.read().as_ref() {
                div { class: "bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4",
                    {err.clone()}
                }
            }

            form {

                div { class: "mb-4",
                    label { class: "block text-sm font-medium text-gray-700 mb-2",
                        {i18n.t(Key::ProductEan)}
                    }
                    div { class: "flex gap-2",
                        input {
                            class: "flex-1 px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500",
                            r#type: "text",
                            value: "{product.read().ean}",
                            oninput: move |e| product.write().ean = e.value(),
                            required: true,
                        }
                        button {
                            class: "px-4 py-2 bg-gray-600 text-white rounded hover:bg-gray-700",
                            r#type: "button",
                            onclick: move |_| show_scanner.set(true),
                            "📷 Scan"
                        }
                    }
                }

                div { class: "mb-4",
                    label { class: "block text-sm font-medium text-gray-700 mb-2",
                        {i18n.t(Key::ProductName)}
                    }
                    input {
                        class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500",
                        r#type: "text",
                        value: "{product.read().name}",
                        oninput: move |e| product.write().name = e.value(),
                        required: true,
                    }
                }

                div { class: "mb-4",
                    label { class: "block text-sm font-medium text-gray-700 mb-2",
                        {i18n.t(Key::ProductShortName)}
                    }
                    input {
                        class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500",
                        r#type: "text",
                        value: "{product.read().short_name}",
                        oninput: move |e| product.write().short_name = e.value(),
                        required: true,
                    }
                }

                div { class: "mb-4",
                    label { class: "block text-sm font-medium text-gray-700 mb-2",
                        {i18n.t(Key::ProductSalesUnit)}
                    }
                    input {
                        class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500",
                        r#type: "text",
                        value: "{product.read().sales_unit}",
                        oninput: move |e| product.write().sales_unit = e.value(),
                        required: true,
                    }
                }

                div { class: "mb-4",
                    label { class: "block text-sm font-medium text-gray-700 mb-2",
                        {i18n.t(Key::ProductPrice)} " (in cents)"
                    }
                    input {
                        class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500",
                        r#type: "number",
                        value: "{product.read().price.to_cents()}",
                        oninput: move |e| {
                            if let Ok(cents) = e.value().parse::<i64>() {
                                product.write().price = Price::from_cents(cents);
                            }
                        },
                        required: true,
                    }
                }

                div { class: "mb-4",
                    label { class: "block text-sm font-medium text-gray-700 mb-2",
                        {i18n.t(Key::ProductDeposit)} " (in cents)"
                    }
                    input {
                        class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500",
                        r#type: "number",
                        value: "{product.read().deposit.to_cents()}",
                        oninput: move |e| {
                            if let Ok(cents) = e.value().parse::<i64>() {
                                product.write().deposit = Price::from_cents(cents);
                            }
                        },
                    }
                }

                div { class: "mb-6",
                    label { class: "flex items-center",
                        input {
                            class: "mr-2",
                            r#type: "checkbox",
                            checked: product.read().requires_weighing,
                            oninput: move |e| product.write().requires_weighing = e.checked(),
                        }
                        {i18n.t(Key::ProductRequiresWeighing)}
                    }
                }

                div { class: "flex gap-4",
                    button {
                        class: "px-6 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50",
                        r#type: "button",
                        disabled: *loading.read(),
                        onclick: save_product,
                        {i18n.t(Key::Save)}
                    }
                    button {
                        class: "px-6 py-2 bg-gray-300 text-gray-700 rounded hover:bg-gray-400",
                        r#type: "button",
                        onclick: move |_| { nav.push(Route::Products {}); },
                        {i18n.t(Key::Cancel)}
                    }
                }
            }

            if *show_scanner.read() {
                BarcodeScanner {
                    on_scan: move |result: ScanResult| {
                        product.write().ean = result.barcode;
                        show_scanner.set(false);
                    },
                    on_close: move |_| {
                        show_scanner.set(false);
                    }
                }
            }
        }
    }
}
