use async_trait::async_trait;
use std::sync::Arc;
use time::PrimitiveDateTime;
use uuid::Uuid;

use crate::DaoError;

#[derive(Debug, Clone)]
pub struct InventurMeasurementEntity {
    pub id: Uuid,
    pub inventur_id: Uuid,
    pub product_id: Uuid,
    pub rack_id: Option<Uuid>,
    pub container_id: Option<Uuid>,
    pub count: Option<i64>,
    pub weight_grams: Option<i64>,
    pub measured_by: Arc<str>,
    pub measured_at: PrimitiveDateTime,
    pub notes: Option<Arc<str>>,
    pub created: PrimitiveDateTime,
    pub deleted: Option<PrimitiveDateTime>,
    pub version: Uuid,
}

#[async_trait]
pub trait InventurMeasurementDao: Send + Sync {
    type Transaction: Send + Sync;

    async fn dump_all(
        &self,
        tx: Self::Transaction,
    ) -> Result<Arc<[InventurMeasurementEntity]>, DaoError>;

    async fn create(
        &self,
        entity: &InventurMeasurementEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    async fn update(
        &self,
        entity: &InventurMeasurementEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    // Default implementation that filters dump_all results
    async fn all(
        &self,
        tx: Self::Transaction,
    ) -> Result<Arc<[InventurMeasurementEntity]>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        let active_entities: Vec<InventurMeasurementEntity> = all_entities
            .iter()
            .filter(|e| e.deleted.is_none())
            .cloned()
            .collect();
        Ok(active_entities.into())
    }

    // Default implementation that finds by ID from dump_all results
    async fn find_by_id(
        &self,
        id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Option<InventurMeasurementEntity>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        Ok(all_entities
            .iter()
            .find(|e| e.deleted.is_none() && e.id == id)
            .cloned())
    }

    // Find all measurements for a specific inventur
    async fn find_by_inventur_id(
        &self,
        inventur_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Arc<[InventurMeasurementEntity]>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        let matching: Vec<InventurMeasurementEntity> = all_entities
            .iter()
            .filter(|e| e.deleted.is_none() && e.inventur_id == inventur_id)
            .cloned()
            .collect();
        Ok(matching.into())
    }

    // Find measurements for a specific product within an inventur
    async fn find_by_product_and_inventur(
        &self,
        product_id: Uuid,
        inventur_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Arc<[InventurMeasurementEntity]>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        let matching: Vec<InventurMeasurementEntity> = all_entities
            .iter()
            .filter(|e| {
                e.deleted.is_none()
                    && e.product_id == product_id
                    && e.inventur_id == inventur_id
            })
            .cloned()
            .collect();
        Ok(matching.into())
    }

    // Find measurements for a specific rack within an inventur
    async fn find_by_rack_and_inventur(
        &self,
        rack_id: Uuid,
        inventur_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Arc<[InventurMeasurementEntity]>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        let matching: Vec<InventurMeasurementEntity> = all_entities
            .iter()
            .filter(|e| {
                e.deleted.is_none()
                    && e.rack_id == Some(rack_id)
                    && e.inventur_id == inventur_id
            })
            .cloned()
            .collect();
        Ok(matching.into())
    }
}
