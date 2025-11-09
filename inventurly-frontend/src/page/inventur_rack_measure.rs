use crate::api;
use crate::component::{CustomEntryForm, CustomEntryList, QuickMeasureForm, RackProductMeasureList, TopBar};
use crate::i18n::{use_i18n, Key};
use crate::router::Route;
use crate::service::config::CONFIG;
use crate::service::container::CONTAINERS;
use crate::service::inventur::MEASUREMENTS;
use crate::service::product::PRODUCTS;
use crate::service::product_rack::get_products_in_rack_action;
use dioxus::prelude::*;
use rest_types::{InventurCustomEntryTO, InventurTO, ProductTO, RackTO};
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
    let mut custom_entries = use_signal(|| Vec::<InventurCustomEntryTO>::new());
    let mut selected_custom_entry = use_signal(|| None::<InventurCustomEntryTO>);
    let mut show_custom_entry_form = use_signal(|| false);
    let mut entry_to_delete = use_signal(|| None::<InventurCustomEntryTO>);
    let mut filter_query = use_signal(|| String::new());

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

            // Load custom entries for this inventur
            match api::get_custom_entries_by_inventur(&config, inventur_uuid).await {
                Ok(entries) => {
                    custom_entries.set(entries);
                }
                Err(e) => {
                    error.set(Some(format!("Failed to load custom entries: {}", e)));
                }
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

    // Filter custom entries for this rack
    let rack_custom_entries: Vec<_> = custom_entries
        .read()
        .iter()
        .filter(|e| e.rack_id == Some(rack_uuid) && e.deleted.is_none())
        .cloned()
        .collect();

    // Filter products by search query
    let filtered_products: Vec<ProductTO> = rack_products
        .read()
        .iter()
        .filter(|p| {
            if filter_query().is_empty() {
                return true;
            }
            let query = filter_query().to_lowercase();
            p.name.to_lowercase().contains(&query)
                || p.ean.to_lowercase().contains(&query)
                || p.short_name.to_lowercase().contains(&query)
        })
        .cloned()
        .collect();

    // Handler to reload custom entries
    let reload_custom_entries = move || {
        spawn(async move {
            let config = CONFIG.read().clone();
            match api::get_custom_entries_by_inventur(&config, inventur_uuid).await {
                Ok(entries) => {
                    custom_entries.set(entries);
                }
                Err(_e) => {
                    // Silently fail - user will see stale data
                }
            }
        });
    };

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
                    // Custom entry form modal (available regardless of inventur status)
                    if *show_custom_entry_form.read() {
                        CustomEntryForm {
                            inventur_id: inventur_uuid,
                            rack_id: rack_uuid,
                            containers: active_containers.clone(),
                            existing_entry: selected_custom_entry.read().clone(),
                            on_save: move |_| {
                                show_custom_entry_form.set(false);
                                selected_custom_entry.set(None);
                                reload_custom_entries();
                            },
                            on_cancel: move |_| {
                                show_custom_entry_form.set(false);
                                selected_custom_entry.set(None);
                            }
                        }
                    }

                    // Delete confirmation modal (available regardless of inventur status)
                    if let Some(entry) = entry_to_delete.read().as_ref() {
                        div {
                            class: "fixed inset-0 bg-black bg-opacity-50 z-40 flex items-center justify-center p-4",
                            onclick: move |_| entry_to_delete.set(None),

                            div {
                                class: "bg-white rounded-lg shadow-xl max-w-md w-full p-6 relative z-50",
                                onclick: move |e| e.stop_propagation(),

                                h3 { class: "text-xl font-semibold mb-4",
                                    {i18n.t(Key::ConfirmDeleteCustomEntry)}
                                }

                                p { class: "mb-6 text-gray-700",
                                    "Are you sure you want to delete \""
                                    {entry.custom_product_name.clone()}
                                    "\"?"
                                }

                                div { class: "flex gap-2",
                                    button {
                                        class: "flex-1 px-4 py-2 bg-red-600 text-white rounded hover:bg-red-700",
                                        onclick: {
                                            let entry_id = entry.id.unwrap();
                                            move |_| {
                                                spawn({
                                                    let mut entry_to_delete = entry_to_delete.clone();
                                                    async move {
                                                        let config = CONFIG.read().clone();
                                                        if let Ok(_) = api::delete_custom_entry(&config, entry_id).await {
                                                            reload_custom_entries();
                                                        }
                                                        entry_to_delete.set(None);
                                                    }
                                                });
                                            }
                                        },
                                        {i18n.t(Key::DeleteCustomEntry)}
                                    }
                                    button {
                                        class: "flex-1 px-4 py-2 bg-gray-300 text-gray-700 rounded hover:bg-gray-400",
                                        onclick: move |_| entry_to_delete.set(None),
                                        {i18n.t(Key::Cancel)}
                                    }
                                }
                            }
                        }
                    }

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

                        // Product filter
                        div { class: "mb-4",
                            div { class: "relative",
                                input {
                                    r#type: "text",
                                    class: "w-full px-4 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-blue-500 focus:border-blue-500",
                                    placeholder: "{i18n.t(Key::FilterProducts)}",
                                    value: "{filter_query}",
                                    oninput: move |evt| {
                                        filter_query.set(evt.value());
                                    },
                                }
                                if !filter_query().is_empty() {
                                    button {
                                        class: "absolute right-2 top-1/2 -translate-y-1/2 px-3 py-1 text-gray-500 hover:text-gray-700",
                                        onclick: move |_| filter_query.set(String::new()),
                                        title: "{i18n.t(Key::ClearFilter)}",
                                        "✕"
                                    }
                                }
                            }
                        }

                        // Product list
                        RackProductMeasureList {
                            products: filtered_products.clone(),
                            measurements: rack_measurements.clone(),
                            rack_id: rack_uuid,
                            on_measure: move |product| {
                                selected_product.set(Some(product));
                            }
                        }

                        // Custom entries section
                        div { class: "mt-6",
                            div { class: "mb-4",
                                button {
                                    class: "px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700",
                                    onclick: move |_| {
                                        selected_custom_entry.set(None);
                                        show_custom_entry_form.set(true);
                                    },
                                    "+ "
                                    {i18n.t(Key::AddCustomEntry)}
                                }
                            }

                            CustomEntryList {
                                entries: rack_custom_entries.clone(),
                                on_edit: move |entry| {
                                    selected_custom_entry.set(Some(entry));
                                    show_custom_entry_form.set(true);
                                },
                                on_delete: move |entry| {
                                    entry_to_delete.set(Some(entry));
                                }
                            }
                        }
                    } else {
                        // Product filter
                        div { class: "mb-4",
                            div { class: "relative",
                                input {
                                    r#type: "text",
                                    class: "w-full px-4 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-blue-500 focus:border-blue-500",
                                    placeholder: "{i18n.t(Key::FilterProducts)}",
                                    value: "{filter_query}",
                                    oninput: move |evt| {
                                        filter_query.set(evt.value());
                                    },
                                }
                                if !filter_query().is_empty() {
                                    button {
                                        class: "absolute right-2 top-1/2 -translate-y-1/2 px-3 py-1 text-gray-500 hover:text-gray-700",
                                        onclick: move |_| filter_query.set(String::new()),
                                        title: "{i18n.t(Key::ClearFilter)}",
                                        "✕"
                                    }
                                }
                            }
                        }

                        // Show product list but without ability to measure
                        RackProductMeasureList {
                            products: filtered_products.clone(),
                            measurements: rack_measurements.clone(),
                            rack_id: rack_uuid,
                            on_measure: move |_| {
                                // Do nothing - inventur is not active
                            }
                        }

                        // Custom entries section
                        div { class: "mt-6",
                            div { class: "mb-4",
                                button {
                                    class: "px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700",
                                    onclick: move |_| {
                                        selected_custom_entry.set(None);
                                        show_custom_entry_form.set(true);
                                    },
                                    "+ "
                                    {i18n.t(Key::AddCustomEntry)}
                                }
                            }

                            CustomEntryList {
                                entries: rack_custom_entries.clone(),
                                on_edit: move |entry| {
                                    selected_custom_entry.set(Some(entry));
                                    show_custom_entry_form.set(true);
                                },
                                on_delete: move |entry| {
                                    entry_to_delete.set(Some(entry));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
