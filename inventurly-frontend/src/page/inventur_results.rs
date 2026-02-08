use crate::api;
use crate::component::TopBar;
use crate::i18n::{use_i18n, Key};
use crate::router::Route;
use crate::service::config::CONFIG;
use dioxus::prelude::*;
use rest_types::{ContainerTO, InventurCustomEntryTO, InventurMeasurementTO, InventurProductReportItemTO, RackMeasuredTO, RackTO};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use uuid::Uuid;

/// Combined data for expanded product details (measurements + custom entries)
#[derive(Clone, Default)]
struct ExpandedProductData {
    measurements: Vec<InventurMeasurementTO>,
    custom_entries: Vec<InventurCustomEntryTO>,
}

#[component]
pub fn InventurResults(id: String) -> Element {
    let i18n = use_i18n();
    let nav = navigator();

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

    // Local state
    let mut results = use_signal::<Vec<InventurProductReportItemTO>>(Vec::new);
    let mut loading = use_signal(|| true);
    let mut error = use_signal::<Option<String>>(|| None);

    // Filter state - filter_racks now stores rack IDs as strings
    let mut filter_input = use_signal(String::new);
    let mut filter_query = use_signal(String::new);
    let mut filter_rack_ids = use_signal::<Vec<Uuid>>(Vec::new);
    let mut filter_has_count = use_signal::<Option<bool>>(|| None);
    let mut filter_has_weight = use_signal::<Option<bool>>(|| None);
    let mut filters_expanded = use_signal(|| false);

    // Expandable measurement details state
    // We use EAN as the key since that's what we aggregate by, and custom entries don't have product_id
    let mut expanded_eans = use_signal::<HashSet<Arc<str>>>(HashSet::new);
    let mut ean_data = use_signal::<HashMap<Arc<str>, ExpandedProductData>>(HashMap::new);
    let mut loading_eans = use_signal::<HashSet<Arc<str>>>(HashSet::new);

    // Rack lookup map for displaying rack names in expanded details
    let mut rack_map = use_signal::<HashMap<Uuid, RackTO>>(HashMap::new);

    // Container lookup map for subtracting container weight
    let mut container_map = use_signal::<HashMap<Uuid, ContainerTO>>(HashMap::new);

    // Load data on mount
    use_effect(move || {
        spawn(async move {
            loading.set(true);
            error.set(None);

            let config = CONFIG.read().clone();
            if config.backend.is_empty() {
                loading.set(false);
                return;
            }

            // Load report, racks, and containers in parallel
            let report_future = api::get_inventur_product_report(&config, inventur_id);
            let racks_future = api::get_racks(&config);
            let containers_future = api::get_containers(&config);

            let (report_result, racks_result, containers_result) = futures::join!(report_future, racks_future, containers_future);

            match report_result {
                Ok(report) => {
                    results.set(report);
                }
                Err(e) => {
                    error.set(Some(format!("Failed to load results: {}", e)));
                }
            }

            // Build rack lookup map
            if let Ok(racks) = racks_result {
                let map: HashMap<Uuid, RackTO> = racks
                    .into_iter()
                    .filter_map(|r| r.id.map(|id| (id, r)))
                    .collect();
                rack_map.set(map);
            }

            // Build container lookup map
            if let Ok(containers) = containers_result {
                let map: HashMap<Uuid, ContainerTO> = containers
                    .into_iter()
                    .filter_map(|c| c.id.map(|id| (id, c)))
                    .collect();
                container_map.set(map);
            }

            loading.set(false);
        });
    });

    // Debounced text filter update
    use_effect(move || {
        let query = filter_input();
        spawn(async move {
            gloo_timers::future::TimeoutFuture::new(500).await;
            if query == filter_input() {
                filter_query.set(query);
            }
        });
    });

    // Get distinct racks from results (deduplicated by ID)
    let all_results = results.read();
    let distinct_racks: Vec<RackMeasuredTO> = all_results
        .iter()
        .flat_map(|r| r.racks_measured.iter().cloned())
        .map(|r| (r.id, r))
        .collect::<HashMap<Uuid, RackMeasuredTO>>()
        .into_values()
        .collect();
    let mut sorted_racks = distinct_racks.clone();
    sorted_racks.sort_by(|a, b| a.name.cmp(&b.name));

    // Apply filters
    let query = filter_query.read().to_lowercase();
    let selected_rack_ids = filter_rack_ids.read();
    let has_count_filter = *filter_has_count.read();
    let has_weight_filter = *filter_has_weight.read();

    let filtered_results: Vec<_> = all_results
        .iter()
        .filter(|r| {
            // Text search filter
            if !query.is_empty() {
                let matches = r.ean.to_lowercase().contains(&query)
                    || r.product_name.to_lowercase().contains(&query)
                    || r.short_name.to_lowercase().contains(&query);
                if !matches {
                    return false;
                }
            }

            // Rack filter (by ID)
            if !selected_rack_ids.is_empty() {
                let has_matching_rack = r.racks_measured.iter().any(|rack| selected_rack_ids.contains(&rack.id));
                if !has_matching_rack {
                    return false;
                }
            }

            // Has count filter
            if let Some(has_count) = has_count_filter {
                let actually_has_count = r.total_count.map(|c| c > 0).unwrap_or(false);
                if has_count != actually_has_count {
                    return false;
                }
            }

            // Has weight filter
            if let Some(has_weight) = has_weight_filter {
                let actually_has_weight = r.total_weight_grams.map(|w| w > 0).unwrap_or(false);
                if has_weight != actually_has_weight {
                    return false;
                }
            }

            true
        })
        .collect();

    // Handlers
    let mut toggle_rack = move |rack_id: Uuid| {
        let mut rack_ids = filter_rack_ids.write();
        if rack_ids.contains(&rack_id) {
            rack_ids.retain(|r| *r != rack_id);
        } else {
            rack_ids.push(rack_id);
        }
    };

    let clear_all_filters = move |_| {
        filter_input.set(String::new());
        filter_query.set(String::new());
        filter_rack_ids.write().clear();
        filter_has_count.set(None);
        filter_has_weight.set(None);
    };

    // Handler to toggle expanded state and load measurements + custom entries if needed
    let mut toggle_details = move |ean: Arc<str>, product_id: Option<Uuid>| {
        let is_expanded = expanded_eans.read().contains(&ean);

        if is_expanded {
            // Collapse
            expanded_eans.write().remove(&ean);
        } else {
            // Expand
            expanded_eans.write().insert(ean.clone());

            // Load data if not already loaded
            if !ean_data.read().contains_key(&ean) {
                loading_eans.write().insert(ean.clone());

                let ean_for_spawn = ean.clone();
                spawn(async move {
                    let config = CONFIG.read().clone();
                    if config.backend.is_empty() {
                        loading_eans.write().remove(&ean_for_spawn);
                        return;
                    }

                    let mut data = ExpandedProductData::default();

                    // Load measurements (only if we have a product_id)
                    if let Some(pid) = product_id {
                        match api::get_measurements_by_product_and_inventur(&config, pid, inventur_id).await {
                            Ok(measurements) => {
                                data.measurements = measurements;
                            }
                            Err(e) => {
                                tracing::error!("Failed to load measurements for product {}: {}", pid, e);
                            }
                        }
                    }

                    // Load custom entries (by EAN)
                    match api::get_custom_entries_by_ean_and_inventur(&config, &ean_for_spawn, inventur_id).await {
                        Ok(entries) => {
                            data.custom_entries = entries;
                        }
                        Err(e) => {
                            tracing::error!("Failed to load custom entries for EAN {}: {}", ean_for_spawn, e);
                        }
                    }

                    ean_data.write().insert(ean_for_spawn.clone(), data);
                    loading_eans.write().remove(&ean_for_spawn);
                });
            }
        }
    };

    let is_loading = *loading.read();
    let error_msg = error.read().clone();

    rsx! {
        div { class: "flex flex-col min-h-screen",
            TopBar {}
            div { class: "flex-1 container mx-auto px-4 py-8",
                // Header with back button
                div { class: "flex justify-between items-center mb-6",
                    div { class: "flex items-center gap-4",
                        button {
                            class: "px-3 py-2 text-gray-600 hover:text-gray-900 hover:bg-gray-100 rounded transition-colors",
                            onclick: {
                                let id_clone = id.clone();
                                move |_| {
                                    nav.push(Route::InventurDetails { id: id_clone.clone() });
                                }
                            },
                            "< {i18n.t(Key::Back)}"
                        }
                        h1 { class: "text-3xl font-bold",
                            "{i18n.t(Key::InventurResults)}"
                        }
                    }
                    div { class: "flex items-center gap-4",
                        a {
                            class: "px-4 py-2 bg-green-600 text-white rounded-md hover:bg-green-700 text-sm font-medium",
                            href: "{CONFIG.read().backend}/inventur-report/{inventur_id}/report/csv",
                            "{i18n.t(Key::DownloadCsv)}"
                        }
                        span { class: "text-gray-500",
                            "({filtered_results.len()} / {all_results.len()})"
                        }
                    }
                }

                // Filter section
                div { class: "bg-white rounded-lg shadow mb-6",
                    // Search input
                    div { class: "p-4 border-b",
                        div { class: "flex gap-4 items-center",
                            input {
                                class: "flex-1 px-4 py-2 border rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500",
                                r#type: "text",
                                placeholder: "{i18n.t(Key::Search)}...",
                                value: "{filter_input}",
                                oninput: move |e| filter_input.set(e.value().clone()),
                            }
                            button {
                                class: "px-4 py-2 text-gray-600 hover:text-gray-900 border rounded-lg hover:bg-gray-50 transition-colors",
                                onclick: move |_| {
                                    let current = *filters_expanded.read();
                                    filters_expanded.set(!current);
                                },
                                if *filters_expanded.read() {
                                    "{i18n.t(Key::HideFilters)}"
                                } else {
                                    "{i18n.t(Key::ShowFilters)}"
                                }
                            }
                        }
                    }

                    // Expanded filters
                    if *filters_expanded.read() {
                        div { class: "p-4 border-b bg-gray-50",
                            div { class: "grid grid-cols-1 md:grid-cols-3 gap-6",
                                // Rack filter
                                div {
                                    h4 { class: "font-medium text-gray-700 mb-2",
                                        "{i18n.t(Key::FilterByRack)}"
                                    }
                                    div { class: "space-y-1 max-h-40 overflow-y-auto",
                                        for rack in sorted_racks.iter() {
                                            {
                                                let rack_id = rack.id;
                                                let rack_name = rack.name.clone();
                                                rsx! {
                                                    label { class: "flex items-center gap-2 cursor-pointer hover:bg-gray-100 p-1 rounded",
                                                        input {
                                                            r#type: "checkbox",
                                                            checked: selected_rack_ids.contains(&rack_id),
                                                            onchange: move |_| toggle_rack(rack_id),
                                                        }
                                                        span { "{rack_name}" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                // Has count filter
                                div {
                                    h4 { class: "font-medium text-gray-700 mb-2",
                                        "{i18n.t(Key::HasCount)}"
                                    }
                                    div { class: "space-y-1",
                                        label { class: "flex items-center gap-2 cursor-pointer hover:bg-gray-100 p-1 rounded",
                                            input {
                                                r#type: "radio",
                                                name: "has_count",
                                                checked: has_count_filter.is_none(),
                                                onchange: move |_| filter_has_count.set(None),
                                            }
                                            span { "{i18n.t(Key::All)}" }
                                        }
                                        label { class: "flex items-center gap-2 cursor-pointer hover:bg-gray-100 p-1 rounded",
                                            input {
                                                r#type: "radio",
                                                name: "has_count",
                                                checked: has_count_filter == Some(true),
                                                onchange: move |_| filter_has_count.set(Some(true)),
                                            }
                                            span { "{i18n.t(Key::Yes)}" }
                                        }
                                        label { class: "flex items-center gap-2 cursor-pointer hover:bg-gray-100 p-1 rounded",
                                            input {
                                                r#type: "radio",
                                                name: "has_count",
                                                checked: has_count_filter == Some(false),
                                                onchange: move |_| filter_has_count.set(Some(false)),
                                            }
                                            span { "{i18n.t(Key::No)}" }
                                        }
                                    }
                                }

                                // Has weight filter
                                div {
                                    h4 { class: "font-medium text-gray-700 mb-2",
                                        "{i18n.t(Key::HasWeight)}"
                                    }
                                    div { class: "space-y-1",
                                        label { class: "flex items-center gap-2 cursor-pointer hover:bg-gray-100 p-1 rounded",
                                            input {
                                                r#type: "radio",
                                                name: "has_weight",
                                                checked: has_weight_filter.is_none(),
                                                onchange: move |_| filter_has_weight.set(None),
                                            }
                                            span { "{i18n.t(Key::All)}" }
                                        }
                                        label { class: "flex items-center gap-2 cursor-pointer hover:bg-gray-100 p-1 rounded",
                                            input {
                                                r#type: "radio",
                                                name: "has_weight",
                                                checked: has_weight_filter == Some(true),
                                                onchange: move |_| filter_has_weight.set(Some(true)),
                                            }
                                            span { "{i18n.t(Key::Yes)}" }
                                        }
                                        label { class: "flex items-center gap-2 cursor-pointer hover:bg-gray-100 p-1 rounded",
                                            input {
                                                r#type: "radio",
                                                name: "has_weight",
                                                checked: has_weight_filter == Some(false),
                                                onchange: move |_| filter_has_weight.set(Some(false)),
                                            }
                                            span { "{i18n.t(Key::No)}" }
                                        }
                                    }
                                }
                            }

                            // Clear all filters button
                            div { class: "mt-4 flex justify-end",
                                button {
                                    class: "px-4 py-2 text-red-600 hover:text-red-800 hover:bg-red-50 rounded transition-colors",
                                    onclick: clear_all_filters,
                                    "{i18n.t(Key::ClearAllFilters)}"
                                }
                            }
                        }
                    }
                }

                // Results table
                div { class: "bg-white rounded-lg shadow overflow-auto max-h-[calc(100vh-300px)]",
                    if is_loading {
                        div { class: "p-6 text-center text-gray-500",
                            "{i18n.t(Key::Loading)}"
                        }
                    } else if let Some(err) = error_msg {
                        div { class: "p-6 text-center text-red-500",
                            "{err}"
                        }
                    } else if filtered_results.is_empty() {
                        div { class: "p-6 text-center text-gray-500",
                            "{i18n.t(Key::NoResultsFound)}"
                        }
                    } else {
                        table { class: "w-full",
                            thead {
                                tr { class: "border-b bg-gray-50",
                                    th { class: "px-2 py-3 text-center text-xs font-medium text-gray-500 uppercase tracking-wider w-12",
                                        // Empty header for expand/collapse column
                                    }
                                    th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                        "{i18n.t(Key::ProductEan)}"
                                    }
                                    th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                        "{i18n.t(Key::ProductName)}"
                                    }
                                    th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                        "{i18n.t(Key::ProductShortName)}"
                                    }
                                    th { class: "px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider",
                                        "{i18n.t(Key::TotalCount)}"
                                    }
                                    th { class: "px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider",
                                        "{i18n.t(Key::TotalWeight)} (g)"
                                    }
                                    th { class: "px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider",
                                        "{i18n.t(Key::MeasurementCountHeader)}"
                                    }
                                    th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                        "{i18n.t(Key::RacksMeasured)}"
                                    }
                                    th { class: "px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider",
                                        "{i18n.t(Key::PricePerKg)}"
                                    }
                                    th { class: "px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider",
                                        "{i18n.t(Key::TotalValue)}"
                                    }
                                    th { class: "px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider",
                                        "{i18n.t(Key::Deposit)}"
                                    }
                                    th { class: "px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider",
                                        "{i18n.t(Key::TotalWithDeposit)}"
                                    }
                                }
                            }
                            tbody {
                                for item in filtered_results.iter() {
                                    {
                                        let ean: Arc<str> = item.ean.clone().into();
                                        let product_id = item.product_id;
                                        let is_expanded = expanded_eans.read().contains(&ean);
                                        let is_loading_data = loading_eans.read().contains(&ean);
                                        let expanded_data = ean_data.read().get(&ean).cloned();
                                        let id_clone = id.clone();
                                        let ean_for_click = ean.clone();

                                        rsx! {
                                            // Summary row
                                            tr { class: "border-b hover:bg-gray-50",
                                                // Expand/collapse button cell
                                                td { class: "px-2 py-4 text-center",
                                                    button {
                                                        class: "p-1 rounded hover:bg-gray-200 transition-colors text-gray-500 hover:text-gray-700",
                                                        onclick: move |_| toggle_details(ean_for_click.clone(), product_id),
                                                        title: if is_expanded { i18n.t(Key::HideMeasurements).to_string() } else { i18n.t(Key::ShowMeasurements).to_string() },
                                                        if is_loading_data {
                                                            // Loading spinner
                                                            span { class: "inline-block w-4 h-4 border-2 border-gray-300 border-t-blue-500 rounded-full animate-spin" }
                                                        } else if is_expanded {
                                                            "▼"
                                                        } else {
                                                            "▶"
                                                        }
                                                    }
                                                }
                                                td { class: "px-6 py-4 whitespace-nowrap text-sm font-mono",
                                                    "{item.ean}"
                                                }
                                                td { class: "px-6 py-4 text-sm",
                                                    "{item.product_name}"
                                                }
                                                td { class: "px-6 py-4 text-sm text-gray-500",
                                                    "{item.short_name}"
                                                }
                                                td { class: "px-6 py-4 whitespace-nowrap text-sm text-right",
                                                    if let Some(count) = item.total_count {
                                                        "{count}"
                                                    } else {
                                                        "-"
                                                    }
                                                }
                                                td { class: "px-6 py-4 whitespace-nowrap text-sm text-right",
                                                    if let Some(weight) = item.total_weight_grams {
                                                        "{weight}"
                                                    } else {
                                                        "-"
                                                    }
                                                }
                                                td { class: "px-6 py-4 whitespace-nowrap text-sm text-right",
                                                    "{item.measurement_count}"
                                                }
                                                td { class: "px-6 py-4 text-sm",
                                                    for (i, rack) in item.racks_measured.iter().enumerate() {
                                                        if i > 0 {
                                                            span { ", " }
                                                        }
                                                        Link {
                                                            to: Route::InventurRackMeasure {
                                                                inventur_id: id_clone.clone(),
                                                                rack_id: rack.id.to_string(),
                                                            },
                                                            class: "text-blue-600 hover:text-blue-800 hover:underline",
                                                            "{rack.name}"
                                                        }
                                                    }
                                                }
                                                td { class: "px-6 py-4 whitespace-nowrap text-sm text-right",
                                                    if let Some(price_cents) = item.price_cents {
                                                        "{i18n.format_price(price_cents)}"
                                                    } else {
                                                        "-"
                                                    }
                                                }
                                                td { class: "px-6 py-4 whitespace-nowrap text-sm text-right",
                                                    if let Some(total_value_cents) = item.total_value_cents {
                                                        "{i18n.format_price(total_value_cents)}"
                                                    } else {
                                                        "-"
                                                    }
                                                }
                                                td { class: "px-6 py-4 whitespace-nowrap text-sm text-right",
                                                    if let Some(deposit_value_cents) = item.deposit_value_cents {
                                                        "{i18n.format_price(deposit_value_cents)}"
                                                    } else {
                                                        "-"
                                                    }
                                                }
                                                td { class: "px-6 py-4 whitespace-nowrap text-sm text-right font-semibold",
                                                    if let Some(total_with_deposit_cents) = item.total_with_deposit_cents {
                                                        "{i18n.format_price(total_with_deposit_cents)}"
                                                    } else {
                                                        "-"
                                                    }
                                                }
                                            }

                                            // Expanded details row (measurements + custom entries)
                                            if is_expanded {
                                                if let Some(ref data) = expanded_data {
                                                    tr { class: "bg-gray-50",
                                                        td { colspan: "12", class: "px-6 py-4",
                                                            div { class: "ml-8 border-l-2 border-blue-200 pl-4 space-y-4",
                                                                // Measurements section
                                                                if !data.measurements.is_empty() {
                                                                    div {
                                                                        h4 { class: "text-sm font-medium text-gray-700 mb-2",
                                                                            "{i18n.t(Key::Measurements)} ({data.measurements.len()})"
                                                                        }
                                                                        div { class: "space-y-2",
                                                                            for measurement in data.measurements.iter() {
                                                                                {
                                                                                    let rack_name = measurement.rack_id
                                                                                        .and_then(|rid| rack_map.read().get(&rid).map(|r| r.name.clone()));
                                                                                    // Calculate adjusted weight (subtract container weight if weight > 0)
                                                                                    let adjusted_weight = measurement.weight_grams.map(|w| {
                                                                                        if w > 0 {
                                                                                            let container_weight = measurement.container_id
                                                                                                .and_then(|cid| container_map.read().get(&cid).map(|c| c.weight_grams))
                                                                                                .unwrap_or(0);
                                                                                            w - container_weight
                                                                                        } else {
                                                                                            w
                                                                                        }
                                                                                    });
                                                                                    rsx! {
                                                                                        div { class: "bg-white rounded border p-3 text-sm",
                                                                                            div { class: "grid grid-cols-2 md:grid-cols-5 gap-4",
                                                                                                div {
                                                                                                    span { class: "text-gray-500", "{i18n.t(Key::Rack)}: " }
                                                                                                    span { class: "font-medium",
                                                                                                        if let Some(ref name) = rack_name {
                                                                                                            "{name}"
                                                                                                        } else {
                                                                                                            "-"
                                                                                                        }
                                                                                                    }
                                                                                                }
                                                                                                div {
                                                                                                    span { class: "text-gray-500", "{i18n.t(Key::Count)}: " }
                                                                                                    span { class: "font-medium",
                                                                                                        if let Some(count) = measurement.count {
                                                                                                            "{count}"
                                                                                                        } else {
                                                                                                            "-"
                                                                                                        }
                                                                                                    }
                                                                                                }
                                                                                                div {
                                                                                                    span { class: "text-gray-500", "{i18n.t(Key::WeightGrams)}: " }
                                                                                                    span { class: "font-medium",
                                                                                                        if let Some(weight) = adjusted_weight {
                                                                                                            "{weight}"
                                                                                                        } else {
                                                                                                            "-"
                                                                                                        }
                                                                                                    }
                                                                                                }
                                                                                                div {
                                                                                                    span { class: "text-gray-500", "{i18n.t(Key::MeasuredBy)}: " }
                                                                                                    span { class: "font-medium",
                                                                                                        if let Some(ref measured_by) = measurement.measured_by {
                                                                                                            "{measured_by}"
                                                                                                        } else {
                                                                                                            "-"
                                                                                                        }
                                                                                                    }
                                                                                                }
                                                                                                div {
                                                                                                    span { class: "text-gray-500", "{i18n.t(Key::MeasuredAt)}: " }
                                                                                                    span { class: "font-medium",
                                                                                                        if let Some(measured_at) = measurement.measured_at {
                                                                                                            "{i18n.format_datetime(measured_at)}"
                                                                                                        } else {
                                                                                                            "-"
                                                                                                        }
                                                                                                    }
                                                                                                }
                                                                                            }
                                                                                            if let Some(ref notes) = measurement.notes {
                                                                                                if !notes.is_empty() {
                                                                                                    div { class: "mt-2 text-gray-600 italic",
                                                                                                        span { class: "text-gray-500", "{i18n.t(Key::Notes)}: " }
                                                                                                        "{notes}"
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

                                                                // Custom entries section
                                                                if !data.custom_entries.is_empty() {
                                                                    div {
                                                                        h4 { class: "text-sm font-medium text-gray-700 mb-2",
                                                                            "{i18n.t(Key::CustomEntries)} ({data.custom_entries.len()})"
                                                                        }
                                                                        div { class: "space-y-2",
                                                                            for entry in data.custom_entries.iter() {
                                                                                {
                                                                                    let rack_name = entry.rack_id
                                                                                        .and_then(|rid| rack_map.read().get(&rid).map(|r| r.name.clone()));
                                                                                    // Calculate adjusted weight (subtract container weight if weight > 0)
                                                                                    let adjusted_weight = entry.weight_grams.map(|w| {
                                                                                        if w > 0 {
                                                                                            let container_weight = entry.container_id
                                                                                                .and_then(|cid| container_map.read().get(&cid).map(|c| c.weight_grams))
                                                                                                .unwrap_or(0);
                                                                                            w - container_weight
                                                                                        } else {
                                                                                            w
                                                                                        }
                                                                                    });
                                                                                    rsx! {
                                                                                        div { class: "bg-yellow-50 rounded border border-yellow-200 p-3 text-sm",
                                                                                            div { class: "grid grid-cols-2 md:grid-cols-5 gap-4",
                                                                                                div {
                                                                                                    span { class: "text-gray-500", "{i18n.t(Key::Rack)}: " }
                                                                                                    span { class: "font-medium",
                                                                                                        if let Some(ref name) = rack_name {
                                                                                                            "{name}"
                                                                                                        } else {
                                                                                                            "-"
                                                                                                        }
                                                                                                    }
                                                                                                }
                                                                                                div {
                                                                                                    span { class: "text-gray-500", "{i18n.t(Key::Count)}: " }
                                                                                                    span { class: "font-medium",
                                                                                                        if let Some(count) = entry.count {
                                                                                                            "{count}"
                                                                                                        } else {
                                                                                                            "-"
                                                                                                        }
                                                                                                    }
                                                                                                }
                                                                                                div {
                                                                                                    span { class: "text-gray-500", "{i18n.t(Key::WeightGrams)}: " }
                                                                                                    span { class: "font-medium",
                                                                                                        if let Some(weight) = adjusted_weight {
                                                                                                            "{weight}"
                                                                                                        } else {
                                                                                                            "-"
                                                                                                        }
                                                                                                    }
                                                                                                }
                                                                                                div {
                                                                                                    span { class: "text-gray-500", "{i18n.t(Key::MeasuredBy)}: " }
                                                                                                    span { class: "font-medium",
                                                                                                        if let Some(ref measured_by) = entry.measured_by {
                                                                                                            "{measured_by}"
                                                                                                        } else {
                                                                                                            "-"
                                                                                                        }
                                                                                                    }
                                                                                                }
                                                                                                div {
                                                                                                    span { class: "text-gray-500", "{i18n.t(Key::MeasuredAt)}: " }
                                                                                                    span { class: "font-medium",
                                                                                                        if let Some(measured_at) = entry.measured_at {
                                                                                                            "{i18n.format_datetime(measured_at)}"
                                                                                                        } else {
                                                                                                            "-"
                                                                                                        }
                                                                                                    }
                                                                                                }
                                                                                            }
                                                                                            if let Some(ref notes) = entry.notes {
                                                                                                if !notes.is_empty() {
                                                                                                    div { class: "mt-2 text-gray-600 italic",
                                                                                                        span { class: "text-gray-500", "{i18n.t(Key::Notes)}: " }
                                                                                                        "{notes}"
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

                                                                // No data message
                                                                if data.measurements.is_empty() && data.custom_entries.is_empty() {
                                                                    p { class: "text-sm text-gray-500 italic",
                                                                        "{i18n.t(Key::NoMeasurementsFound)}"
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                } else if is_loading_data {
                                                    tr { class: "bg-gray-50",
                                                        td { colspan: "12", class: "px-6 py-4 text-center text-gray-500",
                                                            "{i18n.t(Key::LoadingMeasurements)}"
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
