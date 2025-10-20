use async_trait::async_trait;
use std::sync::Arc;
use time::PrimitiveDateTime;
use uuid::Uuid;

use crate::{permission::Authentication, ServiceError};

#[derive(Debug, Clone, PartialEq)]
pub struct ProductRack {
    pub product_id: Uuid,
    pub rack_id: Uuid,
    pub created: PrimitiveDateTime,
    pub deleted: Option<PrimitiveDateTime>,
    pub version: Uuid,
}

impl From<&inventurly_dao::product_rack::ProductRackEntity> for ProductRack {
    fn from(entity: &inventurly_dao::product_rack::ProductRackEntity) -> Self {
        Self {
            product_id: entity.product_id,
            rack_id: entity.rack_id,
            created: entity.created,
            deleted: entity.deleted,
            version: entity.version,
        }
    }
}

impl From<&ProductRack> for inventurly_dao::product_rack::ProductRackEntity {
    fn from(domain: &ProductRack) -> Self {
        Self {
            product_id: domain.product_id,
            rack_id: domain.rack_id,
            created: domain.created,
            deleted: domain.deleted,
            version: domain.version,
        }
    }
}

#[async_trait]
pub trait ProductRackService {
    type Context: Send + Sync + Clone + Eq + std::fmt::Debug + 'static;
    type Transaction;

    /// Add a product to a rack
    async fn add_product_to_rack(
        &self,
        product_id: Uuid,
        rack_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<ProductRack, ServiceError>;

    /// Remove a product from a rack (soft delete)
    async fn remove_product_from_rack(
        &self,
        product_id: Uuid,
        rack_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<(), ServiceError>;

    /// Get all racks that contain a specific product
    async fn get_racks_for_product(
        &self,
        product_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[ProductRack]>, ServiceError>;

    /// Get all products in a specific rack
    async fn get_products_in_rack(
        &self,
        rack_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[ProductRack]>, ServiceError>;

    /// Get a specific product-rack relationship
    async fn get_product_rack_relationship(
        &self,
        product_id: Uuid,
        rack_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Option<ProductRack>, ServiceError>;

    /// Get all active product-rack relationships
    async fn get_all_relationships(
        &self,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[ProductRack]>, ServiceError>;
}
