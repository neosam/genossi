use async_trait::async_trait;
use sqlx::SqlitePool;
use std::sync::Arc;
use time::PrimitiveDateTime;
use uuid::Uuid;

use crate::dao::{
    CommunicationDao, CommunicationDirection, CommunicationEntry, DigestStateDao, InboundMail,
    InboundMailAttachment, InboundMailAttachmentDao, InboundMailDao, MailDaoError, MailJob,
    MailJobDao, MailJobStaticAttachment, MailJobStaticAttachmentDao, MailRecipient,
    MailRecipientAttachment, MailRecipientAttachmentDao, MailRecipientDao, MailTemplate,
    MailTemplateDao, StaticDocument, StaticDocumentDao,
};

fn parse_datetime(s: &str) -> Result<PrimitiveDateTime, time::error::Parse> {
    if let Ok(dt) =
        PrimitiveDateTime::parse(s, &time::format_description::well_known::Iso8601::DEFAULT)
    {
        return Ok(dt);
    }
    let sqlite_format = time::format_description::parse(
        "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond]",
    )
    .unwrap();
    if let Ok(dt) = PrimitiveDateTime::parse(s, &sqlite_format) {
        return Ok(dt);
    }
    let sqlite_simple =
        time::format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]").unwrap();
    PrimitiveDateTime::parse(s, &sqlite_simple)
}

fn format_datetime(dt: &PrimitiveDateTime) -> Result<String, MailDaoError> {
    let format = &time::format_description::well_known::Iso8601::DEFAULT;
    dt.assume_utc()
        .format(format)
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))
}

fn parse_optional_datetime(s: &Option<String>) -> Result<Option<PrimitiveDateTime>, MailDaoError> {
    s.as_ref()
        .map(|d| parse_datetime(d))
        .transpose()
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))
}

fn parse_optional_uuid(bytes: &Option<Vec<u8>>) -> Result<Option<Uuid>, MailDaoError> {
    bytes
        .as_ref()
        .map(|b| Uuid::from_slice(b))
        .transpose()
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))
}

fn parse_uuid(bytes: &[u8]) -> Result<Uuid, MailDaoError> {
    Uuid::from_slice(bytes).map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))
}

// MailJob SQLite

#[derive(Debug, sqlx::FromRow)]
struct MailJobDb {
    id: Vec<u8>,
    created: String,
    deleted: Option<String>,
    version: Vec<u8>,
    subject: String,
    body: String,
    status: String,
    total_count: i64,
    sent_count: i64,
    failed_count: i64,
    reply_to_inbound_mail_id: Option<Vec<u8>>,
    // Phase 10 D-12 / D-03
    template_id: Option<Vec<u8>>,
    repayment_phase_id: Option<Vec<u8>>,
    // Quick 260603-cz6: opt-in bool flag (SQLite INTEGER 0/1)
    attach_repayment_letter: i64,
    // Phase 23 D-07: optional HTML body
    body_html: Option<String>,
}

impl TryFrom<&MailJobDb> for MailJob {
    type Error = MailDaoError;

    fn try_from(db: &MailJobDb) -> Result<Self, Self::Error> {
        Ok(MailJob {
            id: parse_uuid(&db.id)?,
            created: parse_datetime(&db.created)
                .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?,
            deleted: parse_optional_datetime(&db.deleted)?,
            version: parse_uuid(&db.version)?,
            subject: Arc::from(db.subject.as_str()),
            body: Arc::from(db.body.as_str()),
            status: Arc::from(db.status.as_str()),
            total_count: db.total_count,
            sent_count: db.sent_count,
            failed_count: db.failed_count,
            reply_to_inbound_mail_id: parse_optional_uuid(&db.reply_to_inbound_mail_id)?,
            template_id: parse_optional_uuid(&db.template_id)?,
            repayment_phase_id: parse_optional_uuid(&db.repayment_phase_id)?,
            attach_repayment_letter: db.attach_repayment_letter != 0,
            body_html: db.body_html.as_deref().map(Arc::from),
        })
    }
}

pub struct MailJobDaoSqlite {
    pool: Arc<SqlitePool>,
}

