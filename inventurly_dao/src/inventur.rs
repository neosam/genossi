use async_trait::async_trait;
use std::sync::Arc;
use time::PrimitiveDateTime;
use uuid::Uuid;

use crate::DaoError;

#[derive(Debug, Clone)]
pub struct InventurEntity {
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

#[async_trait]
pub trait InventurDao: Send + Sync {
    type Transaction: Send + Sync;

    async fn dump_all(&self, tx: Self::Transaction) -> Result<Arc<[InventurEntity]>, DaoError>;

    async fn create(
        &self,
        entity: &InventurEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    async fn update(
        &self,
        entity: &InventurEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    // Default implementation that filters dump_all results
    async fn all(&self, tx: Self::Transaction) -> Result<Arc<[InventurEntity]>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        let active_entities: Vec<InventurEntity> = all_entities
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
    ) -> Result<Option<InventurEntity>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        Ok(all_entities
            .iter()
            .find(|e| e.deleted.is_none() && e.id == id)
            .cloned())
    }

    // Find inventurs by status
    async fn find_by_status(
        &self,
        status: &str,
        tx: Self::Transaction,
    ) -> Result<Arc<[InventurEntity]>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        let matching: Vec<InventurEntity> = all_entities
            .iter()
            .filter(|e| e.deleted.is_none() && e.status.as_ref() == status)
            .cloned()
            .collect();
        Ok(matching.into())
    }

    // Find inventur by token
    async fn find_by_token(
        &self,
        token: &str,
        tx: Self::Transaction,
    ) -> Result<Option<InventurEntity>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        Ok(all_entities
            .iter()
            .find(|e| e.deleted.is_none() && e.token.as_ref().map(|t| t.as_ref()) == Some(token))
            .cloned())
    }
}
