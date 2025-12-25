use crate::api;
use crate::service::config::CONFIG;
use crate::state::ProductRack;
use dioxus::prelude::*;
use rest_types::ProductRackTO;
use uuid::Uuid;

pub static PRODUCT_RACKS: GlobalSignal<ProductRack> = GlobalSignal::new(ProductRack::default);

#[derive(Debug)]
#[allow(dead_code)]
pub enum ProductRackService {
    LoadProductRacks,
    AddProductToRack(Uuid, Uuid),
    RemoveProductFromRack(Uuid, Uuid),
    GetRacksForProduct(Uuid),
    GetProductsInRack(Uuid),
}

pub fn product_rack_service() {
    spawn(async move {
        // Initialize product racks loading on startup
        let config = CONFIG.read().clone();
        if !config.backend.is_empty() {
            PRODUCT_RACKS.write().loading = true;
            match api::get_all_product_rack_relationships(&config).await {
                Ok(product_racks) => {
                    PRODUCT_RACKS.write().items = product_racks;
                    PRODUCT_RACKS.write().error = None;
                }
                Err(e) => {
                    PRODUCT_RACKS.write().error =
                        Some(format!("Failed to load product-rack relationships: {}", e));
                }
            }
            PRODUCT_RACKS.write().loading = false;
        }
    });
}

pub async fn add_product_to_rack_action(product_id: Uuid, rack_id: Uuid) -> Result<(), String> {
    let config = CONFIG.read().clone();

    match api::add_product_to_rack(&config, product_id, rack_id).await {
        Ok(new_relationship) => {
            // Add to local state
            PRODUCT_RACKS.write().items.push(new_relationship);
            PRODUCT_RACKS.write().error = None;
            Ok(())
        }
        Err(e) => {
            let error_msg = format!("Failed to add product to rack: {}", e);
            PRODUCT_RACKS.write().error = Some(error_msg.clone());
            Err(error_msg)
        }
    }
}

pub async fn remove_product_from_rack_action(
    product_id: Uuid,
    rack_id: Uuid,
) -> Result<(), String> {
    let config = CONFIG.read().clone();

    match api::remove_product_from_rack(&config, product_id, rack_id).await {
        Ok(()) => {
            // Remove from local state
            PRODUCT_RACKS
                .write()
                .items
                .retain(|item| !(item.product_id == product_id && item.rack_id == rack_id));
            PRODUCT_RACKS.write().error = None;
            Ok(())
        }
        Err(e) => {
            let error_msg = format!("Failed to remove product from rack: {}", e);
            PRODUCT_RACKS.write().error = Some(error_msg.clone());
            Err(error_msg)
        }
    }
}

pub async fn get_racks_for_product_action(product_id: Uuid) -> Result<Vec<ProductRackTO>, String> {
    let config = CONFIG.read().clone();

    match api::get_racks_for_product(&config, product_id).await {
        Ok(racks) => {
            PRODUCT_RACKS.write().error = None;
            Ok(racks)
        }
        Err(e) => {
            let error_msg = format!("Failed to get racks for product: {}", e);
            PRODUCT_RACKS.write().error = Some(error_msg.clone());
            Err(error_msg)
        }
    }
}

pub async fn get_products_in_rack_action(rack_id: Uuid) -> Result<Vec<ProductRackTO>, String> {
    let config = CONFIG.read().clone();

    match api::get_products_in_rack(&config, rack_id).await {
        Ok(products) => {
            PRODUCT_RACKS.write().error = None;
            Ok(products)
        }
        Err(e) => {
            let error_msg = format!("Failed to get products in rack: {}", e);
            PRODUCT_RACKS.write().error = Some(error_msg.clone());
            Err(error_msg)
        }
    }
}

pub async fn set_product_position_action(
    product_id: Uuid,
    rack_id: Uuid,
    position: i32,
) -> Result<ProductRackTO, String> {
    let config = CONFIG.read().clone();

    match api::set_product_position(&config, product_id, rack_id, position).await {
        Ok(updated_relationship) => {
            PRODUCT_RACKS.write().error = None;
            Ok(updated_relationship)
        }
        Err(e) => {
            let error_msg = format!("Failed to update product position: {}", e);
            PRODUCT_RACKS.write().error = Some(error_msg.clone());
            Err(error_msg)
        }
    }
}

pub async fn reorder_products_in_rack_action(
    rack_id: Uuid,
    product_order: Vec<Uuid>,
) -> Result<Vec<ProductRackTO>, String> {
    let config = CONFIG.read().clone();

    match api::reorder_products_in_rack(&config, rack_id, product_order).await {
        Ok(updated_relationships) => {
            PRODUCT_RACKS.write().error = None;
            Ok(updated_relationships)
        }
        Err(e) => {
            let error_msg = format!("Failed to reorder products: {}", e);
            PRODUCT_RACKS.write().error = Some(error_msg.clone());
            Err(error_msg)
        }
    }
}