impl MailJobDaoSqlite {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MailJobDao for MailJobDaoSqlite {
    async fn create(&self, job: &MailJob) -> Result<(), MailDaoError> {
        let id = job.id.as_bytes().to_vec();
        let version = job.version.as_bytes().to_vec();
        let created = format_datetime(&job.created)?;

        let reply_to = job.reply_to_inbound_mail_id.map(|u| u.as_bytes().to_vec());
        let template_id = job.template_id.map(|u| u.as_bytes().to_vec());
        let repayment_phase_id = job.repayment_phase_id.map(|u| u.as_bytes().to_vec());
        let attach_repayment_letter: i64 = if job.attach_repayment_letter { 1 } else { 0 };

        sqlx::query(
            "INSERT INTO mail_jobs (id, created, deleted, version, subject, body, status, total_count, sent_count, failed_count, reply_to_inbound_mail_id, template_id, repayment_phase_id, attach_repayment_letter, body_html) \
             VALUES (?, ?, NULL, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(created)
        .bind(version)
        .bind(job.subject.as_ref())
        .bind(job.body.as_ref())
        .bind(job.status.as_ref())
        .bind(job.total_count)
        .bind(job.sent_count)
        .bind(job.failed_count)
        .bind(reply_to)
        .bind(template_id)
        .bind(repayment_phase_id)
        .bind(attach_repayment_letter)
        .bind(job.body_html.as_deref())
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<MailJob, MailDaoError> {
        let id_bytes = id.as_bytes().to_vec();
        let row = sqlx::query_as::<_, MailJobDb>(
            "SELECT id, created, deleted, version, subject, body, status, total_count, sent_count, failed_count, reply_to_inbound_mail_id, template_id, repayment_phase_id, attach_repayment_letter, body_html \
             FROM mail_jobs WHERE id = ?",
        )
        .bind(id_bytes)
        .fetch_optional(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?
        .ok_or(MailDaoError::NotFound)?;

        MailJob::try_from(&row)
    }

    async fn all(&self) -> Result<Arc<[MailJob]>, MailDaoError> {
        let rows = sqlx::query_as::<_, MailJobDb>(
            "SELECT id, created, deleted, version, subject, body, status, total_count, sent_count, failed_count, reply_to_inbound_mail_id, template_id, repayment_phase_id, attach_repayment_letter, body_html \
             FROM mail_jobs ORDER BY created DESC",
        )
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;

        rows.iter()
            .map(MailJob::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into())
    }

    async fn update(&self, job: &MailJob) -> Result<(), MailDaoError> {
        let id = job.id.as_bytes().to_vec();
        let version = job.version.as_bytes().to_vec();

        sqlx::query(
            "UPDATE mail_jobs SET status = ?, sent_count = ?, failed_count = ?, version = ? WHERE id = ?",
        )
        .bind(job.status.as_ref())
        .bind(job.sent_count)
        .bind(job.failed_count)
        .bind(version)
        .bind(id)
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(())
    }
}

// MailRecipient SQLite

#[derive(Debug, sqlx::FromRow)]
struct MailRecipientDb {
    id: Vec<u8>,
    created: String,
    deleted: Option<String>,
    version: Vec<u8>,
    mail_job_id: Vec<u8>,
    to_address: String,
    member_id: Option<Vec<u8>>,
    // Phase 29 (APHIST-01): optional Application-Linkage, spiegelbildlich zu member_id
    application_id: Option<Vec<u8>>,
    status: String,
    error: Option<String>,
    sent_at: Option<String>,
    message_id: Option<String>,
    rendered_subject: Option<String>,
    rendered_body: Option<String>,
    rendered_reconstructed: i64,
    // Phase 23 D-08: optional per-recipient rendered HTML body
    rendered_html_body: Option<String>,
}

impl TryFrom<&MailRecipientDb> for MailRecipient {
    type Error = MailDaoError;

    fn try_from(db: &MailRecipientDb) -> Result<Self, Self::Error> {
        Ok(MailRecipient {
            id: parse_uuid(&db.id)?,
            created: parse_datetime(&db.created)
                .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?,
            deleted: parse_optional_datetime(&db.deleted)?,
            version: parse_uuid(&db.version)?,
            mail_job_id: parse_uuid(&db.mail_job_id)?,
            to_address: Arc::from(db.to_address.as_str()),
            member_id: parse_optional_uuid(&db.member_id)?,
            application_id: parse_optional_uuid(&db.application_id)?,
            status: Arc::from(db.status.as_str()),
            error: db.error.as_deref().map(Arc::from),
            sent_at: parse_optional_datetime(&db.sent_at)?,
            message_id: db.message_id.as_deref().map(Arc::from),
            rendered_subject: db.rendered_subject.as_deref().map(Arc::from),
            rendered_body: db.rendered_body.as_deref().map(Arc::from),
            rendered_html_body: db.rendered_html_body.as_deref().map(Arc::from),
            rendered_reconstructed: db.rendered_reconstructed != 0,
        })
    }
}

pub struct MailRecipientDaoSqlite {
    pool: Arc<SqlitePool>,
}

impl MailRecipientDaoSqlite {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MailRecipientDao for MailRecipientDaoSqlite {
    async fn create(&self, recipient: &MailRecipient) -> Result<(), MailDaoError> {
        let id = recipient.id.as_bytes().to_vec();
        let version = recipient.version.as_bytes().to_vec();
        let created = format_datetime(&recipient.created)?;
        let mail_job_id = recipient.mail_job_id.as_bytes().to_vec();
        let member_id = recipient.member_id.map(|m| m.as_bytes().to_vec());
        let application_id = recipient.application_id.map(|a| a.as_bytes().to_vec());

        sqlx::query(
            "INSERT INTO mail_recipients (id, created, deleted, version, mail_job_id, to_address, member_id, application_id, status, error, sent_at, message_id, rendered_subject, rendered_body, rendered_reconstructed, rendered_html_body) \
             VALUES (?, ?, NULL, ?, ?, ?, ?, ?, ?, ?, ?, NULL, NULL, NULL, ?, NULL)",
        )
        .bind(id)
        .bind(created)
        .bind(version)
        .bind(mail_job_id)
        .bind(recipient.to_address.as_ref())
        .bind(member_id)
        .bind(application_id)
        .bind(recipient.status.as_ref())
        .bind(recipient.error.as_deref())
        .bind(Option::<String>::None) // sent_at is NULL on creation
        .bind(if recipient.rendered_reconstructed { 1i64 } else { 0i64 })
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(())
    }

    async fn find_by_job_id(&self, job_id: Uuid) -> Result<Arc<[MailRecipient]>, MailDaoError> {
        let job_id_bytes = job_id.as_bytes().to_vec();
        let rows = sqlx::query_as::<_, MailRecipientDb>(
            "SELECT id, created, deleted, version, mail_job_id, to_address, member_id, application_id, status, error, sent_at, message_id, rendered_subject, rendered_body, rendered_reconstructed, rendered_html_body \
             FROM mail_recipients WHERE mail_job_id = ? ORDER BY created ASC",
        )
        .bind(job_id_bytes)
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;

        rows.iter()
            .map(MailRecipient::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into())
    }

    async fn next_pending(&self) -> Result<Option<MailRecipient>, MailDaoError> {
        let row = sqlx::query_as::<_, MailRecipientDb>(
            "SELECT r.id, r.created, r.deleted, r.version, r.mail_job_id, r.to_address, r.member_id, r.application_id, r.status, r.error, r.sent_at, r.message_id, r.rendered_subject, r.rendered_body, r.rendered_reconstructed, r.rendered_html_body \
             FROM mail_recipients r \
             INNER JOIN mail_jobs j ON r.mail_job_id = j.id \
             WHERE r.status = 'pending' AND j.status = 'running' \
             ORDER BY j.created ASC, r.created ASC \
             LIMIT 1",
        )
        .fetch_optional(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;

        match row {
            Some(ref db) => Ok(Some(MailRecipient::try_from(db)?)),
            None => Ok(None),
        }
    }

    async fn update(&self, recipient: &MailRecipient) -> Result<(), MailDaoError> {
        let id = recipient.id.as_bytes().to_vec();
        let version = recipient.version.as_bytes().to_vec();
        let sent_at = recipient
            .sent_at
            .as_ref()
            .map(format_datetime)
            .transpose()?;

        sqlx::query(
            "UPDATE mail_recipients SET status = ?, error = ?, sent_at = ?, message_id = ?, rendered_subject = ?, rendered_body = ?, rendered_html_body = ?, rendered_reconstructed = ?, version = ? WHERE id = ?",
        )
        .bind(recipient.status.as_ref())
        .bind(recipient.error.as_deref())
        .bind(sent_at)
        .bind(recipient.message_id.as_deref())
        .bind(recipient.rendered_subject.as_deref())
        .bind(recipient.rendered_body.as_deref())
        .bind(recipient.rendered_html_body.as_deref())
        .bind(if recipient.rendered_reconstructed { 1i64 } else { 0i64 })
        .bind(version)
        .bind(id)
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(())
    }

    async fn find_sent_member_ids_by_job_id(
        &self,
        job_id: Uuid,
    ) -> Result<Arc<[Uuid]>, MailDaoError> {
        // Phase 29 (APHIST-01): bewusst UNVERAENDERT — selektiert ausschliesslich
        // member_id (nie application_id). Der member_id-Namespace bleibt sauber,
        // Application-Sends (member_id IS NULL) tauchen hier nie auf (Pitfall 2).
        let job_id_bytes = job_id.as_bytes().to_vec();
        let rows: Vec<(Vec<u8>,)> = sqlx::query_as(
            "SELECT member_id FROM mail_recipients \
             WHERE mail_job_id = ? AND status = 'sent' AND member_id IS NOT NULL",
        )
        .bind(job_id_bytes)
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;

        rows.iter()
            .map(|(bytes,)| parse_uuid(bytes))
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into())
    }

    async fn find_recipients_without_rendered(&self) -> Result<Arc<[MailRecipient]>, MailDaoError> {
        let rows = sqlx::query_as::<_, MailRecipientDb>(
            "SELECT id, created, deleted, version, mail_job_id, to_address, member_id, application_id, status, error, sent_at, message_id, rendered_subject, rendered_body, rendered_reconstructed, rendered_html_body \
             FROM mail_recipients \
             WHERE rendered_subject IS NULL AND rendered_body IS NULL AND deleted IS NULL \
             ORDER BY created ASC",
        )
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;

        rows.iter()
            .map(MailRecipient::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into())
    }

    async fn link_application_to_member(
        &self,
        application_id: Uuid,
        member_id: Uuid,
    ) -> Result<(), MailDaoError> {
        // Phase 29 (APHIST-03, D2 Option A): schreibt die genuine neue member_id auf
        // die als Antragsteller gesendeten Zeilen zurueck. `member_id IS NULL` in der
        // WHERE-Klausel verhindert das Ueberschreiben bereits zugeordneter Zeilen;
        // gesetzt wird ausschliesslich member_id (nie die Application-UUID, Pitfall 2).
        sqlx::query(
            "UPDATE mail_recipients SET member_id = ? WHERE application_id = ? AND member_id IS NULL",
        )
        .bind(member_id.as_bytes().to_vec())
        .bind(application_id.as_bytes().to_vec())
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(())
    }
}

// MailRecipientAttachment SQLite

#[derive(Debug, sqlx::FromRow)]
struct MailRecipientAttachmentDb {
    recipient_id: Vec<u8>,
    document_id: Vec<u8>,
    file_name: String,
    mime_type: String,
    relative_path: String,
}

impl TryFrom<&MailRecipientAttachmentDb> for MailRecipientAttachment {
    type Error = MailDaoError;

    fn try_from(db: &MailRecipientAttachmentDb) -> Result<Self, Self::Error> {
        Ok(MailRecipientAttachment {
            recipient_id: parse_uuid(&db.recipient_id)?,
            document_id: parse_uuid(&db.document_id)?,
            file_name: Arc::from(db.file_name.as_str()),
            mime_type: Arc::from(db.mime_type.as_str()),
            relative_path: Arc::from(db.relative_path.as_str()),
        })
    }
}

pub struct MailRecipientAttachmentDaoSqlite {
    pool: Arc<SqlitePool>,
}

impl MailRecipientAttachmentDaoSqlite {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MailRecipientAttachmentDao for MailRecipientAttachmentDaoSqlite {
    async fn create(&self, attachment: &MailRecipientAttachment) -> Result<(), MailDaoError> {
        let recipient_id = attachment.recipient_id.as_bytes().to_vec();
        let document_id = attachment.document_id.as_bytes().to_vec();

        sqlx::query(
            "INSERT INTO mail_recipient_attachments (recipient_id, document_id, file_name, mime_type, relative_path) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(recipient_id)
        .bind(document_id)
        .bind(attachment.file_name.as_ref())
        .bind(attachment.mime_type.as_ref())
        .bind(attachment.relative_path.as_ref())
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(())
    }

    async fn find_by_recipient_id(
        &self,
        recipient_id: Uuid,
    ) -> Result<Arc<[MailRecipientAttachment]>, MailDaoError> {
        let recipient_id_bytes = recipient_id.as_bytes().to_vec();
        let rows = sqlx::query_as::<_, MailRecipientAttachmentDb>(
            "SELECT recipient_id, document_id, file_name, mime_type, relative_path \
             FROM mail_recipient_attachments WHERE recipient_id = ?",
        )
        .bind(recipient_id_bytes)
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;

        rows.iter()
            .map(MailRecipientAttachment::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into())
    }
}

// InboundMailAttachment SQLite (Phase 19 — Backend für Inbox-Attachment-Anzeige)

#[derive(Debug, sqlx::FromRow)]
struct InboundMailAttachmentDb {
    id: Vec<u8>,
    inbound_mail_id: Vec<u8>,
    created: String,
    file_name: String,
    mime_type: String,
    size_bytes: i64,
    relative_path: Option<String>,
    oversized: i64,
}

impl TryFrom<&InboundMailAttachmentDb> for InboundMailAttachment {
    type Error = MailDaoError;

    fn try_from(db: &InboundMailAttachmentDb) -> Result<Self, Self::Error> {
        Ok(InboundMailAttachment {
            id: parse_uuid(&db.id)?,
            inbound_mail_id: parse_uuid(&db.inbound_mail_id)?,
            created: parse_datetime(&db.created)
                .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?,
            file_name: Arc::from(db.file_name.as_str()),
            mime_type: Arc::from(db.mime_type.as_str()),
            size_bytes: db.size_bytes,
            relative_path: db.relative_path.as_deref().map(Arc::from),
            oversized: db.oversized != 0,
        })
    }
}

pub struct InboundMailAttachmentDaoSqlite {
    pool: Arc<SqlitePool>,
}

impl InboundMailAttachmentDaoSqlite {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl InboundMailAttachmentDao for InboundMailAttachmentDaoSqlite {
    async fn create(&self, attachment: &InboundMailAttachment) -> Result<(), MailDaoError> {
        let id = attachment.id.as_bytes().to_vec();
        let inbound_mail_id = attachment.inbound_mail_id.as_bytes().to_vec();
        let created = format_datetime(&attachment.created)?;
        let oversized = if attachment.oversized { 1i64 } else { 0i64 };
        let relative_path = attachment.relative_path.as_ref().map(|r| r.to_string());

        sqlx::query(
            "INSERT INTO inbound_mail_attachments (id, inbound_mail_id, created, file_name, mime_type, size_bytes, relative_path, oversized) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(inbound_mail_id)
        .bind(created)
        .bind(attachment.file_name.as_ref())
        .bind(attachment.mime_type.as_ref())
        .bind(attachment.size_bytes)
        .bind(relative_path)
        .bind(oversized)
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(())
    }

    async fn find_by_inbound_mail_id(
        &self,
        inbound_mail_id: Uuid,
    ) -> Result<Arc<[InboundMailAttachment]>, MailDaoError> {
        let inbound_mail_id_bytes = inbound_mail_id.as_bytes().to_vec();
        let rows = sqlx::query_as::<_, InboundMailAttachmentDb>(
            "SELECT id, inbound_mail_id, created, file_name, mime_type, size_bytes, relative_path, oversized \
             FROM inbound_mail_attachments WHERE inbound_mail_id = ? ORDER BY created ASC",
        )
        .bind(inbound_mail_id_bytes)
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;

        rows.iter()
            .map(InboundMailAttachment::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into())
    }

    async fn find_by_id_and_mail(
        &self,
        mail_id: Uuid,
        attachment_id: Uuid,
    ) -> Result<Option<InboundMailAttachment>, MailDaoError> {
        let attachment_id_bytes = attachment_id.as_bytes().to_vec();
        let mail_id_bytes = mail_id.as_bytes().to_vec();
        let row = sqlx::query_as::<_, InboundMailAttachmentDb>(
            "SELECT id, inbound_mail_id, created, file_name, mime_type, size_bytes, relative_path, oversized \
             FROM inbound_mail_attachments WHERE id = ? AND inbound_mail_id = ?",
        )
        .bind(attachment_id_bytes)
        .bind(mail_id_bytes)
        .fetch_optional(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;

        row.as_ref()
            .map(InboundMailAttachment::try_from)
            .transpose()
    }

    async fn count_for_mail(&self, mail_id: Uuid) -> Result<i64, MailDaoError> {
        let mail_id_bytes = mail_id.as_bytes().to_vec();
        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM inbound_mail_attachments WHERE inbound_mail_id = ?",
        )
        .bind(mail_id_bytes)
        .fetch_one(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(count)
    }
}

// StaticDocument SQLite

#[derive(Debug, sqlx::FromRow)]
struct StaticDocumentDb {
    id: Vec<u8>,
    created: String,
    deleted: Option<String>,
    version: Vec<u8>,
    name: String,
    filename: String,
    content_type: String,
    size_bytes: i64,
}

impl TryFrom<&StaticDocumentDb> for StaticDocument {
    type Error = MailDaoError;

    fn try_from(db: &StaticDocumentDb) -> Result<Self, Self::Error> {
        Ok(StaticDocument {
            id: parse_uuid(&db.id)?,
            created: parse_datetime(&db.created)
                .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?,
            deleted: parse_optional_datetime(&db.deleted)?,
            version: parse_uuid(&db.version)?,
            name: Arc::from(db.name.as_str()),
            filename: Arc::from(db.filename.as_str()),
            content_type: Arc::from(db.content_type.as_str()),
            size_bytes: db.size_bytes,
        })
    }
}

pub struct StaticDocumentDaoSqlite {
    pool: Arc<SqlitePool>,
}

impl StaticDocumentDaoSqlite {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl StaticDocumentDao for StaticDocumentDaoSqlite {
    async fn create(&self, doc: &StaticDocument) -> Result<(), MailDaoError> {
        let id = doc.id.as_bytes().to_vec();
        let version = doc.version.as_bytes().to_vec();
        let created = format_datetime(&doc.created)?;

        sqlx::query(
            "INSERT INTO static_documents (id, created, version, name, filename, \
             content_type, size_bytes) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(created)
        .bind(version)
        .bind(doc.name.as_ref())
        .bind(doc.filename.as_ref())
        .bind(doc.content_type.as_ref())
        .bind(doc.size_bytes)
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<StaticDocument>, MailDaoError> {
        let id_bytes = id.as_bytes().to_vec();
        let row = sqlx::query_as::<_, StaticDocumentDb>(
            "SELECT id, created, deleted, version, name, filename, content_type, size_bytes \
             FROM static_documents WHERE id = ? AND deleted IS NULL",
        )
        .bind(id_bytes)
        .fetch_optional(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;

        row.as_ref().map(StaticDocument::try_from).transpose()
    }

    async fn find_many_by_ids(&self, ids: &[Uuid]) -> Result<Arc<[StaticDocument]>, MailDaoError> {
        if ids.is_empty() {
            return Ok(Arc::from(vec![]));
        }
        let placeholders = std::iter::repeat_n("?", ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let query = format!(
            "SELECT id, created, deleted, version, name, filename, content_type, size_bytes \
             FROM static_documents WHERE deleted IS NULL AND id IN ({})",
            placeholders
        );
        let mut q = sqlx::query_as::<_, StaticDocumentDb>(&query);
        for id in ids {
            q = q.bind(id.as_bytes().to_vec());
        }
        let rows = q
            .fetch_all(self.pool.as_ref())
            .await
            .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;
        rows.iter()
            .map(StaticDocument::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into())
    }

    async fn all_active(&self) -> Result<Arc<[StaticDocument]>, MailDaoError> {
        let rows = sqlx::query_as::<_, StaticDocumentDb>(
            "SELECT id, created, deleted, version, name, filename, content_type, size_bytes \
             FROM static_documents WHERE deleted IS NULL ORDER BY name",
        )
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;
        rows.iter()
            .map(StaticDocument::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into())
    }

    async fn soft_delete(&self, id: Uuid) -> Result<(), MailDaoError> {
        let id_bytes = id.as_bytes().to_vec();
        let now = time::OffsetDateTime::now_utc();
        let now_primitive = time::PrimitiveDateTime::new(now.date(), now.time());
        let deleted = format_datetime(&now_primitive)?;
        let new_version = Uuid::new_v4().as_bytes().to_vec();

        let rows_affected = sqlx::query(
            "UPDATE static_documents SET deleted = ?, version = ? \
             WHERE id = ? AND deleted IS NULL",
        )
        .bind(deleted)
        .bind(new_version)
        .bind(id_bytes)
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?
        .rows_affected();

        if rows_affected == 0 {
            return Err(MailDaoError::NotFound);
        }
        Ok(())
    }
}

// MailJobStaticAttachment SQLite

pub struct MailJobStaticAttachmentDaoSqlite {
    pool: Arc<SqlitePool>,
}

impl MailJobStaticAttachmentDaoSqlite {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MailJobStaticAttachmentDao for MailJobStaticAttachmentDaoSqlite {
    async fn create(&self, entity: &MailJobStaticAttachment) -> Result<(), MailDaoError> {
        let mail_job_id = entity.mail_job_id.as_bytes().to_vec();
        let static_document_id = entity.static_document_id.as_bytes().to_vec();

        sqlx::query(
            "INSERT INTO mail_job_static_attachments (mail_job_id, static_document_id) \
             VALUES (?, ?)",
        )
        .bind(mail_job_id)
        .bind(static_document_id)
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(())
    }

    async fn find_static_documents_by_job_id(
        &self,
        mail_job_id: Uuid,
    ) -> Result<Arc<[StaticDocument]>, MailDaoError> {
        let id_bytes = mail_job_id.as_bytes().to_vec();
        let rows = sqlx::query_as::<_, StaticDocumentDb>(
            "SELECT sd.id, sd.created, sd.deleted, sd.version, sd.name, sd.filename, \
             sd.content_type, sd.size_bytes \
             FROM static_documents sd \
             INNER JOIN mail_job_static_attachments msa ON msa.static_document_id = sd.id \
             WHERE msa.mail_job_id = ? AND sd.deleted IS NULL",
        )
        .bind(id_bytes)
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;
        rows.iter()
            .map(StaticDocument::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into())
    }
}

// ────────────────────────────────────────────────────────────────────────────
// InboundMail SQLite
// ────────────────────────────────────────────────────────────────────────────

#[derive(Debug, sqlx::FromRow)]
struct InboundMailDb {
    id: Vec<u8>,
    created: String,
    version: Vec<u8>,
    uid_validity: i64,
    imap_uid: i64,
    from_address: String,
    subject: String,
    received_at: String,
    body_text: String,
    has_attachments: i64,
    has_html_body: i64,
    raw_html_body: Option<String>,
    in_reply_to: Option<String>,
    message_id: Option<String>,
    replied: i64,
    done: i64,
    archived: i64,
    assigned_member_id: Option<Vec<u8>>,
}

impl TryFrom<&InboundMailDb> for InboundMail {
    type Error = MailDaoError;

    fn try_from(db: &InboundMailDb) -> Result<Self, Self::Error> {
        Ok(InboundMail {
            id: parse_uuid(&db.id)?,
            created: parse_datetime(&db.created)
                .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?,
            version: parse_uuid(&db.version)?,
            uid_validity: db.uid_validity,
            imap_uid: db.imap_uid,
            from_address: Arc::from(db.from_address.as_str()),
            subject: Arc::from(db.subject.as_str()),
            received_at: parse_datetime(&db.received_at)
                .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?,
            body_text: Arc::from(db.body_text.as_str()),
            has_attachments: db.has_attachments != 0,
            has_html_body: db.has_html_body != 0,
            raw_html_body: db.raw_html_body.as_deref().map(Arc::from),
            in_reply_to: db.in_reply_to.as_deref().map(Arc::from),
            message_id: db.message_id.as_deref().map(Arc::from),
            replied: db.replied != 0,
            done: db.done != 0,
            archived: db.archived != 0,
            assigned_member_id: parse_optional_uuid(&db.assigned_member_id)?,
        })
    }
}

pub struct InboundMailDaoSqlite {
    pool: Arc<SqlitePool>,
}

impl InboundMailDaoSqlite {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl InboundMailDao for InboundMailDaoSqlite {
    async fn create(&self, mail: &InboundMail) -> Result<(), MailDaoError> {
        let id = mail.id.as_bytes().to_vec();
        let version = mail.version.as_bytes().to_vec();
        let created = format_datetime(&mail.created)?;
        let received_at = format_datetime(&mail.received_at)?;
        let assigned = mail.assigned_member_id.map(|m| m.as_bytes().to_vec());

        sqlx::query(
            "INSERT INTO inbound_mails (id, created, version, uid_validity, imap_uid, from_address, subject, received_at, body_text, has_attachments, has_html_body, raw_html_body, in_reply_to, message_id, replied, done, archived, assigned_member_id) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(created)
        .bind(version)
        .bind(mail.uid_validity)
        .bind(mail.imap_uid)
        .bind(mail.from_address.as_ref())
        .bind(mail.subject.as_ref())
        .bind(received_at)
        .bind(mail.body_text.as_ref())
        .bind(if mail.has_attachments { 1i64 } else { 0 })
        .bind(if mail.has_html_body { 1i64 } else { 0 })
        .bind(mail.raw_html_body.as_deref())
        .bind(mail.in_reply_to.as_deref())
        .bind(mail.message_id.as_deref())
        .bind(if mail.replied { 1i64 } else { 0 })
        .bind(if mail.done { 1i64 } else { 0 })
        .bind(if mail.archived { 1i64 } else { 0 })
        .bind(assigned)
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<InboundMail>, MailDaoError> {
        let id_bytes = id.as_bytes().to_vec();
        let row = sqlx::query_as::<_, InboundMailDb>(
            "SELECT id, created, version, uid_validity, imap_uid, from_address, subject, received_at, body_text, has_attachments, has_html_body, raw_html_body, in_reply_to, message_id, replied, done, archived, assigned_member_id \
             FROM inbound_mails WHERE id = ?",
        )
        .bind(id_bytes)
        .fetch_optional(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;

        match row {
            Some(ref db) => Ok(Some(InboundMail::try_from(db)?)),
            None => Ok(None),
        }
    }

    async fn list_active(&self) -> Result<Arc<[InboundMail]>, MailDaoError> {
        let rows = sqlx::query_as::<_, InboundMailDb>(
            "SELECT id, created, version, uid_validity, imap_uid, from_address, subject, received_at, body_text, has_attachments, has_html_body, raw_html_body, in_reply_to, message_id, replied, done, archived, assigned_member_id \
             FROM inbound_mails ORDER BY received_at DESC",
        )
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;

        rows.iter()
            .map(InboundMail::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into())
    }

    async fn exists_by_uid(&self, uid_validity: i64, imap_uid: i64) -> Result<bool, MailDaoError> {
        let row: Option<(i64,)> = sqlx::query_as(
            "SELECT 1 FROM inbound_mails WHERE uid_validity = ? AND imap_uid = ? LIMIT 1",
        )
        .bind(uid_validity)
        .bind(imap_uid)
        .fetch_optional(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;
        Ok(row.is_some())
    }

    async fn max_uid_for_validity(&self, uid_validity: i64) -> Result<Option<i64>, MailDaoError> {
        let row: Option<(Option<i64>,)> =
            sqlx::query_as("SELECT MAX(imap_uid) FROM inbound_mails WHERE uid_validity = ?")
                .bind(uid_validity)
                .fetch_optional(self.pool.as_ref())
                .await
                .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;
        Ok(row.and_then(|r| r.0))
    }

    async fn update(&self, mail: &InboundMail) -> Result<(), MailDaoError> {
        let id = mail.id.as_bytes().to_vec();
        let version = mail.version.as_bytes().to_vec();
        let assigned = mail.assigned_member_id.map(|m| m.as_bytes().to_vec());

        sqlx::query(
            "UPDATE inbound_mails SET replied = ?, done = ?, archived = ?, assigned_member_id = ?, version = ? WHERE id = ?",
        )
        .bind(if mail.replied { 1i64 } else { 0 })
        .bind(if mail.done { 1i64 } else { 0 })
        .bind(if mail.archived { 1i64 } else { 0 })
        .bind(assigned)
        .bind(version)
        .bind(id)
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(())
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Communication timeline (unified view per member)
// ────────────────────────────────────────────────────────────────────────────

#[derive(Debug, sqlx::FromRow)]
struct CommunicationEntryDb {
    direction: String,
    date: String,
    subject: String,
    inbox_id: Option<Vec<u8>>,
    from_address: Option<String>,
    inbound_done: Option<i64>,
    inbound_replied: Option<i64>,
    inbound_archived: Option<i64>,
    mail_job_id: Option<Vec<u8>>,
    recipient_id: Option<Vec<u8>>,
    to_address: Option<String>,
    outbound_status: Option<String>,
}

impl TryFrom<&CommunicationEntryDb> for CommunicationEntry {
    type Error = MailDaoError;

    fn try_from(db: &CommunicationEntryDb) -> Result<Self, Self::Error> {
        let direction = match db.direction.as_str() {
            "inbound" => CommunicationDirection::Inbound,
            "outbound" => CommunicationDirection::Outbound,
            other => {
                return Err(MailDaoError::DatabaseError(Arc::from(format!(
                    "unknown direction: {other}"
                ))))
            }
        };
        Ok(CommunicationEntry {
            direction,
            date: parse_datetime(&db.date)
                .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?,
            subject: Arc::from(db.subject.as_str()),
            inbox_id: parse_optional_uuid(&db.inbox_id)?,
            from_address: db.from_address.as_deref().map(Arc::from),
            inbound_done: db.inbound_done.map(|v| v != 0),
            inbound_replied: db.inbound_replied.map(|v| v != 0),
            inbound_archived: db.inbound_archived.map(|v| v != 0),
            mail_job_id: parse_optional_uuid(&db.mail_job_id)?,
            recipient_id: parse_optional_uuid(&db.recipient_id)?,
            to_address: db.to_address.as_deref().map(Arc::from),
            outbound_status: db.outbound_status.as_deref().map(Arc::from),
        })
    }
}

pub struct CommunicationDaoSqlite {
    pool: Arc<SqlitePool>,
}

impl CommunicationDaoSqlite {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CommunicationDao for CommunicationDaoSqlite {
    async fn get_member_communications(
        &self,
        member_id: Uuid,
    ) -> Result<Arc<[CommunicationEntry]>, MailDaoError> {
        let member_bytes = member_id.as_bytes().to_vec();
        let rows = sqlx::query_as::<_, CommunicationEntryDb>(
            r#"
            SELECT
                'inbound' AS direction,
                i.received_at AS date,
                i.subject,
                i.id AS inbox_id,
                i.from_address,
                i.done AS inbound_done,
                i.replied AS inbound_replied,
                i.archived AS inbound_archived,
                NULL AS mail_job_id,
                NULL AS recipient_id,
                NULL AS to_address,
                NULL AS outbound_status
            FROM inbound_mails i
            WHERE i.assigned_member_id = ?1

            UNION ALL

            SELECT
                'outbound' AS direction,
                COALESCE(r.sent_at, r.created) AS date,
                j.subject,
                NULL AS inbox_id,
                NULL AS from_address,
                NULL AS inbound_done,
                NULL AS inbound_replied,
                NULL AS inbound_archived,
                j.id AS mail_job_id,
                r.id AS recipient_id,
                r.to_address,
                r.status AS outbound_status
            FROM mail_recipients r
            JOIN mail_jobs j ON j.id = r.mail_job_id
            WHERE r.member_id = ?1
              AND r.deleted IS NULL
              AND j.deleted IS NULL

            ORDER BY date DESC
            "#,
        )
        .bind(&member_bytes)
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;

        rows.iter()
            .map(CommunicationEntry::try_from)
            .collect::<Result<Arc<[_]>, _>>()
    }

    async fn get_application_communications(
        &self,
        application_id: Uuid,
    ) -> Result<Arc<[CommunicationEntry]>, MailDaoError> {
        // Phase 29 (APHIST-01): outbound-only Antragsteller-Timeline. Reduzierter Klon
        // von get_member_communications OHNE inbound-Zweig (Antragsteller haben keine
        // assigned_member_id). Filter auf r.application_id; Soft-Delete beibehalten.
        // Alle 12 Spalten (inkl. NULL-Platzhalter) bleiben, damit CommunicationEntryDb
        // per FromRow passt.
        let application_bytes = application_id.as_bytes().to_vec();
        let rows = sqlx::query_as::<_, CommunicationEntryDb>(
            r#"
            SELECT
                'outbound' AS direction,
                COALESCE(r.sent_at, r.created) AS date,
                j.subject,
                NULL AS inbox_id,
                NULL AS from_address,
                NULL AS inbound_done,
                NULL AS inbound_replied,
                NULL AS inbound_archived,
                j.id AS mail_job_id,
                r.id AS recipient_id,
                r.to_address,
                r.status AS outbound_status
            FROM mail_recipients r
            JOIN mail_jobs j ON j.id = r.mail_job_id
            WHERE r.application_id = ?1
              AND r.deleted IS NULL
              AND j.deleted IS NULL

            ORDER BY date DESC
            "#,
        )
        .bind(&application_bytes)
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;

        rows.iter()
            .map(CommunicationEntry::try_from)
            .collect::<Result<Arc<[_]>, _>>()
    }
}

// ────────────────────────────────────────────────────────────────────────────
// MailTemplate SQLite
// ────────────────────────────────────────────────────────────────────────────

#[derive(Debug, sqlx::FromRow)]
struct MailTemplateDb {
    id: Vec<u8>,
    created: String,
    deleted: Option<String>,
    version: Vec<u8>,
    name: String,
    subject: String,
    body: String,
    // Phase 23 D-06: optional HTML body
    body_html: Option<String>,
}

impl TryFrom<&MailTemplateDb> for MailTemplate {
    type Error = MailDaoError;

    fn try_from(db: &MailTemplateDb) -> Result<Self, Self::Error> {
        Ok(MailTemplate {
            id: parse_uuid(&db.id)?,
            created: parse_datetime(&db.created)
                .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?,
            deleted: parse_optional_datetime(&db.deleted)?,
            version: parse_uuid(&db.version)?,
            name: Arc::from(db.name.as_str()),
            subject: Arc::from(db.subject.as_str()),
            body: Arc::from(db.body.as_str()),
            body_html: db.body_html.as_deref().map(Arc::from),
        })
    }
}

pub struct MailTemplateDaoSqlite {
    pool: Arc<SqlitePool>,
}

impl MailTemplateDaoSqlite {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MailTemplateDao for MailTemplateDaoSqlite {
    async fn create(&self, template: &MailTemplate) -> Result<(), MailDaoError> {
        let id = template.id.as_bytes().to_vec();
        let version = template.version.as_bytes().to_vec();
        let created = format_datetime(&template.created)?;

        sqlx::query(
            "INSERT INTO mail_templates (id, created, deleted, version, name, subject, body, body_html) \
             VALUES (?, ?, NULL, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(created)
        .bind(version)
        .bind(template.name.as_ref())
        .bind(template.subject.as_ref())
        .bind(template.body.as_ref())
        .bind(template.body_html.as_deref())
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(())
    }

    async fn update(&self, template: &MailTemplate) -> Result<(), MailDaoError> {
        let id = template.id.as_bytes().to_vec();
        let version = template.version.as_bytes().to_vec();
        let deleted = template.deleted.as_ref().map(format_datetime).transpose()?;

        sqlx::query(
            "UPDATE mail_templates SET name = ?, subject = ?, body = ?, body_html = ?, version = ?, deleted = ? WHERE id = ?",
        )
        .bind(template.name.as_ref())
        .bind(template.subject.as_ref())
        .bind(template.body.as_ref())
        .bind(template.body_html.as_deref())
        .bind(version)
        .bind(deleted)
        .bind(id)
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(())
    }

    async fn dump_all(&self) -> Result<Arc<[MailTemplate]>, MailDaoError> {
        let rows = sqlx::query_as::<_, MailTemplateDb>(
            "SELECT id, created, deleted, version, name, subject, body, body_html FROM mail_templates",
        )
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;

        rows.iter()
            .map(MailTemplate::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<MailTemplate>, MailDaoError> {
        let id_bytes = id.as_bytes().to_vec();
        let row = sqlx::query_as::<_, MailTemplateDb>(
            "SELECT id, created, deleted, version, name, subject, body, body_html \
             FROM mail_templates WHERE id = ? AND deleted IS NULL",
        )
        .bind(id_bytes)
        .fetch_optional(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;

        row.as_ref().map(MailTemplate::try_from).transpose()
    }

    async fn all(&self) -> Result<Arc<[MailTemplate]>, MailDaoError> {
        let rows = sqlx::query_as::<_, MailTemplateDb>(
            "SELECT id, created, deleted, version, name, subject, body, body_html \
             FROM mail_templates WHERE deleted IS NULL ORDER BY name ASC",
        )
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;

        rows.iter()
            .map(MailTemplate::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into())
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<MailTemplate>, MailDaoError> {
        let row = sqlx::query_as::<_, MailTemplateDb>(
            "SELECT id, created, deleted, version, name, subject, body, body_html \
             FROM mail_templates WHERE name = ? AND deleted IS NULL",
        )
        .bind(name)
        .fetch_optional(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;

        row.as_ref().map(MailTemplate::try_from).transpose()
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Digest state (Phase 20 — D-03: persistiertes letztes Digest-Versanddatum)
// ────────────────────────────────────────────────────────────────────────────

/// Singleton-Key in der digest_state-Tabelle (max. 1 Row).
const LAST_SENT_DATE_KEY: &str = "last_sent_date";

/// `[year]-[month]-[day]`-Maske für das ISO-Datum 'YYYY-MM-DD'.
fn digest_date_format() -> Vec<time::format_description::FormatItem<'static>> {
    time::format_description::parse("[year]-[month]-[day]")
        .expect("static digest date format is valid")
}

pub struct DigestStateDaoSqlite {
    pool: Arc<SqlitePool>,
}

impl DigestStateDaoSqlite {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DigestStateDao for DigestStateDaoSqlite {
    async fn get_last_sent_date(&self) -> Result<Option<time::Date>, MailDaoError> {
        let row: Option<(String,)> = sqlx::query_as("SELECT value FROM digest_state WHERE key = ?")
            .bind(LAST_SENT_DATE_KEY)
            .fetch_optional(self.pool.as_ref())
            .await
            .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;

        match row {
            None => Ok(None),
            Some((s,)) => {
                let fmt = digest_date_format();
                let date = time::Date::parse(&s, &fmt)
                    .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;
                Ok(Some(date))
            }
        }
    }

    async fn set_last_sent_date(&self, date: time::Date) -> Result<(), MailDaoError> {
        let fmt = digest_date_format();
        let value = date
            .format(&fmt)
            .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;
        sqlx::query(
            "INSERT INTO digest_state (key, value) VALUES (?, ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        )
        .bind(LAST_SENT_DATE_KEY)
        .bind(value)
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_db() -> Arc<SqlitePool> {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create in-memory database");
        sqlx::query(
            "CREATE TABLE mail_jobs (
                id BLOB PRIMARY KEY,
                created TEXT NOT NULL,
                deleted TEXT,
                version BLOB NOT NULL,
                subject TEXT NOT NULL,
                body TEXT NOT NULL,
                status TEXT NOT NULL,
                total_count INTEGER NOT NULL,
                sent_count INTEGER NOT NULL DEFAULT 0,
                failed_count INTEGER NOT NULL DEFAULT 0,
                reply_to_inbound_mail_id BLOB,
                template_id BLOB,
                repayment_phase_id BLOB,
                attach_repayment_letter INTEGER NOT NULL DEFAULT 0,
                body_html TEXT
            )",
        )
        .execute(&pool)
        .await
        .expect("Failed to create mail_jobs table");
        sqlx::query(
            "CREATE TABLE mail_recipients (
                id BLOB PRIMARY KEY,
                created TEXT NOT NULL,
                deleted TEXT,
                version BLOB NOT NULL,
                mail_job_id BLOB NOT NULL REFERENCES mail_jobs(id),
                to_address TEXT NOT NULL,
                member_id BLOB,
                application_id BLOB,
                status TEXT NOT NULL,
                error TEXT,
                sent_at TEXT,
                message_id TEXT,
                rendered_subject TEXT,
                rendered_body TEXT,
                rendered_reconstructed INTEGER NOT NULL DEFAULT 0,
                rendered_html_body TEXT
            )",
        )
        .execute(&pool)
        .await
        .expect("Failed to create mail_recipients table");
        sqlx::query(
            "CREATE TABLE mail_recipient_attachments (
                recipient_id BLOB NOT NULL REFERENCES mail_recipients(id),
                document_id BLOB NOT NULL,
                file_name TEXT NOT NULL,
                mime_type TEXT NOT NULL,
                relative_path TEXT NOT NULL,
                PRIMARY KEY (recipient_id, document_id)
            )",
        )
        .execute(&pool)
        .await
        .expect("Failed to create mail_recipient_attachments table");
        sqlx::query(
            "CREATE TABLE static_documents (
                id BLOB PRIMARY KEY NOT NULL,
                created TEXT NOT NULL,
                deleted TEXT,
                version BLOB NOT NULL,
                name TEXT NOT NULL,
                filename TEXT NOT NULL,
                content_type TEXT NOT NULL,
                size_bytes INTEGER NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .expect("Failed to create static_documents table");
        sqlx::query(
            "CREATE TABLE mail_job_static_attachments (
                mail_job_id BLOB NOT NULL REFERENCES mail_jobs(id),
                static_document_id BLOB NOT NULL REFERENCES static_documents(id),
                PRIMARY KEY (mail_job_id, static_document_id)
            )",
        )
        .execute(&pool)
        .await
        .expect("Failed to create mail_job_static_attachments table");
        sqlx::query(
            "CREATE TABLE inbound_mails (
                id BLOB PRIMARY KEY NOT NULL,
                created TEXT NOT NULL,
                version BLOB NOT NULL,
                uid_validity INTEGER NOT NULL,
                imap_uid INTEGER NOT NULL,
                from_address TEXT NOT NULL,
                subject TEXT NOT NULL,
                received_at TEXT NOT NULL,
                body_text TEXT NOT NULL,
                has_attachments INTEGER NOT NULL,
                has_html_body INTEGER NOT NULL,
                raw_html_body TEXT,
                in_reply_to TEXT,
                message_id TEXT,
                replied INTEGER NOT NULL DEFAULT 0,
                done INTEGER NOT NULL DEFAULT 0,
                archived INTEGER NOT NULL DEFAULT 0,
                assigned_member_id BLOB,
                UNIQUE (uid_validity, imap_uid)
            )",
        )
        .execute(&pool)
        .await
        .expect("Failed to create inbound_mails table");
        sqlx::query(
            "CREATE TABLE inbound_mail_attachments (
                id BLOB PRIMARY KEY NOT NULL,
                inbound_mail_id BLOB NOT NULL REFERENCES inbound_mails(id),
                created TEXT NOT NULL,
                file_name TEXT NOT NULL,
                mime_type TEXT NOT NULL,
                size_bytes INTEGER NOT NULL,
                relative_path TEXT,
                oversized INTEGER NOT NULL DEFAULT 0
            )",
        )
        .execute(&pool)
        .await
        .expect("Failed to create inbound_mail_attachments table");
        Arc::new(pool)
    }

    fn sample_datetime() -> PrimitiveDateTime {
        PrimitiveDateTime::new(
            time::Date::from_calendar_date(2026, time::Month::April, 3).unwrap(),
            time::Time::from_hms(10, 0, 0).unwrap(),
        )
    }

    fn sample_job() -> MailJob {
        MailJob {
            id: Uuid::new_v4(),
            created: sample_datetime(),
            deleted: None,
            version: Uuid::new_v4(),
            subject: Arc::from("Test Subject"),
            body: Arc::from("Test Body"),
            status: Arc::from("running"),
            total_count: 3,
            sent_count: 0,
            failed_count: 0,
            reply_to_inbound_mail_id: None,
            template_id: None,
            repayment_phase_id: None,
            attach_repayment_letter: false,
            body_html: None,
        }
    }

    fn sample_recipient(job_id: Uuid) -> MailRecipient {
        MailRecipient {
            id: Uuid::new_v4(),
            created: sample_datetime(),
            deleted: None,
            version: Uuid::new_v4(),
            mail_job_id: job_id,
            to_address: Arc::from("user@example.com"),
            member_id: None,
            application_id: None,
            status: Arc::from("pending"),
            error: None,
            sent_at: None,
            message_id: None,
            rendered_subject: None,
            rendered_body: None,
            rendered_html_body: None,
            rendered_reconstructed: false,
        }
    }

    // MailJob tests

    #[tokio::test]
    async fn test_job_create_and_find_by_id() {
        let pool = setup_db().await;
        let dao = MailJobDaoSqlite::new(pool);

        let job = sample_job();
        dao.create(&job).await.unwrap();

        let found = dao.find_by_id(job.id).await.unwrap();
        assert_eq!(found.id, job.id);
        assert_eq!(found.subject.as_ref(), "Test Subject");
        assert_eq!(found.status.as_ref(), "running");
        assert_eq!(found.total_count, 3);
    }

    // Quick 260603-cz6: MailJob.attach_repayment_letter roundtrip
    #[tokio::test]
    async fn test_job_roundtrip_attach_repayment_letter_default_false() {
        let pool = setup_db().await;
        let dao = MailJobDaoSqlite::new(pool);

        let job = sample_job();
        assert!(!job.attach_repayment_letter, "sample default must be false");
        dao.create(&job).await.unwrap();

        let found = dao.find_by_id(job.id).await.unwrap();
        assert!(!found.attach_repayment_letter);
    }

    #[tokio::test]
    async fn test_job_roundtrip_attach_repayment_letter_true() {
        let pool = setup_db().await;
        let dao = MailJobDaoSqlite::new(pool);

        let mut job = sample_job();
        job.attach_repayment_letter = true;
        job.repayment_phase_id = Some(Uuid::new_v4());
        dao.create(&job).await.unwrap();

        let found = dao.find_by_id(job.id).await.unwrap();
        assert!(found.attach_repayment_letter);
        assert_eq!(found.repayment_phase_id, job.repayment_phase_id);
    }

    #[tokio::test]
    async fn test_job_find_by_id_not_found() {
        let pool = setup_db().await;
        let dao = MailJobDaoSqlite::new(pool);

        let result = dao.find_by_id(Uuid::new_v4()).await;
        assert!(matches!(result, Err(MailDaoError::NotFound)));
    }

    #[tokio::test]
    async fn test_job_all_ordered_by_created_desc() {
        let pool = setup_db().await;
        let dao = MailJobDaoSqlite::new(pool);

        let mut job1 = sample_job();
        job1.created = PrimitiveDateTime::new(
            time::Date::from_calendar_date(2026, time::Month::April, 1).unwrap(),
            time::Time::from_hms(10, 0, 0).unwrap(),
        );
        job1.subject = Arc::from("First");

        let mut job2 = sample_job();
        job2.created = PrimitiveDateTime::new(
            time::Date::from_calendar_date(2026, time::Month::April, 2).unwrap(),
            time::Time::from_hms(10, 0, 0).unwrap(),
        );
        job2.subject = Arc::from("Second");

        dao.create(&job1).await.unwrap();
        dao.create(&job2).await.unwrap();

        let all = dao.all().await.unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].subject.as_ref(), "Second");
        assert_eq!(all[1].subject.as_ref(), "First");
    }

    #[tokio::test]
    async fn test_job_update() {
        let pool = setup_db().await;
        let dao = MailJobDaoSqlite::new(pool);

        let job = sample_job();
        dao.create(&job).await.unwrap();

        let mut updated = job.clone();
        updated.status = Arc::from("done");
        updated.sent_count = 2;
        updated.failed_count = 1;
        updated.version = Uuid::new_v4();
        dao.update(&updated).await.unwrap();

        let found = dao.find_by_id(job.id).await.unwrap();
        assert_eq!(found.status.as_ref(), "done");
        assert_eq!(found.sent_count, 2);
        assert_eq!(found.failed_count, 1);
    }

    // MailRecipient tests

    #[tokio::test]
    async fn test_recipient_create_and_find_by_job_id() {
        let pool = setup_db().await;
        let job_dao = MailJobDaoSqlite::new(pool.clone());
        let recipient_dao = MailRecipientDaoSqlite::new(pool);

        let job = sample_job();
        job_dao.create(&job).await.unwrap();

        let r1 = sample_recipient(job.id);
        let mut r2 = sample_recipient(job.id);
        r2.to_address = Arc::from("other@example.com");
        recipient_dao.create(&r1).await.unwrap();
        recipient_dao.create(&r2).await.unwrap();

        let found = recipient_dao.find_by_job_id(job.id).await.unwrap();
        assert_eq!(found.len(), 2);
    }

    // Phase 29 (APHIST-01): application_id-Roundtrip — eine an einen Antragsteller
    // gesendete Mail wird mit gesetztem application_id (member_id: None) persistiert
    // und byte-gleich wieder ausgelesen.
    #[tokio::test]
    async fn test_recipient_roundtrip_application_id() {
        let pool = setup_db().await;
        let job_dao = MailJobDaoSqlite::new(pool.clone());
        let recipient_dao = MailRecipientDaoSqlite::new(pool);

        let job = sample_job();
        job_dao.create(&job).await.unwrap();

        let app_uuid = Uuid::new_v4();
        let mut r = sample_recipient(job.id);
        r.application_id = Some(app_uuid);
        r.member_id = None;
        recipient_dao.create(&r).await.unwrap();

        let found = recipient_dao.find_by_job_id(job.id).await.unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].application_id, Some(app_uuid));
        assert_eq!(found[0].member_id, None);
    }

    // Phase 29 (APHIST-01, T-29-04): NULL-Legacy-Roundtrip — ein Recipient wie vor der
    // Migration (nur member_id gesetzt, kein application_id) liest application_id=NULL
    // byte-identisch zurueck; bestehende member_id-Semantik unberuehrt.
    #[tokio::test]
    async fn test_recipient_roundtrip_null_legacy_application_id() {
        let pool = setup_db().await;
        let job_dao = MailJobDaoSqlite::new(pool.clone());
        let recipient_dao = MailRecipientDaoSqlite::new(pool);

        let job = sample_job();
        job_dao.create(&job).await.unwrap();

        let mid = Uuid::new_v4();
        let mut r = sample_recipient(job.id);
        r.member_id = Some(mid);
        r.application_id = None;
        recipient_dao.create(&r).await.unwrap();

        let found = recipient_dao.find_by_job_id(job.id).await.unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].application_id, None);
        assert_eq!(found[0].member_id, Some(mid));
    }

    // Phase 29 (APHIST-01, T-29-01, Pitfall 2): Namespace-Gate — ein Antragsteller-Send
    // trennt die Namespaces sauber. Beweist per Persistenz-Roundtrip, dass eine
    // Application-UUID nie in member_id gelangt (member_id.is_none() bei gesetztem
    // application_id).
    #[tokio::test]
    async fn test_recipient_application_send_keeps_member_id_namespace_clean() {
        let pool = setup_db().await;
        let job_dao = MailJobDaoSqlite::new(pool.clone());
        let recipient_dao = MailRecipientDaoSqlite::new(pool);

        let job = sample_job();
        job_dao.create(&job).await.unwrap();

        let app_uuid = Uuid::new_v4();
        let mut r = sample_recipient(job.id);
        r.application_id = Some(app_uuid);
        r.member_id = None;
        recipient_dao.create(&r).await.unwrap();

        let found = recipient_dao.find_by_job_id(job.id).await.unwrap();
        assert_eq!(found.len(), 1);
        assert!(
            found[0].member_id.is_none(),
            "Application-Send darf member_id nicht vergiften"
        );
        assert_eq!(found[0].application_id, Some(app_uuid));
    }

    #[tokio::test]
    async fn test_recipient_next_pending() {
        let pool = setup_db().await;
        let job_dao = MailJobDaoSqlite::new(pool.clone());
        let recipient_dao = MailRecipientDaoSqlite::new(pool);

        let job = sample_job();
        job_dao.create(&job).await.unwrap();

        let r = sample_recipient(job.id);
        recipient_dao.create(&r).await.unwrap();

        let next = recipient_dao.next_pending().await.unwrap();
        assert!(next.is_some());
        assert_eq!(next.unwrap().id, r.id);
    }

    #[tokio::test]
    async fn test_recipient_next_pending_none_when_empty() {
        let pool = setup_db().await;
        let recipient_dao = MailRecipientDaoSqlite::new(pool);

        let next = recipient_dao.next_pending().await.unwrap();
        assert!(next.is_none());
    }

    #[tokio::test]
    async fn test_recipient_next_pending_skips_non_running_jobs() {
        let pool = setup_db().await;
        let job_dao = MailJobDaoSqlite::new(pool.clone());
        let recipient_dao = MailRecipientDaoSqlite::new(pool);

        let mut job = sample_job();
        job.status = Arc::from("done");
        job_dao.create(&job).await.unwrap();

        let r = sample_recipient(job.id);
        recipient_dao.create(&r).await.unwrap();

        let next = recipient_dao.next_pending().await.unwrap();
        assert!(next.is_none());
    }

    #[tokio::test]
    async fn test_recipient_update() {
        let pool = setup_db().await;
        let job_dao = MailJobDaoSqlite::new(pool.clone());
        let recipient_dao = MailRecipientDaoSqlite::new(pool);

        let job = sample_job();
        job_dao.create(&job).await.unwrap();

        let r = sample_recipient(job.id);
        recipient_dao.create(&r).await.unwrap();

        let mut updated = r.clone();
        updated.status = Arc::from("sent");
        updated.sent_at = Some(sample_datetime());
        updated.version = Uuid::new_v4();
        recipient_dao.update(&updated).await.unwrap();

        let found = recipient_dao.find_by_job_id(job.id).await.unwrap();
        assert_eq!(found[0].status.as_ref(), "sent");
        assert!(found[0].sent_at.is_some());
    }

    #[tokio::test]
    async fn test_recipient_update_failed() {
        let pool = setup_db().await;
        let job_dao = MailJobDaoSqlite::new(pool.clone());
        let recipient_dao = MailRecipientDaoSqlite::new(pool);

        let job = sample_job();
        job_dao.create(&job).await.unwrap();

        let r = sample_recipient(job.id);
        recipient_dao.create(&r).await.unwrap();

        let mut updated = r.clone();
        updated.status = Arc::from("failed");
        updated.error = Some(Arc::from("Connection refused"));
        updated.version = Uuid::new_v4();
        recipient_dao.update(&updated).await.unwrap();

        let found = recipient_dao.find_by_job_id(job.id).await.unwrap();
        assert_eq!(found[0].status.as_ref(), "failed");
        assert_eq!(found[0].error.as_deref(), Some("Connection refused"));
        assert!(found[0].message_id.is_none());
    }

    #[tokio::test]
    async fn test_recipient_update_persists_message_id() {
        let pool = setup_db().await;
        let job_dao = MailJobDaoSqlite::new(pool.clone());
        let recipient_dao = MailRecipientDaoSqlite::new(pool);

        let job = sample_job();
        job_dao.create(&job).await.unwrap();

        let r = sample_recipient(job.id);
        recipient_dao.create(&r).await.unwrap();

        // Fresh recipient has no message_id.
        let found = recipient_dao.find_by_job_id(job.id).await.unwrap();
        assert!(found[0].message_id.is_none());

        // Successful send updates message_id.
        let mut updated = r.clone();
        updated.status = Arc::from("sent");
        updated.sent_at = Some(sample_datetime());
        updated.message_id = Some(Arc::from("abc.123@example.com"));
        updated.version = Uuid::new_v4();
        recipient_dao.update(&updated).await.unwrap();

        let found = recipient_dao.find_by_job_id(job.id).await.unwrap();
        assert_eq!(found[0].status.as_ref(), "sent");
        assert_eq!(found[0].message_id.as_deref(), Some("abc.123@example.com"));
    }

    // Quick 260614-9zf: per-recipient rendered subject/body roundtrip.
    #[tokio::test]
    async fn test_recipient_update_persists_rendered_subject_body() {
        let pool = setup_db().await;
        let job_dao = MailJobDaoSqlite::new(pool.clone());
        let recipient_dao = MailRecipientDaoSqlite::new(pool);

        let job = sample_job();
        job_dao.create(&job).await.unwrap();

        let r = sample_recipient(job.id);
        recipient_dao.create(&r).await.unwrap();

        // Fresh recipient has no rendered content (NULL columns).
        let found = recipient_dao.find_by_job_id(job.id).await.unwrap();
        assert!(found[0].rendered_subject.is_none());
        assert!(found[0].rendered_body.is_none());

        // Worker persists the rendered subject + body (plus the normal sent state).
        let mut updated = r.clone();
        updated.status = Arc::from("sent");
        updated.sent_at = Some(sample_datetime());
        updated.message_id = Some(Arc::from("xyz.789@example.com"));
        updated.rendered_subject = Some(Arc::from("Hallo Max"));
        updated.rendered_body = Some(Arc::from("Text für Max"));
        updated.version = Uuid::new_v4();
        recipient_dao.update(&updated).await.unwrap();

        let found = recipient_dao.find_by_job_id(job.id).await.unwrap();
        // Rendered fields persisted verbatim.
        assert_eq!(found[0].rendered_subject.as_deref(), Some("Hallo Max"));
        assert_eq!(found[0].rendered_body.as_deref(), Some("Text für Max"));
        // Existing fields preserved alongside the rendered content.
        assert_eq!(found[0].status.as_ref(), "sent");
        assert!(found[0].sent_at.is_some());
        assert_eq!(found[0].message_id.as_deref(), Some("xyz.789@example.com"));
    }

    // Quick 260614-b1t: rendered_reconstructed flag roundtrips through create + update.
    #[tokio::test]
    async fn test_recipient_roundtrip_rendered_reconstructed_flag() {
        let pool = setup_db().await;
        let job_dao = MailJobDaoSqlite::new(pool.clone());
        let recipient_dao = MailRecipientDaoSqlite::new(pool);

        let job = sample_job();
        job_dao.create(&job).await.unwrap();

        // create with rendered_reconstructed=false (sample_recipient default).
        let r = sample_recipient(job.id);
        recipient_dao.create(&r).await.unwrap();
        let found = recipient_dao.find_by_job_id(job.id).await.unwrap();
        assert!(
            !found[0].rendered_reconstructed,
            "freshly created recipient must read back rendered_reconstructed=false"
        );

        // update flips it to true (backfill path).
        let mut updated = r.clone();
        updated.rendered_subject = Some(Arc::from("Reconstructed Subject"));
        updated.rendered_body = Some(Arc::from("Reconstructed Body"));
        updated.rendered_reconstructed = true;
        updated.version = Uuid::new_v4();
        recipient_dao.update(&updated).await.unwrap();

        let found = recipient_dao.find_by_job_id(job.id).await.unwrap();
        assert!(
            found[0].rendered_reconstructed,
            "after backfill update, rendered_reconstructed must be true"
        );

        // update can flip it back to false (live worker overwrite).
        let mut updated2 = found[0].clone();
        updated2.rendered_reconstructed = false;
        updated2.version = Uuid::new_v4();
        recipient_dao.update(&updated2).await.unwrap();
        let found = recipient_dao.find_by_job_id(job.id).await.unwrap();
        assert!(!found[0].rendered_reconstructed);
    }

    // Quick 260614-b1t: find_recipients_without_rendered only returns NULL-rendered rows.
    #[tokio::test]
    async fn test_find_recipients_without_rendered_filters_filled_rows() {
        let pool = setup_db().await;
        let job_dao = MailJobDaoSqlite::new(pool.clone());
        let recipient_dao = MailRecipientDaoSqlite::new(pool.clone());

        let job = sample_job();
        job_dao.create(&job).await.unwrap();

        // Row 1: NULL rendered_* (legacy / not yet rendered) — must be returned.
        let null_row = sample_recipient(job.id);
        recipient_dao.create(&null_row).await.unwrap();

        // Row 2: already filled rendered_* — must NOT be returned.
        let mut filled_row = sample_recipient(job.id);
        recipient_dao.create(&filled_row).await.unwrap();
        filled_row.rendered_subject = Some(Arc::from("Filled"));
        filled_row.rendered_body = Some(Arc::from("Filled body"));
        filled_row.version = Uuid::new_v4();
        recipient_dao.update(&filled_row).await.unwrap();

        let without = recipient_dao
            .find_recipients_without_rendered()
            .await
            .unwrap();
        assert_eq!(
            without.len(),
            1,
            "only the NULL-rendered row should be returned"
        );
        assert_eq!(without[0].id, null_row.id);

        // Soft-deleted NULL rows are also excluded.
        let deleted_row = sample_recipient(job.id);
        recipient_dao.create(&deleted_row).await.unwrap();
        // simulate soft-delete via raw SQL (DAO has no delete method).
        sqlx::query("UPDATE mail_recipients SET deleted = '2026-04-03T10:00:00' WHERE id = ?")
            .bind(deleted_row.id.as_bytes().to_vec())
            .execute(pool.as_ref())
            .await
            .ok();
        let without = recipient_dao
            .find_recipients_without_rendered()
            .await
            .unwrap();
        assert_eq!(
            without.len(),
            1,
            "soft-deleted NULL row must be excluded; still only the original NULL row"
        );
    }

    // Quick 260614-9zf: next_pending maps the new columns (no panic, None for pending).
    #[tokio::test]
    async fn test_recipient_next_pending_maps_rendered_fields_as_none() {
        let pool = setup_db().await;
        let job_dao = MailJobDaoSqlite::new(pool.clone());
        let recipient_dao = MailRecipientDaoSqlite::new(pool);

        let job = sample_job();
        job_dao.create(&job).await.unwrap();

        let r = sample_recipient(job.id);
        recipient_dao.create(&r).await.unwrap();

        let next = recipient_dao.next_pending().await.unwrap().unwrap();
        assert!(next.rendered_subject.is_none());
        assert!(next.rendered_body.is_none());
    }

    #[tokio::test]
    async fn test_find_sent_member_ids_by_job_id() {
        let pool = setup_db().await;
        let job_dao = MailJobDaoSqlite::new(pool.clone());
        let recipient_dao = MailRecipientDaoSqlite::new(pool);

        let job = sample_job();
        job_dao.create(&job).await.unwrap();

        let member1 = Uuid::new_v4();
        let member2 = Uuid::new_v4();
        let member3 = Uuid::new_v4();

        // sent recipient with member_id
        let mut r1 = sample_recipient(job.id);
        r1.member_id = Some(member1);
        r1.status = Arc::from("sent");
        recipient_dao.create(&r1).await.unwrap();

        // failed recipient with member_id
        let mut r2 = sample_recipient(job.id);
        r2.member_id = Some(member2);
        r2.status = Arc::from("failed");
        recipient_dao.create(&r2).await.unwrap();

        // sent recipient with member_id
        let mut r3 = sample_recipient(job.id);
        r3.member_id = Some(member3);
        r3.status = Arc::from("sent");
        recipient_dao.create(&r3).await.unwrap();

        // sent recipient without member_id (should be excluded)
        let mut r4 = sample_recipient(job.id);
        r4.status = Arc::from("sent");
        r4.member_id = None;
        recipient_dao.create(&r4).await.unwrap();

        let sent_ids = recipient_dao
            .find_sent_member_ids_by_job_id(job.id)
            .await
            .unwrap();
        assert_eq!(sent_ids.len(), 2);
        assert!(sent_ids.contains(&member1));
        assert!(sent_ids.contains(&member3));
        assert!(!sent_ids.contains(&member2));
    }

    // MailRecipientAttachment tests

    #[tokio::test]
    async fn test_attachment_create_and_find_by_recipient_id() {
        let pool = setup_db().await;
        let job_dao = MailJobDaoSqlite::new(pool.clone());
        let recipient_dao = MailRecipientDaoSqlite::new(pool.clone());
        let attachment_dao = MailRecipientAttachmentDaoSqlite::new(pool);

        let job = sample_job();
        job_dao.create(&job).await.unwrap();

        let r = sample_recipient(job.id);
        recipient_dao.create(&r).await.unwrap();

        let doc_id = Uuid::new_v4();
        let attachment = MailRecipientAttachment {
            recipient_id: r.id,
            document_id: doc_id,
            file_name: Arc::from("report.pdf"),
            mime_type: Arc::from("application/pdf"),
            relative_path: Arc::from("abc123.pdf"),
        };
        attachment_dao.create(&attachment).await.unwrap();

        let found = attachment_dao.find_by_recipient_id(r.id).await.unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].document_id, doc_id);
        assert_eq!(found[0].file_name.as_ref(), "report.pdf");
        assert_eq!(found[0].mime_type.as_ref(), "application/pdf");
        assert_eq!(found[0].relative_path.as_ref(), "abc123.pdf");
    }

    #[tokio::test]
    async fn test_attachment_multiple_per_recipient() {
        let pool = setup_db().await;
        let job_dao = MailJobDaoSqlite::new(pool.clone());
        let recipient_dao = MailRecipientDaoSqlite::new(pool.clone());
        let attachment_dao = MailRecipientAttachmentDaoSqlite::new(pool);

        let job = sample_job();
        job_dao.create(&job).await.unwrap();

        let r = sample_recipient(job.id);
        recipient_dao.create(&r).await.unwrap();

        let a1 = MailRecipientAttachment {
            recipient_id: r.id,
            document_id: Uuid::new_v4(),
            file_name: Arc::from("doc1.pdf"),
            mime_type: Arc::from("application/pdf"),
            relative_path: Arc::from("aaa.pdf"),
        };
        let a2 = MailRecipientAttachment {
            recipient_id: r.id,
            document_id: Uuid::new_v4(),
            file_name: Arc::from("doc2.pdf"),
            mime_type: Arc::from("application/pdf"),
            relative_path: Arc::from("bbb.pdf"),
        };
        attachment_dao.create(&a1).await.unwrap();
        attachment_dao.create(&a2).await.unwrap();

        let found = attachment_dao.find_by_recipient_id(r.id).await.unwrap();
        assert_eq!(found.len(), 2);
    }

    // StaticDocument tests

    fn sample_static_document() -> StaticDocument {
        StaticDocument {
            id: Uuid::new_v4(),
            created: sample_datetime(),
            deleted: None,
            version: Uuid::new_v4(),
            name: Arc::from("Satzung"),
            filename: Arc::from("satzung.pdf"),
            content_type: Arc::from("application/pdf"),
            size_bytes: 12345,
        }
    }

    #[tokio::test]
    async fn test_static_document_create_and_find() {
        let pool = setup_db().await;
        let dao = StaticDocumentDaoSqlite::new(pool);

        let doc = sample_static_document();
        dao.create(&doc).await.unwrap();

        let found = dao.find_by_id(doc.id).await.unwrap().unwrap();
        assert_eq!(found.id, doc.id);
        assert_eq!(found.name.as_ref(), "Satzung");
        assert_eq!(found.filename.as_ref(), "satzung.pdf");
        assert_eq!(found.content_type.as_ref(), "application/pdf");
        assert_eq!(found.size_bytes, 12345);
    }

    #[tokio::test]
    async fn test_static_document_soft_delete_hides_from_find() {
        let pool = setup_db().await;
        let dao = StaticDocumentDaoSqlite::new(pool);

        let doc = sample_static_document();
        dao.create(&doc).await.unwrap();

        dao.soft_delete(doc.id).await.unwrap();
        assert!(dao.find_by_id(doc.id).await.unwrap().is_none());
        assert!(dao.all_active().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_static_document_all_active_sorted_by_name() {
        let pool = setup_db().await;
        let dao = StaticDocumentDaoSqlite::new(pool);

        let mut a = sample_static_document();
        a.id = Uuid::new_v4();
        a.name = Arc::from("Zeta");
        let mut b = sample_static_document();
        b.id = Uuid::new_v4();
        b.name = Arc::from("Alpha");
        dao.create(&a).await.unwrap();
        dao.create(&b).await.unwrap();

        let all = dao.all_active().await.unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].name.as_ref(), "Alpha");
        assert_eq!(all[1].name.as_ref(), "Zeta");
    }

    #[tokio::test]
    async fn test_static_document_find_many_by_ids() {
        let pool = setup_db().await;
        let dao = StaticDocumentDaoSqlite::new(pool);

        let mut a = sample_static_document();
        a.id = Uuid::new_v4();
        let mut b = sample_static_document();
        b.id = Uuid::new_v4();
        dao.create(&a).await.unwrap();
        dao.create(&b).await.unwrap();

        let found = dao.find_many_by_ids(&[a.id, b.id]).await.unwrap();
        assert_eq!(found.len(), 2);

        let empty = dao.find_many_by_ids(&[]).await.unwrap();
        assert!(empty.is_empty());
    }

    #[tokio::test]
    async fn test_mail_job_static_attachment_create_and_find() {
        let pool = setup_db().await;
        let job_dao = MailJobDaoSqlite::new(pool.clone());
        let static_dao = StaticDocumentDaoSqlite::new(pool.clone());
        let join_dao = MailJobStaticAttachmentDaoSqlite::new(pool);

        let job = sample_job();
        job_dao.create(&job).await.unwrap();

        let doc = sample_static_document();
        static_dao.create(&doc).await.unwrap();

        let join = MailJobStaticAttachment {
            mail_job_id: job.id,
            static_document_id: doc.id,
        };
        join_dao.create(&join).await.unwrap();

        let docs = join_dao
            .find_static_documents_by_job_id(job.id)
            .await
            .unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].id, doc.id);
        assert_eq!(docs[0].name.as_ref(), "Satzung");
    }

    #[tokio::test]
    async fn test_mail_job_static_attachment_skips_soft_deleted() {
        let pool = setup_db().await;
        let job_dao = MailJobDaoSqlite::new(pool.clone());
        let static_dao = StaticDocumentDaoSqlite::new(pool.clone());
        let join_dao = MailJobStaticAttachmentDaoSqlite::new(pool);

        let job = sample_job();
        job_dao.create(&job).await.unwrap();

        let doc = sample_static_document();
        static_dao.create(&doc).await.unwrap();

        let join = MailJobStaticAttachment {
            mail_job_id: job.id,
            static_document_id: doc.id,
        };
        join_dao.create(&join).await.unwrap();

        // Soft-delete the document; the join still exists but the worker shouldn't see it.
        static_dao.soft_delete(doc.id).await.unwrap();

        let docs = join_dao
            .find_static_documents_by_job_id(job.id)
            .await
            .unwrap();
        assert!(docs.is_empty());
    }

    #[tokio::test]
    async fn test_attachment_empty_for_unknown_recipient() {
        let pool = setup_db().await;
        let attachment_dao = MailRecipientAttachmentDaoSqlite::new(pool);

        let found = attachment_dao
            .find_by_recipient_id(Uuid::new_v4())
            .await
            .unwrap();
        assert!(found.is_empty());
    }

    // ── InboundMail tests ───────────────────────────────────────────────

    fn sample_inbound(uid_validity: i64, imap_uid: i64) -> InboundMail {
        InboundMail {
            id: Uuid::new_v4(),
            created: sample_datetime(),
            version: Uuid::new_v4(),
            uid_validity,
            imap_uid,
            from_address: Arc::from("sender@example.com"),
            subject: Arc::from("Re: Beitrag"),
            received_at: sample_datetime(),
            body_text: Arc::from("Hallo, hier meine Antwort."),
            has_attachments: false,
            has_html_body: false,
            raw_html_body: None,
            in_reply_to: None,
            message_id: None,
            replied: false,
            done: false,
            archived: false,
            assigned_member_id: None,
        }
    }

    #[tokio::test]
    async fn test_inbound_create_and_list() {
        let pool = setup_db().await;
        let dao = InboundMailDaoSqlite::new(pool);
        dao.create(&sample_inbound(1, 10)).await.unwrap();
        dao.create(&sample_inbound(1, 11)).await.unwrap();
        let all = dao.list_active().await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_inbound_exists_by_uid() {
        let pool = setup_db().await;
        let dao = InboundMailDaoSqlite::new(pool);
        assert!(!dao.exists_by_uid(1, 10).await.unwrap());
        dao.create(&sample_inbound(1, 10)).await.unwrap();
        assert!(dao.exists_by_uid(1, 10).await.unwrap());
        assert!(!dao.exists_by_uid(1, 11).await.unwrap());
        assert!(!dao.exists_by_uid(2, 10).await.unwrap());
    }

    #[tokio::test]
    async fn test_inbound_max_uid() {
        let pool = setup_db().await;
        let dao = InboundMailDaoSqlite::new(pool);
        assert_eq!(dao.max_uid_for_validity(1).await.unwrap(), None);
        dao.create(&sample_inbound(1, 5)).await.unwrap();
        dao.create(&sample_inbound(1, 12)).await.unwrap();
        dao.create(&sample_inbound(1, 8)).await.unwrap();
        dao.create(&sample_inbound(2, 100)).await.unwrap();
        assert_eq!(dao.max_uid_for_validity(1).await.unwrap(), Some(12));
        assert_eq!(dao.max_uid_for_validity(2).await.unwrap(), Some(100));
    }

    #[tokio::test]
    async fn test_inbound_update_assigns_member() {
        let pool = setup_db().await;
        let dao = InboundMailDaoSqlite::new(pool);
        let mail = sample_inbound(1, 10);
        dao.create(&mail).await.unwrap();

        let member_id = Uuid::new_v4();
        let mut updated = mail.clone();
        updated.assigned_member_id = Some(member_id);
        updated.version = Uuid::new_v4();
        dao.update(&updated).await.unwrap();

        let found = dao.find_by_id(mail.id).await.unwrap().unwrap();
        assert_eq!(found.assigned_member_id, Some(member_id));
        assert!(!found.done);
    }

    #[tokio::test]
    async fn test_inbound_list_includes_done() {
        let pool = setup_db().await;
        let dao = InboundMailDaoSqlite::new(pool);
        let m1 = sample_inbound(1, 10);
        let m2 = sample_inbound(1, 11);
        dao.create(&m1).await.unwrap();
        dao.create(&m2).await.unwrap();

        let mut done = m2.clone();
        done.done = true;
        done.version = Uuid::new_v4();
        dao.update(&done).await.unwrap();

        let all = dao.list_active().await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_inbound_dedup_unique_constraint() {
        let pool = setup_db().await;
        let dao = InboundMailDaoSqlite::new(pool);
        dao.create(&sample_inbound(1, 10)).await.unwrap();
        let res = dao.create(&sample_inbound(1, 10)).await;
        assert!(res.is_err(), "duplicate (uid_validity, imap_uid) must fail");
    }

    // ── Communication timeline tests ─────────────────────────────────────

    #[tokio::test]
    async fn test_communication_empty_for_unknown_member() {
        let pool = setup_db().await;
        let dao = CommunicationDaoSqlite::new(pool);
        let result = dao.get_member_communications(Uuid::new_v4()).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_communication_returns_outbound_entries() {
        let pool = setup_db().await;
        let member_id = Uuid::new_v4();

        // Create a mail job + recipient linked to the member
        let job_dao = MailJobDaoSqlite::new(pool.clone());
        let recipient_dao = MailRecipientDaoSqlite::new(pool.clone());
        let job = sample_job();
        job_dao.create(&job).await.unwrap();

        let mut recipient = sample_recipient(job.id);
        recipient.member_id = Some(member_id);
        recipient.status = Arc::from("sent");
        recipient_dao.create(&recipient).await.unwrap();

        let dao = CommunicationDaoSqlite::new(pool);
        let result = dao.get_member_communications(member_id).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].direction, CommunicationDirection::Outbound);
        assert_eq!(result[0].subject.as_ref(), "Test Subject");
        assert_eq!(result[0].outbound_status.as_deref(), Some("sent"));
        assert_eq!(result[0].mail_job_id, Some(job.id));
        assert_eq!(result[0].recipient_id, Some(recipient.id));
    }

    #[tokio::test]
    async fn test_communication_returns_inbound_entries() {
        let pool = setup_db().await;
        let member_id = Uuid::new_v4();

        let inbound_dao = InboundMailDaoSqlite::new(pool.clone());
        let mut mail = sample_inbound(1, 10);
        mail.assigned_member_id = Some(member_id);
        mail.done = true;
        inbound_dao.create(&mail).await.unwrap();

        let dao = CommunicationDaoSqlite::new(pool);
        let result = dao.get_member_communications(member_id).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].direction, CommunicationDirection::Inbound);
        assert_eq!(result[0].subject.as_ref(), "Re: Beitrag");
        assert_eq!(result[0].inbox_id, Some(mail.id));
        assert_eq!(result[0].inbound_done, Some(true));
        assert_eq!(result[0].inbound_replied, Some(false));
        assert_eq!(result[0].inbound_archived, Some(false));
    }

    #[tokio::test]
    async fn test_communication_merges_and_sorts_by_date_desc() {
        let pool = setup_db().await;
        let member_id = Uuid::new_v4();

        // Outbound: earlier date (April 3)
        let job_dao = MailJobDaoSqlite::new(pool.clone());
        let recipient_dao = MailRecipientDaoSqlite::new(pool.clone());
        let job = sample_job();
        job_dao.create(&job).await.unwrap();
        let mut recipient = sample_recipient(job.id);
        recipient.member_id = Some(member_id);
        recipient_dao.create(&recipient).await.unwrap();

        // Inbound: later date (April 5)
        let inbound_dao = InboundMailDaoSqlite::new(pool.clone());
        let mut mail = sample_inbound(1, 20);
        mail.assigned_member_id = Some(member_id);
        mail.received_at = PrimitiveDateTime::new(
            time::Date::from_calendar_date(2026, time::Month::April, 5).unwrap(),
            time::Time::from_hms(10, 0, 0).unwrap(),
        );
        inbound_dao.create(&mail).await.unwrap();

        let dao = CommunicationDaoSqlite::new(pool);
        let result = dao.get_member_communications(member_id).await.unwrap();
        assert_eq!(result.len(), 2);
        // Newest first: inbound (April 5) then outbound (April 3)
        assert_eq!(result[0].direction, CommunicationDirection::Inbound);
        assert_eq!(result[1].direction, CommunicationDirection::Outbound);
    }

    #[tokio::test]
    async fn test_communication_excludes_soft_deleted_outbound() {
        let pool = setup_db().await;
        let member_id = Uuid::new_v4();

        let job_dao = MailJobDaoSqlite::new(pool.clone());
        let recipient_dao = MailRecipientDaoSqlite::new(pool.clone());

        let job = sample_job();
        job_dao.create(&job).await.unwrap();

        let mut recipient = sample_recipient(job.id);
        recipient.member_id = Some(member_id);
        recipient_dao.create(&recipient).await.unwrap();

        // Soft-delete the recipient via raw SQL (DAO update doesn't cover deleted)
        sqlx::query("UPDATE mail_recipients SET deleted = '2026-04-03T10:00:00' WHERE id = ?")
            .bind(recipient.id.as_bytes().to_vec())
            .execute(pool.as_ref())
            .await
            .unwrap();

        let dao = CommunicationDaoSqlite::new(pool);
        let result = dao.get_member_communications(member_id).await.unwrap();
        assert!(result.is_empty());
    }

    // ── Phase 29 (APHIST-01/APHIST-03): Antragsteller-Timeline + Carry-over ──

    // Behavior 1: eine als Antragsteller gesendete outbound-Zeile → genau dieser
    // Eintrag (outbound, korrekte subject/to_address/status).
    #[tokio::test]
    async fn test_application_communications_returns_outbound_entry() {
        let pool = setup_db().await;
        let application_id = Uuid::new_v4();

        let job_dao = MailJobDaoSqlite::new(pool.clone());
        let recipient_dao = MailRecipientDaoSqlite::new(pool.clone());
        let job = sample_job();
        job_dao.create(&job).await.unwrap();

        let mut recipient = sample_recipient(job.id);
        recipient.application_id = Some(application_id);
        recipient.member_id = None;
        recipient.status = Arc::from("sent");
        recipient_dao.create(&recipient).await.unwrap();

        let dao = CommunicationDaoSqlite::new(pool);
        let result = dao
            .get_application_communications(application_id)
            .await
            .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].direction, CommunicationDirection::Outbound);
        assert_eq!(result[0].subject.as_ref(), "Test Subject");
        assert_eq!(result[0].outbound_status.as_deref(), Some("sent"));
        assert_eq!(result[0].to_address.as_deref(), Some("user@example.com"));
        assert_eq!(result[0].mail_job_id, Some(job.id));
        assert_eq!(result[0].recipient_id, Some(recipient.id));
    }

    // Behavior 2: fremde application_id → leeres Arr.
    #[tokio::test]
    async fn test_application_communications_empty_for_foreign_application() {
        let pool = setup_db().await;

        let job_dao = MailJobDaoSqlite::new(pool.clone());
        let recipient_dao = MailRecipientDaoSqlite::new(pool.clone());
        let job = sample_job();
        job_dao.create(&job).await.unwrap();

        let mut recipient = sample_recipient(job.id);
        recipient.application_id = Some(Uuid::new_v4());
        recipient.member_id = None;
        recipient.status = Arc::from("sent");
        recipient_dao.create(&recipient).await.unwrap();

        let dao = CommunicationDaoSqlite::new(pool);
        // Query fuer eine ANDERE application_id → keine Zeilen.
        let result = dao
            .get_application_communications(Uuid::new_v4())
            .await
            .unwrap();
        assert!(result.is_empty());
    }

    // Behavior 3a: Recipient soft-deleted → NICHT zurueckgeliefert.
    #[tokio::test]
    async fn test_application_communications_excludes_soft_deleted_recipient() {
        let pool = setup_db().await;
        let application_id = Uuid::new_v4();

        let job_dao = MailJobDaoSqlite::new(pool.clone());
        let recipient_dao = MailRecipientDaoSqlite::new(pool.clone());
        let job = sample_job();
        job_dao.create(&job).await.unwrap();

        let mut recipient = sample_recipient(job.id);
        recipient.application_id = Some(application_id);
        recipient.member_id = None;
        recipient.status = Arc::from("sent");
        recipient_dao.create(&recipient).await.unwrap();

        sqlx::query("UPDATE mail_recipients SET deleted = '2026-04-03T10:00:00' WHERE id = ?")
            .bind(recipient.id.as_bytes().to_vec())
            .execute(pool.as_ref())
            .await
            .unwrap();

        let dao = CommunicationDaoSqlite::new(pool);
        let result = dao
            .get_application_communications(application_id)
            .await
            .unwrap();
        assert!(result.is_empty());
    }

    // Behavior 3b: zugehoeriger Job soft-deleted → NICHT zurueckgeliefert.
    #[tokio::test]
    async fn test_application_communications_excludes_soft_deleted_job() {
        let pool = setup_db().await;
        let application_id = Uuid::new_v4();

        let job_dao = MailJobDaoSqlite::new(pool.clone());
        let recipient_dao = MailRecipientDaoSqlite::new(pool.clone());
        let job = sample_job();
        job_dao.create(&job).await.unwrap();

        let mut recipient = sample_recipient(job.id);
        recipient.application_id = Some(application_id);
        recipient.member_id = None;
        recipient.status = Arc::from("sent");
        recipient_dao.create(&recipient).await.unwrap();

        sqlx::query("UPDATE mail_jobs SET deleted = '2026-04-03T10:00:00' WHERE id = ?")
            .bind(job.id.as_bytes().to_vec())
            .execute(pool.as_ref())
            .await
            .unwrap();

        let dao = CommunicationDaoSqlite::new(pool);
        let result = dao
            .get_application_communications(application_id)
            .await
            .unwrap();
        assert!(result.is_empty());
    }

    // Behavior 4: kein inbound-Eintrag taucht auf. Ein inbound-Mail mit derselben
    // UUID als assigned_member_id darf die Antragsteller-Timeline NICHT verfaelschen
    // (outbound-only Query).
    #[tokio::test]
    async fn test_application_communications_is_outbound_only() {
        let pool = setup_db().await;
        let application_id = Uuid::new_v4();

        // Ein inbound-Mail, dessen assigned_member_id zufaellig == application_id ist.
        let inbound_dao = InboundMailDaoSqlite::new(pool.clone());
        let mut mail = sample_inbound(1, 10);
        mail.assigned_member_id = Some(application_id);
        inbound_dao.create(&mail).await.unwrap();

        let dao = CommunicationDaoSqlite::new(pool);
        let result = dao
            .get_application_communications(application_id)
            .await
            .unwrap();
        // Kein inbound-Zweig → keine Zeile trotz passender assigned_member_id.
        assert!(result.is_empty());
    }

    // Behavior 5: link_application_to_member schreibt genuine member_id gefiltert
    // zurueck; fremde application_id bleibt unangetastet; danach liefert
    // get_member_communications(new_member_id) den Eintrag.
    #[tokio::test]
    async fn test_link_application_to_member_backfills_and_is_visible_in_member_timeline() {
        let pool = setup_db().await;
        let application_id = Uuid::new_v4();
        let other_application_id = Uuid::new_v4();
        let new_member_id = Uuid::new_v4();

        let job_dao = MailJobDaoSqlite::new(pool.clone());
        let recipient_dao = MailRecipientDaoSqlite::new(pool.clone());
        let job = sample_job();
        job_dao.create(&job).await.unwrap();

        // Ziel-Zeile: application_id gesetzt, member_id None.
        let mut target = sample_recipient(job.id);
        target.application_id = Some(application_id);
        target.member_id = None;
        target.status = Arc::from("sent");
        recipient_dao.create(&target).await.unwrap();

        // Fremd-Zeile: andere application_id — muss unangetastet bleiben.
        let mut other = sample_recipient(job.id);
        other.application_id = Some(other_application_id);
        other.member_id = None;
        other.status = Arc::from("sent");
        recipient_dao.create(&other).await.unwrap();

        recipient_dao
            .link_application_to_member(application_id, new_member_id)
            .await
            .unwrap();

        // Ziel-Zeile jetzt via genuiner member_id in der Member-Timeline sichtbar.
        let comm_dao = CommunicationDaoSqlite::new(pool.clone());
        let member_timeline = comm_dao
            .get_member_communications(new_member_id)
            .await
            .unwrap();
        assert_eq!(member_timeline.len(), 1);
        assert_eq!(member_timeline[0].recipient_id, Some(target.id));

        // Fremd-Zeile blieb member_id NULL → nicht in dieser Member-Timeline.
        let found = recipient_dao.find_by_job_id(job.id).await.unwrap();
        let other_row = found.iter().find(|r| r.id == other.id).unwrap();
        assert_eq!(other_row.member_id, None);
        let target_row = found.iter().find(|r| r.id == target.id).unwrap();
        assert_eq!(target_row.member_id, Some(new_member_id));
    }

    // ── Phase 10 D-12 / D-03: MailJob template_id + repayment_phase_id ──

    #[tokio::test]
    async fn test_mail_job_roundtrip_with_template_and_phase_ids() {
        let pool = setup_db().await;
        let dao = MailJobDaoSqlite::new(pool);

        let template_id = Uuid::new_v4();
        let phase_id = Uuid::new_v4();
        let mut job = sample_job();
        job.template_id = Some(template_id);
        job.repayment_phase_id = Some(phase_id);

        dao.create(&job).await.unwrap();
        let loaded = dao.find_by_id(job.id).await.unwrap();
        assert_eq!(loaded.template_id, Some(template_id));
        assert_eq!(loaded.repayment_phase_id, Some(phase_id));
    }

    #[tokio::test]
    async fn test_mail_job_roundtrip_with_null_template_and_phase_ids() {
        let pool = setup_db().await;
        let dao = MailJobDaoSqlite::new(pool);

        let job = sample_job(); // sample_job() defaults to None / None
        dao.create(&job).await.unwrap();
        let loaded = dao.find_by_id(job.id).await.unwrap();
        assert_eq!(loaded.template_id, None);
        assert_eq!(loaded.repayment_phase_id, None);
    }

    // ── InboundMailAttachment tests (Phase 19) ──────────────────────────

    async fn seed_inbound_mail(pool: &Arc<SqlitePool>, uid_validity: i64, imap_uid: i64) -> Uuid {
        let dao = InboundMailDaoSqlite::new(pool.clone());
        let mail = sample_inbound(uid_validity, imap_uid);
        let id = mail.id;
        dao.create(&mail).await.unwrap();
        id
    }

    fn sample_attachment(
        inbound_mail_id: Uuid,
        file_name: &str,
        relative_path: Option<&str>,
        oversized: bool,
    ) -> InboundMailAttachment {
        InboundMailAttachment {
            id: Uuid::new_v4(),
            inbound_mail_id,
            created: sample_datetime(),
            file_name: Arc::from(file_name),
            mime_type: Arc::from("application/pdf"),
            size_bytes: 12345,
            relative_path: relative_path.map(Arc::from),
            oversized,
        }
    }

    #[tokio::test]
    async fn test_inbound_mail_attachment_roundtrip() {
        let pool = setup_db().await;
        let parent_mail_id = seed_inbound_mail(&pool, 1, 10).await;
        let dao = InboundMailAttachmentDaoSqlite::new(pool.clone());

        // First insert: normal attachment with relative_path Some, oversized=false
        let a = sample_attachment(
            parent_mail_id,
            "invoice.pdf",
            Some("inbound_mail_attachments/mid/aid"),
            false,
        );
        dao.create(&a).await.unwrap();

        let list = dao.find_by_inbound_mail_id(parent_mail_id).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].file_name.as_ref(), "invoice.pdf");
        assert!(!list[0].oversized);
        assert!(list[0].relative_path.is_some());
        assert_eq!(
            list[0].relative_path.as_ref().unwrap().as_ref(),
            "inbound_mail_attachments/mid/aid"
        );
        assert_eq!(list[0].size_bytes, 12345);

        // Second insert: oversized=true, relative_path=None (D-02 hard 10 MB cap)
        let b = sample_attachment(parent_mail_id, "huge-video.mp4", None, true);
        dao.create(&b).await.unwrap();

        let list = dao.find_by_inbound_mail_id(parent_mail_id).await.unwrap();
        assert_eq!(list.len(), 2);
        let oversized_entry = list
            .iter()
            .find(|x| x.file_name.as_ref() == "huge-video.mp4")
            .expect("must find oversized entry");
        assert!(oversized_entry.relative_path.is_none());
        assert!(oversized_entry.oversized);

        // count_for_mail returns 2
        assert_eq!(dao.count_for_mail(parent_mail_id).await.unwrap(), 2);
    }

    // Phase 23 D-07: MailJob.body_html roundtrip — Some(...) persists byte-identically.
    #[tokio::test]
    async fn mail_job_body_html_roundtrip() {
        let pool = setup_db().await;
        let dao = MailJobDaoSqlite::new(pool);

        let mut job = sample_job();
        job.body_html = Some(Arc::from("<b>Hallo</b>"));
        dao.create(&job).await.unwrap();

        let found = dao.find_by_id(job.id).await.unwrap();
        assert_eq!(
            found.body_html.as_deref(),
            Some("<b>Hallo</b>"),
            "body_html must roundtrip byte-identically"
        );
    }

    // Phase 23 D-09 / RESEARCH Pitfall 4: NULL body_html must NOT coerce to Some("").
    #[tokio::test]
    async fn mail_job_body_html_null_roundtrip() {
        let pool = setup_db().await;
        let dao = MailJobDaoSqlite::new(pool);

        let job = sample_job();
        assert!(job.body_html.is_none(), "sample default must be None");
        dao.create(&job).await.unwrap();

        let found = dao.find_by_id(job.id).await.unwrap();
        assert!(
            found.body_html.is_none(),
            "legacy NULL body_html must read back as None, not Some(\"\")"
        );
    }

    // Phase 23 D-08: MailRecipient.rendered_html_body UPDATE persists per-recipient HTML.
    #[tokio::test]
    async fn mail_recipient_update_persists_rendered_html_body() {
        let pool = setup_db().await;
        let job_dao = MailJobDaoSqlite::new(pool.clone());
        let recipient_dao = MailRecipientDaoSqlite::new(pool);

        let job = sample_job();
        job_dao.create(&job).await.unwrap();

        let r = sample_recipient(job.id);
        recipient_dao.create(&r).await.unwrap();

        // Fresh recipient has no rendered HTML.
        let found = recipient_dao.find_by_job_id(job.id).await.unwrap();
        assert!(found[0].rendered_html_body.is_none());

        // Worker persists the rendered HTML body alongside the plain-text render.
        let mut updated = r.clone();
        updated.status = Arc::from("sent");
        updated.sent_at = Some(sample_datetime());
        updated.message_id = Some(Arc::from("html.777@example.com"));
        updated.rendered_subject = Some(Arc::from("Hallo Max"));
        updated.rendered_body = Some(Arc::from("Text für Max"));
        updated.rendered_html_body = Some(Arc::from("<p>Rendered</p>"));
        updated.version = Uuid::new_v4();
        recipient_dao.update(&updated).await.unwrap();

        let found = recipient_dao.find_by_job_id(job.id).await.unwrap();
        assert_eq!(
            found[0].rendered_html_body.as_deref(),
            Some("<p>Rendered</p>"),
            "rendered_html_body must roundtrip byte-identically"
        );
        // Existing rendered fields preserved alongside the HTML variant.
        assert_eq!(found[0].rendered_subject.as_deref(), Some("Hallo Max"));
        assert_eq!(found[0].rendered_body.as_deref(), Some("Text für Max"));
        assert_eq!(found[0].status.as_ref(), "sent");
    }

    #[tokio::test]
    async fn test_find_by_id_and_mail_wrong_mail_returns_none() {
        let pool = setup_db().await;
        // Seed mail A with attachment A1
        let mail_a_id = seed_inbound_mail(&pool, 1, 10).await;
        // Seed mail B (no attachments)
        let mail_b_id = seed_inbound_mail(&pool, 1, 11).await;
        let dao = InboundMailAttachmentDaoSqlite::new(pool.clone());

        let a1 = sample_attachment(
            mail_a_id,
            "doc.pdf",
            Some("inbound_mail_attachments/A/a1"),
            false,
        );
        let attachment_a1_id = a1.id;
        dao.create(&a1).await.unwrap();

        // Cross-mail enumeration: (mail_B_id, attachment_A1_id) must return None
        // (T-03 IDOR cross-mail mitigation)
        let res_wrong = dao
            .find_by_id_and_mail(mail_b_id, attachment_a1_id)
            .await
            .unwrap();
        assert!(
            res_wrong.is_none(),
            "T-03: cross-mail lookup must return None"
        );

        // Positive control: (mail_A_id, attachment_A1_id) returns Some
        let res_ok = dao
            .find_by_id_and_mail(mail_a_id, attachment_a1_id)
            .await
            .unwrap();
        assert!(
            res_ok.is_some(),
            "positive control: correct pair returns Some"
        );
        let found = res_ok.unwrap();
        assert_eq!(found.id, attachment_a1_id);
        assert_eq!(found.inbound_mail_id, mail_a_id);
    }
}

#[cfg(test)]
mod digest_state_tests {
    use super::*;

    async fn setup_db() -> Arc<SqlitePool> {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create in-memory database");
        sqlx::query(
            "CREATE TABLE digest_state (
                key TEXT PRIMARY KEY NOT NULL,
                value TEXT NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .expect("Failed to create digest_state table");
        Arc::new(pool)
    }

    fn date(year: i32, month: time::Month, day: u8) -> time::Date {
        time::Date::from_calendar_date(year, month, day).unwrap()
    }

    /// Test 1: get_last_sent_date on an empty table returns Ok(None).
    #[tokio::test]
    async fn test_get_on_empty_returns_none() {
        let pool = setup_db().await;
        let dao = DigestStateDaoSqlite::new(pool);

        let result = dao.get_last_sent_date().await.unwrap();
        assert_eq!(result, None);
    }

    /// Test 2: after set_last_sent_date(2026-06-26), get returns Ok(Some(2026-06-26)).
    #[tokio::test]
    async fn test_set_then_get_returns_date() {
        let pool = setup_db().await;
        let dao = DigestStateDaoSqlite::new(pool);

        let d = date(2026, time::Month::June, 26);
        dao.set_last_sent_date(d).await.unwrap();

        let result = dao.get_last_sent_date().await.unwrap();
        assert_eq!(result, Some(d));
    }

    /// Test 3: a second set overwrites (upsert) — get returns the new date and the
    /// table still holds exactly one row (no duplicate insert).
    #[tokio::test]
    async fn test_second_set_overwrites_singleton() {
        let pool = setup_db().await;
        let dao = DigestStateDaoSqlite::new(pool.clone());

        dao.set_last_sent_date(date(2026, time::Month::June, 26))
            .await
            .unwrap();
        dao.set_last_sent_date(date(2026, time::Month::June, 27))
            .await
            .unwrap();

        let result = dao.get_last_sent_date().await.unwrap();
        assert_eq!(result, Some(date(2026, time::Month::June, 27)));

        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM digest_state")
            .fetch_one(pool.as_ref())
            .await
            .unwrap();
        assert_eq!(count, 1, "upsert must not create a second row");
    }
}
