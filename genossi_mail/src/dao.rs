use async_trait::async_trait;
use mockall::automock;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum MailDaoError {
    DatabaseError(Arc<str>),
    NotFound,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailJob {
    pub id: Uuid,
    pub created: time::PrimitiveDateTime,
    pub deleted: Option<time::PrimitiveDateTime>,
    pub version: Uuid,
    pub subject: Arc<str>,
    pub body: Arc<str>,
    pub status: Arc<str>,
    pub total_count: i64,
    pub sent_count: i64,
    pub failed_count: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailRecipient {
    pub id: Uuid,
    pub created: time::PrimitiveDateTime,
    pub deleted: Option<time::PrimitiveDateTime>,
    pub version: Uuid,
    pub mail_job_id: Uuid,
    pub to_address: Arc<str>,
    pub member_id: Option<Uuid>,
    pub status: Arc<str>,
    pub error: Option<Arc<str>>,
    pub sent_at: Option<time::PrimitiveDateTime>,
}

#[automock]
#[async_trait]
pub trait MailJobDao: Send + Sync + 'static {
    async fn create(&self, job: &MailJob) -> Result<(), MailDaoError>;
    async fn find_by_id(&self, id: Uuid) -> Result<MailJob, MailDaoError>;
    async fn all(&self) -> Result<Arc<[MailJob]>, MailDaoError>;
    async fn update(&self, job: &MailJob) -> Result<(), MailDaoError>;
}

#[automock]
#[async_trait]
pub trait MailRecipientDao: Send + Sync + 'static {
    async fn create(&self, recipient: &MailRecipient) -> Result<(), MailDaoError>;
    async fn find_by_job_id(&self, job_id: Uuid) -> Result<Arc<[MailRecipient]>, MailDaoError>;
    async fn next_pending(&self) -> Result<Option<MailRecipient>, MailDaoError>;
    async fn update(&self, recipient: &MailRecipient) -> Result<(), MailDaoError>;
}
