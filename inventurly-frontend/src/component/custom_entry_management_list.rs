use crate::i18n::{use_i18n, Key};
use crate::service::custom_entry::CUSTOM_ENTRIES;
use crate::service::rack::RACKS;
use dioxus::prelude::*;
use rest_types::InventurCustomEntryTO;
use uuid::Uuid;

#[component]
pub fn CustomEntryManagementList(
    on_edit: EventHandler<InventurCustomEntryTO>,
) -> Element {
    let i18n = use_i18n();
    let custom_entries = CUSTOM_ENTRIES.read();
    let racks = RACKS.read();

    // Helper to get rack name by ID
    let get_rack_name = |rack_id: Option<Uuid>| -> String {
        rack_id
            .and_then(|id| racks.items.iter().find(|r| r.id == Some(id)))
            .map(|r| r.name.clone())
            .unwrap_or_else(|| "-".to_string())
    };

    // Helper to get measurement value
    let get_value = |count: Option<i64>, weight: Option<i64>| -> String {
        let mut parts = Vec::new();
        if let Some(c) = count {
            if c >= 0 {
                parts.push(format!("{}: {}", i18n.t(Key::Count), c));
            }
        }
        if let Some(w) = weight {
            if w >= 0 {
                parts.push(format!("{}: {} g", i18n.t(Key::MeasurementWeight), w));
            }
        }
        if parts.is_empty() {
            "-".to_string()
        } else {
            parts.join(", ")
        }
    };

    // Apply filters
    let filtered_entries: Vec<_> = custom_entries
        .items
        .iter()
        .filter(|e| e.deleted.is_none())
        // Text filter
        .filter(|e| {
            if custom_entries.filter_query.is_empty() {
                return true;
            }
            let query = custom_entries.filter_query.to_lowercase();
            e.custom_product_name.to_lowercase().contains(&query)
                || e.notes
                    .as_ref()
                    .map(|n| n.to_lowercase().contains(&query))
                    .unwrap_or(false)
                || e.ean
                    .as_ref()
                    .map(|ean| ean.to_lowercase().contains(&query))
                    .unwrap_or(false)
        })
        // EAN filter
        .filter(|e| match custom_entries.filter_has_ean {
            None => true,
            Some(true) => e.ean.is_some(),
            Some(false) => e.ean.is_none(),
        })
        // Rack filter
        .filter(|e| {
            if custom_entries.filter_rack_ids.is_empty() {
                return true;
            }
            e.rack_id
                .map(|id| custom_entries.filter_rack_ids.contains(&id))
                .unwrap_or(false)
        })
        // Measured by filter
        .filter(|e| {
            if custom_entries.filter_measured_by.is_empty() {
                return true;
            }
            e.measured_by
                .as_ref()
                .map(|m| custom_entries.filter_measured_by.contains(m))
                .unwrap_or(false)
        })
        .cloned()
        .collect();

    rsx! {
        div { class: "bg-white rounded-lg shadow",
            div { class: "px-6 py-4 border-b",
                h2 { class: "text-xl font-semibold",
                    {i18n.t(Key::CustomEntries)}
                }
                p { class: "text-sm text-gray-600 mt-1",
                    "Showing {filtered_entries.len()} of {custom_entries.items.iter().filter(|e| e.deleted.is_none()).count()} entries"
                }
            }

            if custom_entries.loading {
                div { class: "p-6 text-center text-gray-500",
                    {i18n.t(Key::Loading)}
                }
            } else if filtered_entries.is_empty() {
                div { class: "p-6 text-center text-gray-500",
                    {i18n.t(Key::NoDataFound)}
                }
            } else {
                div { class: "overflow-x-auto",
                    table { class: "w-full",
                        thead {
                            tr { class: "border-b bg-gray-50",
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    {i18n.t(Key::CustomProductName)}
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    "EAN"
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    {i18n.t(Key::Rack)}
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    "Value"
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    {i18n.t(Key::MeasuredBy)}
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    {i18n.t(Key::MeasuredAt)}
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    {i18n.t(Key::Actions)}
                                }
                            }
                        }
                        tbody {
                            for entry in filtered_entries.iter() {
                                {
                                    let entry_clone = entry.clone();
                                    let name = entry.custom_product_name.clone();
                                    let ean_display = entry.ean.clone().unwrap_or_else(|| "-".to_string());
                                    let has_ean = entry.ean.is_some();
                                    let rack_name = get_rack_name(entry.rack_id);
                                    let value = get_value(entry.count, entry.weight_grams);
                                    let measured_by = entry.measured_by.clone().unwrap_or_else(|| "-".to_string());
                                    let measured_at = entry.measured_at
                                        .map(|dt| i18n.format_datetime(dt))
                                        .unwrap_or_else(|| "-".to_string());

                                    rsx! {
                                        tr { class: "border-b hover:bg-gray-50",
                                            td { class: "px-6 py-4 text-sm font-medium",
                                                {name}
                                            }
                                            td { class: "px-6 py-4 text-sm",
                                                if has_ean {
                                                    span { class: "text-green-600",
                                                        {ean_display}
                                                    }
                                                } else {
                                                    span { class: "text-gray-400 italic",
                                                        {i18n.t(Key::CustomEntry)}
                                                    }
                                                }
                                            }
                                            td { class: "px-6 py-4 text-sm text-gray-700",
                                                {rack_name}
                                            }
                                            td { class: "px-6 py-4 text-sm text-gray-700",
                                                {value}
                                            }
                                            td { class: "px-6 py-4 text-sm text-gray-600",
                                                {measured_by}
                                            }
                                            td { class: "px-6 py-4 text-sm text-gray-600",
                                                {measured_at}
                                            }
                                            td { class: "px-6 py-4 whitespace-nowrap text-sm",
                                                button {
                                                    class: "text-blue-600 hover:text-blue-800",
                                                    onclick: move |_| {
                                                        on_edit.call(entry_clone.clone());
                                                    },
                                                    {i18n.t(Key::Edit)}
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

            if let Some(error) = custom_entries.error.as_ref() {
                div { class: "p-4 bg-red-100 border-t border-red-400 text-red-700",
                    {error.clone()}
                }
            }
        }
    }
}
