use crate::i18n::{use_i18n, Key};
use crate::router::Route;
use crate::service::product::PRODUCTS;
use dioxus::prelude::*;

#[component]
pub fn ProductList() -> Element {
    let i18n = use_i18n();
    let nav = navigator();
    let products = PRODUCTS.read();

    // Filter products based on all filter criteria and exclude deleted items
    let filtered_products: Vec<_> = products.items.iter()
        .filter(|p| p.deleted.is_none()) // Hide deleted products
        .filter(|p| {
            // Text search filter
            if !products.filter_query.is_empty() {
                let query = products.filter_query.to_lowercase();
                let matches = p.name.to_lowercase().contains(&query)
                    || p.ean.to_lowercase().contains(&query)
                    || p.short_name.to_lowercase().contains(&query);
                if !matches {
                    return false;
                }
            }

            // Sales unit filter
            if !products.filter_sales_units.is_empty() {
                if !products.filter_sales_units.contains(&p.sales_unit) {
                    return false;
                }
            }

            // Requires weighing filter
            if let Some(requires_weighing) = products.filter_requires_weighing {
                if p.requires_weighing != requires_weighing {
                    return false;
                }
            }

            // Price range filter
            let price_cents = p.price.to_cents();
            if let Some(min_price) = products.filter_price_min {
                if price_cents < min_price {
                    return false;
                }
            }
            if let Some(max_price) = products.filter_price_max {
                if price_cents > max_price {
                    return false;
                }
            }

            // Rack assignment filter
            if let Some(assigned) = products.filter_rack_assignment {
                let is_assigned = p.rack_count.unwrap_or(0) > 0;
                if is_assigned != assigned {
                    return false;
                }
            }

            true
        })
        .collect();

    rsx! {
        div { class: "bg-white rounded-lg shadow",
            div { class: "px-6 py-4 border-b flex justify-between items-center",
                h2 { class: "text-xl font-semibold",
                    {i18n.t(Key::Products)}
                }
                button {
                    class: "px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700",
                    onclick: move |_| { nav.push(Route::ProductDetails { id: "new".to_string() }); },
                    {i18n.t(Key::Create)}
                }
            }

            if filtered_products.is_empty() {
                div { class: "p-6 text-center text-gray-500",
                    if products.items.is_empty() {
                        {i18n.t(Key::NoDataFound)}
                    } else {
                        {i18n.t(Key::NoProductsFound)}
                    }
                }
            } else {
                div { class: "overflow-x-auto",
                    table { class: "w-full",
                        thead {
                            tr { class: "border-b bg-gray-50",
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    {i18n.t(Key::ProductEan)}
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    {i18n.t(Key::ProductName)}
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    {i18n.t(Key::ProductSalesUnit)}
                                }
                                th { class: "px-6 py-3 text-center text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    {i18n.t(Key::ProductRequiresWeighing)}
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    {i18n.t(Key::ProductPrice)}
                                }
                                th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                    {i18n.t(Key::Actions)}
                                }
                            }
                        }
                        tbody {
                            for product in filtered_products.iter() {
                                {
                                    let product_id = product.id;
                                    rsx! {
                                        tr {
                                            class: "border-b hover:bg-gray-50 cursor-pointer",
                                            onclick: move |_| {
                                                if let Some(id) = product_id {
                                                    nav.push(Route::ProductDetails { id: id.to_string() });
                                                }
                                            },
                                    td { class: "px-6 py-4 whitespace-nowrap text-sm",
                                        {product.ean.clone()}
                                    }
                                    td { class: "px-6 py-4 whitespace-nowrap text-sm",
                                        {product.name.clone()}
                                    }
                                    td { class: "px-6 py-4 whitespace-nowrap text-sm",
                                        {product.sales_unit.clone()}
                                    }
                                    td { class: "px-6 py-4 whitespace-nowrap text-sm text-center",
                                        {if product.requires_weighing { "✓" } else { "—" }}
                                    }
                                    td { class: "px-6 py-4 whitespace-nowrap text-sm",
                                        {i18n.format_price(product.price.to_cents())}
                                    }
                                            td { class: "px-6 py-4 whitespace-nowrap text-sm",
                                                button {
                                                    class: "text-blue-600 hover:text-blue-800",
                                                    onclick: move |e| {
                                                        e.stop_propagation();
                                                        if let Some(id) = product_id {
                                                            nav.push(Route::ProductDetails { id: id.to_string() });
                                                        }
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
        }
    }
}
