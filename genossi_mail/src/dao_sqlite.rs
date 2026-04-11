use async_trait::async_trait;
use sqlx::SqlitePool;
use std::sync::Arc;
use time::PrimitiveDateTime;
use uuid::Uuid;

use crate::dao::{
    CommunicationDao, CommunicationDirection, CommunicationEntry, InboundMail, InboundMailDao,
    MailDaoError, MailJob, MailJobDao, MailJobStaticAttachment, MailJobStaticAttachmentDao,
    MailRecipient, MailRecipientAttachment, MailRecipientAttachmentDao, MailRecipientDao,
    StaticDocument, StaticDocumentDao,
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

        let reply_to = job
            .reply_to_inbound_mail_id
            .map(|u| u.as_bytes().to_vec());

        sqlx::query(
            "INSERT INTO mail_jobs (id, created, deleted, version, subject, body, status, total_count, sent_count, failed_count, reply_to_inbound_mail_id) \
             VALUES (?, ?, NULL, ?, ?, ?, ?, ?, ?, ?, ?)",
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
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<MailJob, MailDaoError> {
        let id_bytes = id.as_bytes().to_vec();
        let row = sqlx::query_as::<_, MailJobDb>(
            "SELECT id, created, deleted, version, subject, body, status, total_count, sent_count, failed_count, reply_to_inbound_mail_id \
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
            "SELECT id, created, deleted, version, subject, body, status, total_count, sent_count, failed_count, reply_to_inbound_mail_id \
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
    status: String,
    error: Option<String>,
    sent_at: Option<String>,
    message_id: Option<String>,
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
            status: Arc::from(db.status.as_str()),
            error: db.error.as_deref().map(Arc::from),
            sent_at: parse_optional_datetime(&db.sent_at)?,
            message_id: db.message_id.as_deref().map(Arc::from),
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

        sqlx::query(
            "INSERT INTO mail_recipients (id, created, deleted, version, mail_job_id, to_address, member_id, status, error, sent_at, message_id) \
             VALUES (?, ?, NULL, ?, ?, ?, ?, ?, ?, ?, NULL)",
        )
        .bind(id)
        .bind(created)
        .bind(version)
        .bind(mail_job_id)
        .bind(recipient.to_address.as_ref())
        .bind(member_id)
        .bind(recipient.status.as_ref())
        .bind(recipient.error.as_deref())
        .bind(Option::<String>::None) // sent_at is NULL on creation
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(())
    }

    async fn find_by_job_id(&self, job_id: Uuid) -> Result<Arc<[MailRecipient]>, MailDaoError> {
        let job_id_bytes = job_id.as_bytes().to_vec();
        let rows = sqlx::query_as::<_, MailRecipientDb>(
            "SELECT id, created, deleted, version, mail_job_id, to_address, member_id, status, error, sent_at, message_id \
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
            "SELECT r.id, r.created, r.deleted, r.version, r.mail_job_id, r.to_address, r.member_id, r.status, r.error, r.sent_at, r.message_id \
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
            "UPDATE mail_recipients SET status = ?, error = ?, sent_at = ?, message_id = ?, version = ? WHERE id = ?",
        )
        .bind(recipient.status.as_ref())
        .bind(recipient.error.as_deref())
        .bind(sent_at)
        .bind(recipient.message_id.as_deref())
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

    async fn find_many_by_ids(
        &self,
        ids: &[Uuid],
    ) -> Result<Arc<[StaticDocument]>, MailDaoError> {
        if ids.is_empty() {
            return Ok(Arc::from(vec![]));
        }
        let placeholders = std::iter::repeat("?")
            .take(ids.len())
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

    async fn exists_by_uid(
        &self,
        uid_validity: i64,
        imap_uid: i64,
    ) -> Result<bool, MailDaoError> {
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

    async fn max_uid_for_validity(
        &self,
        uid_validity: i64,
    ) -> Result<Option<i64>, MailDaoError> {
        let row: Option<(Option<i64>,)> = sqlx::query_as(
            "SELECT MAX(imap_uid) FROM inbound_mails WHERE uid_validity = ?",
        )
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
                return Err(MailDaoError::DatabaseError(
                    Arc::from(format!("unknown direction: {other}")),
                ))
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
                reply_to_inbound_mail_id BLOB
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
                status TEXT NOT NULL,
                error TEXT,
                sent_at TEXT,
                message_id TEXT
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
            status: Arc::from("pending"),
            error: None,
            sent_at: None,
            message_id: None,
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
        assert_eq!(
            found[0].message_id.as_deref(),
            Some("abc.123@example.com")
        );
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
        let result = dao
            .get_member_communications(Uuid::new_v4())
            .await
            .unwrap();
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
}
