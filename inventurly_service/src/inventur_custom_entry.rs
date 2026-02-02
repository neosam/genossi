use async_trait::async_trait;
use mockall::automock;
use std::sync::Arc;
use time::PrimitiveDateTime;
use uuid::Uuid;

use crate::{permission::Authentication, ServiceError};

#[derive(Debug, Clone)]
pub struct InventurCustomEntry {
    pub id: Uuid,
    pub inventur_id: Uuid,
    pub custom_product_name: Arc<str>,
    pub rack_id: Option<Uuid>,
    pub container_id: Option<Uuid>,
    pub count: Option<i64>,
    pub weight_grams: Option<i64>,
    pub measured_by: Arc<str>,
    pub measured_at: PrimitiveDateTime,
    pub notes: Option<Arc<str>>,
    pub ean: Option<Arc<str>>,
    pub review_state: Arc<str>,
    pub created: PrimitiveDateTime,
    pub deleted: Option<PrimitiveDateTime>,
    pub version: Uuid,
}

impl From<&inventurly_dao::inventur_custom_entry::InventurCustomEntryEntity>
    for InventurCustomEntry
{
    fn from(
        entity: &inventurly_dao::inventur_custom_entry::InventurCustomEntryEntity,
    ) -> Self {
        Self {
            id: entity.id,
            inventur_id: entity.inventur_id,
            custom_product_name: entity.custom_product_name.clone(),
            rack_id: entity.rack_id,
            container_id: entity.container_id,
            count: entity.count,
            weight_grams: entity.weight_grams,
            measured_by: entity.measured_by.clone(),
            measured_at: entity.measured_at,
            notes: entity.notes.clone(),
            ean: entity.ean.clone(),
            review_state: entity.review_state.clone(),
            created: entity.created,
            deleted: entity.deleted,
            version: entity.version,
        }
    }
}

impl From<&InventurCustomEntry>
    for inventurly_dao::inventur_custom_entry::InventurCustomEntryEntity
{
    fn from(entry: &InventurCustomEntry) -> Self {
        Self {
            id: entry.id,
            inventur_id: entry.inventur_id,
            custom_product_name: entry.custom_product_name.clone(),
            rack_id: entry.rack_id,
            container_id: entry.container_id,
            count: entry.count,
            weight_grams: entry.weight_grams,
            measured_by: entry.measured_by.clone(),
            measured_at: entry.measured_at,
            notes: entry.notes.clone(),
            ean: entry.ean.clone(),
            review_state: entry.review_state.clone(),
            created: entry.created,
            deleted: entry.deleted,
            version: entry.version,
        }
    }
}

#[automock(type Context = MockContext; type Transaction = MockTransaction;)]
#[async_trait]
pub trait InventurCustomEntryService: Send + Sync {
    type Context: Send + Sync;
    type Transaction: Send + Sync;

    async fn get_all(
        &self,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[InventurCustomEntry]>, ServiceError>;

    async fn get_by_id(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<InventurCustomEntry, ServiceError>;

    async fn get_by_inventur_id(
        &self,
        inventur_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[InventurCustomEntry]>, ServiceError>;

    async fn get_by_ean_and_inventur_id(
        &self,
        ean: &str,
        inventur_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[InventurCustomEntry]>, ServiceError>;

    async fn create(
        &self,
        item: &InventurCustomEntry,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<InventurCustomEntry, ServiceError>;

    async fn update(
        &self,
        item: &InventurCustomEntry,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<InventurCustomEntry, ServiceError>;

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
