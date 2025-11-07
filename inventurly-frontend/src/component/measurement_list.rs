use crate::i18n::{use_i18n, Key};
use crate::service::inventur::MEASUREMENTS;
use crate::service::product::PRODUCTS;
use dioxus::prelude::*;
use uuid::Uuid;

#[component]
pub fn MeasurementList(
    inventur_id: Uuid,
    on_edit: EventHandler<Uuid>,
    on_delete: EventHandler<Uuid>,
) -> Element {
    let i18n = use_i18n();
    let measurements = MEASUREMENTS.read();
    let products = PRODUCTS.read();

    // Filter measurements for this inventur and exclude deleted ones
    let active_measurements: Vec<_> = measurements
        .items
        .iter()
        .filter(|m| m.inventur_id == inventur_id && m.deleted.is_none())
        .collect();

    // Helper function to get product name by ID
    let get_product_name = |product_id: Uuid| {
        products
            .items
            .iter()
            .find(|p| p.id == Some(product_id))
            .map(|p| p.name.as_str())
            .unwrap_or("Unknown Product")
    };

    rsx! {
        div { class: "bg-white rounded-lg shadow",
            div { class: "px-6 py-4 border-b flex justify-between items-center",
                h2 { class: "text-xl font-semibold",
                    {i18n.t(Key::Measurements)}
                }
                button {
                    class: "px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700",
                    onclick: move |_| on_edit.call(Uuid::nil()),
                    {i18n.t(Key::RecordMeasurement)}
                }
            }

            if measurements.loading {
                div { class: "p-6 text-center text-gray-500",
                    {i18n.t(Key::Loading)}
                }
            } else if let Some(ref error) = measurements.error {
                div { class: "p-6 text-center text-red-500",
                    {error.clone()}
                }
            } else if active_measurements.is_empty() {
                div { class: "p-6 text-center text-gray-500",
                    {i18n.t(Key::NoMeasurementsFound)}
                }
            } else {
                div { class: "overflow-x-auto",
                    table { class: "w-full",
                        thead {
                            tr { class: "border-b bg-gray-50",
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    {i18n.t(Key::ProductName)}
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    {i18n.t(Key::MeasurementCount)}
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    {i18n.t(Key::MeasurementWeightGrams)}
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    {i18n.t(Key::MeasuredAt)}
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    {i18n.t(Key::MeasuredBy)}
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    {i18n.t(Key::Actions)}
                                }
                            }
                        }
                        tbody {
                            for measurement in active_measurements.iter() {
                                {
                                    let measurement_id = measurement.id;
                                    let product_id = measurement.product_id;
                                    let product_name = get_product_name(product_id).to_string();
                                    let count = measurement.count;
                                    let weight = measurement.weight_grams;
                                    let measured_at = measurement.measured_at;
                                    let measured_by = measurement.measured_by.clone().unwrap_or_default();

                                    rsx! {
                                        tr {
                                            class: "border-b hover:bg-gray-50",
                                            td { class: "px-6 py-4 whitespace-nowrap text-sm font-medium",
                                                {product_name}
                                            }
                                            td { class: "px-6 py-4 whitespace-nowrap text-sm",
                                                if let Some(c) = count {
                                                    {c.to_string()}
                                                } else {
                                                    "-"
                                                }
                                            }
                                            td { class: "px-6 py-4 whitespace-nowrap text-sm",
                                                if let Some(w) = weight {
                                                    {format!("{} g", w)}
                                                } else {
                                                    "-"
                                                }
                                            }
                                            td { class: "px-6 py-4 whitespace-nowrap text-sm",
                                                if let Some(date) = measured_at {
                                                    {i18n.format_datetime(date)}
                                                } else {
                                                    "-"
                                                }
                                            }
                                            td { class: "px-6 py-4 whitespace-nowrap text-sm",
                                                {measured_by}
                                            }
                                            td { class: "px-6 py-4 whitespace-nowrap text-sm space-x-2",
                                                button {
                                                    class: "text-blue-600 hover:text-blue-800",
                                                    onclick: move |_| {
                                                        if let Some(id) = measurement_id {
                                                            on_edit.call(id);
                                                        }
                                                    },
                                                    {i18n.t(Key::Edit)}
                                                }
                                                " | "
                                                {
                                                    let i18n_clone = i18n.clone();
                                                    rsx! {
                                                        button {
                                                            class: "text-red-600 hover:text-red-800",
                                                            onclick: move |_| {
                                                                if let Some(id) = measurement_id {
                                                                    let window = web_sys::window().unwrap();
                                                                    let confirmed = window
                                                                        .confirm_with_message(&i18n_clone.t(Key::ConfirmDelete).to_string())
                                                                        .unwrap_or(false);
                                                                    if confirmed {
                                                                        on_delete.call(id);
                                                                    }
                                                                }
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
    }
}
