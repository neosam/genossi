use async_trait::async_trait;
use mockall::automock;
use std::sync::Arc;
use time::PrimitiveDateTime;
use uuid::Uuid;

use crate::{permission::Authentication, ServiceError};

#[derive(Debug, Clone)]
pub struct Inventur {
    pub id: Uuid,
    pub name: Arc<str>,
    pub description: Arc<str>,
    pub start_date: PrimitiveDateTime,
    pub end_date: Option<PrimitiveDateTime>,
    pub status: Arc<str>, // "draft", "active", "completed"
    pub created_by: Arc<str>,
    pub created: PrimitiveDateTime,
    pub deleted: Option<PrimitiveDateTime>,
    pub version: Uuid,
    pub token: Option<Arc<str>>, // Optional token for quick access during inventory
}

impl From<&inventurly_dao::inventur::InventurEntity> for Inventur {
    fn from(entity: &inventurly_dao::inventur::InventurEntity) -> Self {
        Self {
            id: entity.id,
            name: entity.name.clone(),
            description: entity.description.clone(),
            start_date: entity.start_date,
            end_date: entity.end_date,
            status: entity.status.clone(),
            created_by: entity.created_by.clone(),
            created: entity.created,
            deleted: entity.deleted,
            version: entity.version,
            token: entity.token.clone(),
        }
    }
}

impl From<&Inventur> for inventurly_dao::inventur::InventurEntity {
    fn from(inventur: &Inventur) -> Self {
        Self {
            id: inventur.id,
            name: inventur.name.clone(),
            description: inventur.description.clone(),
            start_date: inventur.start_date,
            end_date: inventur.end_date,
            status: inventur.status.clone(),
            created_by: inventur.created_by.clone(),
            created: inventur.created,
            deleted: inventur.deleted,
            version: inventur.version,
            token: inventur.token.clone(),
        }
    }
}

#[automock(type Context = MockContext; type Transaction = MockTransaction;)]
#[async_trait]
pub trait InventurService: Send + Sync {
    type Context: Send + Sync;
    type Transaction: Send + Sync;

    async fn get_all(
        &self,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[Inventur]>, ServiceError>;

    async fn get_by_id(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Inventur, ServiceError>;

    async fn get_by_status(
        &self,
        status: &str,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[Inventur]>, ServiceError>;

    async fn create(
        &self,
        item: &Inventur,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Inventur, ServiceError>;

    async fn update(
        &self,
        item: &Inventur,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Inventur, ServiceError>;

    async fn change_status(
        &self,
        id: Uuid,
        new_status: &str,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Inventur, ServiceError>;

    async fn delete(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<(), ServiceError>;

    async fn find_by_token(
        &self,
        token: &str,
        tx: Option<Self::Transaction>,
    ) -> Result<Option<Inventur>, ServiceError>;
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
