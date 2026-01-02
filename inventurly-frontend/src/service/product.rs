use crate::api;
use crate::service::config::CONFIG;
use crate::state::Product;
use dioxus::prelude::*;
use futures_util::StreamExt;
use rest_types::ProductTO;
use uuid::Uuid;

pub static PRODUCTS: GlobalSignal<Product> = GlobalSignal::new(Product::default);

/// Refresh products from the backend
pub async fn refresh_products() {
    let config = CONFIG.read().clone();
    if !config.backend.is_empty() {
        tracing::info!("Refreshing products from backend: {}", config.backend);
        PRODUCTS.write().loading = true;
        match api::get_products(&config).await {
            Ok(products) => {
                tracing::info!("Refreshed {} products successfully", products.len());
                PRODUCTS.write().items = products;
                PRODUCTS.write().error = None;
            }
            Err(e) => {
                let error_msg = format!("Failed to refresh products: {}", e);
                tracing::error!("{}", error_msg);
                PRODUCTS.write().error = Some(error_msg);
            }
        }
        PRODUCTS.write().loading = false;
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum ProductService {
    LoadProducts,
    GetProduct(Uuid),
    CreateProduct(ProductTO),
    UpdateProduct(ProductTO),
    DeleteProduct(Uuid),
    SearchProducts(String),
}

pub async fn product_service(mut rx: UnboundedReceiver<ProductService>) {
    // Handle incoming events
    while let Some(action) = rx.next().await {
        match action {
            ProductService::LoadProducts => {
                let config = CONFIG.read().clone();
                if !config.backend.is_empty() {
                    tracing::info!("Loading products from backend: {}", config.backend);
                    PRODUCTS.write().loading = true;
                    match api::get_products(&config).await {
                        Ok(products) => {
                            tracing::info!("Loaded {} products successfully", products.len());
                            PRODUCTS.write().items = products;
                            PRODUCTS.write().error = None;
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to load products: {}", e);
                            tracing::error!("{}", error_msg);
                            PRODUCTS.write().error = Some(error_msg);
                        }
                    }
                    PRODUCTS.write().loading = false;
                } else {
                    tracing::warn!("Cannot load products: backend URL is empty");
                }
            }
            ProductService::SearchProducts(query) => {
                tracing::info!("Received search event for: '{}'", query);
                PRODUCTS.write().search_query = query.clone();

                if query.len() >= 2 {
                    PRODUCTS.write().search_loading = true;

                    // No need for debounce here - it's handled in the component
                    let query_lower = query.to_lowercase();
                    let all_products = PRODUCTS.read().items.clone();

                    // Verify products are loaded
                    if all_products.is_empty() {
                        tracing::warn!("No products loaded yet, skipping search");
                        PRODUCTS.write().search_results = vec![];
                        PRODUCTS.write().search_loading = false;
                        return;
                    }

                    tracing::info!("Searching {} products for: '{}'", all_products.len(), query);

                    // Filter products locally (same logic as backend)
                    let mut matching_products: Vec<ProductTO> = all_products
                        .into_iter()
                        .filter(|product| {
                            product.name.to_lowercase().contains(&query_lower)
                                || product.ean.to_lowercase().contains(&query_lower)
                                || product.short_name.to_lowercase().contains(&query_lower)
                        })
                        .collect();

                    tracing::info!("Found {} matching products", matching_products.len());

                    // Sort by relevance (same logic as backend)
                    matching_products.sort_by(|a, b| {
                        let a_exact = a.name.to_lowercase() == query_lower
                            || a.ean.to_lowercase() == query_lower;
                        let b_exact = b.name.to_lowercase() == query_lower
                            || b.ean.to_lowercase() == query_lower;

                        if a_exact && !b_exact {
                            std::cmp::Ordering::Less
                        } else if !a_exact && b_exact {
                            std::cmp::Ordering::Greater
                        } else {
                            let a_starts = a.name.to_lowercase().starts_with(&query_lower)
                                || a.ean.to_lowercase().starts_with(&query_lower);
                            let b_starts = b.name.to_lowercase().starts_with(&query_lower)
                                || b.ean.to_lowercase().starts_with(&query_lower);

                            if a_starts && !b_starts {
                                std::cmp::Ordering::Less
                            } else if !a_starts && b_starts {
                                std::cmp::Ordering::Greater
                            } else {
                                a.name.cmp(&b.name)
                            }
                        }
                    });

                    // Limit results to 20 for performance
                    matching_products.truncate(20);

                    PRODUCTS.write().search_results = matching_products;
                } else {
                    // Clear search results for short queries
                    PRODUCTS.write().search_results = vec![];
                }

                PRODUCTS.write().search_loading = false;
            }
            // TODO: Implement other actions as needed
            _ => {
                // For now, just log unhandled actions
                tracing::debug!("Unhandled ProductService action: {:?}", action);
            }
        }
    }
}
