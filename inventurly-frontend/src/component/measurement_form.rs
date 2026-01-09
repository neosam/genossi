use crate::api;
use crate::component::SearchableProductSelector;
use crate::i18n::{use_i18n, Key};
use crate::service::config::CONFIG;
use crate::service::container::CONTAINERS;
use crate::service::container_rack::get_containers_in_rack_action;
use crate::service::inventur::MEASUREMENTS;
use crate::service::rack::RACKS;
use dioxus::prelude::*;
use rest_types::InventurMeasurementTO;
use uuid::Uuid;

#[component]
pub fn MeasurementForm(
    inventur_id: Uuid,
    measurement_id: Option<Uuid>,
    on_cancel: EventHandler<()>,
    on_save: EventHandler<()>,
) -> Element {
    let i18n = use_i18n();
    let racks = RACKS.read();
    let containers = CONTAINERS.read();

    let mut measurement = use_signal(|| InventurMeasurementTO {
        id: measurement_id,
        inventur_id,
        product_id: Uuid::nil(),
        rack_id: None,
        container_id: None,
        count: None,
        weight_grams: None,
        measured_by: None,
        measured_at: None,
        notes: None,
        created: None,
        deleted: None,
        version: None,
    });

    let loading = use_signal(|| false);
    let error = use_signal(|| None::<String>);

    // Load rack-assigned container IDs for sorting (updates when rack_id changes)
    let rack_container_ids = use_signal(|| Vec::<Uuid>::new());
    let current_rack_id = use_memo(move || measurement.read().rack_id);
    use_effect({
        let mut rack_container_ids = rack_container_ids.clone();
        move || {
            if let Some(rack_id) = current_rack_id() {
                spawn(async move {
                    match get_containers_in_rack_action(rack_id).await {
                        Ok(container_racks) => {
                            let mut sorted: Vec<_> = container_racks.into_iter().collect();
                            sorted.sort_by_key(|cr| cr.sort_order.unwrap_or(i32::MAX));
                            rack_container_ids.set(sorted.iter().map(|cr| cr.container_id).collect());
                        }
                        Err(_) => {
                            rack_container_ids.set(vec![]);
                        }
                    }
                });
            } else {
                rack_container_ids.set(vec![]);
            }
        }
    });

    // Load existing measurement data if editing
    use_effect(move || {
        if let Some(id) = measurement_id {
            spawn({
                let mut measurement = measurement.clone();
                let mut loading = loading.clone();
                let mut error = error.clone();

                async move {
                    loading.set(true);
                    let config = CONFIG.read().clone();

                    // Get all measurements and find the one we want
                    match api::get_measurements_by_inventur(&config, inventur_id).await {
                        Ok(measurements) => {
                            if let Some(found) = measurements.iter().find(|m| m.id == Some(id)) {
                                *measurement.write() = found.clone();
                                error.set(None);
                            } else {
                                error.set(Some(format!("Measurement not found")));
                            }
                        }
                        Err(e) => {
                            error.set(Some(format!("Failed to load measurement: {}", e)));
                        }
                    }

                    loading.set(false);
                }
            });
        }
    });

    let save_measurement = move || {
        spawn({
            let mut loading = loading.clone();
            let mut error = error.clone();
            let on_save = on_save.clone();

            async move {
                loading.set(true);
                error.set(None);

                let config = CONFIG.read().clone();
                let measurement_data = measurement.read().clone();

                // Validate that product is selected
                if measurement_data.product_id == Uuid::nil() {
                    error.set(Some("Please select a product".to_string()));
                    loading.set(false);
                    return;
                }

                let result = if measurement_data.id.is_some() {
                    // Update existing measurement
                    api::update_measurement(&config, measurement_data).await
                } else {
                    // Create new measurement
                    api::create_measurement(&config, measurement_data).await
                };

                match result {
                    Ok(_) => {
                        // Reload the measurements list
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

                        // Notify parent component
                        on_save.call(());
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to save measurement: {}", e)));
                    }
                }

                loading.set(false);
            }
        });
    };

    // Get active (non-deleted) racks and containers
    let active_racks: Vec<_> = racks.items.iter().filter(|r| r.deleted.is_none()).collect();
    let active_containers: Vec<_> = containers.items.iter().filter(|c| c.deleted.is_none()).collect();

    rsx! {
        div { class: "bg-white rounded-lg shadow p-6",
            h2 { class: "text-2xl font-bold mb-6",
                if measurement_id.is_some() {
                    {i18n.t(Key::Edit)}
                } else {
                    {i18n.t(Key::RecordMeasurement)}
                }
            }

            if let Some(err) = error.read().as_ref() {
                div { class: "bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4",
                    {err.clone()}
                }
            }

            div {
                div { class: "mb-4",
                    label {
                        class: "block text-sm font-medium text-gray-700 mb-2",
                        {i18n.t(Key::SelectProduct)}
                    }
                    SearchableProductSelector {
                        selected_product_id: if measurement.read().product_id == Uuid::nil() {
                            None
                        } else {
                            Some(measurement.read().product_id)
                        },
                        disabled: false,
                        on_product_selected: move |product_id| {
                            if let Some(id) = product_id {
                                measurement.write().product_id = id;
                            } else {
                                measurement.write().product_id = Uuid::nil();
                            }
                        }
                    }
                }

                div { class: "grid grid-cols-1 md:grid-cols-2 gap-4 mb-4",
                    div {
                        label {
                            class: "block text-sm font-medium text-gray-700 mb-2",
                            r#for: "rack",
                            {i18n.t(Key::SelectRack)}
                        }
                        select {
                            id: "rack",
                            class: "w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500",
                            onchange: move |e| {
                                let value = e.value();
                                measurement.write().rack_id = if value.is_empty() {
                                    None
                                } else {
                                    Uuid::parse_str(&value).ok()
                                };
                            },
                            option { value: "", "- None -" }
                            for rack in active_racks.iter() {
                                option {
                                    value: "{rack.id.unwrap_or(Uuid::nil())}",
                                    selected: measurement.read().rack_id == rack.id,
                                    {rack.name.clone()}
                                }
                            }
                        }
                    }

                    div {
                        label {
                            class: "block text-sm font-medium text-gray-700 mb-2",
                            r#for: "container",
                            "Container"
                        }
                        select {
                            id: "container",
                            class: "w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500",
                            onchange: move |e| {
                                let value = e.value();
                                measurement.write().container_id = if value.is_empty() {
                                    None
                                } else {
                                    Uuid::parse_str(&value).ok()
                                };
                            },
                            option { value: "", "- None -" }

                            // Containers assigned to selected rack (sorted by sort_order)
                            {
                                let rack_ids = rack_container_ids();
                                let rack_containers: Vec<_> = rack_ids.iter()
                                    .filter_map(|id| active_containers.iter().find(|c| c.id == Some(*id)))
                                    .collect();

                                rsx! {
                                    for container in rack_containers.iter() {
                                        option {
                                            value: "{container.id.unwrap_or(Uuid::nil())}",
                                            selected: measurement.read().container_id == container.id,
                                            {container.name.clone()}
                                        }
                                    }
                                }
                            }

                            // Separator (only if there are rack containers and other containers)
                            {
                                let rack_ids = rack_container_ids();
                                let has_rack_containers = rack_ids.iter()
                                    .any(|id| active_containers.iter().any(|c| c.id == Some(*id)));
                                let has_other_containers = active_containers.iter()
                                    .any(|c| !rack_ids.contains(&c.id.unwrap_or(Uuid::nil())));

                                if has_rack_containers && has_other_containers {
                                    rsx! {
                                        option { disabled: true, "───────────────" }
                                    }
                                } else {
                                    rsx! {}
                                }
                            }

                            // Other containers (alphabetically sorted)
                            {
                                let rack_ids = rack_container_ids();
                                let mut other_containers: Vec<_> = active_containers.iter()
                                    .filter(|c| !rack_ids.contains(&c.id.unwrap_or(Uuid::nil())))
                                    .cloned()
                                    .collect();
                                other_containers.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

                                rsx! {
                                    for container in other_containers.iter() {
                                        option {
                                            value: "{container.id.unwrap_or(Uuid::nil())}",
                                            selected: measurement.read().container_id == container.id,
                                            {container.name.clone()}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                div { class: "grid grid-cols-1 md:grid-cols-2 gap-4 mb-4",
                    div {
                        label {
                            class: "block text-sm font-medium text-gray-700 mb-2",
                            r#for: "count",
                            {i18n.t(Key::MeasurementCount)}
                        }
                        input {
                            id: "count",
                            r#type: "number",
                            class: "w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500",
                            value: "{measurement.read().count.map(|c| c.to_string()).unwrap_or_default()}",
                            oninput: move |e| {
                                let value = e.value();
                                measurement.write().count = if value.is_empty() {
                                    None
                                } else {
                                    value.parse().ok()
                                };
                            },
                        }
                    }

                    div {
                        label {
                            class: "block text-sm font-medium text-gray-700 mb-2",
                            r#for: "weight",
                            {i18n.t(Key::MeasurementWeightGrams)}
                        }
                        input {
                            id: "weight",
                            r#type: "number",
                            class: "w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500",
                            value: "{measurement.read().weight_grams.map(|w| w.to_string()).unwrap_or_default()}",
                            oninput: move |e| {
                                let value = e.value();
                                measurement.write().weight_grams = if value.is_empty() {
                                    None
                                } else {
                                    value.parse().ok()
                                };
                            },
                        }
                    }
                }

                div { class: "mb-4",
                    label {
                        class: "block text-sm font-medium text-gray-700 mb-2",
                        r#for: "notes",
                        {i18n.t(Key::MeasurementNotes)}
                    }
                    textarea {
                        id: "notes",
                        class: "w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500",
                        rows: "3",
                        value: "{measurement.read().notes.as_ref().map(|s| s.as_str()).unwrap_or_default()}",
                        oninput: move |e| {
                            let value = e.value();
                            measurement.write().notes = if value.is_empty() {
                                None
                            } else {
                                Some(value)
                            };
                        },
                    }
                }

                div { class: "flex gap-4",
                    button {
                        r#type: "button",
                        class: "px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50",
                        disabled: *loading.read(),
                        onclick: move |_| save_measurement(),
                        if *loading.read() {
                            {i18n.t(Key::Loading)}
                        } else {
                            {i18n.t(Key::Save)}
                        }
                    }
                    button {
                        r#type: "button",
                        class: "px-4 py-2 bg-gray-300 text-gray-700 rounded hover:bg-gray-400",
                        onclick: move |_| on_cancel.call(()),
                        {i18n.t(Key::Cancel)}
                    }
                }
            }
        }
    }
}
