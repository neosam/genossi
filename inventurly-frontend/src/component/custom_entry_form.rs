use crate::api;
use crate::i18n::{use_i18n, Key};
use crate::service::config::CONFIG;
use dioxus::prelude::*;
use rest_types::{ContainerTO, InventurCustomEntryTO};
use uuid::Uuid;

#[component]
pub fn CustomEntryForm(
    inventur_id: Uuid,
    rack_id: Uuid,
    containers: Vec<ContainerTO>,
    existing_entry: Option<InventurCustomEntryTO>,
    on_save: EventHandler<()>,
    on_cancel: EventHandler<()>,
) -> Element {
    let i18n = use_i18n();

    // Initialize form with existing entry or defaults
    let mut product_name = use_signal(|| {
        existing_entry
            .as_ref()
            .map(|e| e.custom_product_name.clone())
            .unwrap_or_default()
    });

    let mut count = use_signal(|| {
        existing_entry
            .as_ref()
            .and_then(|e| e.count)
            .map(|c| c.to_string())
            .unwrap_or_default()
    });

    let mut weight = use_signal(|| {
        existing_entry
            .as_ref()
            .and_then(|e| e.weight_grams)
            .map(|w| w.to_string())
            .unwrap_or_default()
    });

    let mut notes = use_signal(|| {
        existing_entry
            .as_ref()
            .and_then(|e| e.notes.clone())
            .unwrap_or_default()
    });

    let mut selected_container = use_signal(|| {
        existing_entry
            .as_ref()
            .and_then(|e| e.container_id)
    });

    let loading = use_signal(|| false);
    let error = use_signal(|| None::<String>);

    // Calculate if the entry is valid
    let is_valid = {
        let containers_clone = containers.clone();
        move || {
            // Must have a product name
            if product_name.read().trim().is_empty() {
                return false;
            }

            // Must have at least count or weight
            let parsed_count = count.read().parse::<i64>().ok();
            let parsed_weight = weight.read().parse::<i64>().ok();

            if parsed_count.is_none() && parsed_weight.is_none() {
                return false;
            }

            // If weight is provided with container, weight must be > container weight
            if let Some(weight_val) = parsed_weight {
                if weight_val > 0 {
                    if let Some(container_id) = *selected_container.read() {
                        if let Some(container) = containers_clone.iter().find(|c| c.id == Some(container_id)) {
                            if weight_val <= container.weight_grams {
                                return false;
                            }
                        }
                    }
                }
            }

            true
        }
    };

    let save_entry = {
        let existing_id = existing_entry.as_ref().and_then(|e| e.id);
        let existing_created = existing_entry.as_ref().and_then(|e| e.created);
        let existing_version = existing_entry.as_ref().and_then(|e| e.version);

        move || {
            spawn({
                let mut loading = loading.clone();
                let mut error = error.clone();
                let on_save = on_save.clone();
                let product_name = product_name.clone();
                let count = count.clone();
                let weight = weight.clone();
                let notes = notes.clone();
                let selected_container = selected_container.clone();

                async move {
                    loading.set(true);
                    error.set(None);

                    let config = CONFIG.read().clone();

                    // Validate product name
                    if product_name.read().trim().is_empty() {
                        error.set(Some("Please enter a product name".to_string()));
                        loading.set(false);
                        return;
                    }

                    // Parse values
                    let parsed_count = count.read().parse::<i64>().ok();
                    let parsed_weight = weight.read().parse::<i64>().ok();

                    if parsed_count.is_none() && parsed_weight.is_none() {
                        error.set(Some("Please enter at least count or weight".to_string()));
                        loading.set(false);
                        return;
                    }

                    let entry = InventurCustomEntryTO {
                        id: existing_id,
                        inventur_id,
                        custom_product_name: product_name.read().trim().to_string(),
                        rack_id: Some(rack_id),
                        container_id: *selected_container.read(),
                        count: parsed_count,
                        weight_grams: parsed_weight,
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

                    let result = if entry.id.is_some() {
                        api::update_custom_entry(&config, entry).await
                    } else {
                        api::create_custom_entry(&config, entry).await
                    };

                    match result {
                        Ok(_) => {
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
        // Modal backdrop
        div {
            class: "fixed inset-0 bg-black bg-opacity-50 z-40 flex items-center justify-center p-4",
            onclick: move |_| on_cancel.call(()),

            // Modal content
            div {
                class: "bg-white rounded-lg shadow-xl max-w-md w-full p-6 relative z-50",
                onclick: move |e| e.stop_propagation(),

                h3 { class: "text-xl font-semibold mb-4",
                    {i18n.t(if existing_entry.is_some() { Key::EditCustomEntry } else { Key::AddCustomEntry })}
                }

                if let Some(err) = error.read().as_ref() {
                    div { class: "bg-red-100 border border-red-400 text-red-700 px-3 py-2 rounded mb-4 text-sm",
                        {err.clone()}
                    }
                }

                // Show validation warning for weight vs container
                if let Ok(weight_val) = weight.read().parse::<i64>() {
                    if weight_val > 0 {
                        if let Some(container_id) = *selected_container.read() {
                            if let Some(container) = containers.iter().find(|c| c.id == Some(container_id)) {
                                if weight_val <= container.weight_grams {
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
                    // Product name
                    div {
                        label {
                            class: "block text-sm font-medium text-gray-700 mb-1",
                            {i18n.t(Key::CustomProductName)}
                        }
                        input {
                            r#type: "text",
                            class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-blue-500 focus:border-blue-500",
                            value: "{product_name.read()}",
                            oninput: move |e| {
                                product_name.set(e.value());
                            },
                            autofocus: true,
                            placeholder: "Unknown Product",
                        }
                    }

                    // Count
                    div {
                        label {
                            class: "block text-sm font-medium text-gray-700 mb-1",
                            {i18n.t(Key::Count)}
                            " (optional)"
                        }
                        input {
                            r#type: "number",
                            class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-blue-500 focus:border-blue-500",
                            value: "{count.read()}",
                            oninput: move |e| {
                                count.set(e.value());
                            },
                            placeholder: "0",
                        }
                    }

                    // Weight
                    div {
                        label {
                            class: "block text-sm font-medium text-gray-700 mb-1",
                            {i18n.t(Key::WeightGrams)}
                            " (optional)"
                        }
                        input {
                            r#type: "number",
                            class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-blue-500 focus:border-blue-500",
                            value: "{weight.read()}",
                            oninput: move |e| {
                                weight.set(e.value());
                            },
                            placeholder: "0",
                        }
                    }

                    // Container selection
                    div {
                        label {
                            class: "block text-sm font-medium text-gray-700 mb-1",
                            {i18n.t(Key::ContainerName)}
                            " (optional)"
                        }
                        select {
                            class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-blue-500 focus:border-blue-500",
                            value: "{selected_container.read().as_ref().map(|id| id.to_string()).unwrap_or_default()}",
                            onchange: move |e| {
                                if e.value().is_empty() {
                                    selected_container.set(None);
                                } else if let Ok(uuid) = Uuid::parse_str(&e.value()) {
                                    selected_container.set(Some(uuid));
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

                    // Notes
                    div {
                        label {
                            class: "block text-sm font-medium text-gray-700 mb-1",
                            {i18n.t(Key::Notes)}
                        }
                        textarea {
                            class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-blue-500 focus:border-blue-500",
                            rows: "2",
                            value: "{notes.read()}",
                            oninput: move |e| {
                                notes.set(e.value());
                            },
                            placeholder: "Additional information...",
                        }
                    }

                    div { class: "flex gap-2 pt-2",
                        button {
                            r#type: "button",
                            class: "flex-1 px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed",
                            disabled: *loading.read() || !is_valid(),
                            onclick: {
                                let save = save_entry.clone();
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
                            class: "flex-1 px-4 py-2 bg-gray-300 text-gray-700 rounded hover:bg-gray-400",
                            onclick: move |_| on_cancel.call(()),
                            {i18n.t(Key::Cancel)}
                        }
                    }
                }
            }
        }
    }
}
