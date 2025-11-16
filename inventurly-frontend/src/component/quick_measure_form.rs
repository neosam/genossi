use crate::api;
use crate::i18n::{use_i18n, Key};
use crate::service::config::CONFIG;
use crate::service::inventur::MEASUREMENTS;
use crate::service::tara::{self, WeightUnit};
use dioxus::prelude::*;
use rest_types::{ContainerTO, InventurMeasurementTO, ProductTO};
use uuid::Uuid;

#[component]
pub fn QuickMeasureForm(
    inventur_id: Uuid,
    rack_id: Uuid,
    product: ProductTO,
    containers: Vec<ContainerTO>,
    existing_measurement: Option<InventurMeasurementTO>,
    on_save: EventHandler<()>,
    on_cancel: EventHandler<()>,
) -> Element {
    let i18n = use_i18n();

    // Initialize form with existing measurement or defaults
    let mut weight_unit = use_signal(|| tara::get_preferred_weight_unit());
    let mut value = use_signal(|| {
        if let Some(ref m) = existing_measurement {
            if product.requires_weighing {
                // Convert existing grams to kg for display
                let grams = m.weight_grams.unwrap_or(0);
                (grams as f64 / 1000.0).to_string()
            } else {
                m.count.unwrap_or(0).to_string()
            }
        } else {
            String::new()
        }
    });

    let mut notes = use_signal(|| {
        existing_measurement
            .as_ref()
            .and_then(|m| m.notes.clone())
            .unwrap_or_default()
    });

    let mut selected_container = use_signal(|| {
        // Use existing measurement's container if editing, otherwise try localStorage
        if let Some(container_id) = existing_measurement.as_ref().and_then(|m| m.container_id) {
            Some(container_id)
        } else {
            // Try to load last used container from localStorage
            // Only use it if it exists in the available containers list
            tara::get_last_container_id().filter(|id| {
                containers.iter().any(|c| c.id == Some(*id) && c.deleted.is_none())
            })
        }
    });

    let loading = use_signal(|| false);
    let error = use_signal(|| None::<String>);

    // Calculate if the measurement is valid
    let is_valid = {
        let containers_clone = containers.clone();
        move || {
            // Parse the value - support both i64 and f64
            let parsed_grams = if product.requires_weighing {
                // For weight, parse as float and convert to grams
                value.read().parse::<f64>().ok().map(|v| {
                    match *weight_unit.read() {
                        WeightUnit::Kilogram => (v * 1000.0) as i64,
                        WeightUnit::Gram => v as i64,
                    }
                })
            } else {
                // For count, parse as integer
                value.read().parse::<i64>().ok()
            };

            if parsed_grams.is_none() {
                return false;
            }

            // If product requires weighing and container is selected, check weight > (user_tara + container_weight)
            // Skip validation for 0 (assume 0 = empty container)
            if product.requires_weighing {
                if let Some(container_id) = *selected_container.read() {
                    if let Some(container) = containers_clone.iter().find(|c| c.id == Some(container_id)) {
                        if let Some(weight) = parsed_grams {
                            let min_weight = tara::get_tara_grams() + container.weight_grams;
                            if weight > 0 && weight <= min_weight {
                                return false;
                            }
                        }
                    }
                }
            }

            true
        }
    };

    let save_measurement = {
        let existing_id = existing_measurement.as_ref().and_then(|m| m.id);
        let existing_created = existing_measurement.as_ref().and_then(|m| m.created);
        let existing_version = existing_measurement.as_ref().and_then(|m| m.version);
        let product_id = product.id.unwrap();
        let requires_weighing = product.requires_weighing;

        move || {
            spawn({
                let mut loading = loading.clone();
                let mut error = error.clone();
                let on_save = on_save.clone();
                let value = value.clone();
                let notes = notes.clone();
                let selected_container = selected_container.clone();

                async move {
                    loading.set(true);
                    error.set(None);

                    let config = CONFIG.read().clone();

                    // Parse the value - support both i64 and f64 for weight
                    let parsed_grams = if requires_weighing {
                        // For weight, parse as float and convert to grams
                        value.read().parse::<f64>().ok().map(|v| {
                            match *weight_unit.read() {
                                WeightUnit::Kilogram => (v * 1000.0) as i64,
                                WeightUnit::Gram => v as i64,
                            }
                        })
                    } else {
                        // For count, parse as integer
                        value.read().parse::<i64>().ok()
                    };

                    if parsed_grams.is_none() {
                        error.set(Some("Please enter a valid value".to_string()));
                        loading.set(false);
                        return;
                    }

                    // Apply custom tara (body weight) subtraction for weight measurements
                    let final_weight = if requires_weighing {
                        parsed_grams.map(|w| w - tara::get_tara_grams())
                    } else {
                        None
                    };

                    let measurement = InventurMeasurementTO {
                        id: existing_id,
                        inventur_id,
                        product_id,
                        rack_id: Some(rack_id),
                        container_id: *selected_container.read(),
                        count: if requires_weighing {
                            None
                        } else {
                            parsed_grams
                        },
                        weight_grams: if requires_weighing {
                            final_weight
                        } else {
                            None
                        },
                        measured_by: None,
                        measured_at: None,
                        notes: if notes.read().is_empty() {
                            None
                        } else {
                            Some(notes.read().clone())
                        },
                        created: existing_created,
                        deleted: None,
                        version: existing_version,
                    };

                let result = if measurement.id.is_some() {
                    api::update_measurement(&config, measurement).await
                } else {
                    api::create_measurement(&config, measurement).await
                };

                match result {
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

                        on_save.call(());
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to save: {}", e)));
                    }
                }

                loading.set(false);
            }
        });
        }
    };

    rsx! {
        // Modal backdrop - positioned at top on mobile
        div {
            class: "fixed inset-0 bg-black bg-opacity-50 z-40 flex items-start md:items-center justify-center p-4",
            onclick: move |_| on_cancel.call(()),

            // Modal content - with max height and flex layout
            div {
                class: "bg-white rounded-lg shadow-xl max-w-md w-full max-h-[90vh] flex flex-col relative z-50",
                onclick: move |e| e.stop_propagation(),

                // Sticky header with title and buttons
                div {
                    class: "sticky top-0 bg-white border-b border-gray-200 p-4 flex flex-col gap-3 z-10 rounded-t-lg",

                    h3 { class: "text-xl font-semibold",
                        {i18n.t(Key::QuickMeasure)}
                        " - "
                        {product.name.clone()}
                    }

                    // Buttons at top
                    div { class: "flex gap-2",
                        button {
                            r#type: "button",
                            class: "flex-1 px-4 py-3 md:py-2 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed",
                            disabled: *loading.read() || !is_valid(),
                            onclick: {
                                let save = save_measurement.clone();
                                move |_| save()
                            },
                            if *loading.read() {
                                {i18n.t(Key::Loading)}
                            } else {
                                {i18n.t(Key::Save)}
                            }
                        }
                        button {
                            r#type: "button",
                            class: "flex-1 px-4 py-3 md:py-2 bg-gray-300 text-gray-700 rounded hover:bg-gray-400",
                            onclick: move |_| on_cancel.call(()),
                            {i18n.t(Key::Cancel)}
                        }
                    }
                }

                // Scrollable body
                div {
                    class: "flex-1 overflow-y-auto p-4",

                    if let Some(err) = error.read().as_ref() {
                        div { class: "bg-red-100 border border-red-400 text-red-700 px-3 py-2 rounded mb-4 text-sm",
                            {err.clone()}
                        }
                    }

                    // Show validation warning for weight vs container
                    if product.requires_weighing {
                        if let Some(container_id) = *selected_container.read() {
                            if let Some(container) = containers.iter().find(|c| c.id == Some(container_id)) {
                                if let Ok(weight_val) = value.read().parse::<f64>() {
                                    if {
                                        let weight_grams = match *weight_unit.read() {
                                            WeightUnit::Kilogram => (weight_val * 1000.0) as i64,
                                            WeightUnit::Gram => weight_val as i64,
                                        };
                                        weight_grams > 0 && weight_grams <= (tara::get_tara_grams() + container.weight_grams)
                                    } {
                                        div { class: "bg-yellow-100 border border-yellow-400 text-yellow-700 px-3 py-2 rounded mb-4 text-sm",
                                            "Weight must be greater than container weight ("
                                            {container.weight_grams.to_string()}
                                            "g)"
                                        }
                                    }
                                }
                            }
                        }
                    }

                    div { class: "space-y-4",
                        div {
                            label {
                                class: "block text-sm font-medium text-gray-700 mb-1",
                                if product.requires_weighing {
                                    {i18n.t(Key::EnterWeight)}
                                } else {
                                    {i18n.t(Key::EnterCount)}
                                }
                            }
                            input {
                                r#type: "number",
                                inputmode: "decimal",
                                step: if product.requires_weighing && *weight_unit.read() == WeightUnit::Kilogram { "0.001" } else { "1" },
                                class: "w-full px-3 py-3 md:py-2 text-lg md:text-base border border-gray-300 rounded-md focus:outline-none focus:ring-blue-500 focus:border-blue-500",
                                value: "{value.read()}",
                                onmounted: move |event| async move {
                                    let _ = event.set_focus(true).await;
                                },
                                oninput: move |e| {
                                    value.set(e.value());
                                },
                                onkeydown: move |evt| {
                                    if evt.key().to_string() == "Enter" && is_valid() {
                                        save_measurement();
                                    }
                                },
                            }
                            // Unit selector for weighing products
                            if product.requires_weighing {
                                div { class: "flex gap-2 mt-2",
                                    button {
                                        r#type: "button",
                                        class: if *weight_unit.read() == WeightUnit::Kilogram {
                                            "flex-1 px-3 py-2 bg-blue-600 text-white rounded font-medium"
                                        } else {
                                            "flex-1 px-3 py-2 bg-gray-200 text-gray-700 rounded hover:bg-gray-300"
                                        },
                                        onclick: move |_| {
                                            weight_unit.set(WeightUnit::Kilogram);
                                            tara::set_preferred_weight_unit(WeightUnit::Kilogram);
                                        },
                                        "kg"
                                    }
                                    button {
                                        r#type: "button",
                                        class: if *weight_unit.read() == WeightUnit::Gram {
                                            "flex-1 px-3 py-2 bg-blue-600 text-white rounded font-medium"
                                        } else {
                                            "flex-1 px-3 py-2 bg-gray-200 text-gray-700 rounded hover:bg-gray-300"
                                        },
                                        onclick: move |_| {
                                            weight_unit.set(WeightUnit::Gram);
                                            tara::set_preferred_weight_unit(WeightUnit::Gram);
                                        },
                                        "g"
                                    }
                                }
                            }
                        }

                        // Container selection (especially important for weighing products)
                        if product.requires_weighing {
                            div {
                                label {
                                    class: "block text-sm font-medium text-gray-700 mb-1",
                                    {i18n.t(Key::ContainerName)}
                                    " (Tara)"
                                }
                                select {
                                    class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-blue-500 focus:border-blue-500",
                                    value: "{selected_container.read().as_ref().map(|id| id.to_string()).unwrap_or_default()}",
                                    onchange: move |e| {
                                        if e.value().is_empty() {
                                            selected_container.set(None);
                                            tara::clear_last_container_id();
                                        } else if let Ok(uuid) = Uuid::parse_str(&e.value()) {
                                            selected_container.set(Some(uuid));
                                            tara::set_last_container_id(uuid);
                                        }
                                    },
                                    option { value: "", "No container" }
                                    for container in containers.iter().filter(|c| c.deleted.is_none()) {
                                        option {
                                            value: "{container.id.unwrap_or(Uuid::nil())}",
                                            selected: selected_container.read().as_ref().map(|id| *id == container.id.unwrap_or(Uuid::nil())).unwrap_or(false),
                                            {container.name.clone()}
                                            " ("
                                            {container.weight_grams.to_string()}
                                            "g)"
                                        }
                                    }
                                }
                            }
                        }

                        div {
                            label {
                                class: "block text-sm font-medium text-gray-700 mb-1",
                                {i18n.t(Key::MeasurementNotes)}
                            }
                            textarea {
                                class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-blue-500 focus:border-blue-500",
                                rows: "2",
                                value: "{notes.read()}",
                                oninput: move |e| {
                                    notes.set(e.value());
                                },
                            }
                        }
                    }
                }
            }
        }
    }
}
