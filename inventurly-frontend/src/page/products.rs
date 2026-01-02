use crate::auth::RequirePrivilege;
use crate::component::{BarcodeScanner, ProductList, ScanResult, TopBar};
use crate::i18n::{use_i18n, Key};
use crate::page::AccessDeniedPage;
use crate::router::Route;
use crate::service::product::PRODUCTS;
use dioxus::prelude::*;
use dioxus_router::prelude::use_navigator;

#[component]
pub fn Products() -> Element {
    let i18n = use_i18n();
    let navigator = use_navigator();

    // Local state for the filter input
    let mut filter_input = use_signal(|| String::new());

    // Barcode scanner state
    let mut show_scanner = use_signal(|| false);
    let mut scanner_message = use_signal(|| None::<String>);

    // Advanced filters state
    let mut filters_expanded = use_signal(|| false);
    let mut price_min_input = use_signal(|| String::new());
    let mut price_max_input = use_signal(|| String::new());

    // Debounced filter update effect
    use_effect(move || {
        let query = filter_input();
        spawn(async move {
            // Debounce: wait 500ms before updating the filter
            gloo_timers::future::TimeoutFuture::new(500).await;
            // Only update if the input hasn't changed
            if query == filter_input() {
                PRODUCTS.write().filter_query = query;
            }
        });
    });

    // Function to clear the filter
    let clear_filter = move |_| {
        filter_input.set(String::new());
        PRODUCTS.write().filter_query = String::new();
    };

    // Handle barcode scan
    let i18n_clone = i18n.clone();
    let handle_barcode_scan = move |result: ScanResult| {
        // Strip checksum digit (scanner returns EAN with checksum, DB stores without)
        let ean = if result.barcode.len() > 1 {
            &result.barcode[..result.barcode.len() - 1]
        } else {
            &result.barcode
        };

        // Set filter input and update global filter immediately (no debounce for scans)
        filter_input.set(ean.to_string());
        PRODUCTS.write().filter_query = ean.to_string();

        // Show feedback message
        scanner_message.set(Some(format!("{}: {}", i18n_clone.t(Key::SearchingForEAN), ean)));

        // Close scanner
        show_scanner.set(false);
    };

    // Extract distinct sales units from filtered products (excluding sales_units filter)
    let products_read = PRODUCTS.read();
    let mut distinct_sales_units: Vec<String> = products_read.items.iter()
        .filter(|p| p.deleted.is_none()) // Hide deleted products
        .filter(|p| {
            // Apply text search filter
            if !products_read.filter_query.is_empty() {
                let query = products_read.filter_query.to_lowercase();
                let matches = p.name.to_lowercase().contains(&query)
                    || p.ean.to_lowercase().contains(&query)
                    || p.short_name.to_lowercase().contains(&query);
                if !matches {
                    return false;
                }
            }

            // Apply requires weighing filter
            if let Some(requires_weighing) = products_read.filter_requires_weighing {
                if p.requires_weighing != requires_weighing {
                    return false;
                }
            }

            // Apply price range filter
            let price_cents = p.price.to_cents();
            if let Some(min_price) = products_read.filter_price_min {
                if price_cents < min_price {
                    return false;
                }
            }
            if let Some(max_price) = products_read.filter_price_max {
                if price_cents > max_price {
                    return false;
                }
            }

            // Apply rack assignment filter
            if let Some(assigned) = products_read.filter_rack_assignment {
                let is_assigned = p.rack_count.unwrap_or(0) > 0;
                if is_assigned != assigned {
                    return false;
                }
            }

            true
        })
        .map(|p| p.sales_unit.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    distinct_sales_units.sort();

    // Handler to toggle sales unit filter
    let toggle_sales_unit = move |unit: String| {
        let mut products = PRODUCTS.write();
        if products.filter_sales_units.contains(&unit) {
            products.filter_sales_units.retain(|u| u != &unit);
        } else {
            products.filter_sales_units.push(unit);
        }
    };

    // Handler to toggle select all / deselect all sales units
    let all_units_selected = products_read.filter_sales_units.len() == distinct_sales_units.len()
        && !distinct_sales_units.is_empty();

    let distinct_sales_units_for_toggle = distinct_sales_units.clone();
    let toggle_all_sales_units = move |_| {
        let mut products = PRODUCTS.write();
        if products.filter_sales_units.len() == distinct_sales_units_for_toggle.len() {
            // Deselect all
            products.filter_sales_units.clear();
        } else {
            // Select all
            products.filter_sales_units = distinct_sales_units_for_toggle.clone();
        }
    };

    // Handler to update requires weighing filter
    let set_requires_weighing = move |value: Option<bool>| {
        PRODUCTS.write().filter_requires_weighing = value;
    };

    // Handler to update rack assignment filter
    let set_rack_assignment = move |value: Option<bool>| {
        PRODUCTS.write().filter_rack_assignment = value;
    };

    // Handler to update price min filter
    let update_price_min = move |evt: Event<FormData>| {
        let value = evt.value();
        price_min_input.set(value.clone());

        if value.is_empty() {
            PRODUCTS.write().filter_price_min = None;
        } else if let Ok(euros) = value.parse::<f64>() {
            PRODUCTS.write().filter_price_min = Some((euros * 100.0) as i64);
        }
    };

    // Handler to update price max filter
    let update_price_max = move |evt: Event<FormData>| {
        let value = evt.value();
        price_max_input.set(value.clone());

        if value.is_empty() {
            PRODUCTS.write().filter_price_max = None;
        } else if let Ok(euros) = value.parse::<f64>() {
            PRODUCTS.write().filter_price_max = Some((euros * 100.0) as i64);
        }
    };

    // Handler to clear all filters
    let clear_all_filters = move |_| {
        filter_input.set(String::new());
        price_min_input.set(String::new());
        price_max_input.set(String::new());

        let mut products = PRODUCTS.write();
        products.filter_query = String::new();
        products.filter_sales_units.clear();
        products.filter_requires_weighing = None;
        products.filter_price_min = None;
        products.filter_price_max = None;
        products.filter_rack_assignment = None;
    };

    rsx! {
        RequirePrivilege {
            privilege: "view_inventory",
            fallback: rsx! { AccessDeniedPage { required_privilege: "view_inventory".to_string() } },
            div { class: "flex flex-col min-h-screen",
                TopBar {}
                div { class: "flex-1 container mx-auto px-4 py-8",
                    div { class: "flex justify-between items-center mb-6",
                        h1 { class: "text-3xl font-bold",
                            {i18n.t(Key::Products)}
                        }
                        div { class: "flex space-x-3",
                            button {
                                class: "px-4 py-2 bg-orange-600 text-white rounded-md hover:bg-orange-700 text-sm font-medium",
                                onclick: move |_| {
                                    navigator.push(Route::DuplicateDetection {});
                                },
                                {i18n.t(Key::CheckDuplicates)}
                            }
                        }
                    }

                    // Filter input bar
                    div { class: "mb-4",
                        // Scanner feedback message
                        if let Some(msg) = scanner_message() {
                            div { class: "mb-2 px-4 py-2 bg-blue-50 text-blue-700 rounded-md text-sm",
                                {msg}
                            }
                        }

                        div { class: "flex flex-col sm:flex-row gap-2",
                            div { class: "relative flex-1",
                                input {
                                    r#type: "text",
                                    class: "w-full px-4 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500 focus:border-transparent",
                                    placeholder: "{i18n.t(Key::FilterProducts)}",
                                    value: "{filter_input}",
                                    oninput: move |evt| {
                                        filter_input.set(evt.value().clone());
                                        scanner_message.set(None); // Clear message when typing
                                    },
                                }
                                if !filter_input().is_empty() {
                                    button {
                                        class: "absolute right-2 top-1/2 -translate-y-1/2 px-3 py-1 text-gray-500 hover:text-gray-700",
                                        onclick: clear_filter,
                                        title: "{i18n.t(Key::ClearFilter)}",
                                        "✕"
                                    }
                                }
                            }
                            button {
                                class: "w-full sm:w-auto px-4 py-2 bg-gray-600 text-white rounded-md hover:bg-gray-700 text-sm font-medium flex items-center gap-2 justify-center",
                                onclick: move |_| {
                                    scanner_message.set(None);
                                    show_scanner.set(true);
                                },
                                "📷 {i18n.t(Key::ScanBarcode)}"
                            }
                            button {
                                class: "w-full sm:w-auto px-4 py-2 bg-gray-100 text-gray-700 rounded-md hover:bg-gray-200 text-sm font-medium",
                                onclick: move |_| filters_expanded.set(!filters_expanded()),
                                if filters_expanded() {
                                    "▲ {i18n.t(Key::HideFilters)}"
                                } else {
                                    "▼ {i18n.t(Key::ShowFilters)}"
                                }
                            }
                        }

                        // Expandable filter panel
                        if filters_expanded() {
                            div { class: "mt-4 p-4 bg-gray-50 rounded-md border border-gray-200",
                                div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4",
                                    // Sales Unit filter
                                    div {
                                        label { class: "block text-sm font-medium text-gray-700 mb-2",
                                            {i18n.t(Key::ProductSalesUnit)}
                                        }
                                        button {
                                            class: "mb-2 text-xs text-blue-600 hover:text-blue-800 underline",
                                            onclick: toggle_all_sales_units,
                                            if all_units_selected {
                                                {i18n.t(Key::DeselectAll)}
                                            } else {
                                                {i18n.t(Key::SelectAll)}
                                            }
                                        }
                                        div { class: "space-y-2 max-h-40 overflow-y-auto border border-gray-200 rounded p-2 bg-white",
                                            for unit in distinct_sales_units.iter() {
                                                {
                                                    let unit_clone = unit.clone();
                                                    let is_selected = products_read.filter_sales_units.contains(unit);
                                                    rsx! {
                                                        label { class: "flex items-center space-x-2 cursor-pointer hover:bg-gray-50 p-1 rounded",
                                                            input {
                                                                r#type: "checkbox",
                                                                class: "rounded border-gray-300",
                                                                checked: is_selected,
                                                                onchange: move |_| toggle_sales_unit(unit_clone.clone()),
                                                            }
                                                            span { class: "text-sm", {unit.clone()} }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    // Requires Weighing filter
                                    div {
                                        label { class: "block text-sm font-medium text-gray-700 mb-2",
                                            {i18n.t(Key::ProductRequiresWeighing)}
                                        }
                                        div { class: "space-y-2",
                                            label { class: "flex items-center space-x-2 cursor-pointer",
                                                input {
                                                    r#type: "radio",
                                                    name: "requires_weighing",
                                                    checked: products_read.filter_requires_weighing.is_none(),
                                                    onchange: move |_| set_requires_weighing(None),
                                                }
                                                span { class: "text-sm", {i18n.t(Key::Both)} }
                                            }
                                            label { class: "flex items-center space-x-2 cursor-pointer",
                                                input {
                                                    r#type: "radio",
                                                    name: "requires_weighing",
                                                    checked: products_read.filter_requires_weighing == Some(true),
                                                    onchange: move |_| set_requires_weighing(Some(true)),
                                                }
                                                span { class: "text-sm", {i18n.t(Key::Yes)} }
                                            }
                                            label { class: "flex items-center space-x-2 cursor-pointer",
                                                input {
                                                    r#type: "radio",
                                                    name: "requires_weighing",
                                                    checked: products_read.filter_requires_weighing == Some(false),
                                                    onchange: move |_| set_requires_weighing(Some(false)),
                                                }
                                                span { class: "text-sm", {i18n.t(Key::No)} }
                                            }
                                        }
                                    }

                                    // Rack Assignment filter
                                    div {
                                        label { class: "block text-sm font-medium text-gray-700 mb-2",
                                            {i18n.t(Key::RackAssignment)}
                                        }
                                        div { class: "space-y-2",
                                            label { class: "flex items-center space-x-2 cursor-pointer",
                                                input {
                                                    r#type: "radio",
                                                    name: "rack_assignment",
                                                    checked: products_read.filter_rack_assignment.is_none(),
                                                    onchange: move |_| set_rack_assignment(None),
                                                }
                                                span { class: "text-sm", {i18n.t(Key::Both)} }
                                            }
                                            label { class: "flex items-center space-x-2 cursor-pointer",
                                                input {
                                                    r#type: "radio",
                                                    name: "rack_assignment",
                                                    checked: products_read.filter_rack_assignment == Some(true),
                                                    onchange: move |_| set_rack_assignment(Some(true)),
                                                }
                                                span { class: "text-sm", {i18n.t(Key::Assigned)} }
                                            }
                                            label { class: "flex items-center space-x-2 cursor-pointer",
                                                input {
                                                    r#type: "radio",
                                                    name: "rack_assignment",
                                                    checked: products_read.filter_rack_assignment == Some(false),
                                                    onchange: move |_| set_rack_assignment(Some(false)),
                                                }
                                                span { class: "text-sm", {i18n.t(Key::Unassigned)} }
                                            }
                                        }
                                    }

                                    // Price Range filter
                                    div {
                                        label { class: "block text-sm font-medium text-gray-700 mb-2",
                                            {i18n.t(Key::ProductPrice)}
                                        }
                                        div { class: "space-y-2",
                                            div {
                                                label { class: "block text-xs text-gray-600 mb-1",
                                                    {i18n.t(Key::MinimumPrice)}
                                                }
                                                input {
                                                    r#type: "number",
                                                    class: "w-full px-3 py-2 border border-gray-300 rounded-md text-sm",
                                                    placeholder: "0.00",
                                                    step: "0.01",
                                                    value: "{price_min_input}",
                                                    oninput: update_price_min,
                                                }
                                            }
                                            div {
                                                label { class: "block text-xs text-gray-600 mb-1",
                                                    {i18n.t(Key::MaximumPrice)}
                                                }
                                                input {
                                                    r#type: "number",
                                                    class: "w-full px-3 py-2 border border-gray-300 rounded-md text-sm",
                                                    placeholder: "999.99",
                                                    step: "0.01",
                                                    value: "{price_max_input}",
                                                    oninput: update_price_max,
                                                }
                                            }
                                        }
                                    }
                                }

                                // Clear all filters button
                                div { class: "mt-4 flex justify-end",
                                    button {
                                        class: "px-4 py-2 bg-red-600 text-white rounded-md hover:bg-red-700 text-sm font-medium",
                                        onclick: clear_all_filters,
                                        {i18n.t(Key::ClearAllFilters)}
                                    }
                                }
                            }
                        }
                    }

                    ProductList {}
                }
            }

            // Barcode scanner modal
            if show_scanner() {
                BarcodeScanner {
                    on_scan: handle_barcode_scan,
                    on_close: move |_| show_scanner.set(false)
                }
            }
        }
    }
}
