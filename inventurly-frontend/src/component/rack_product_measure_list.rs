use crate::i18n::{use_i18n, Key};
use dioxus::prelude::*;
use rest_types::{InventurMeasurementTO, ProductTO};
use uuid::Uuid;

#[component]
pub fn RackProductMeasureList(
    products: Vec<ProductTO>,
    measurements: Vec<InventurMeasurementTO>,
    rack_id: Uuid,
    on_measure: EventHandler<ProductTO>,
) -> Element {
    let i18n = use_i18n();

    // Helper to check if a product has been measured
    let is_measured = |product_id: Uuid| {
        measurements.iter().any(|m| {
            m.product_id == product_id
                && m.rack_id == Some(rack_id)
                && m.deleted.is_none()
        })
    };

    // Helper to get measurement value
    let get_measurement_value = |product_id: Uuid| -> Option<String> {
        measurements
            .iter()
            .find(|m| {
                m.product_id == product_id
                    && m.rack_id == Some(rack_id)
                    && m.deleted.is_none()
            })
            .map(|m| {
                if let Some(count) = m.count {
                    format!("Count: {}", count)
                } else if let Some(weight) = m.weight_grams {
                    format!("Weight: {} g", weight)
                } else {
                    "Measured".to_string()
                }
            })
    };

    // Count measured products
    let measured_count = products
        .iter()
        .filter(|p| {
            if let Some(id) = p.id {
                is_measured(id)
            } else {
                false
            }
        })
        .count();

    rsx! {
        div { class: "bg-white rounded-lg shadow",
            div { class: "px-6 py-4 border-b",
                h2 { class: "text-xl font-semibold",
                    {i18n.t(Key::Products)}
                }
                p { class: "text-sm text-gray-600 mt-1",
                    {i18n.t(Key::ProductsMeasured)}
                    ": {measured_count} / {products.len()}"
                }
            }

            if products.is_empty() {
                div { class: "p-6 text-center text-gray-500",
                    {i18n.t(Key::NoDataFound)}
                }
            } else {
                div { class: "overflow-x-auto",
                    table { class: "w-full",
                        thead {
                            tr { class: "border-b bg-gray-50",
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider w-16",
                                    {i18n.t(Key::InventurStatus)}
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    {i18n.t(Key::ProductName)}
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    {i18n.t(Key::ProductEan)}
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    "Value"
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    {i18n.t(Key::Actions)}
                                }
                            }
                        }
                        tbody {
                            for product in products.iter() {
                                {
                                    let product_id = product.id.unwrap_or(Uuid::nil());
                                    let name = product.name.clone();
                                    let ean = product.ean.clone();
                                    let measured = is_measured(product_id);
                                    let value = get_measurement_value(product_id);
                                    let product_clone = product.clone();

                                    rsx! {
                                        tr {
                                            class: "border-b hover:bg-gray-50 cursor-pointer",
                                            onclick: {
                                                let product_for_click = product_clone.clone();
                                                move |_| {
                                                    on_measure.call(product_for_click.clone());
                                                }
                                            },
                                            td { class: "px-6 py-4 whitespace-nowrap text-center",
                                                if measured {
                                                    span {
                                                        class: "inline-flex items-center justify-center w-6 h-6 rounded-full bg-green-100 text-green-600",
                                                        title: "{i18n.t(Key::Measured)}",
                                                        "✓"
                                                    }
                                                } else {
                                                    span {
                                                        class: "inline-flex items-center justify-center w-6 h-6 rounded-full bg-gray-100 text-gray-400",
                                                        title: "{i18n.t(Key::NotMeasured)}",
                                                        "○"
                                                    }
                                                }
                                            }
                                            td { class: "px-6 py-4 text-sm font-medium",
                                                {name}
                                            }
                                            td { class: "px-6 py-4 text-sm text-gray-600",
                                                {ean}
                                            }
                                            td { class: "px-6 py-4 text-sm",
                                                if let Some(val) = value {
                                                    span { class: "text-gray-700", {val} }
                                                } else {
                                                    span { class: "text-gray-400 italic", "-" }
                                                }
                                            }
                                            td { class: "px-6 py-4 whitespace-nowrap text-sm",
                                                button {
                                                    class: if measured {
                                                        "text-blue-600 hover:text-blue-800"
                                                    } else {
                                                        "text-green-600 hover:text-green-800 font-medium"
                                                    },
                                                    onclick: move |e| {
                                                        e.stop_propagation(); // Prevent row click from firing
                                                        on_measure.call(product_clone.clone());
                                                    },
                                                    if measured {
                                                        {i18n.t(Key::Edit)}
                                                    } else {
                                                        {i18n.t(Key::QuickMeasure)}
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
