use crate::api;
use crate::component::barcode_scanner::{BarcodeScanner, ScanResult};
use crate::component::searchable_product_selector::SearchableProductSelector;
use crate::i18n::{use_i18n, Key};
use crate::service::config::CONFIG;
use crate::service::container_rack::get_containers_in_rack_action;
use crate::service::product::PRODUCTS;
use crate::service::tara::{self, WeightUnit};
use dioxus::prelude::*;
use rest_types::{ContainerTO, InventurCustomEntryTO, ProductTO};
use uuid::Uuid;

#[derive(Clone, Copy, PartialEq)]
enum EntryMode {
    Custom,
    SelectProduct,
}

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

    // Determine initial mode based on existing entry
    let initial_mode = if existing_entry
        .as_ref()
        .map(|e| e.ean.is_none())
        .unwrap_or(false)
    {
        // Only use Custom mode if editing an entry that has no EAN (truly custom)
        EntryMode::Custom
    } else {
        // Default to SelectProduct mode for new entries or entries with EAN
        EntryMode::SelectProduct
    };

    let mut entry_mode = use_signal(|| initial_mode);
    let mut show_scanner = use_signal(|| false);

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

    let mut weight_unit = use_signal(|| tara::get_preferred_weight_unit());

    let mut notes = use_signal(|| {
        existing_entry
            .as_ref()
            .and_then(|e| e.notes.clone())
            .unwrap_or_default()
    });

    let mut selected_container = use_signal(|| {
        // Use existing entry's container if editing, otherwise try localStorage
        if let Some(container_id) = existing_entry.as_ref().and_then(|e| e.container_id) {
            Some(container_id)
        } else {
            // Try to load last used container from localStorage
            // Only use it if it exists in the available containers list
            tara::get_last_container_id().filter(|id| {
                containers.iter().any(|c| c.id == Some(*id) && c.deleted.is_none())
            })
        }
    });

    // Selected product state - try to load from existing entry's EAN
    let mut selected_product = use_signal(|| None::<ProductTO>);
    let mut selected_product_id = use_signal(|| None::<Uuid>);

    // Load product from existing entry's EAN on mount
    use_effect({
        let existing_entry = existing_entry.clone();
        move || {
            if let Some(ean) = existing_entry.as_ref().and_then(|e| e.ean.clone()) {
                let products = PRODUCTS.read();
                if let Some(product) = products.items.iter().find(|p| p.ean == ean) {
                    selected_product.set(Some(product.clone()));
                    selected_product_id.set(product.id);
                }
            }
        }
    });

    let loading = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    // Load rack-assigned container IDs for sorting
    let rack_container_ids = use_signal(|| Vec::<Uuid>::new());
    use_effect({
        let mut rack_container_ids = rack_container_ids.clone();
        move || {
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
        }
    });

    // Calculate if the entry is valid
    let is_valid = {
        let containers_clone = containers.clone();
        move || {
            let mode = *entry_mode.read();

            // Must have a product name (either manually entered or from selected product)
            if product_name.read().trim().is_empty() {
                return false;
            }

            let parsed_count = count.read().parse::<i64>().ok();
            let parsed_weight = weight.read().parse::<f64>().ok().map(|v| {
                match *weight_unit.read() {
                    WeightUnit::Kilogram => (v * 1000.0) as i64,
                    WeightUnit::Gram => v as i64,
                }
            });

            // Validation based on mode
            match mode {
                EntryMode::Custom => {
                    // Custom mode: must have at least count or weight
                    if parsed_count.is_none() && parsed_weight.is_none() {
                        return false;
                    }
                }
                EntryMode::SelectProduct => {
                    // Must have a product selected
                    if selected_product.read().is_none() {
                        return false;
                    }

                    // Check product's requires_weighing flag
                    if let Some(product) = selected_product.read().as_ref() {
                        if product.requires_weighing {
                            // Require weight
                            if parsed_weight.is_none() || parsed_weight == Some(0) {
                                return false;
                            }
                        } else {
                            // Require count
                            if parsed_count.is_none() {
                                return false;
                            }
                        }
                    }
                }
            }

            // If weight is provided with container, weight must be > container weight
            if let Some(weight_val) = parsed_weight {
                if weight_val > 0 {
                    if let Some(container_id) = *selected_container.read() {
                        if let Some(container) =
                            containers_clone.iter().find(|c| c.id == Some(container_id))
                        {
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
                let weight_unit = weight_unit.clone();
                let notes = notes.clone();
                let selected_container = selected_container.clone();
                let entry_mode = entry_mode.clone();
                let selected_product = selected_product.clone();

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
                    let parsed_weight = weight.read().parse::<f64>().ok().map(|v| {
                        match *weight_unit.read() {
                            WeightUnit::Kilogram => (v * 1000.0) as i64,
                            WeightUnit::Gram => v as i64,
                        }
                    });

                    // Get EAN if in SelectProduct mode
                    let ean = if *entry_mode.read() == EntryMode::SelectProduct {
                        selected_product.read().as_ref().map(|p| p.ean.clone())
                    } else {
                        None
                    };

                    // Subtract global tara (body weight) from weight measurement
                    let final_weight = parsed_weight.map(|w| w - tara::get_tara_grams());

                    let entry = InventurCustomEntryTO {
                        id: existing_id,
                        inventur_id,
                        custom_product_name: product_name.read().trim().to_string(),
                        ean,
                        rack_id: Some(rack_id),
                        container_id: *selected_container.read(),
                        count: parsed_count,
                        weight_grams: final_weight,
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

    // Handle barcode scan result
    let handle_scan = move |result: ScanResult| {
        let products = PRODUCTS.read();
        if let Some(product) = products.items.iter().find(|p| p.ean == result.barcode) {
            selected_product.set(Some(product.clone()));
            selected_product_id.set(product.id);
            product_name.set(product.name.clone());
            // Clear fields that don't apply to this product type
            if product.requires_weighing {
                count.set(String::new());
            } else {
                weight.set(String::new());
                selected_container.set(None);
            }
        } else {
            // Product not found - show error briefly
            error.set(Some(format!(
                "Product with EAN {} not found",
                result.barcode
            )));
        }
        show_scanner.set(false);
    };

    // Handle product selection from dropdown
    let handle_product_selected = move |product_id: Option<Uuid>| {
        if let Some(id) = product_id {
            let products = PRODUCTS.read();
            if let Some(product) = products.items.iter().find(|p| p.id == Some(id)) {
                selected_product.set(Some(product.clone()));
                selected_product_id.set(Some(id));
                product_name.set(product.name.clone());
                // Clear fields that don't apply to this product type
                if product.requires_weighing {
                    count.set(String::new());
                } else {
                    weight.set(String::new());
                    selected_container.set(None);
                }
            }
        } else {
            selected_product.set(None);
            selected_product_id.set(None);
            product_name.set(String::new());
        }
    };

    // Determine which fields to show based on mode and product
    let (show_count, show_weight, show_container) = {
        match *entry_mode.read() {
            EntryMode::Custom => (true, true, true), // Show all in custom mode
            EntryMode::SelectProduct => {
                if let Some(product) = selected_product.read().as_ref() {
                    if product.requires_weighing {
                        (false, true, true) // Weighing: show weight + container
                    } else {
                        (true, false, false) // Counting: show only count
                    }
                } else {
                    (false, false, false) // No product selected yet, hide all
                }
            }
        }
    };

    rsx! {
        // Barcode scanner modal
        if *show_scanner.read() {
            BarcodeScanner {
                on_scan: handle_scan,
                on_close: move |_| show_scanner.set(false),
            }
        }

        // Modal backdrop
        div {
            class: "fixed inset-0 bg-black bg-opacity-50 z-40 flex items-center justify-center p-4",
            onclick: move |_| on_cancel.call(()),

            // Modal content
            div {
                class: "bg-white rounded-lg shadow-xl max-w-md w-full p-6 relative z-50",
                onclick: move |e| e.stop_propagation(),

                h3 { class: "text-xl font-semibold mb-4",
                    {
                        i18n.t(
                            if existing_entry.is_some() {
                                Key::EditCustomEntry
                            } else {
                                Key::AddCustomEntry
                            },
                        )
                    }
                }

                if let Some(err) = error.read().as_ref() {
                    div { class: "bg-red-100 border border-red-400 text-red-700 px-3 py-2 rounded mb-4 text-sm",
                        {err.clone()}
                    }
                }

                // Tab buttons
                div { class: "flex border-b mb-4",
                    button {
                        r#type: "button",
                        class: if *entry_mode.read() == EntryMode::SelectProduct { "px-4 py-2 border-b-2 border-blue-500 text-blue-600 font-medium" } else { "px-4 py-2 border-b-2 border-transparent text-gray-500 hover:text-gray-700" },
                        onclick: move |_| {
                            entry_mode.set(EntryMode::SelectProduct);
                        },
                        {i18n.t(Key::SelectProduct)}
                    }
                    button {
                        r#type: "button",
                        class: if *entry_mode.read() == EntryMode::Custom { "px-4 py-2 border-b-2 border-blue-500 text-blue-600 font-medium" } else { "px-4 py-2 border-b-2 border-transparent text-gray-500 hover:text-gray-700" },
                        onclick: move |_| {
                            entry_mode.set(EntryMode::Custom);
                            // Clear product selection when switching to custom
                            selected_product.set(None);
                            selected_product_id.set(None);
                        },
                        {i18n.t(Key::CustomEntry)}
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
                    // Product name / Product selection based on mode
                    match *entry_mode.read() {
                        EntryMode::Custom => rsx! {
                            div {
                                // Show selected product info
                                label { class: "block text-sm font-medium text-gray-700 mb-1", {i18n.t(Key::CustomProductName)} }
                                input {
                                    r#type: "text",
                                    class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-blue-500 focus:border-blue-500",
                                    value: "{product_name.read()}",
                                    onmounted: move |event| async move {
                                        let _ = event.set_focus(true).await;
                                    },
                                    oninput: move |e| {
                                        product_name.set(e.value());
                                    },
                                    placeholder: "Unknown Product",
                                }
                            }
                        },
                        EntryMode::SelectProduct => rsx! {
                            div {
                                label { class: "block text-sm font-medium text-gray-700 mb-1", {i18n.t(Key::SelectProduct)} }
                                div { class: "flex gap-2",
                                    div { class: "flex-1",
                                        SearchableProductSelector {
                                            selected_product_id: *selected_product_id.read(),
                                            disabled: false,
                                            on_product_selected: handle_product_selected,
                                        }
                                    }
                                    button {
                                        r#type: "button",
                                        class: "px-3 py-2 bg-gray-100 hover:bg-gray-200 rounded-md text-gray-700 flex items-center gap-1",
                                        onclick: move |_| show_scanner.set(true),
                                        title: i18n.t(Key::ScanToSelectProduct).to_string(),
                                        "📷"
                                    }
                                }
                                if let Some(product) = selected_product.read().as_ref() {
                                    div { class: "mt-2 text-sm text-gray-600",
                                        "EAN: {product.ean}"
                                        if product.requires_weighing {
                                            span { class: "ml-2 text-orange-600", "(Requires weighing)" }
                                        }
                                    }
                                }
                            }
                        },
                    }

                    // Count (shown in custom mode, or for non-weighing products)
                    if show_count {
                        div {
                            label { class: "block text-sm font-medium text-gray-700 mb-1",
                                {i18n.t(Key::Count)}
                                // In custom mode, show (optional); in product mode, it's required
                                if *entry_mode.read() == EntryMode::Custom {
                                    " (optional)"
                                } else {
                                    span { class: "text-red-500 ml-1", "*" }
                                }
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
                    }

                    // Weight (shown in custom mode, or for weighing products)
                    if show_weight {
                        div {
                            label { class: "block text-sm font-medium text-gray-700 mb-1",
                                {i18n.t(Key::MeasurementWeight)}
                                // In custom mode, show (optional); in product mode, it's required
                                if *entry_mode.read() == EntryMode::Custom {
                                    " (optional)"
                                } else {
                                    span { class: "text-red-500 ml-1", "*" }
                                }
                            }
                            input {
                                r#type: "number",
                                inputmode: "decimal",
                                step: if *weight_unit.read() == WeightUnit::Kilogram { "0.001" } else { "1" },
                                class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-blue-500 focus:border-blue-500",
                                value: "{weight.read()}",
                                oninput: move |e| {
                                    weight.set(e.value());
                                },
                                placeholder: "0",
                            }
                            // Unit selector (kg/g toggle)
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

                    // Container selection (only for weighing products or custom mode)
                    if show_container {
                        div {
                            label { class: "block text-sm font-medium text-gray-700 mb-1",
                                {i18n.t(Key::Container)}
                                " (optional)"
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

                                // Containers assigned to this rack (sorted by sort_order)
                                {
                                    let rack_ids = rack_container_ids();
                                    let rack_containers: Vec<_> = rack_ids.iter()
                                        .filter_map(|id| containers.iter().find(|c| c.id == Some(*id) && c.deleted.is_none()))
                                        .collect();

                                    rsx! {
                                        for container in rack_containers.iter() {
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

                                // Separator (only if there are rack containers and other containers)
                                {
                                    let rack_ids = rack_container_ids();
                                    let has_rack_containers = rack_ids.iter()
                                        .any(|id| containers.iter().any(|c| c.id == Some(*id) && c.deleted.is_none()));
                                    let has_other_containers = containers.iter()
                                        .any(|c| c.deleted.is_none() && !rack_ids.contains(&c.id.unwrap_or(Uuid::nil())));

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
                                    let mut other_containers: Vec<_> = containers.iter()
                                        .filter(|c| c.deleted.is_none() && !rack_ids.contains(&c.id.unwrap_or(Uuid::nil())))
                                        .collect();
                                    other_containers.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

                                    rsx! {
                                        for container in other_containers.iter() {
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
                        }
                    }

                    // Notes
                    div {
                        label { class: "block text-sm font-medium text-gray-700 mb-1",
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
