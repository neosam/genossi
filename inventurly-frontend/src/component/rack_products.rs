use crate::api;
use crate::component::{modal::Modal, ProductRackForm};
use crate::i18n::{use_i18n, Key};
use crate::service::config::CONFIG;
use crate::service::product_rack::{get_products_in_rack_action, reorder_products_in_rack_action, remove_product_from_rack_action, set_product_position_action};
use dioxus::prelude::*;
use rest_types::{ProductRackTO, ProductTO};
use std::collections::HashSet;
use uuid::Uuid;

#[component]
pub fn RackProducts(rack_id: Uuid) -> Element {
    let i18n = use_i18n();

    let products_in_rack = use_signal(|| Vec::<ProductRackTO>::new());
    let products_map = use_signal(|| std::collections::HashMap::<Uuid, ProductTO>::new());
    let loading = use_signal(|| false);
    let error = use_signal(|| None::<String>);
    let mut show_add_form = use_signal(|| false);
    let mut selected_products = use_signal(|| HashSet::<Uuid>::new());

    // Load products in rack
    let load_products = use_callback({
        let products_in_rack = products_in_rack.clone();
        let products_map = products_map.clone();
        let loading = loading.clone();
        let error = error.clone();

        move |_| {
            spawn({
                let mut products_in_rack = products_in_rack.clone();
                let mut products_map = products_map.clone();
                let mut loading = loading.clone();
                let mut error = error.clone();

                async move {
                    loading.set(true);
                    error.set(None);

                    // Load products in rack and all products
                    let config = CONFIG.read().clone();
                    let (rack_products_result, all_products_result) = futures_util::join!(
                        get_products_in_rack_action(rack_id),
                        api::get_products(&config)
                    );

                    match rack_products_result {
                        Ok(rack_products) => {
                            products_in_rack.set(rack_products);
                        }
                        Err(e) => {
                            error.set(Some(format!("Failed to load products in rack: {}", e)));
                            loading.set(false);
                            return;
                        }
                    }

                    match all_products_result {
                        Ok(all_products) => {
                            let mut map = std::collections::HashMap::new();
                            for product in all_products {
                                if let Some(id) = product.id {
                                    map.insert(id, product);
                                }
                            }
                            products_map.set(map);
                        }
                        Err(e) => {
                            error.set(Some(format!("Failed to load products: {}", e)));
                        }
                    }

                    loading.set(false);
                }
            });
        }
    });

    // Load products on mount
    use_effect(move || {
        load_products.call(());
    });

    let handle_remove_product = move |product_id: Uuid| {
        spawn({
            let mut error = error.clone();
            let load_products = load_products.clone();

            async move {
                error.set(None);

                match remove_product_from_rack_action(product_id, rack_id).await {
                    Ok(()) => {
                        load_products.call(());
                    }
                    Err(e) => {
                        error.set(Some(e));
                    }
                }
            }
        });
    };

    let handle_product_saved = move |_| {
        let mut show_add_form = show_add_form.clone();
        show_add_form.set(false);
        load_products.call(());
    };

    let handle_move_up = move |product_id: Uuid, current_position: i32| {
        spawn({
            let mut error = error.clone();
            let load_products = load_products.clone();

            async move {
                error.set(None);

                match set_product_position_action(product_id, rack_id, current_position - 1).await {
                    Ok(_) => {
                        load_products.call(());
                    }
                    Err(e) => {
                        error.set(Some(e));
                    }
                }
            }
        });
    };

    let handle_move_down = move |product_id: Uuid, current_position: i32| {
        spawn({
            let mut error = error.clone();
            let load_products = load_products.clone();

            async move {
                error.set(None);

                match set_product_position_action(product_id, rack_id, current_position + 1).await {
                    Ok(_) => {
                        load_products.call(());
                    }
                    Err(e) => {
                        error.set(Some(e));
                    }
                }
            }
        });
    };

    // Handler to toggle product selection
    let mut handle_toggle_selection = move |product_id: Uuid| {
        let mut selection = selected_products.write();
        if selection.contains(&product_id) {
            selection.remove(&product_id);
        } else {
            selection.insert(product_id);
        }
    };

    // Handler for bulk move above
    let handle_move_selected_above = move |target_idx: usize| {
        spawn({
            let mut error = error.clone();
            let mut selected_products = selected_products.clone();
            let load_products = load_products.clone();
            let products = products_in_rack.read().clone();
            let selected = selected_products.read().clone();

            async move {
                error.set(None);

                // Build new order: non-selected items up to target, then selected items (in their original order), then target and rest
                let mut new_order: Vec<Uuid> = Vec::new();
                let mut selected_in_order: Vec<Uuid> = Vec::new();

                // Collect selected items in their original order
                for pr in products.iter() {
                    if selected.contains(&pr.product_id) {
                        selected_in_order.push(pr.product_id);
                    }
                }

                // Build the new order
                for (idx, pr) in products.iter().enumerate() {
                    if idx == target_idx {
                        // Insert selected items before the target
                        new_order.extend(selected_in_order.iter().cloned());
                    }
                    if !selected.contains(&pr.product_id) {
                        new_order.push(pr.product_id);
                    }
                }

                match reorder_products_in_rack_action(rack_id, new_order).await {
                    Ok(_) => {
                        selected_products.set(HashSet::new());
                        load_products.call(());
                    }
                    Err(e) => {
                        error.set(Some(e));
                    }
                }
            }
        });
    };

    // Handler for bulk move below
    let handle_move_selected_below = move |target_idx: usize| {
        spawn({
            let mut error = error.clone();
            let mut selected_products = selected_products.clone();
            let load_products = load_products.clone();
            let products = products_in_rack.read().clone();
            let selected = selected_products.read().clone();

            async move {
                error.set(None);

                // Build new order: non-selected items up to and including target, then selected items, then rest
                let mut new_order: Vec<Uuid> = Vec::new();
                let mut selected_in_order: Vec<Uuid> = Vec::new();

                // Collect selected items in their original order
                for pr in products.iter() {
                    if selected.contains(&pr.product_id) {
                        selected_in_order.push(pr.product_id);
                    }
                }

                // Build the new order
                for (idx, pr) in products.iter().enumerate() {
                    if !selected.contains(&pr.product_id) {
                        new_order.push(pr.product_id);
                    }
                    if idx == target_idx {
                        // Insert selected items after the target
                        new_order.extend(selected_in_order.iter().cloned());
                    }
                }

                match reorder_products_in_rack_action(rack_id, new_order).await {
                    Ok(_) => {
                        selected_products.set(HashSet::new());
                        load_products.call(());
                    }
                    Err(e) => {
                        error.set(Some(e));
                    }
                }
            }
        });
    };

    rsx! {
        div { class: "space-y-4",
            div { class: "flex justify-between items-center",
                h3 { class: "text-lg font-semibold",
                    {i18n.t(Key::ProductsInRack)}
                }
                button {
                    class: "px-4 py-2 bg-blue-500 text-white rounded-md hover:bg-blue-600",
                    onclick: move |_| show_add_form.set(true),
                    {i18n.t(Key::AddProductToRack)}
                }
            }

            if let Some(error_msg) = error() {
                div { class: "bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded",
                    {error_msg}
                }
            }

            if loading() {
                div { class: "text-center py-4",
                    {i18n.t(Key::Loading)}
                }
            } else if products_in_rack().is_empty() {
                div { class: "text-center py-8 text-gray-500",
                    {i18n.t(Key::NoDataFound)}
                }
            } else {
                div { class: "overflow-x-auto",
                    table { class: "min-w-full bg-white border border-gray-200",
                        thead { class: "bg-gray-50",
                            tr {
                                th { class: "px-3 py-3 text-center text-xs font-medium text-gray-500 uppercase tracking-wider border-b w-10",
                                    // Empty header for checkbox column
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider border-b",
                                    {i18n.t(Key::ProductName)}
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider border-b",
                                    {i18n.t(Key::ProductEan)}
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider border-b",
                                    {i18n.t(Key::Order)}
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider border-b",
                                    {i18n.t(Key::Actions)}
                                }
                            }
                        }
                        tbody { class: "bg-white divide-y divide-gray-200",
                            {
                                let products = products_in_rack();
                                let total_count = products.len();
                                let selection = selected_products();
                                let has_selection = !selection.is_empty();
                                rsx! {
                                    for (idx, product_rack) in products.iter().enumerate() {
                                        if let Some(product) = products_map().get(&product_rack.product_id) {
                                            {
                                                let is_selected = selection.contains(&product_rack.product_id);
                                                let row_class = if is_selected {
                                                    "bg-blue-50 hover:bg-blue-100"
                                                } else {
                                                    "hover:bg-gray-50"
                                                };
                                                rsx! {
                                                    tr {
                                                        key: "{idx}",
                                                        class: "{row_class}",
                                                        td { class: "px-3 py-4 text-center",
                                                            input {
                                                                r#type: "checkbox",
                                                                class: "h-4 w-4 text-blue-600 rounded border-gray-300 focus:ring-blue-500",
                                                                checked: is_selected,
                                                                onchange: {
                                                                    let product_id = product_rack.product_id;
                                                                    move |_| handle_toggle_selection(product_id)
                                                                },
                                                            }
                                                        }
                                                        td { class: "px-6 py-4 whitespace-nowrap text-sm text-gray-900",
                                                            {product.name.clone()}
                                                        }
                                                        td { class: "px-6 py-4 whitespace-nowrap text-sm text-gray-500",
                                                            {product.ean.clone()}
                                                        }
                                                        td { class: "px-6 py-4 whitespace-nowrap text-sm font-medium space-x-1",
                                                            button {
                                                                class: if idx == 0 {
                                                                    "px-2 py-1 text-gray-300 cursor-not-allowed"
                                                                } else {
                                                                    "px-2 py-1 text-gray-600 hover:text-blue-600"
                                                                },
                                                                disabled: idx == 0,
                                                                title: i18n.t(Key::MoveUp).to_string(),
                                                                onclick: {
                                                                    let product_id = product_rack.product_id;
                                                                    let current_position = product_rack.sort_order.unwrap_or(idx as i32 + 1);
                                                                    move |_| handle_move_up(product_id, current_position)
                                                                },
                                                                "↑"
                                                            }
                                                            button {
                                                                class: if idx == total_count - 1 {
                                                                    "px-2 py-1 text-gray-300 cursor-not-allowed"
                                                                } else {
                                                                    "px-2 py-1 text-gray-600 hover:text-blue-600"
                                                                },
                                                                disabled: idx == total_count - 1,
                                                                title: i18n.t(Key::MoveDown).to_string(),
                                                                onclick: {
                                                                    let product_id = product_rack.product_id;
                                                                    let current_position = product_rack.sort_order.unwrap_or(idx as i32 + 1);
                                                                    move |_| handle_move_down(product_id, current_position)
                                                                },
                                                                "↓"
                                                            }
                                                            // Show move here buttons only for unselected rows when there's a selection
                                                            if has_selection && !is_selected {
                                                                button {
                                                                    class: "px-2 py-1 text-green-600 hover:text-green-800",
                                                                    title: i18n.t(Key::MoveAbove).to_string(),
                                                                    onclick: move |_| handle_move_selected_above(idx),
                                                                    "▲"
                                                                }
                                                                button {
                                                                    class: "px-2 py-1 text-green-600 hover:text-green-800",
                                                                    title: i18n.t(Key::MoveBelow).to_string(),
                                                                    onclick: move |_| handle_move_selected_below(idx),
                                                                    "▼"
                                                                }
                                                            }
                                                        }
                                                        td { class: "px-6 py-4 whitespace-nowrap text-sm font-medium space-x-2",
                                                            button {
                                                                class: "text-red-600 hover:text-red-900",
                                                                onclick: {
                                                                    let product_id = product_rack.product_id;
                                                                    move |_| handle_remove_product(product_id)
                                                                },
                                                                {i18n.t(Key::RemoveProductFromRack)}
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

            // Add Product Modal
            if show_add_form() {
                Modal {
                    ProductRackForm {
                        product_id: None,
                        rack_id: Some(rack_id),
                        on_saved: handle_product_saved,
                        on_cancel: move |_| {
                            let mut show_add_form = show_add_form.clone();
                            show_add_form.set(false);
                        },
                    }
                }
            }

        }
    }
}
