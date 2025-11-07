use crate::api;
use crate::component::{QuickMeasureForm, RackProductMeasureList, TopBar};
use crate::i18n::{use_i18n, Key};
use crate::router::Route;
use crate::service::config::CONFIG;
use crate::service::container::CONTAINERS;
use crate::service::inventur::MEASUREMENTS;
use crate::service::product::PRODUCTS;
use crate::service::product_rack::get_products_in_rack_action;
use dioxus::prelude::*;
use rest_types::{InventurTO, ProductTO, RackTO};
use std::collections::HashMap;
use uuid::Uuid;

#[component]
pub fn InventurRackMeasure(inventur_id: String, rack_id: String) -> Element {
    let i18n = use_i18n();
    let nav = navigator();

    // Parse UUIDs
    let inventur_uuid = match Uuid::parse_str(&inventur_id) {
        Ok(uuid) => uuid,
        Err(_) => {
            return rsx! {
                div { class: "flex flex-col min-h-screen",
                    TopBar {}
                    div { class: "flex-1 container mx-auto px-4 py-8",
                        div { class: "bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded",
                            "Invalid inventur ID"
                        }
                    }
                }
            };
        }
    };

    let rack_uuid = match Uuid::parse_str(&rack_id) {
        Ok(uuid) => uuid,
        Err(_) => {
            return rsx! {
                div { class: "flex flex-col min-h-screen",
                    TopBar {}
                    div { class: "flex-1 container mx-auto px-4 py-8",
                        div { class: "bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded",
                            "Invalid rack ID"
                        }
                    }
                }
            };
        }
    };

    // Local state
    let mut inventur = use_signal(|| None::<InventurTO>);
    let mut rack = use_signal(|| None::<RackTO>);
    let mut rack_products = use_signal(|| Vec::<ProductTO>::new());
    let mut loading = use_signal(|| true);
    let mut error = use_signal(|| None::<String>);
    let mut selected_product = use_signal(|| None::<ProductTO>);

    // Load data
    use_effect(move || {
        spawn(async move {
            loading.set(true);
            error.set(None);

            let config = CONFIG.read().clone();

            // Load inventur to check status
            match api::get_inventur(&config, inventur_uuid).await {
                Ok(inventur_data) => {
                    inventur.set(Some(inventur_data));
                }
                Err(e) => {
                    error.set(Some(format!("Failed to load inventur: {}", e)));
                    loading.set(false);
                    return;
                }
            }

            // Load rack details
            match api::get_rack(&config, rack_uuid).await {
                Ok(rack_data) => {
                    rack.set(Some(rack_data));
                }
                Err(e) => {
                    error.set(Some(format!("Failed to load rack: {}", e)));
                    loading.set(false);
                    return;
                }
            }

            // Load products in rack
            match get_products_in_rack_action(rack_uuid).await {
                Ok(product_rack_relationships) => {
                    // Load all products if not loaded
                    if PRODUCTS.read().items.is_empty() {
                        PRODUCTS.write().loading = true;
                        match api::get_products(&config).await {
                            Ok(products) => {
                                PRODUCTS.write().items = products;
                                PRODUCTS.write().error = None;
                            }
                            Err(e) => {
                                error.set(Some(format!("Failed to load products: {}", e)));
                                loading.set(false);
                                return;
                            }
                        }
                        PRODUCTS.write().loading = false;
                    }

                    // Create product lookup map
                    let products = PRODUCTS.read();
                    let product_map: HashMap<Uuid, &ProductTO> = products
                        .items
                        .iter()
                        .filter_map(|p| p.id.map(|id| (id, p)))
                        .collect();

                    // Get products for this rack (excluding deleted relationships)
                    let rack_product_list: Vec<ProductTO> = product_rack_relationships
                        .iter()
                        .filter(|pr| pr.deleted.is_none())
                        .filter_map(|pr| product_map.get(&pr.product_id).map(|p| (*p).clone()))
                        .filter(|p| p.deleted.is_none()) // Exclude deleted products
                        .collect();

                    rack_products.set(rack_product_list);
                }
                Err(e) => {
                    error.set(Some(format!("Failed to load products: {}", e)));
                    loading.set(false);
                    return;
                }
            }

            // Load measurements for this inventur
            MEASUREMENTS.write().loading = true;
            match api::get_measurements_by_inventur(&config, inventur_uuid).await {
                Ok(measurements) => {
                    MEASUREMENTS.write().items = measurements;
                    MEASUREMENTS.write().error = None;
                }
                Err(e) => {
                    error.set(Some(format!("Failed to load measurements: {}", e)));
                }
            }
            MEASUREMENTS.write().loading = false;

            // Load containers if not already loaded
            if CONTAINERS.read().items.is_empty() {
                CONTAINERS.write().loading = true;
                match api::get_containers(&config).await {
                    Ok(containers) => {
                        CONTAINERS.write().items = containers;
                        CONTAINERS.write().error = None;
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to load containers: {}", e)));
                    }
                }
                CONTAINERS.write().loading = false;
            }

            loading.set(false);
        });
    });

    let measurements = MEASUREMENTS.read();
    let rack_measurements: Vec<_> = measurements
        .items
        .iter()
        .filter(|m| m.rack_id == Some(rack_uuid))
        .cloned()
        .collect();

    let containers = CONTAINERS.read();
    let active_containers: Vec<_> = containers
        .items
        .iter()
        .filter(|c| c.deleted.is_none())
        .cloned()
        .collect();

    // Check if inventur is active
    let is_inventur_active = inventur.read().as_ref().map(|inv| inv.status == "active").unwrap_or(false);

    rsx! {
        div { class: "flex flex-col min-h-screen",
            TopBar {}
            div { class: "flex-1 container mx-auto px-4 py-8",
                // Header
                div { class: "mb-6",
                    button {
                        class: "text-blue-600 hover:text-blue-800 mb-2 flex items-center",
                        onclick: move |_| {
                            nav.push(Route::InventurRackSelection { id: inventur_id.clone() });
                        },
                        "← "
                        {i18n.t(Key::Back)}
                    }

                    h1 { class: "text-3xl font-bold",
                        {i18n.t(Key::MeasureRack)}
                    }

                    if let Some(r) = rack.read().as_ref() {
                        p { class: "text-lg text-gray-700 mt-2",
                            {r.name.clone()}
                        }
                        if !r.description.is_empty() {
                            p { class: "text-sm text-gray-600",
                                {r.description.clone()}
                            }
                        }
                    }
                }

                // Loading state
                if *loading.read() {
                    div { class: "text-center py-12",
                        p { class: "text-gray-500",
                            {i18n.t(Key::Loading)}
                        }
                    }
                }

                // Error state
                if let Some(err) = error.read().as_ref() {
                    div { class: "bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-6",
                        {err.clone()}
                    }
                }

                // Warning if inventur is not active
                if let Some(inv) = inventur.read().as_ref() {
                    if inv.status != "active" {
                        div { class: "bg-yellow-100 border border-yellow-400 text-yellow-700 px-4 py-3 rounded mb-6",
                            p { class: "font-semibold", "Cannot measure - Inventur is not active" }
                            p { class: "text-sm mt-1",
                                "Current status: "
                                {inv.status.clone()}
                                ". Please set the inventur to 'active' status before recording measurements."
                            }
                        }
                    }
                }

                // Main content
                if !*loading.read() && error.read().is_none() {
                    if is_inventur_active {
                        if let Some(product) = selected_product.read().as_ref() {
                            // Show measurement form (as modal overlay)
                            QuickMeasureForm {
                                inventur_id: inventur_uuid,
                                rack_id: rack_uuid,
                                product: product.clone(),
                                containers: active_containers.clone(),
                                existing_measurement: rack_measurements.iter()
                                    .find(|m| m.product_id == product.id.unwrap_or(Uuid::nil()))
                                    .cloned(),
                                on_save: move |_| {
                                    selected_product.set(None);
                                },
                                on_cancel: move |_| {
                                    selected_product.set(None);
                                }
                            }
                        }

                        // Product list
                        RackProductMeasureList {
                            products: rack_products.read().clone(),
                            measurements: rack_measurements.clone(),
                            rack_id: rack_uuid,
                            on_measure: move |product| {
                                selected_product.set(Some(product));
                            }
                        }
                    } else {
                        // Show product list but without ability to measure
                        RackProductMeasureList {
                            products: rack_products.read().clone(),
                            measurements: rack_measurements.clone(),
                            rack_id: rack_uuid,
                            on_measure: move |_| {
                                // Do nothing - inventur is not active
                            }
                        }
                    }
                }
            }
        }
    }
}
