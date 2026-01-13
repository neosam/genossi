use async_trait::async_trait;
use mockall::automock;
use std::sync::Arc;
use time::PrimitiveDateTime;
use uuid::Uuid;

use crate::{permission::Authentication, ServiceError};

#[derive(Debug, Clone)]
pub struct InventurMeasurement {
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
    pub review_state: Arc<str>,
    pub created: PrimitiveDateTime,
    pub deleted: Option<PrimitiveDateTime>,
    pub version: Uuid,
}

impl From<&inventurly_dao::inventur_measurement::InventurMeasurementEntity>
    for InventurMeasurement
{
    fn from(
        entity: &inventurly_dao::inventur_measurement::InventurMeasurementEntity,
    ) -> Self {
        Self {
            id: entity.id,
            inventur_id: entity.inventur_id,
            product_id: entity.product_id,
            rack_id: entity.rack_id,
            container_id: entity.container_id,
            count: entity.count,
            weight_grams: entity.weight_grams,
            measured_by: entity.measured_by.clone(),
            measured_at: entity.measured_at,
            notes: entity.notes.clone(),
            review_state: entity.review_state.clone(),
            created: entity.created,
            deleted: entity.deleted,
            version: entity.version,
        }
    }
}

impl From<&InventurMeasurement>
    for inventurly_dao::inventur_measurement::InventurMeasurementEntity
{
    fn from(measurement: &InventurMeasurement) -> Self {
        Self {
            id: measurement.id,
            inventur_id: measurement.inventur_id,
            product_id: measurement.product_id,
            rack_id: measurement.rack_id,
            container_id: measurement.container_id,
            count: measurement.count,
            weight_grams: measurement.weight_grams,
            measured_by: measurement.measured_by.clone(),
            measured_at: measurement.measured_at,
            notes: measurement.notes.clone(),
            review_state: measurement.review_state.clone(),
            created: measurement.created,
            deleted: measurement.deleted,
            version: measurement.version,
        }
    }
}

#[automock(type Context = MockContext; type Transaction = MockTransaction;)]
#[async_trait]
pub trait InventurMeasurementService: Send + Sync {
    type Context: Send + Sync;
    type Transaction: Send + Sync;

    async fn get_all(
        &self,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[InventurMeasurement]>, ServiceError>;

    async fn get_by_id(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<InventurMeasurement, ServiceError>;

    async fn get_by_inventur_id(
        &self,
        inventur_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[InventurMeasurement]>, ServiceError>;

    async fn get_by_product_and_inventur(
        &self,
        product_id: Uuid,
        inventur_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[InventurMeasurement]>, ServiceError>;

    async fn create(
        &self,
        item: &InventurMeasurement,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<InventurMeasurement, ServiceError>;

    async fn update(
        &self,
        item: &InventurMeasurement,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<InventurMeasurement, ServiceError>;

    async fn delete(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<(), ServiceError>;
}

mockall::mock! {
    pub Context {}
    impl Clone for Context {
        fn clone(&self) -> Self;
    }
    unsafe impl Send for Context {}
    unsafe impl Sync for Context {}
}

mockall::mock! {
    pub Transaction {}
    impl Clone for Transaction {
        fn clone(&self) -> Self;
    }
    unsafe impl Send for Transaction {}
    unsafe impl Sync for Transaction {}
}
