use dioxus::prelude::*;
use uuid::Uuid;
use rest_types::{ProductRackTO, ProductTO};
use crate::i18n::{use_i18n, Key};
use crate::component::{ProductRackForm, modal::Modal};
use crate::service::product_rack::{get_products_in_rack_action, remove_product_from_rack_action};
use crate::service::config::CONFIG;
use crate::api;

#[component]
pub fn RackProducts(rack_id: Uuid) -> Element {
    let i18n = use_i18n();
    
    let products_in_rack = use_signal(|| Vec::<ProductRackTO>::new());
    let products_map = use_signal(|| std::collections::HashMap::<Uuid, ProductTO>::new());
    let loading = use_signal(|| false);
    let error = use_signal(|| None::<String>);
    let mut show_add_form = use_signal(|| false);

    // Load products in rack
    let load_products = use_callback({
        let products_in_rack = products_in_rack.clone();
        let products_map = products_map.clone();
        let loading = loading.clone();
        let error = error.clone();
        
        move |_| {
            spawn({
                let mut products_in_rack = products_in_rack.clone();
                let mut products_map = products_map.clone();
                let mut loading = loading.clone();
                let mut error = error.clone();
                
                async move {
                    loading.set(true);
                    error.set(None);
                    
                    // Load products in rack and all products
                    let config = CONFIG.read().clone();
                    let (rack_products_result, all_products_result) = futures_util::join!(
                        get_products_in_rack_action(rack_id),
                        api::get_products(&config)
                    );
                    
                    match rack_products_result {
                        Ok(rack_products) => {
                            products_in_rack.set(rack_products);
                        }
                        Err(e) => {
                            error.set(Some(format!("Failed to load products in rack: {}", e)));
                            loading.set(false);
                            return;
                        }
                    }
                    
                    match all_products_result {
                        Ok(all_products) => {
                            let mut map = std::collections::HashMap::new();
                            for product in all_products {
                                if let Some(id) = product.id {
                                    map.insert(id, product);
                                }
                            }
                            products_map.set(map);
                        }
                        Err(e) => {
                            error.set(Some(format!("Failed to load products: {}", e)));
                        }
                    }
                    
                    loading.set(false);
                }
            });
        }
    });

    // Load products on mount
    use_effect(move || {
        load_products.call(());
    });

    let handle_remove_product = move |product_id: Uuid| {
        spawn({
            let mut error = error.clone();
            let load_products = load_products.clone();
            
            async move {
                error.set(None);
                
                match remove_product_from_rack_action(product_id, rack_id).await {
                    Ok(()) => {
                        load_products.call(());
                    }
                    Err(e) => {
                        error.set(Some(e));
                    }
                }
            }
        });
    };

    let handle_product_saved = move |_| {
        let mut show_add_form = show_add_form.clone();
        show_add_form.set(false);
        load_products.call(());
    };

    rsx! {
        div { class: "space-y-4",
            div { class: "flex justify-between items-center",
                h3 { class: "text-lg font-semibold",
                    {i18n.t(Key::ProductsInRack)}
                }
                button {
                    class: "px-4 py-2 bg-blue-500 text-white rounded-md hover:bg-blue-600",
                    onclick: move |_| show_add_form.set(true),
                    {i18n.t(Key::AddProductToRack)}
                }
            }

            if let Some(error_msg) = error() {
                div { class: "bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded",
                    {error_msg}
                }
            }

            if loading() {
                div { class: "text-center py-4",
                    {i18n.t(Key::Loading)}
                }
            } else if products_in_rack().is_empty() {
                div { class: "text-center py-8 text-gray-500",
                    {i18n.t(Key::NoDataFound)}
                }
            } else {
                div { class: "overflow-x-auto",
                    table { class: "min-w-full bg-white border border-gray-200",
                        thead { class: "bg-gray-50",
                            tr {
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider border-b",
                                    {i18n.t(Key::ProductName)}
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider border-b",
                                    {i18n.t(Key::ProductEan)}
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider border-b",
                                    {i18n.t(Key::Actions)}
                                }
                            }
                        }
                        tbody { class: "bg-white divide-y divide-gray-200",
                            for (idx, product_rack) in products_in_rack().iter().enumerate() {
                                if let Some(product) = products_map().get(&product_rack.product_id) {
                                    tr { 
                                        key: "{idx}",
                                        class: "hover:bg-gray-50",
                                        td { class: "px-6 py-4 whitespace-nowrap text-sm text-gray-900",
                                            {product.name.clone()}
                                        }
                                        td { class: "px-6 py-4 whitespace-nowrap text-sm text-gray-500",
                                            {product.ean.clone()}
                                        }
                                        td { class: "px-6 py-4 whitespace-nowrap text-sm font-medium space-x-2",
                                            button {
                                                class: "text-red-600 hover:text-red-900",
                                                onclick: {
                                                    let product_id = product_rack.product_id;
                                                    move |_| handle_remove_product(product_id)
                                                },
                                                {i18n.t(Key::RemoveProductFromRack)}
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Add Product Modal
            if show_add_form() {
                Modal {
                    ProductRackForm {
                        product_id: None,
                        rack_id: Some(rack_id),
                        on_saved: handle_product_saved,
                        on_cancel: move |_| {
                            let mut show_add_form = show_add_form.clone();
                            show_add_form.set(false);
                        },
                    }
                }
            }

        }
    }
}