use crate::api;
use crate::component::{MeasurementForm, MeasurementList, TopBar};
use crate::i18n::{use_i18n, Key};
use crate::router::Route;
use crate::service::config::CONFIG;
use crate::service::container::CONTAINERS;
use crate::service::inventur::MEASUREMENTS;
use crate::service::product::PRODUCTS;
use crate::service::rack::RACKS;
use dioxus::prelude::*;
use rest_types::InventurTO;
use uuid::Uuid;

#[component]
pub fn InventurMeasurements(id: String) -> Element {
    let i18n = use_i18n();
    let mut inventur = use_signal(|| None::<InventurTO>);

    // Parse the inventur ID
    let inventur_id = match Uuid::parse_str(&id) {
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

    // State for showing/hiding the measurement form
    let mut show_form = use_signal(|| false);
    let mut editing_measurement_id = use_signal(|| None::<Uuid>);

    // Load data when the page mounts
    use_effect(move || {
        spawn(async move {
            let config = CONFIG.read().clone();
            if !config.backend.is_empty() {
                // Load inventur to check status
                if let Ok(inventur_data) = api::get_inventur(&config, inventur_id).await {
                    inventur.set(Some(inventur_data));
                }

                // Load measurements for this inventur
                MEASUREMENTS.write().loading = true;
                match api::get_measurements_by_inventur(&config, inventur_id).await {
                    Ok(measurements) => {
                        MEASUREMENTS.write().items = measurements;
                        MEASUREMENTS.write().error = None;
                    }
                    Err(e) => {
                        MEASUREMENTS.write().error =
                            Some(format!("Failed to load measurements: {}", e));
                    }
                }
                MEASUREMENTS.write().loading = false;

                // Load products if not already loaded
                if PRODUCTS.read().items.is_empty() {
                    PRODUCTS.write().loading = true;
                    match api::get_products(&config).await {
                        Ok(products) => {
                            PRODUCTS.write().items = products;
                            PRODUCTS.write().error = None;
                        }
                        Err(e) => {
                            PRODUCTS.write().error = Some(format!("Failed to load products: {}", e));
                        }
                    }
                    PRODUCTS.write().loading = false;
                }

                // Load racks if not already loaded
                if RACKS.read().items.is_empty() {
                    RACKS.write().loading = true;
                    match api::get_racks(&config).await {
                        Ok(racks) => {
                            RACKS.write().items = racks;
                            RACKS.write().error = None;
                        }
                        Err(e) => {
                            RACKS.write().error = Some(format!("Failed to load racks: {}", e));
                        }
                    }
                    RACKS.write().loading = false;
                }

                // Load containers if not already loaded
                if CONTAINERS.read().items.is_empty() {
                    CONTAINERS.write().loading = true;
                    match api::get_containers(&config).await {
                        Ok(containers) => {
                            CONTAINERS.write().items = containers;
                            CONTAINERS.write().error = None;
                        }
                        Err(e) => {
                            CONTAINERS.write().error =
                                Some(format!("Failed to load containers: {}", e));
                        }
                    }
                    CONTAINERS.write().loading = false;
                }
            }
        });
    });

    let nav = navigator();

    rsx! {
        div { class: "flex flex-col min-h-screen",
            TopBar {}
            div { class: "flex-1 container mx-auto px-4 py-8",
                div { class: "flex justify-between items-center mb-6",
                    h1 { class: "text-3xl font-bold",
                        {i18n.t(Key::Measurements)}
                    }
                    if let Some(inv) = inventur.read().as_ref() {
                        if inv.status == "active" {
                            button {
                                class: "px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors",
                                onclick: {
                                    let inventur_id = id.clone();
                                    move |_| {
                                        nav.push(Route::InventurRackSelection { id: inventur_id.clone() });
                                    }
                                },
                                {i18n.t(Key::MeasureByRack)}
                            }
                        } else {
                            button {
                                class: "px-4 py-2 bg-gray-400 text-white rounded cursor-not-allowed",
                                disabled: true,
                                title: "Inventur must be active to measure",
                                {i18n.t(Key::MeasureByRack)}
                            }
                        }
                    }
                }

                if *show_form.read() {
                    div { class: "mb-6",
                        MeasurementForm {
                            inventur_id,
                            measurement_id: *editing_measurement_id.read(),
                            on_cancel: move |_| {
                                show_form.set(false);
                                editing_measurement_id.set(None);
                            },
                            on_save: move |_| {
                                show_form.set(false);
                                editing_measurement_id.set(None);
                            }
                        }
                    }
                } else {
                    MeasurementList {
                        inventur_id,
                        on_edit: move |measurement_id| {
                            editing_measurement_id.set(if measurement_id == Uuid::nil() {
                                None
                            } else {
                                Some(measurement_id)
                            });
                            show_form.set(true);
                        },
                        on_delete: move |measurement_id| {
                            spawn({
                                async move {
                                    let config = CONFIG.read().clone();
                                    match api::delete_measurement(&config, measurement_id).await {
                                        Ok(_) => {
                                            // Reload measurements
                                            MEASUREMENTS.write().loading = true;
                                            match api::get_measurements_by_inventur(&config, inventur_id).await {
                                                Ok(measurements) => {
                                                    MEASUREMENTS.write().items = measurements;
                                                    MEASUREMENTS.write().error = None;
                                                }
                                                Err(e) => {
                                                    MEASUREMENTS.write().error =
                                                        Some(format!("Failed to reload measurements: {}", e));
                                                }
                                            }
                                            MEASUREMENTS.write().loading = false;
                                        }
                                        Err(e) => {
                                            MEASUREMENTS.write().error =
                                                Some(format!("Failed to delete measurement: {}", e));
                                        }
                                    }
                                }
                            });
                        }
                    }
                }
            }
        }
    }
}
