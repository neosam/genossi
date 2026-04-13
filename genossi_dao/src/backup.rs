use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

use crate::DaoError;

#[derive(Clone, Debug)]
pub struct MemberBackupRow {
    pub member_number: i64,
    pub salutation: Option<Arc<str>>,
    pub title: Option<Arc<str>>,
    pub first_name: Arc<str>,
    pub last_name: Arc<str>,
    pub company: Option<Arc<str>>,
    pub street: Option<Arc<str>>,
    pub house_number: Option<Arc<str>>,
    pub postal_code: Option<Arc<str>>,
    pub city: Option<Arc<str>>,
    pub email: Option<Arc<str>>,
    pub bank_account: Option<Arc<str>>,
    pub join_date: Arc<str>,
    pub exit_date: Option<Arc<str>>,
    pub shares_at_joining: i32,
    pub shares_at_date: i32,
    pub comment: Option<Arc<str>>,
}

#[derive(Clone, Debug)]
pub struct ActionBackupRow {
    pub member_number: i64,
    pub first_name: Arc<str>,
    pub last_name: Arc<str>,
    pub action_type: Arc<str>,
    pub date: Arc<str>,
    pub shares_change: i32,
    pub transfer_member_number: Option<i64>,
    pub effective_date: Option<Arc<str>>,
    pub comment: Option<Arc<str>>,
}

#[derive(Clone, Debug)]
pub struct DocumentBackupRow {
    pub member_number: i64,
    pub first_name: Arc<str>,
    pub last_name: Arc<str>,
    pub document_type: Arc<str>,
    pub file_name: Arc<str>,
    pub relative_path: Arc<str>,
}

#[derive(Clone, Debug)]
pub struct CommunicationBackupRow {
    pub member_number: i64,
    pub first_name: Arc<str>,
    pub last_name: Arc<str>,
    pub direction: Arc<str>,
    pub date: Arc<str>,
    pub subject: Arc<str>,
    pub body: Arc<str>,
    pub from_address: Option<Arc<str>>,
    pub to_address: Option<Arc<str>>,
    pub mail_id: Uuid,
    pub mail_type: Arc<str>,
}

#[async_trait]
pub trait BackupDao: Send + Sync {
    async fn members_at_date(&self, date: time::Date) -> Result<Arc<[MemberBackupRow]>, DaoError>;
    async fn all_actions(&self) -> Result<Arc<[ActionBackupRow]>, DaoError>;
    async fn all_documents(&self) -> Result<Arc<[DocumentBackupRow]>, DaoError>;
    async fn earliest_join_year(&self) -> Result<Option<i32>, DaoError>;
    async fn all_communications(&self) -> Result<Arc<[CommunicationBackupRow]>, DaoError>;
}

#[async_trait]
pub trait BackupCommunicationSyncDao: Send + Sync {
    async fn is_synced(&self, mail_type: &str, mail_id: Uuid) -> Result<bool, DaoError>;
    async fn mark_synced(&self, mail_type: &str, mail_id: Uuid) -> Result<(), DaoError>;
}

#[async_trait]
pub trait BackupDocumentSyncDao: Send + Sync {
    async fn get_hash(&self, relative_path: &str) -> Result<Option<Arc<str>>, DaoError>;
    async fn upsert_hash(
        &self,
        relative_path: &str,
        content_hash: &str,
        last_uploaded: &str,
    ) -> Result<(), DaoError>;
}
