use dioxus::prelude::*;
use rest_types::ProductTO;
use uuid::Uuid;
use crate::api;
use crate::service::config::CONFIG;
use crate::state::Product;

pub static PRODUCTS: GlobalSignal<Product> = GlobalSignal::new(Product::default);

#[derive(Debug)]
pub enum ProductService {
    LoadProducts,
    GetProduct(Uuid),
    CreateProduct(ProductTO),
    UpdateProduct(ProductTO),
    DeleteProduct(Uuid),
}

pub fn product_service() {
    spawn(async move {
        // Initialize products loading on startup
        let config = CONFIG.read().clone();
        if !config.backend.is_empty() {
            PRODUCTS.write().loading = true;
            match api::get_products(&config).await {
                Ok(products) => {
                    PRODUCTS.write().items = products;
                    PRODUCTS.write().error = None;
                }
                Err(e) => {
                    PRODUCTS.write().error = Some(format!("Failed to load products: {}", e));
                }
            }
            PRODUCTS.write().loading = false;
        }
    });
}