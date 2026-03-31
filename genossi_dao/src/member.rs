use async_trait::async_trait;
use mockall::automock;
use std::sync::Arc;
use uuid::Uuid;

use crate::DaoError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberEntity {
    pub id: Uuid,
    pub member_number: i64,
    pub first_name: Arc<str>,
    pub last_name: Arc<str>,
    pub email: Option<Arc<str>>,
    pub company: Option<Arc<str>>,
    pub comment: Option<Arc<str>>,
    pub street: Option<Arc<str>>,
    pub house_number: Option<Arc<str>>,
    pub postal_code: Option<Arc<str>>,
    pub city: Option<Arc<str>>,
    pub join_date: time::Date,
    pub shares_at_joining: i32,
    pub current_shares: i32,
    pub current_balance: i64,
    pub exit_date: Option<time::Date>,
    pub bank_account: Option<Arc<str>>,
    pub created: time::PrimitiveDateTime,
    pub deleted: Option<time::PrimitiveDateTime>,
    pub version: Uuid,
}

#[automock(type Transaction = crate::MockTransaction;)]
#[async_trait]
pub trait MemberDao {
    type Transaction: crate::Transaction;

    async fn dump_all(&self, tx: Self::Transaction) -> Result<Arc<[MemberEntity]>, DaoError>;

    async fn create(
        &self,
        entity: &MemberEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    async fn update(
        &self,
        entity: &MemberEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    async fn all(&self, tx: Self::Transaction) -> Result<Arc<[MemberEntity]>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        let active_entities: Vec<MemberEntity> = all_entities
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
    ) -> Result<Option<MemberEntity>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        Ok(all_entities
            .iter()
            .find(|e| e.id == id && e.deleted.is_none())
            .cloned())
    }

    async fn find_by_member_number(
        &self,
        member_number: i64,
        tx: Self::Transaction,
    ) -> Result<Option<MemberEntity>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        Ok(all_entities
            .iter()
            .find(|e| e.member_number == member_number && e.deleted.is_none())
            .cloned())
    }
}
