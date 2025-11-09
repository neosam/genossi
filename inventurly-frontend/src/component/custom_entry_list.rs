use crate::i18n::{use_i18n, Key};
use dioxus::prelude::*;
use rest_types::InventurCustomEntryTO;

#[component]
pub fn CustomEntryList(
    entries: Vec<InventurCustomEntryTO>,
    on_edit: EventHandler<InventurCustomEntryTO>,
    on_delete: EventHandler<InventurCustomEntryTO>,
) -> Element {
    let i18n = use_i18n();

    // Helper to get measurement value
    let get_value = |entry: &InventurCustomEntryTO| -> String {
        let mut parts = Vec::new();
        if let Some(count) = entry.count {
            if count >= 0 {
                parts.push(format!("Count: {}", count));
            }
        }
        if let Some(weight) = entry.weight_grams {
            if weight >= 0 {
                parts.push(format!("Weight: {} g", weight));
            }
        }
        if parts.is_empty() {
            "-".to_string()
        } else {
            parts.join(", ")
        }
    };

    rsx! {
        div { class: "bg-white rounded-lg shadow mt-6",
            div { class: "px-6 py-4 border-b",
                h2 { class: "text-xl font-semibold",
                    {i18n.t(Key::CustomEntries)}
                }
                p { class: "text-sm text-gray-600 mt-1",
                    "Total: {entries.len()}"
                }
            }

            if entries.is_empty() {
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
                                    "Value"
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    {i18n.t(Key::Notes)}
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    "Measured By"
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    {i18n.t(Key::Actions)}
                                }
                            }
                        }
                        tbody {
                            for entry in entries.iter().filter(|e| e.deleted.is_none()) {
                                {
                                    let name = entry.custom_product_name.clone();
                                    let value = get_value(entry);
                                    let notes = entry.notes.clone().unwrap_or_default();
                                    let measured_by = entry.measured_by.clone().unwrap_or_else(|| "-".to_string());
                                    let entry_for_edit = entry.clone();
                                    let entry_for_delete = entry.clone();

                                    rsx! {
                                        tr { class: "border-b hover:bg-gray-50",
                                            td { class: "px-6 py-4 text-sm font-medium",
                                                {name}
                                            }
                                            td { class: "px-6 py-4 text-sm text-gray-700",
                                                {value}
                                            }
                                            td { class: "px-6 py-4 text-sm text-gray-600",
                                                if notes.is_empty() {
                                                    span { class: "text-gray-400 italic", "-" }
                                                } else {
                                                    {notes}
                                                }
                                            }
                                            td { class: "px-6 py-4 text-sm text-gray-600",
                                                {measured_by}
                                            }
                                            td { class: "px-6 py-4 whitespace-nowrap text-sm space-x-3",
                                                button {
                                                    class: "text-blue-600 hover:text-blue-800",
                                                    onclick: move |_| {
                                                        on_edit.call(entry_for_edit.clone());
                                                    },
                                                    {i18n.t(Key::Edit)}
                                                }
                                                button {
                                                    class: "text-red-600 hover:text-red-800",
                                                    onclick: move |_| {
                                                        on_delete.call(entry_for_delete.clone());
                                                    },
                                                    {i18n.t(Key::Delete)}
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
