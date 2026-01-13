use crate::api;
use crate::auth::RequirePrivilege;
use crate::component::{CustomEntryForm, CustomEntryManagementList, TopBar};
use crate::i18n::{use_i18n, Key};
use crate::page::AccessDeniedPage;
use crate::service::config::CONFIG;
use crate::service::custom_entry::CUSTOM_ENTRIES;
use crate::service::rack::RACKS;
use dioxus::prelude::*;
use rest_types::{ContainerTO, InventurCustomEntryTO, InventurTO};
use std::collections::HashSet;
use uuid::Uuid;

#[component]
pub fn CustomEntries(id: String) -> Element {
    let i18n = use_i18n();

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
    let mut inventur = use_signal(|| None::<InventurTO>);
    let mut filter_input = use_signal(|| String::new());
    let mut filters_expanded = use_signal(|| false);
    let mut editing_entry = use_signal(|| None::<InventurCustomEntryTO>);
    let mut containers = use_signal(|| Vec::<ContainerTO>::new());

    // Load data on mount
    use_effect(move || {
        spawn(async move {
            let config = CONFIG.read().clone();
            if config.backend.is_empty() {
                return;
            }

            // Load inventur details
            if let Ok(inventur_data) = api::get_inventur(&config, inventur_id).await {
                inventur.set(Some(inventur_data));
            }

            // Load custom entries
            CUSTOM_ENTRIES.write().loading = true;
            match api::get_custom_entries_by_inventur(&config, inventur_id).await {
                Ok(entries) => {
                    CUSTOM_ENTRIES.write().items = entries;
                    CUSTOM_ENTRIES.write().error = None;
                }
                Err(e) => {
                    CUSTOM_ENTRIES.write().error = Some(format!("Failed to load custom entries: {}", e));
                }
            }
            CUSTOM_ENTRIES.write().loading = false;

            // Load racks if not loaded
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

            // Load containers for editing
            match api::get_containers(&config).await {
                Ok(c) => containers.set(c),
                Err(_) => {}
            }
        });
    });

    // Debounced text filter update
    use_effect(move || {
        let query = filter_input();
        spawn(async move {
            gloo_timers::future::TimeoutFuture::new(500).await;
            if query == filter_input() {
                CUSTOM_ENTRIES.write().filter_query = query;
            }
        });
    });

    // Read global state for filter options
    let custom_entries = CUSTOM_ENTRIES.read();
    let racks = RACKS.read();

    // Get distinct rack IDs from entries
    let distinct_rack_ids: Vec<Uuid> = custom_entries
        .items
        .iter()
        .filter(|e| e.deleted.is_none())
        .filter_map(|e| e.rack_id)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    // Get distinct measured_by values
    let mut distinct_measured_by: Vec<String> = custom_entries
        .items
        .iter()
        .filter(|e| e.deleted.is_none())
        .filter_map(|e| e.measured_by.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    distinct_measured_by.sort();

    // Handlers
    let set_has_ean = move |value: Option<bool>| {
        CUSTOM_ENTRIES.write().filter_has_ean = value;
    };

    let toggle_rack = move |rack_id: Uuid| {
        let mut entries = CUSTOM_ENTRIES.write();
        if entries.filter_rack_ids.contains(&rack_id) {
            entries.filter_rack_ids.retain(|id| *id != rack_id);
        } else {
            entries.filter_rack_ids.push(rack_id);
        }
    };

    let toggle_measured_by = move |name: String| {
        let mut entries = CUSTOM_ENTRIES.write();
        if entries.filter_measured_by.contains(&name) {
            entries.filter_measured_by.retain(|n| *n != name);
        } else {
            entries.filter_measured_by.push(name);
        }
    };

    let clear_all_filters = move |_| {
        filter_input.set(String::new());
        let mut entries = CUSTOM_ENTRIES.write();
        entries.filter_query = String::new();
        entries.filter_has_ean = None;
        entries.filter_rack_ids.clear();
        entries.filter_measured_by.clear();
        entries.filter_review_state = None;
    };

    let set_review_state_filter = move |value: Option<String>| {
        CUSTOM_ENTRIES.write().filter_review_state = value;
    };

    // Edit handler
    let handle_edit = move |entry: InventurCustomEntryTO| {
        editing_entry.set(Some(entry));
    };

    // Mark as reviewed handler
    let handle_mark_reviewed = move |entry: InventurCustomEntryTO| {
        spawn(async move {
            let config = CONFIG.read().clone();
            // Create updated entry with review_state = "reviewed"
            let updated_entry = InventurCustomEntryTO {
                review_state: Some("reviewed".to_string()),
                ..entry
            };
            match api::update_custom_entry(&config, updated_entry).await {
                Ok(_) => {
                    // Reload entries to reflect the change
                    match api::get_custom_entries_by_inventur(&config, inventur_id).await {
                        Ok(entries) => {
                            CUSTOM_ENTRIES.write().items = entries;
                            CUSTOM_ENTRIES.write().error = None;
                        }
                        Err(e) => {
                            CUSTOM_ENTRIES.write().error = Some(format!("Failed to reload: {}", e));
                        }
                    }
                }
                Err(e) => {
                    CUSTOM_ENTRIES.write().error = Some(format!("Failed to mark as reviewed: {}", e));
                }
            }
        });
    };

    // Mark as unreviewed handler
    let handle_mark_unreviewed = move |entry: InventurCustomEntryTO| {
        spawn(async move {
            let config = CONFIG.read().clone();
            // Create updated entry with review_state = "unreviewed"
            let updated_entry = InventurCustomEntryTO {
                review_state: Some("unreviewed".to_string()),
                ..entry
            };
            match api::update_custom_entry(&config, updated_entry).await {
                Ok(_) => {
                    // Reload entries to reflect the change
                    match api::get_custom_entries_by_inventur(&config, inventur_id).await {
                        Ok(entries) => {
                            CUSTOM_ENTRIES.write().items = entries;
                            CUSTOM_ENTRIES.write().error = None;
                        }
                        Err(e) => {
                            CUSTOM_ENTRIES.write().error = Some(format!("Failed to reload: {}", e));
                        }
                    }
                }
                Err(e) => {
                    CUSTOM_ENTRIES.write().error = Some(format!("Failed to mark as unreviewed: {}", e));
                }
            }
        });
    };

    // Reload custom entries helper
    let reload_entries = move || {
        spawn(async move {
            let config = CONFIG.read().clone();
            match api::get_custom_entries_by_inventur(&config, inventur_id).await {
                Ok(entries) => {
                    CUSTOM_ENTRIES.write().items = entries;
                    CUSTOM_ENTRIES.write().error = None;
                }
                Err(e) => {
                    CUSTOM_ENTRIES.write().error = Some(format!("Failed to reload: {}", e));
                }
            }
        });
    };

    rsx! {
        RequirePrivilege {
            privilege: "admin",
            fallback: rsx! { AccessDeniedPage { required_privilege: "admin".to_string() } },
            div { class: "flex flex-col min-h-screen",
                TopBar {}
                div { class: "flex-1 container mx-auto px-4 py-8",
                    // Header
                    div { class: "flex justify-between items-center mb-6",
                        div {
                            h1 { class: "text-3xl font-bold",
                                {i18n.t(Key::ManageCustomEntries)}
                            }
                            if let Some(inv) = inventur.read().as_ref() {
                                p { class: "text-gray-600 mt-1",
                                    {inv.name.clone()}
                                }
                            }
                        }
                    }

                    // Filter bar
                    div { class: "mb-4",
                        div { class: "flex flex-col sm:flex-row gap-2",
                            div { class: "relative flex-1",
                                input {
                                    r#type: "text",
                                    class: "w-full px-4 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500 focus:border-transparent",
                                    placeholder: "{i18n.t(Key::Search)}...",
                                    value: "{filter_input}",
                                    oninput: move |evt| {
                                        filter_input.set(evt.value().clone());
                                    },
                                }
                                if !filter_input().is_empty() {
                                    button {
                                        class: "absolute right-2 top-1/2 -translate-y-1/2 px-3 py-1 text-gray-500 hover:text-gray-700",
                                        onclick: move |_| {
                                            filter_input.set(String::new());
                                            CUSTOM_ENTRIES.write().filter_query = String::new();
                                        },
                                        "✕"
                                    }
                                }
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
                                    // EAN filter
                                    div {
                                        label { class: "block text-sm font-medium text-gray-700 mb-2",
                                            {i18n.t(Key::FilterByEan)}
                                        }
                                        div { class: "space-y-2",
                                            label { class: "flex items-center space-x-2 cursor-pointer",
                                                input {
                                                    r#type: "radio",
                                                    name: "has_ean",
                                                    checked: custom_entries.filter_has_ean.is_none(),
                                                    onchange: move |_| set_has_ean(None),
                                                }
                                                span { class: "text-sm", {i18n.t(Key::All)} }
                                            }
                                            label { class: "flex items-center space-x-2 cursor-pointer",
                                                input {
                                                    r#type: "radio",
                                                    name: "has_ean",
                                                    checked: custom_entries.filter_has_ean == Some(true),
                                                    onchange: move |_| set_has_ean(Some(true)),
                                                }
                                                span { class: "text-sm", {i18n.t(Key::HasEan)} }
                                            }
                                            label { class: "flex items-center space-x-2 cursor-pointer",
                                                input {
                                                    r#type: "radio",
                                                    name: "has_ean",
                                                    checked: custom_entries.filter_has_ean == Some(false),
                                                    onchange: move |_| set_has_ean(Some(false)),
                                                }
                                                span { class: "text-sm", {i18n.t(Key::NoEan)} }
                                            }
                                        }
                                    }

                                    // Rack filter
                                    div {
                                        label { class: "block text-sm font-medium text-gray-700 mb-2",
                                            {i18n.t(Key::FilterByRack)}
                                        }
                                        div { class: "space-y-2 max-h-40 overflow-y-auto border border-gray-200 rounded p-2 bg-white",
                                            for rack_id in distinct_rack_ids.iter() {
                                                {
                                                    let rack_id_clone = *rack_id;
                                                    let rack_name = racks.items.iter()
                                                        .find(|r| r.id == Some(rack_id_clone))
                                                        .map(|r| r.name.clone())
                                                        .unwrap_or_else(|| rack_id_clone.to_string());
                                                    let is_selected = custom_entries.filter_rack_ids.contains(&rack_id_clone);

                                                    rsx! {
                                                        label { class: "flex items-center space-x-2 cursor-pointer hover:bg-gray-50 p-1 rounded",
                                                            input {
                                                                r#type: "checkbox",
                                                                class: "rounded border-gray-300",
                                                                checked: is_selected,
                                                                onchange: move |_| toggle_rack(rack_id_clone),
                                                            }
                                                            span { class: "text-sm", {rack_name} }
                                                        }
                                                    }
                                                }
                                            }
                                            if distinct_rack_ids.is_empty() {
                                                span { class: "text-sm text-gray-400 italic", "-" }
                                            }
                                        }
                                    }

                                    // Measured by filter
                                    div {
                                        label { class: "block text-sm font-medium text-gray-700 mb-2",
                                            {i18n.t(Key::FilterByMeasuredBy)}
                                        }
                                        div { class: "space-y-2 max-h-40 overflow-y-auto border border-gray-200 rounded p-2 bg-white",
                                            for name in distinct_measured_by.iter() {
                                                {
                                                    let name_clone = name.clone();
                                                    let is_selected = custom_entries.filter_measured_by.contains(&name_clone);

                                                    rsx! {
                                                        label { class: "flex items-center space-x-2 cursor-pointer hover:bg-gray-50 p-1 rounded",
                                                            input {
                                                                r#type: "checkbox",
                                                                class: "rounded border-gray-300",
                                                                checked: is_selected,
                                                                onchange: move |_| toggle_measured_by(name_clone.clone()),
                                                            }
                                                            span { class: "text-sm", {name.clone()} }
                                                        }
                                                    }
                                                }
                                            }
                                            if distinct_measured_by.is_empty() {
                                                span { class: "text-sm text-gray-400 italic", "-" }
                                            }
                                        }
                                    }

                                    // Review state filter
                                    div {
                                        label { class: "block text-sm font-medium text-gray-700 mb-2",
                                            {i18n.t(Key::FilterByReviewState)}
                                        }
                                        div { class: "space-y-2",
                                            label { class: "flex items-center space-x-2 cursor-pointer",
                                                input {
                                                    r#type: "radio",
                                                    name: "review_state",
                                                    checked: custom_entries.filter_review_state.is_none(),
                                                    onchange: move |_| set_review_state_filter(None),
                                                }
                                                span { class: "text-sm", {i18n.t(Key::All)} }
                                            }
                                            label { class: "flex items-center space-x-2 cursor-pointer",
                                                input {
                                                    r#type: "radio",
                                                    name: "review_state",
                                                    checked: custom_entries.filter_review_state.as_deref() == Some("unreviewed"),
                                                    onchange: move |_| set_review_state_filter(Some("unreviewed".to_string())),
                                                }
                                                span { class: "text-sm", {i18n.t(Key::Unreviewed)} }
                                            }
                                            label { class: "flex items-center space-x-2 cursor-pointer",
                                                input {
                                                    r#type: "radio",
                                                    name: "review_state",
                                                    checked: custom_entries.filter_review_state.as_deref() == Some("reviewed"),
                                                    onchange: move |_| set_review_state_filter(Some("reviewed".to_string())),
                                                }
                                                span { class: "text-sm", {i18n.t(Key::Reviewed)} }
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

                    // List component
                    CustomEntryManagementList {
                        on_edit: handle_edit,
                        on_mark_reviewed: handle_mark_reviewed,
                        on_mark_unreviewed: handle_mark_unreviewed,
                    }
                }
            }

            // Edit modal
            if let Some(entry) = editing_entry.read().clone() {
                {
                    let rack_id = entry.rack_id.unwrap_or(Uuid::nil());
                    let containers_list = containers.read().clone();
                    rsx! {
                        CustomEntryForm {
                            inventur_id,
                            rack_id,
                            containers: containers_list,
                            existing_entry: Some(entry),
                            on_save: move |_| {
                                editing_entry.set(None);
                                reload_entries();
                            },
                            on_cancel: move |_| {
                                editing_entry.set(None);
                            },
                        }
                    }
                }
            }
        }
    }
}
