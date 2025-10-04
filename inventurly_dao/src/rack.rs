use std::sync::Arc;

use async_trait::async_trait;
use mockall::automock;
use uuid::Uuid;

use crate::DaoError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RackEntity {
    pub id: Uuid,
    pub name: Arc<str>,
    pub description: Arc<str>,
    pub created: time::PrimitiveDateTime,
    pub deleted: Option<time::PrimitiveDateTime>,
    pub version: Uuid,
}

#[automock(type Transaction = crate::MockTransaction;)]
#[async_trait]
pub trait RackDao {
    type Transaction: crate::Transaction;

    // Abstract methods - must be implemented by database-specific implementations
    async fn dump_all(&self, tx: Self::Transaction) -> Result<Arc<[RackEntity]>, DaoError>;
    
    async fn create(
        &self,
        entity: &RackEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;
    
    async fn update(
        &self,
        entity: &RackEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    // Default implementations - can be overridden if needed for optimization
    async fn all(&self, tx: Self::Transaction) -> Result<Arc<[RackEntity]>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        let active_entities: Vec<RackEntity> = all_entities
            .iter()
            .filter(|e| e.deleted.is_none())
            .cloned()
            .collect();
        Ok(active_entities.into())
    }

    async fn find_by_id(
        &self,
        id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Option<RackEntity>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        let found = all_entities
            .iter()
            .find(|e| e.id == id && e.deleted.is_none())
            .cloned();
        Ok(found)
    }
}