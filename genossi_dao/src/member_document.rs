use async_trait::async_trait;
use mockall::automock;
use std::sync::Arc;
use uuid::Uuid;

use crate::DaoError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberDocumentEntity {
    pub id: Uuid,
    pub member_id: Uuid,
    pub document_type: Arc<str>,
    pub description: Option<Arc<str>>,
    pub file_name: Arc<str>,
    pub mime_type: Arc<str>,
    pub relative_path: Arc<str>,
    pub created: time::PrimitiveDateTime,
    pub deleted: Option<time::PrimitiveDateTime>,
    pub version: Uuid,
}

#[automock(type Transaction = crate::MockTransaction;)]
#[async_trait]
pub trait MemberDocumentDao {
    type Transaction: crate::Transaction;

    async fn dump_all(&self, tx: Self::Transaction)
        -> Result<Arc<[MemberDocumentEntity]>, DaoError>;

    async fn create(
        &self,
        entity: &MemberDocumentEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    async fn update(
        &self,
        entity: &MemberDocumentEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    async fn all(&self, tx: Self::Transaction) -> Result<Arc<[MemberDocumentEntity]>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        let active: Vec<MemberDocumentEntity> = all_entities
            .iter()
            .filter(|e| e.deleted.is_none())
            .cloned()
            .collect();
        Ok(active.into())
    }

    async fn find_by_id(
        &self,
        id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Option<MemberDocumentEntity>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        Ok(all_entities
            .iter()
            .find(|e| e.id == id && e.deleted.is_none())
            .cloned())
    }

    async fn find_by_member_id(
        &self,
        member_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Arc<[MemberDocumentEntity]>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        let filtered: Vec<MemberDocumentEntity> = all_entities
            .iter()
            .filter(|e| e.member_id == member_id && e.deleted.is_none())
            .cloned()
            .collect();
        Ok(filtered.into())
    }
}
