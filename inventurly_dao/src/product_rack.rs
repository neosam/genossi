use async_trait::async_trait;
use std::sync::Arc;
use time::PrimitiveDateTime;
use uuid::Uuid;

use crate::DaoError;

#[derive(Debug, Clone, PartialEq)]
pub struct ProductRackEntity {
    pub product_id: Uuid,
    pub rack_id: Uuid,
    pub sort_order: i32,
    pub created: PrimitiveDateTime,
    pub deleted: Option<PrimitiveDateTime>,
    pub version: Uuid,
}

#[async_trait]
pub trait ProductRackDao {
    type Transaction: Send + Sync + Clone;

    /// Get all product-rack relationships (includes soft-deleted)
    async fn dump_all(&self, tx: Self::Transaction) -> Result<Arc<[ProductRackEntity]>, DaoError>;

    /// Create a new product-rack relationship
    async fn create(
        &self,
        entity: &ProductRackEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    /// Update an existing product-rack relationship
    async fn update(
        &self,
        entity: &ProductRackEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    /// Find product-rack relationship by product and rack IDs
    async fn find_by_product_and_rack(
        &self,
        product_id: Uuid,
        rack_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Option<ProductRackEntity>, DaoError>;

    /// Get all racks for a specific product (active relationships only)
    async fn find_racks_by_product(
        &self,
        product_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Arc<[ProductRackEntity]>, DaoError>;

    /// Get all products in a specific rack (active relationships only)
    async fn find_products_by_rack(
        &self,
        rack_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Arc<[ProductRackEntity]>, DaoError>;

    /// Get all active product-rack relationships
    async fn all(&self, tx: Self::Transaction) -> Result<Arc<[ProductRackEntity]>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        let active_entities: Vec<ProductRackEntity> = all_entities
            .iter()
            .filter(|entity| entity.deleted.is_none())
            .cloned()
            .collect();
        Ok(active_entities.into())
    }

    /// Get the next available sort_order for a rack
    async fn get_next_sort_order(
        &self,
        rack_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<i32, DaoError>;
}
