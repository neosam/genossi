use async_trait::async_trait;
use mockall::automock;
use std::sync::Arc;
use uuid::Uuid;

/// Normalize a Message-ID header value by stripping surrounding angle brackets
/// and whitespace. Returns `None` for empty input.
pub fn normalize_message_id(raw: &str) -> Option<String> {
    let trimmed = raw
        .trim()
        .trim_start_matches('<')
        .trim_end_matches('>')
        .trim();
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
    // Phase 23 D-07: optional HTML body. NULL = text-only mail (legacy contract).
    // Sanitized by ammonia at all entry points before it lands here.
    pub body_html: Option<Arc<str>>,
    pub status: Arc<str>,
    pub total_count: i64,
    pub sent_count: i64,
    pub failed_count: i64,
    pub reply_to_inbound_mail_id: Option<Uuid>,
    // Phase 10 D-12: optional template reference (NULL for ad-hoc / non-template mails)
    pub template_id: Option<Uuid>,
    // Phase 10 D-03: optional repayment-phase reference for worker-side aggregation
    pub repayment_phase_id: Option<Uuid>,
    // Quick 260603-cz6: opt-in flag — when true (and repayment_phase_id is Some),
    // the worker resolves the per-recipient RepaymentLetter MemberDocument
    // (Description-Fingerprint "Anschreiben Auszahlung GJ {fy}") and attaches it
    // in-memory. Persisted so retries survive Worker restarts.
    pub attach_repayment_letter: bool,
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
    // Phase 29 (APHIST-01): nullable Geschwisterspalte zu member_id. Gesetzt (Some)
    // wenn diese Mail an einen Antragsteller (Application) geht; sonst None. Trennt
    // den Application-Namespace sauber vom member_id-Namespace (eine Application-UUID
    // landet NIE in member_id).
    pub application_id: Option<Uuid>,
    pub status: Arc<str>,
    pub error: Option<Arc<str>>,
    pub sent_at: Option<time::PrimitiveDateTime>,
    pub message_id: Option<Arc<str>>,
    // Quick 260614-9zf: per-recipient rendered subject/body, persisted by the worker
    // after the template render. None for legacy rows and not-yet-rendered recipients.
    pub rendered_subject: Option<Arc<str>>,
    pub rendered_body: Option<Arc<str>>,
    // Phase 23 D-08: per-recipient rendered HTML body, persisted by the worker at
    // send time when the underlying job carries a body_html. None for text-only
    // jobs and legacy rows (pre-migration).
    pub rendered_html_body: Option<Arc<str>>,
    // Quick 260614-b1t: true when the rendered_subject/rendered_body were filled
    // retroactively by the startup backfill (reconstruction, not the byte-accurate
    // original from the send moment). false for live worker renders and not-yet-rendered
    // rows. NOT NULL DEFAULT 0 in the DB (legacy rows read back as false).
    pub rendered_reconstructed: bool,
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
    /// Quick 260614-b1t: all recipients whose rendered subject AND body are still
    /// NULL (and not soft-deleted). Used by the startup backfill to retroactively
    /// render legacy rows. Includes status='failed' rows — they were rendered at
    /// send time too, just not persisted before 9zf.
    async fn find_recipients_without_rendered(&self) -> Result<Arc<[MailRecipient]>, MailDaoError>;
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

// ────────────────────────────────────────────────────────────────────────────
// Inbound mail attachments (Phase 19 — Backend für Inbox-Attachment-Anzeige)
// ────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InboundMailAttachment {
    pub id: Uuid,
    pub inbound_mail_id: Uuid,
    pub created: time::PrimitiveDateTime,
    pub file_name: Arc<str>,
    pub mime_type: Arc<str>,
    pub size_bytes: i64,
    pub relative_path: Option<Arc<str>>, // NULL when oversized=true (D-02)
    pub oversized: bool,                 // D-02 hard 10 MB cap marker
}

