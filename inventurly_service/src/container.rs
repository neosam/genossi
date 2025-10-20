use async_trait::async_trait;
use mockall::automock;
use std::sync::Arc;
use time::PrimitiveDateTime;
use uuid::Uuid;

use crate::{permission::Authentication, ServiceError};

#[derive(Debug, Clone)]
pub struct Container {
    pub id: Uuid,
    pub name: Arc<str>,
    pub weight_grams: i64,
    pub description: Arc<str>,
    pub created: PrimitiveDateTime,
    pub deleted: Option<PrimitiveDateTime>,
    pub version: Uuid,
}

impl From<&inventurly_dao::container::ContainerEntity> for Container {
    fn from(entity: &inventurly_dao::container::ContainerEntity) -> Self {
        Self {
            id: entity.id,
            name: entity.name.clone(),
            weight_grams: entity.weight_grams,
            description: entity.description.clone(),
            created: entity.created,
            deleted: entity.deleted,
            version: entity.version,
        }
    }
}

impl From<&Container> for inventurly_dao::container::ContainerEntity {
    fn from(container: &Container) -> Self {
        Self {
            id: container.id,
            name: container.name.clone(),
            weight_grams: container.weight_grams,
            description: container.description.clone(),
            created: container.created,
            deleted: container.deleted,
            version: container.version,
        }
    }
}

#[automock(type Context = MockContext; type Transaction = MockTransaction;)]
#[async_trait]
pub trait ContainerService: Send + Sync {
    type Context: Send + Sync;
    type Transaction: Send + Sync;

    async fn get_all(
        &self,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[Container]>, ServiceError>;

    async fn get_by_id(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Container, ServiceError>;

    async fn get_by_name(
        &self,
        name: &str,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Container, ServiceError>;

    async fn create(
        &self,
        item: &Container,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Container, ServiceError>;

    async fn update(
        &self,
        item: &Container,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Container, ServiceError>;

    async fn delete(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<(), ServiceError>;

    async fn search(
        &self,
        query: &str,
        limit: Option<usize>,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[Container]>, ServiceError>;
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
