use async_trait::async_trait;
use mockall::automock;
use std::sync::Arc;
use uuid::Uuid;

use crate::DaoError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditTimestampEntry {
    pub id: Uuid,
    pub timestamp: time::PrimitiveDateTime,
    pub audit_hash: Arc<str>,
    pub audit_entry_count: i64,
    pub tsr_token: Option<Arc<[u8]>>,
    pub webdav_path: Option<Arc<str>>,
    pub status: Arc<str>,
}

#[automock(type Transaction = crate::MockTransaction;)]
#[async_trait]
pub trait AuditTimestampDao {
    type Transaction: crate::Transaction;

    async fn create(
        &self,
        entry: &AuditTimestampEntry,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    async fn get_latest(
        &self,
        tx: Self::Transaction,
    ) -> Result<Option<AuditTimestampEntry>, DaoError>;

    async fn get_all(
        &self,
        tx: Self::Transaction,
    ) -> Result<Arc<[AuditTimestampEntry]>, DaoError>;

    async fn get_by_id(
        &self,
        id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Option<AuditTimestampEntry>, DaoError>;

    async fn get_pending_upload(
        &self,
        tx: Self::Transaction,
    ) -> Result<Arc<[AuditTimestampEntry]>, DaoError>;

    async fn update_webdav_path(
        &self,
        id: Uuid,
        path: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;
}
