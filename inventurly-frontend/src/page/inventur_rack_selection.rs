use crate::api;
use crate::component::TopBar;
use crate::i18n::{use_i18n, Key};
use crate::router::Route;
use crate::service::config::CONFIG;
use crate::service::inventur::MEASUREMENTS;
use crate::service::product::PRODUCTS;
use crate::service::product_rack::get_products_in_rack_action;
use crate::service::rack::RACKS;
use dioxus::prelude::*;
use rest_types::{InventurTO, RackTO};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[component]
pub fn InventurRackSelection(id: String) -> Element {
    let i18n = use_i18n();
    let nav = navigator();

    // Parse inventur UUID
    let inventur_uuid = match Uuid::parse_str(&id) {
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

    // Local state
    let mut inventur = use_signal(|| None::<InventurTO>);
    let mut rack_product_counts = use_signal(|| HashMap::<Uuid, usize>::new());
    let mut loading = use_signal(|| true);
    let mut error = use_signal(|| None::<String>);

    // Load data
    use_effect(move || {
        spawn(async move {
            loading.set(true);
            error.set(None);

            let config = CONFIG.read().clone();

            // Load inventur details
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

            // Load all racks if not loaded
            if RACKS.read().items.is_empty() {
                RACKS.write().loading = true;
                match api::get_racks(&config).await {
                    Ok(racks) => {
                        RACKS.write().items = racks;
                        RACKS.write().error = None;
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to load racks: {}", e)));
                        loading.set(false);
                        return;
                    }
                }
                RACKS.write().loading = false;
            }

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

            // Load measurements for this inventur
            MEASUREMENTS.write().loading = true;
            match api::get_measurements_by_inventur(&config, inventur_uuid).await {
                Ok(measurements) => {
                    MEASUREMENTS.write().items = measurements;
                    MEASUREMENTS.write().error = None;
                }
                Err(e) => {
                    error.set(Some(format!("Failed to load measurements: {}", e)));
                    loading.set(false);
                    return;
                }
            }
            MEASUREMENTS.write().loading = false;

            // Calculate product counts for each rack
            let mut counts = HashMap::new();
            for rack in RACKS.read().items.iter().filter(|r| r.deleted.is_none()) {
                if let Some(rack_id) = rack.id {
                    match get_products_in_rack_action(rack_id).await {
                        Ok(product_racks) => {
                            let product_count = product_racks
                                .iter()
                                .filter(|pr| pr.deleted.is_none())
                                .count();
                            counts.insert(rack_id, product_count);
                        }
                        Err(_) => {
                            counts.insert(rack_id, 0);
                        }
                    }
                }
            }
            rack_product_counts.set(counts);

            loading.set(false);
        });
    });

    let measurements = MEASUREMENTS.read();
    let racks = RACKS.read();
    let active_racks: Vec<RackTO> = racks
        .items
        .iter()
        .filter(|r| r.deleted.is_none())
        .cloned()
        .collect();

    // Check if inventur is active
    let is_inventur_active = inventur.read().as_ref().map(|inv| inv.status == "active").unwrap_or(false);

    // Helper to calculate measurement progress for a rack
    let get_rack_progress = |rack_id: Uuid| -> (usize, usize) {
        // Get measured product IDs for this rack
        let measured_product_ids: HashSet<Uuid> = measurements
            .items
            .iter()
            .filter(|m| m.rack_id == Some(rack_id) && m.deleted.is_none())
            .map(|m| m.product_id)
            .collect();

        let total = rack_product_counts.read().get(&rack_id).copied().unwrap_or(0);
        let measured = measured_product_ids.len();

        (measured, total)
    };

    rsx! {
        div { class: "flex flex-col min-h-screen",
            TopBar {}
            div { class: "flex-1 container mx-auto px-4 py-8",
                // Header
                div { class: "mb-6",
                    button {
                        class: "text-blue-600 hover:text-blue-800 mb-2 flex items-center",
                        onclick: move |_| {
                            nav.push(Route::InventurMeasurements { id: id.clone() });
                        },
                        "← "
                        {i18n.t(Key::Back)}
                    }

                    h1 { class: "text-3xl font-bold",
                        {i18n.t(Key::SelectRack)}
                    }

                    if let Some(inv) = inventur.read().as_ref() {
                        p { class: "text-lg text-gray-700 mt-2",
                            {inv.name.clone()}
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

                // Rack list
                if !*loading.read() && error.read().is_none() {
                    if active_racks.is_empty() {
                        div { class: "text-center py-12 bg-gray-50 rounded-lg",
                            p { class: "text-gray-500",
                                {i18n.t(Key::NoDataFound)}
                            }
                        }
                    } else {
                        div { class: "space-y-4",
                            for rack in active_racks.iter() {
                                {
                                    let rack_id = rack.id.unwrap_or(Uuid::nil());
                                    let (measured, total) = get_rack_progress(rack_id);
                                    let percentage = if total > 0 {
                                        (measured as f64 / total as f64 * 100.0) as usize
                                    } else {
                                        0
                                    };

                                    let status_badge = if total == 0 {
                                        rsx! {
                                            span { class: "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-gray-200 text-gray-800",
                                                {i18n.t(Key::NoProductsInRack)}
                                            }
                                        }
                                    } else if measured == 0 {
                                        rsx! {
                                            span { class: "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-red-100 text-red-800",
                                                {i18n.t(Key::NotStarted)}
                                            }
                                        }
                                    } else if measured < total {
                                        rsx! {
                                            span { class: "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-yellow-100 text-yellow-800",
                                                {i18n.t(Key::InProgress)}
                                            }
                                        }
                                    } else {
                                        rsx! {
                                            span { class: "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-800",
                                                {i18n.t(Key::Complete)}
                                            }
                                        }
                                    };

                                    rsx! {
                                        div { class: "bg-white border rounded-lg shadow-sm p-4 hover:shadow-md transition-shadow",
                                            div { class: "flex justify-between items-start mb-3",
                                                div { class: "flex-1",
                                                    h3 { class: "text-lg font-semibold text-gray-900",
                                                        {rack.name.clone()}
                                                    }
                                                    if !rack.description.is_empty() {
                                                        p { class: "text-sm text-gray-600 mt-1",
                                                            {rack.description.clone()}
                                                        }
                                                    }
                                                }
                                                {status_badge}
                                            }

                                            div { class: "flex items-center justify-between",
                                                div { class: "text-sm text-gray-700",
                                                    span { class: "font-medium",
                                                        {i18n.t(Key::ProductCount)}
                                                        ": "
                                                    }
                                                    {total.to_string()}
                                                    if total > 0 {
                                                        span { class: "ml-4 font-medium",
                                                            {i18n.t(Key::ProductsMeasured)}
                                                            ": "
                                                        }
                                                        {measured.to_string()}
                                                        " / "
                                                        {total.to_string()}
                                                        span { class: "ml-2 text-gray-500",
                                                            "("
                                                            {percentage.to_string()}
                                                            "%)"
                                                        }
                                                    }
                                                }
                                                if total > 0 {
                                                    if is_inventur_active {
                                                        button {
                                                            class: "px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors",
                                                            onclick: {
                                                                let rack_id_str = rack_id.to_string();
                                                                let inventur_id = id.clone();
                                                                move |_| {
                                                                    nav.push(Route::InventurRackMeasure {
                                                                        inventur_id: inventur_id.clone(),
                                                                        rack_id: rack_id_str.clone(),
                                                                    });
                                                                }
                                                            },
                                                            {i18n.t(Key::MeasureRack)}
                                                        }
                                                    } else {
                                                        button {
                                                            class: "px-4 py-2 bg-gray-400 text-white rounded cursor-not-allowed",
                                                            disabled: true,
                                                            title: "Inventur must be active",
                                                            {i18n.t(Key::MeasureRack)}
                                                        }
                                                    }
                                                }
                                            }

                                            // Progress bar
                                            if total > 0 {
                                                div { class: "mt-3",
                                                    div { class: "w-full bg-gray-200 rounded-full h-2",
                                                        div {
                                                            class: "bg-blue-600 h-2 rounded-full transition-all",
                                                            style: "width: {percentage}%"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
