use async_trait::async_trait;
use std::sync::Arc;

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

#[async_trait]
pub trait BackupDao: Send + Sync {
    async fn members_at_date(&self, date: time::Date) -> Result<Arc<[MemberBackupRow]>, DaoError>;
    async fn all_actions(&self) -> Result<Arc<[ActionBackupRow]>, DaoError>;
    async fn all_documents(&self) -> Result<Arc<[DocumentBackupRow]>, DaoError>;
}
