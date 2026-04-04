use async_trait::async_trait;
use mockall::automock;
use std::sync::Arc;
use uuid::Uuid;

use crate::DaoError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserPreferenceEntity {
    pub id: Uuid,
    pub user_id: Arc<str>,
    pub key: Arc<str>,
    pub value: Arc<str>,
    pub created: time::PrimitiveDateTime,
    pub deleted: Option<time::PrimitiveDateTime>,
    pub version: Uuid,
}

#[automock(type Transaction = crate::MockTransaction;)]
#[async_trait]
pub trait UserPreferenceDao {
    type Transaction: crate::Transaction;

    async fn dump_all(
        &self,
        tx: Self::Transaction,
    ) -> Result<Arc<[UserPreferenceEntity]>, DaoError>;

    async fn create(
        &self,
        entity: &UserPreferenceEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    async fn update(
        &self,
        entity: &UserPreferenceEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    async fn all(
        &self,
        tx: Self::Transaction,
    ) -> Result<Arc<[UserPreferenceEntity]>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        let active_entities: Vec<UserPreferenceEntity> = all_entities
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
    ) -> Result<Option<UserPreferenceEntity>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        Ok(all_entities
            .iter()
            .find(|e| e.id == id && e.deleted.is_none())
            .cloned())
    }

    async fn find_by_user_and_key(
        &self,
        user_id: &str,
        key: &str,
        tx: Self::Transaction,
    ) -> Result<Option<UserPreferenceEntity>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        Ok(all_entities
            .iter()
            .find(|e| *e.user_id == *user_id && *e.key == *key && e.deleted.is_none())
            .cloned())
    }
}
