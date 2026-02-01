use crate::api;
use crate::component::TopBar;
use crate::i18n::{use_i18n, Key};
use crate::router::Route;
use crate::service::config::CONFIG;
use dioxus::prelude::*;
use rest_types::InventurProductReportItemTO;
use std::collections::HashSet;
use uuid::Uuid;

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

    // Filter state
    let mut filter_input = use_signal(String::new);
    let mut filter_query = use_signal(String::new);
    let mut filter_racks = use_signal::<Vec<String>>(Vec::new);
    let mut filter_has_count = use_signal::<Option<bool>>(|| None);
    let mut filter_has_weight = use_signal::<Option<bool>>(|| None);
    let mut filters_expanded = use_signal(|| false);

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

            match api::get_inventur_product_report(&config, inventur_id).await {
                Ok(report) => {
                    results.set(report);
                }
                Err(e) => {
                    error.set(Some(format!("Failed to load results: {}", e)));
                }
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

    // Get distinct rack names from results
    let all_results = results.read();
    let distinct_racks: Vec<String> = all_results
        .iter()
        .flat_map(|r| r.racks_measured.iter().cloned())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .into_iter()
        .collect();
    let mut sorted_racks = distinct_racks.clone();
    sorted_racks.sort();

    // Apply filters
    let query = filter_query.read().to_lowercase();
    let selected_racks = filter_racks.read();
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

            // Rack filter
            if !selected_racks.is_empty() {
                let has_matching_rack = r.racks_measured.iter().any(|rack| selected_racks.contains(rack));
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
    let mut toggle_rack = move |rack: String| {
        let mut racks = filter_racks.write();
        if racks.contains(&rack) {
            racks.retain(|r| *r != rack);
        } else {
            racks.push(rack);
        }
    };

    let clear_all_filters = move |_| {
        filter_input.set(String::new());
        filter_query.set(String::new());
        filter_racks.write().clear();
        filter_has_count.set(None);
        filter_has_weight.set(None);
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
                                                let rack_clone = rack.clone();
                                                let rack_for_check = rack.clone();
                                                rsx! {
                                                    label { class: "flex items-center gap-2 cursor-pointer hover:bg-gray-100 p-1 rounded",
                                                        input {
                                                            r#type: "checkbox",
                                                            checked: selected_racks.contains(&rack_for_check),
                                                            onchange: move |_| toggle_rack(rack_clone.clone()),
                                                        }
                                                        span { "{rack}" }
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
                div { class: "bg-white rounded-lg shadow",
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
                        div { class: "overflow-x-auto",
                            table { class: "w-full",
                                thead {
                                    tr { class: "border-b bg-gray-50",
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
                                    }
                                }
                                tbody {
                                    for item in filtered_results.iter() {
                                        tr { class: "border-b hover:bg-gray-50",
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
                                            td { class: "px-6 py-4 text-sm text-gray-500",
                                                "{item.racks_measured.join(\", \")}"
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
