use async_trait::async_trait;
use mockall::automock;
use std::sync::Arc;
use uuid::Uuid;

/// Normalize a Message-ID header value by stripping surrounding angle brackets
/// and whitespace. Returns `None` for empty input.
pub fn normalize_message_id(raw: &str) -> Option<String> {
    let trimmed = raw.trim().trim_start_matches('<').trim_end_matches('>').trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

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
    pub reply_to_inbound_mail_id: Option<Uuid>,
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
    pub message_id: Option<Arc<str>>,
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
    async fn find_sent_member_ids_by_job_id(
        &self,
        job_id: Uuid,
    ) -> Result<Arc<[Uuid]>, MailDaoError>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailRecipientAttachment {
    pub recipient_id: Uuid,
    pub document_id: Uuid,
    pub file_name: Arc<str>,
    pub mime_type: Arc<str>,
    pub relative_path: Arc<str>,
}

#[automock]
#[async_trait]
pub trait MailRecipientAttachmentDao: Send + Sync + 'static {
    async fn create(&self, attachment: &MailRecipientAttachment) -> Result<(), MailDaoError>;
    async fn find_by_recipient_id(
        &self,
        recipient_id: Uuid,
    ) -> Result<Arc<[MailRecipientAttachment]>, MailDaoError>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StaticDocument {
    pub id: Uuid,
    pub created: time::PrimitiveDateTime,
    pub deleted: Option<time::PrimitiveDateTime>,
    pub version: Uuid,
    pub name: Arc<str>,
    pub filename: Arc<str>,
    pub content_type: Arc<str>,
    pub size_bytes: i64,
}

impl StaticDocument {
    /// Relative path inside the DocumentStorage where the file bytes live.
    pub fn relative_path(&self) -> String {
        format!("static_documents/{}", self.id)
    }
}

#[automock]
#[async_trait]
pub trait StaticDocumentDao: Send + Sync + 'static {
    async fn create(&self, doc: &StaticDocument) -> Result<(), MailDaoError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<StaticDocument>, MailDaoError>;
    async fn find_many_by_ids(
        &self,
        ids: &[Uuid],
    ) -> Result<Arc<[StaticDocument]>, MailDaoError>;
    async fn all_active(&self) -> Result<Arc<[StaticDocument]>, MailDaoError>;
    async fn soft_delete(&self, id: Uuid) -> Result<(), MailDaoError>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailJobStaticAttachment {
    pub mail_job_id: Uuid,
    pub static_document_id: Uuid,
}

#[automock]
#[async_trait]
pub trait MailJobStaticAttachmentDao: Send + Sync + 'static {
    async fn create(&self, entity: &MailJobStaticAttachment) -> Result<(), MailDaoError>;
    async fn find_static_documents_by_job_id(
        &self,
        mail_job_id: Uuid,
    ) -> Result<Arc<[StaticDocument]>, MailDaoError>;
}

// ────────────────────────────────────────────────────────────────────────────
// Inbound mails (member-inbox)
// ────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InboundMail {
    pub id: Uuid,
    pub created: time::PrimitiveDateTime,
    pub version: Uuid,
    pub uid_validity: i64,
    pub imap_uid: i64,
    pub from_address: Arc<str>,
    pub subject: Arc<str>,
    pub received_at: time::PrimitiveDateTime,
    pub body_text: Arc<str>,
    pub has_attachments: bool,
    pub has_html_body: bool,
    pub raw_html_body: Option<Arc<str>>,
    pub in_reply_to: Option<Arc<str>>,
    pub message_id: Option<Arc<str>>,
    pub replied: bool,
    pub done: bool,
    pub archived: bool,
    pub assigned_member_id: Option<Uuid>,
}

#[automock]
#[async_trait]
pub trait InboundMailDao: Send + Sync + 'static {
    async fn create(&self, mail: &InboundMail) -> Result<(), MailDaoError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<InboundMail>, MailDaoError>;
    /// All inbound mails, ordered by received_at DESC.
    async fn list_active(&self) -> Result<Arc<[InboundMail]>, MailDaoError>;
    async fn exists_by_uid(
        &self,
        uid_validity: i64,
        imap_uid: i64,
    ) -> Result<bool, MailDaoError>;
    async fn max_uid_for_validity(
        &self,
        uid_validity: i64,
    ) -> Result<Option<i64>, MailDaoError>;
    async fn update(&self, mail: &InboundMail) -> Result<(), MailDaoError>;
}
