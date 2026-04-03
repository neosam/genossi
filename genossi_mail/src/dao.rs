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
pub struct SentMail {
    pub id: Uuid,
    pub created: time::PrimitiveDateTime,
    pub deleted: Option<time::PrimitiveDateTime>,
    pub version: Uuid,
    pub to_address: Arc<str>,
    pub subject: Arc<str>,
    pub body: Arc<str>,
    pub status: Arc<str>,
    pub error: Option<Arc<str>>,
    pub sent_at: Option<time::PrimitiveDateTime>,
}

#[automock]
#[async_trait]
pub trait SentMailDao: Send + Sync + 'static {
    async fn all(&self) -> Result<Arc<[SentMail]>, MailDaoError>;
    async fn create(&self, mail: &SentMail) -> Result<(), MailDaoError>;
}
