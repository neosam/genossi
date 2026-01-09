use crate::api;
use crate::component::container_rack_form::ContainerRackForm;
use crate::component::modal::Modal;
use crate::i18n::{use_i18n, Key};
use crate::service::config::CONFIG;
use crate::service::container_rack::{
    get_containers_in_rack_action, reorder_containers_in_rack_action,
    remove_container_from_rack_action, set_container_position_action,
};
use dioxus::prelude::*;
use rest_types::{ContainerRackTO, ContainerTO};
use std::collections::HashSet;
use uuid::Uuid;

#[component]
pub fn RackContainers(rack_id: Uuid) -> Element {
    let i18n = use_i18n();

    let containers_in_rack = use_signal(|| Vec::<ContainerRackTO>::new());
    let containers_map = use_signal(|| std::collections::HashMap::<Uuid, ContainerTO>::new());
    let loading = use_signal(|| false);
    let error = use_signal(|| None::<String>);
    let mut show_add_form = use_signal(|| false);
    let mut selected_containers = use_signal(|| HashSet::<Uuid>::new());

    // Load containers in rack
    let load_containers = use_callback({
        let containers_in_rack = containers_in_rack.clone();
        let containers_map = containers_map.clone();
        let loading = loading.clone();
        let error = error.clone();

        move |_| {
            spawn({
                let mut containers_in_rack = containers_in_rack.clone();
                let mut containers_map = containers_map.clone();
                let mut loading = loading.clone();
                let mut error = error.clone();

                async move {
                    loading.set(true);
                    error.set(None);

                    // Load containers in rack and all containers
                    let config = CONFIG.read().clone();
                    let (rack_containers_result, all_containers_result) = futures_util::join!(
                        get_containers_in_rack_action(rack_id),
                        api::get_containers(&config)
                    );

                    match rack_containers_result {
                        Ok(rack_containers) => {
                            containers_in_rack.set(rack_containers);
                        }
                        Err(e) => {
                            error.set(Some(format!("Failed to load containers in rack: {}", e)));
                            loading.set(false);
                            return;
                        }
                    }

                    match all_containers_result {
                        Ok(all_containers) => {
                            let mut map = std::collections::HashMap::new();
                            for container in all_containers {
                                if let Some(id) = container.id {
                                    map.insert(id, container);
                                }
                            }
                            containers_map.set(map);
                        }
                        Err(e) => {
                            error.set(Some(format!("Failed to load containers: {}", e)));
                        }
                    }

                    loading.set(false);
                }
            });
        }
    });

    // Load containers on mount
    use_effect(move || {
        load_containers.call(());
    });

    let handle_remove_container = move |container_id: Uuid| {
        spawn({
            let mut error = error.clone();
            let load_containers = load_containers.clone();

            async move {
                error.set(None);

                match remove_container_from_rack_action(container_id, rack_id).await {
                    Ok(()) => {
                        load_containers.call(());
                    }
                    Err(e) => {
                        error.set(Some(e));
                    }
                }
            }
        });
    };

    let handle_container_saved = move |_| {
        let mut show_add_form = show_add_form.clone();
        show_add_form.set(false);
        load_containers.call(());
    };

    let handle_move_up = move |container_id: Uuid, current_position: i32| {
        spawn({
            let mut error = error.clone();
            let load_containers = load_containers.clone();

            async move {
                error.set(None);

                match set_container_position_action(container_id, rack_id, current_position - 1)
                    .await
                {
                    Ok(_) => {
                        load_containers.call(());
                    }
                    Err(e) => {
                        error.set(Some(e));
                    }
                }
            }
        });
    };

    let handle_move_down = move |container_id: Uuid, current_position: i32| {
        spawn({
            let mut error = error.clone();
            let load_containers = load_containers.clone();

            async move {
                error.set(None);

                match set_container_position_action(container_id, rack_id, current_position + 1)
                    .await
                {
                    Ok(_) => {
                        load_containers.call(());
                    }
                    Err(e) => {
                        error.set(Some(e));
                    }
                }
            }
        });
    };

    // Handler to toggle container selection
    let mut handle_toggle_selection = move |container_id: Uuid| {
        let mut selection = selected_containers.write();
        if selection.contains(&container_id) {
            selection.remove(&container_id);
        } else {
            selection.insert(container_id);
        }
    };

    // Handler for bulk move above
    let handle_move_selected_above = move |target_idx: usize| {
        spawn({
            let mut error = error.clone();
            let mut selected_containers = selected_containers.clone();
            let load_containers = load_containers.clone();
            let containers = containers_in_rack.read().clone();
            let selected = selected_containers.read().clone();

            async move {
                error.set(None);

                // Build new order: non-selected items up to target, then selected items (in their original order), then target and rest
                let mut new_order: Vec<Uuid> = Vec::new();
                let mut selected_in_order: Vec<Uuid> = Vec::new();

                // Collect selected items in their original order
                for cr in containers.iter() {
                    if selected.contains(&cr.container_id) {
                        selected_in_order.push(cr.container_id);
                    }
                }

                // Build the new order
                for (idx, cr) in containers.iter().enumerate() {
                    if idx == target_idx {
                        // Insert selected items before the target
                        new_order.extend(selected_in_order.iter().cloned());
                    }
                    if !selected.contains(&cr.container_id) {
                        new_order.push(cr.container_id);
                    }
                }

                match reorder_containers_in_rack_action(rack_id, new_order).await {
                    Ok(_) => {
                        selected_containers.set(HashSet::new());
                        load_containers.call(());
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
            let mut selected_containers = selected_containers.clone();
            let load_containers = load_containers.clone();
            let containers = containers_in_rack.read().clone();
            let selected = selected_containers.read().clone();

            async move {
                error.set(None);

                // Build new order: non-selected items up to and including target, then selected items, then rest
                let mut new_order: Vec<Uuid> = Vec::new();
                let mut selected_in_order: Vec<Uuid> = Vec::new();

                // Collect selected items in their original order
                for cr in containers.iter() {
                    if selected.contains(&cr.container_id) {
                        selected_in_order.push(cr.container_id);
                    }
                }

                // Build the new order
                for (idx, cr) in containers.iter().enumerate() {
                    if !selected.contains(&cr.container_id) {
                        new_order.push(cr.container_id);
                    }
                    if idx == target_idx {
                        // Insert selected items after the target
                        new_order.extend(selected_in_order.iter().cloned());
                    }
                }

                match reorder_containers_in_rack_action(rack_id, new_order).await {
                    Ok(_) => {
                        selected_containers.set(HashSet::new());
                        load_containers.call(());
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
                    {i18n.t(Key::ContainersInRack)}
                }
                button {
                    class: "px-4 py-2 bg-blue-500 text-white rounded-md hover:bg-blue-600",
                    onclick: move |_| show_add_form.set(true),
                    {i18n.t(Key::AddContainerToRack)}
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
            } else if containers_in_rack().is_empty() {
                div { class: "text-center py-8 text-gray-500",
                    {i18n.t(Key::NoContainersInRack)}
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
                                    {i18n.t(Key::ContainerName)}
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider border-b",
                                    {i18n.t(Key::ContainerWeightGrams)}
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
                                let containers = containers_in_rack();
                                let total_count = containers.len();
                                let selection = selected_containers();
                                let has_selection = !selection.is_empty();
                                rsx! {
                                    for (idx, container_rack) in containers.iter().enumerate() {
                                        if let Some(container) = containers_map().get(&container_rack.container_id) {
                                            {
                                                let is_selected = selection.contains(&container_rack.container_id);
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
                                                                    let container_id = container_rack.container_id;
                                                                    move |_| handle_toggle_selection(container_id)
                                                                },
                                                            }
                                                        }
                                                        td { class: "px-6 py-4 whitespace-nowrap text-sm text-gray-900",
                                                            {container.name.clone()}
                                                        }
                                                        td { class: "px-6 py-4 whitespace-nowrap text-sm text-gray-500",
                                                            "{container.weight_grams}g"
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
                                                                    let container_id = container_rack.container_id;
                                                                    let current_position = container_rack.sort_order.unwrap_or(idx as i32 + 1);
                                                                    move |_| handle_move_up(container_id, current_position)
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
                                                                    let container_id = container_rack.container_id;
                                                                    let current_position = container_rack.sort_order.unwrap_or(idx as i32 + 1);
                                                                    move |_| handle_move_down(container_id, current_position)
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
                                                                    let container_id = container_rack.container_id;
                                                                    move |_| handle_remove_container(container_id)
                                                                },
                                                                {i18n.t(Key::RemoveContainerFromRack)}
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

            // Add Container Modal
            if show_add_form() {
                Modal {
                    ContainerRackForm {
                        container_id: None,
                        rack_id: Some(rack_id),
                        on_saved: handle_container_saved,
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
