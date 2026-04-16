use async_trait::async_trait;
use mockall::automock;
use std::sync::Arc;
use uuid::Uuid;

use crate::DaoError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditLogEntry {
    pub id: Uuid,
    pub timestamp: time::PrimitiveDateTime,
    pub user_id: Arc<str>,
    pub process: Arc<str>,
    pub transaction_id: Uuid,
    pub entity_type: Arc<str>,
    pub entity_id: Uuid,
    pub action: Arc<str>,
    pub field_name: Arc<str>,
    pub old_value: Option<Arc<str>>,
    pub new_value: Option<Arc<str>>,
    pub prev_hash: Arc<str>,
    pub entry_hash: Arc<str>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AuditQueryFilter {
    pub entity_type: Option<String>,
    pub entity_id: Option<Uuid>,
    pub user_id: Option<String>,
    pub action: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
}

#[automock(type Transaction = crate::MockTransaction;)]
#[async_trait]
pub trait AuditLogDao {
    type Transaction: crate::Transaction;

    async fn create_entries(
        &self,
        entries: &[AuditLogEntry],
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    async fn get_latest_hash(
        &self,
        tx: Self::Transaction,
    ) -> Result<Option<String>, DaoError>;

    async fn get_by_entity(
        &self,
        entity_type: &str,
        entity_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Arc<[AuditLogEntry]>, DaoError>;

    async fn get_all_ordered(
        &self,
        tx: Self::Transaction,
    ) -> Result<Arc<[AuditLogEntry]>, DaoError>;

    async fn query(
        &self,
        filter: AuditQueryFilter,
        limit: i64,
        offset: i64,
        tx: Self::Transaction,
    ) -> Result<Arc<[AuditLogEntry]>, DaoError>;

    async fn count(
        &self,
        filter: AuditQueryFilter,
        tx: Self::Transaction,
    ) -> Result<i64, DaoError>;
}
