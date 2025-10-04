use std::fmt::Debug;
use std::sync::Arc;

use async_trait::async_trait;
use mockall::automock;
use uuid::Uuid;

use crate::permission::Authentication;
use crate::ServiceError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rack {
    pub id: Uuid,
    pub name: Arc<str>,
    pub description: Arc<str>,
    pub created: time::PrimitiveDateTime,
    pub deleted: Option<time::PrimitiveDateTime>,
    pub version: Uuid,
}

impl From<&inventurly_dao::rack::RackEntity> for Rack {
    fn from(entity: &inventurly_dao::rack::RackEntity) -> Self {
        Self {
            id: entity.id,
            name: entity.name.clone(),
            description: entity.description.clone(),
            created: entity.created,
            deleted: entity.deleted,
            version: entity.version,
        }
    }
}

impl From<&Rack> for inventurly_dao::rack::RackEntity {
    fn from(rack: &Rack) -> Self {
        Self {
            id: rack.id,
            name: rack.name.clone(),
            description: rack.description.clone(),
            created: rack.created,
            deleted: rack.deleted,
            version: rack.version,
        }
    }
}

#[automock(type Context=(); type Transaction = inventurly_dao::MockTransaction;)]
#[async_trait]
pub trait RackService {
    type Context: Clone + Debug + PartialEq + Eq + Send + Sync + 'static;
    type Transaction: inventurly_dao::Transaction;

    async fn get_all(
        &self,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[Rack]>, ServiceError>;

    async fn get_by_id(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Option<Rack>, ServiceError>;

    async fn create(
        &self,
        rack: &Rack,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Rack, ServiceError>;

    async fn update(
        &self,
        rack: &Rack,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Rack, ServiceError>;

    async fn delete(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<(), ServiceError>;
}