#[automock]
#[async_trait]
pub trait InboundMailAttachmentDao: Send + Sync + 'static {
    async fn create(&self, attachment: &InboundMailAttachment) -> Result<(), MailDaoError>;
    async fn find_by_inbound_mail_id(
        &self,
        inbound_mail_id: Uuid,
    ) -> Result<Arc<[InboundMailAttachment]>, MailDaoError>;
    async fn find_by_id_and_mail(
        &self,
        mail_id: Uuid,
        attachment_id: Uuid,
    ) -> Result<Option<InboundMailAttachment>, MailDaoError>;
    async fn count_for_mail(&self, mail_id: Uuid) -> Result<i64, MailDaoError>;
}

// ────────────────────────────────────────────────────────────────────────────
// Digest state (Phase 20 — D-03: persistiertes letztes Digest-Versanddatum)
// ────────────────────────────────────────────────────────────────────────────

#[automock]
#[async_trait]
pub trait DigestStateDao: Send + Sync + 'static {
    /// Letztes Digest-Versanddatum (None = noch nie gesendet).
    async fn get_last_sent_date(&self) -> Result<Option<time::Date>, MailDaoError>;
    /// Setzt (upsert) das letzte Digest-Versanddatum.
    async fn set_last_sent_date(&self, date: time::Date) -> Result<(), MailDaoError>;
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
    async fn find_many_by_ids(&self, ids: &[Uuid]) -> Result<Arc<[StaticDocument]>, MailDaoError>;
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
// Mail templates
// ────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailTemplate {
    pub id: Uuid,
    pub created: time::PrimitiveDateTime,
    pub deleted: Option<time::PrimitiveDateTime>,
    pub version: Uuid,
    pub name: Arc<str>,
    pub subject: Arc<str>,
    pub body: Arc<str>,
    // Phase 23 D-06: optional HTML body. NULL = text-only template (legacy contract).
    // Sanitized by ammonia at template create/update entry points before it lands here.
    pub body_html: Option<Arc<str>>,
}

#[automock]
#[async_trait]
pub trait MailTemplateDao: Send + Sync + 'static {
    async fn create(&self, template: &MailTemplate) -> Result<(), MailDaoError>;
    async fn update(&self, template: &MailTemplate) -> Result<(), MailDaoError>;
    async fn dump_all(&self) -> Result<Arc<[MailTemplate]>, MailDaoError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<MailTemplate>, MailDaoError>;
    async fn all(&self) -> Result<Arc<[MailTemplate]>, MailDaoError>;
    async fn find_by_name(&self, name: &str) -> Result<Option<MailTemplate>, MailDaoError>;
}

// ────────────────────────────────────────────────────────────────────────────
// Communication timeline (unified view per member)
// ────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommunicationDirection {
    Inbound,
    Outbound,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommunicationEntry {
    pub direction: CommunicationDirection,
    pub date: time::PrimitiveDateTime,
    pub subject: Arc<str>,

    // Inbound-specific
    pub inbox_id: Option<Uuid>,
    pub from_address: Option<Arc<str>>,
    pub inbound_done: Option<bool>,
    pub inbound_replied: Option<bool>,
    pub inbound_archived: Option<bool>,

    // Outbound-specific
    pub mail_job_id: Option<Uuid>,
    pub recipient_id: Option<Uuid>,
    pub to_address: Option<Arc<str>>,
    pub outbound_status: Option<Arc<str>>,
}

#[automock]
#[async_trait]
pub trait CommunicationDao: Send + Sync + 'static {
    async fn get_member_communications(
        &self,
        member_id: Uuid,
    ) -> Result<Arc<[CommunicationEntry]>, MailDaoError>;
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
    async fn exists_by_uid(&self, uid_validity: i64, imap_uid: i64) -> Result<bool, MailDaoError>;
    async fn max_uid_for_validity(&self, uid_validity: i64) -> Result<Option<i64>, MailDaoError>;
    async fn update(&self, mail: &InboundMail) -> Result<(), MailDaoError>;
}
