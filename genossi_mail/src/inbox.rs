//! Member inbox: polling IMAP, storing inbound mails, and exposing them for
//! the REST layer. IMAP access is abstracted behind [`InboxImapClient`] so that
//! the service and worker logic can be unit-tested without a real server.

use async_trait::async_trait;
use mockall::automock;
use std::sync::Arc;
use uuid::Uuid;

use crate::dao::{
    InboundMail, InboundMailAttachment, InboundMailAttachmentDao, InboundMailDao, MailJob,
    MailJobDao, MailJobStaticAttachment, MailJobStaticAttachmentDao, MailRecipient,
    MailRecipientAttachment, MailRecipientAttachmentDao, MailRecipientDao, StaticDocumentDao,
};
use crate::service::{AttachmentInput, MailServiceError};
use genossi_config::service::ConfigService;
use genossi_service::document_storage::DocumentStorage;

// ────────────────────────────────────────────────────────────────────────────
// Configuration
// ────────────────────────────────────────────────────────────────────────────

const IMAP_HOST_KEY: &str = "imap_host";
const IMAP_PORT_KEY: &str = "imap_port";
const IMAP_USER_KEY: &str = "imap_user";
const IMAP_PASS_KEY: &str = "imap_pass";
const IMAP_TLS_KEY: &str = "imap_tls";
const IMAP_MAILBOX_KEY: &str = "imap_mailbox";
const IMAP_ARCHIVE_MAILBOX_KEY: &str = "imap_archive_mailbox";
const IMAP_POLL_INTERVAL_KEY: &str = "imap_poll_interval_seconds";

const DEFAULT_MAILBOX: &str = "INBOX";
const DEFAULT_POLL_INTERVAL_SECONDS: u64 = 300;

#[derive(Clone, Debug)]
pub struct ImapConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub pass: String,
    pub tls: bool,
    pub mailbox: String,
    pub archive_mailbox: Option<String>,
}

pub async fn load_imap_config<C: ConfigService>(
    config_service: &C,
) -> Result<ImapConfig, MailServiceError> {
    let all = config_service
        .get_all()
        .await
        .map_err(|e| MailServiceError::DataAccess(Arc::from(format!("{:?}", e))))?;

    let get = |key: &str| -> Option<String> {
        all.iter()
            .find(|e| e.key.as_ref() == key)
            .map(|e| e.value.to_string())
    };

    let host = get(IMAP_HOST_KEY)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| MailServiceError::ConfigMissing(Arc::from("imap_host")))?;
    let port: u16 = get(IMAP_PORT_KEY)
        .and_then(|s| s.parse().ok())
        .unwrap_or(993);
    let user = get(IMAP_USER_KEY)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| MailServiceError::ConfigMissing(Arc::from("imap_user")))?;
    let pass = get(IMAP_PASS_KEY)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| MailServiceError::ConfigMissing(Arc::from("imap_pass")))?;
    let tls = get(IMAP_TLS_KEY)
        .map(|v| v != "false" && v != "0")
        .unwrap_or(true);
    let mailbox = get(IMAP_MAILBOX_KEY)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_MAILBOX.to_string());
    let archive_mailbox = get(IMAP_ARCHIVE_MAILBOX_KEY).filter(|s| !s.is_empty());

    Ok(ImapConfig {
        host,
        port,
        user,
        pass,
        tls,
        mailbox,
        archive_mailbox,
    })
}

pub async fn load_poll_interval<C: ConfigService>(config_service: &C) -> u64 {
    let all = match config_service.get_all().await {
        Ok(c) => c,
        Err(_) => return DEFAULT_POLL_INTERVAL_SECONDS,
    };
    all.iter()
        .find(|e| e.key.as_ref() == IMAP_POLL_INTERVAL_KEY)
        .and_then(|e| e.value.parse::<u64>().ok())
        .unwrap_or(DEFAULT_POLL_INTERVAL_SECONDS)
}

// ────────────────────────────────────────────────────────────────────────────
// IMAP client trait
// ────────────────────────────────────────────────────────────────────────────

/// A raw IMAP message as fetched from the server. `raw` contains the full
/// RFC 5322 bytes so the caller can parse headers and body however they like.
#[derive(Debug, Clone)]
pub struct FetchedMessage {
    pub uid: i64,
    pub raw: Vec<u8>,
}

/// Abstracts the IMAP operations the inbox needs. A real implementation connects
/// via `async-imap`; tests use a mock.
#[automock]
#[async_trait]
pub trait InboxImapClient: Send + Sync + 'static {
    /// Connect, authenticate, select the configured mailbox, and return the
    /// mailbox's current UIDVALIDITY.
    async fn uid_validity(&self, config: &ImapConfig) -> Result<i64, MailServiceError>;

    /// Fetch all messages with UID strictly greater than `min_uid` from the
    /// configured mailbox. Does NOT modify any server-side flags.
    async fn fetch_since(
        &self,
        config: &ImapConfig,
        min_uid: i64,
    ) -> Result<Vec<FetchedMessage>, MailServiceError>;

    /// Fetch a single message by UID, with a UIDVALIDITY drift check.
    /// Returns `Ok(Some(msg))` if the UID exists, `Ok(None)` if it does not
    /// exist in the mailbox, and `Err` if either the IMAP request fails or
    /// the server's current UIDVALIDITY does not match `expected_uid_validity`
    /// (the caller is responsible for silent-skip on drift per D-06).
    async fn fetch_one_by_uid(
        &self,
        config: &ImapConfig,
        expected_uid_validity: i64,
        uid: i64,
    ) -> Result<Option<FetchedMessage>, MailServiceError>;

    /// Set the `\Seen` flag on a specific UID in the configured mailbox.
    async fn mark_seen(&self, config: &ImapConfig, uid: i64) -> Result<(), MailServiceError>;

    /// Move a message to the archive mailbox (must be set in `config`).
    async fn move_to_archive(&self, config: &ImapConfig, uid: i64) -> Result<(), MailServiceError>;

    /// List all mailbox folder names available on the server.
    async fn list_folders(&self, config: &ImapConfig) -> Result<Vec<String>, MailServiceError>;
}

// ────────────────────────────────────────────────────────────────────────────
// Parsing
// ────────────────────────────────────────────────────────────────────────────

/// A single attachment extracted from a parsed mail. Carries the raw bytes
/// so the caller (worker / backfill) can decide whether to persist them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedAttachment {
    pub file_name: String,
    pub mime_type: String,
    /// Materialized bytes — empty Vec when the attachment exceeds
    /// `ATTACHMENT_MAX_BYTES` (Probe-Read pattern, D-02 Memory-DoS guard).
    pub bytes: Vec<u8>,
    /// Real attachment size as reported by `mail_parser`'s
    /// `part.contents().len()`. Unlike `bytes.len()` this is always the
    /// declared size — even when `bytes` is empty due to the oversized
    /// guard. `persist_attachment` uses this to set `size_bytes` and to
    /// decide whether `oversized=true`.
    pub declared_size: u64,
}

/// Result of parsing a raw mail into fields ready for storage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedMail {
    pub from_address: String,
    pub subject: String,
    pub received_at: time::PrimitiveDateTime,
    pub body_text: String,
    pub has_attachments: bool,
    pub has_html_body: bool,
    pub raw_html_body: Option<String>,
    pub in_reply_to: Option<String>,
    pub message_id: Option<String>,
    /// Phase 19: full attachment payloads extracted from the mail. Populated
    /// from `msg.attachments()`. Empty vec when no attachments are present
    /// (kept in sync with `has_attachments`).
    pub attachments: Vec<ParsedAttachment>,
}

/// Phase 19 (D-02): hard upper bound for attachment bytes on disk. Larger
/// attachments are stored metadata-only (`oversized=true`, `relative_path=None`)
/// and the bytes are dropped — they never hit `DocumentStorage::save`.
const ATTACHMENT_MAX_BYTES: u64 = 10 * 1024 * 1024;

/// Extract every attachment part from a parsed mail. Mirrors the
/// `mail_parser` example pattern: walk `msg.attachments()`, treat
/// `is_message()` parts as embedded `.eml`, and fall back to synthetic
/// filenames + `application/octet-stream` when the headers don't carry them.
fn extract_attachments(msg: &mail_parser::Message) -> Vec<ParsedAttachment> {
    use mail_parser::MimeHeaders;

    let mut out = Vec::new();
    for (idx, part) in msg.attachments().enumerate() {
        // Probe-Read (D-02 / CR-01): NEVER materialize bytes above the
        // cap. `part.contents()` returns &[u8] without allocation — only
        // `to_vec()` copies into the heap. A malicious mail with a
        // multi-GB attachment would OOM the worker otherwise.
        let raw_len = part.contents().len();
        let oversized = raw_len as u64 > ATTACHMENT_MAX_BYTES;
        let declared_size = raw_len as u64;

        if part.is_message() {
            // Forwarded-as-attachment .eml: store raw bytes verbatim
            // (unless oversized — then keep the bytes empty and let
            // `persist_attachment` record metadata only).
            let bytes: Vec<u8> = if oversized {
                Vec::new()
            } else {
                part.contents().to_vec()
            };
            let name = part
                .attachment_name()
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("forwarded_{}.eml", idx));
            out.push(ParsedAttachment {
                file_name: name,
                mime_type: "message/rfc822".to_string(),
                bytes,
                declared_size,
            });
            continue;
        }
        let mime = part
            .content_type()
            .map(|ct| {
                let mut s = String::from(ct.ctype());
                if let Some(sub) = ct.subtype() {
                    s.push('/');
                    s.push_str(sub);
                }
                s
            })
            .unwrap_or_else(|| "application/octet-stream".to_string());
        let name = part
            .attachment_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("attachment_{}.bin", idx));
        let bytes: Vec<u8> = if oversized {
            Vec::new()
        } else {
            part.contents().to_vec()
        };
        out.push(ParsedAttachment {
            file_name: name,
            mime_type: mime,
            bytes,
            declared_size,
        });
    }
    out
}

/// Persist one parsed attachment using the save-then-DB pattern (T-07).
///
/// Hard rules:
///  * Oversized payloads (> `ATTACHMENT_MAX_BYTES`, D-02) skip storage entirely
///    — the row is created with `oversized=true` and `relative_path=None`, no
///    bytes touch disk.
///  * Otherwise: write to `DocumentStorage` first, then create the DB row.
///    If the DB insert fails, delete the file (best-effort rollback) before
///    returning the error — leaves at most a logged warning, never a DB row
///    pointing at a missing file (T-07).
///  * The storage path is `inbound_mail_attachments/{mail_id}/{attachment_id}`
///    — UUIDs only, never the attacker-controlled filename (T-02, D-04).
async fn persist_attachment(
    storage: &dyn DocumentStorage,
    dao: &dyn InboundMailAttachmentDao,
    inbound_mail_id: Uuid,
    file_name: &str,
    mime_type: &str,
    bytes: &[u8],
    declared_size: u64,
) -> Result<InboundMailAttachment, MailServiceError> {
    let id = Uuid::new_v4();
    // CR-01: source of truth for size + oversized is `declared_size`
    // (probe-read result), NOT `bytes.len()`. When the attachment is
    // oversized the caller passes an empty `bytes` slice — using
    // `bytes.len()` would lose the real size and break the oversized
    // marker.
    let size = declared_size as i64;
    let oversized = declared_size > ATTACHMENT_MAX_BYTES;

    let relative_path = if oversized {
        None
    } else {
        Some(format!(
            "inbound_mail_attachments/{}/{}",
            inbound_mail_id, id
        ))
    };

    // For non-oversized attachments: filesystem first, then DB.
    if let Some(ref rel_path) = relative_path {
        storage
            .save(rel_path, bytes)
            .await
            .map_err(|e| MailServiceError::DataAccess(Arc::from(format!("storage save: {}", e))))?;
    }

    let entity = InboundMailAttachment {
        id,
        inbound_mail_id,
        created: now_primitive(),
        file_name: Arc::from(file_name),
        mime_type: Arc::from(mime_type),
        size_bytes: size,
        relative_path: relative_path.as_deref().map(Arc::from),
        oversized,
    };

    if let Err(e) = dao.create(&entity).await {
        if let Some(ref rel_path) = relative_path {
            // Best-effort rollback (T-07). A leftover orphaned file is
            // acceptable — bounded by D-02's 10 MB cap; a half-persisted DB
            // row pointing at nothing is NOT.
            if let Err(del_err) = storage.delete(rel_path).await {
                tracing::warn!(
                    "persist_attachment rollback: storage.delete failed: {:?}",
                    del_err
                );
            }
        }
        return Err(e.into());
    }

    Ok(entity)
}

/// Parse raw RFC 5322 bytes into the fields stored on `InboundMail`.
///
/// Follows the MVP rules from the design doc:
///  * prefer `text/plain` for `body_text`
///  * if only HTML exists, keep `body_text` empty and store the raw HTML in
///    `raw_html_body`
///  * count attachments (bool only, contents discarded)
///  * normalize `In-Reply-To` (strip angle brackets)
///  * extract and normalize `Message-ID` header
pub fn parse_raw_mail(raw: &[u8]) -> ParsedMail {
    use mail_parser::MessageParser;

    let parser = MessageParser::default();
    let parsed = parser.parse(raw);

    let (from_address, subject, received_at, body_text, html, attachments, in_reply_to, message_id) =
        if let Some(msg) = parsed {
            let from_address = msg
                .from()
                .and_then(|addrs| addrs.first())
                .and_then(|a| a.address())
                .unwrap_or("")
                .to_string();

            let subject = msg.subject().unwrap_or("").to_string();

            let received_at = msg
                .date()
                .and_then(|d| {
                    let ts = d.to_timestamp();
                    time::OffsetDateTime::from_unix_timestamp(ts).ok()
                })
                .map(|odt| time::PrimitiveDateTime::new(odt.date(), odt.time()))
                .unwrap_or_else(now_primitive);

            let body_text = (0..msg.text_body_count())
                .find_map(|i| msg.body_text(i))
                .map(|s| s.into_owned())
                .unwrap_or_default();

            let html = (0..msg.html_body_count())
                .find_map(|i| msg.body_html(i))
                .map(|s| s.into_owned());

            let attachments = extract_attachments(&msg);

            let in_reply_to = msg
                .in_reply_to()
                .as_text()
                .map(|s| s.to_string())
                .and_then(|s| crate::dao::normalize_message_id(&s));

            let message_id = msg.message_id().and_then(crate::dao::normalize_message_id);

            (
                from_address,
                subject,
                received_at,
                body_text,
                html,
                attachments,
                in_reply_to,
                message_id,
            )
        } else {
            (
                String::new(),
                String::new(),
                now_primitive(),
                String::new(),
                None,
                Vec::new(),
                None,
                None,
            )
        };

    let has_html_body = html.is_some();
    let raw_html_body = html;
    let has_attachments = !attachments.is_empty();

    ParsedMail {
        from_address,
        subject,
        received_at,
        body_text,
        has_attachments,
        has_html_body,
        raw_html_body,
        in_reply_to,
        message_id,
        attachments,
    }
}

fn now_primitive() -> time::PrimitiveDateTime {
    let now = time::OffsetDateTime::now_utc();
    time::PrimitiveDateTime::new(now.date(), now.time())
}

// ────────────────────────────────────────────────────────────────────────────
// Inbox service
// ────────────────────────────────────────────────────────────────────────────

#[automock]
#[async_trait]
pub trait InboxService: Send + Sync + 'static {
    async fn list(&self) -> Result<Arc<[InboundMail]>, MailServiceError>;
    async fn get(&self, id: Uuid) -> Result<InboundMail, MailServiceError>;
    async fn assign_member(
        &self,
        id: Uuid,
        member_id: Uuid,
    ) -> Result<InboundMail, MailServiceError>;
    async fn unassign(&self, id: Uuid) -> Result<InboundMail, MailServiceError>;
    async fn mark_read(&self, id: Uuid) -> Result<InboundMail, MailServiceError>;
    async fn archive(&self, id: Uuid) -> Result<InboundMail, MailServiceError>;
    async fn mark_done(&self, id: Uuid) -> Result<InboundMail, MailServiceError>;
    async fn list_folders(&self) -> Result<Vec<String>, MailServiceError>;
    async fn reply(
        &self,
        id: Uuid,
        subject: &str,
        body: &str,
        attachment_inputs: Vec<AttachmentInput>,
        static_document_ids: Vec<Uuid>,
        // Phase 24 (EDIT-01, D-01): optional HTML sibling; sanitized at the
        // store boundary (Phase 23 D-03 EP wire) before the MailJob is
        // created. `None` ⇒ text-only reply (pre-Phase-24 behavior).
        body_html: Option<String>,
    ) -> Result<MailJob, MailServiceError>;

    /// Phase 19: return one attachment if it belongs to `mail_id`. The DAO
    /// guard (`find_by_id_and_mail`) enforces T-03 IDOR protection: a wrong
    /// `mail_id` returns `Ok(None)` even if the attachment id exists for
    /// another mail.
    async fn find_attachment(
        &self,
        mail_id: Uuid,
        attachment_id: Uuid,
    ) -> Result<Option<InboundMailAttachment>, MailServiceError>;

    /// Phase 19: list all attachments belonging to one inbound mail.
    async fn list_attachments(
        &self,
        mail_id: Uuid,
    ) -> Result<Arc<[InboundMailAttachment]>, MailServiceError>;
}

pub struct InboxServiceImpl<C, D, I, J, R, A, St, RA, JSA, SD>
where
    C: ConfigService,
    D: InboundMailDao,
    I: InboxImapClient,
    J: MailJobDao,
    R: MailRecipientDao,
    A: InboundMailAttachmentDao,
    St: DocumentStorage + 'static,
    RA: MailRecipientAttachmentDao,
    JSA: MailJobStaticAttachmentDao,
    SD: StaticDocumentDao,
{
    pub config_service: Arc<C>,
    pub dao: Arc<D>,
    pub imap_client: Arc<I>,
    pub job_dao: Arc<J>,
    pub recipient_dao: Arc<R>,
    // NOTE: `attachment_dao` here is the **InboundMailAttachmentDao** (Inbound!).
    // The new outbound recipient/static attachment DAOs below have different names
    // to avoid the obvious naming collision.
    pub attachment_dao: Arc<A>,
    pub storage: Arc<St>,
    // Quick 260607-s0s: outbound reply attachments — analog MailServiceImpl::create_job.
    pub recipient_attachment_dao: Arc<RA>,
    pub mail_job_static_attachment_dao: Arc<JSA>,
    pub static_document_dao: Arc<SD>,
}

impl<C, D, I, J, R, A, St, RA, JSA, SD> InboxServiceImpl<C, D, I, J, R, A, St, RA, JSA, SD>
where
    C: ConfigService,
    D: InboundMailDao,
    I: InboxImapClient,
    J: MailJobDao,
    R: MailRecipientDao,
    A: InboundMailAttachmentDao,
    St: DocumentStorage + 'static,
    RA: MailRecipientAttachmentDao,
    JSA: MailJobStaticAttachmentDao,
    SD: StaticDocumentDao,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        config_service: Arc<C>,
        dao: Arc<D>,
        imap_client: Arc<I>,
        job_dao: Arc<J>,
        recipient_dao: Arc<R>,
        attachment_dao: Arc<A>,
        storage: Arc<St>,
        recipient_attachment_dao: Arc<RA>,
        mail_job_static_attachment_dao: Arc<JSA>,
        static_document_dao: Arc<SD>,
    ) -> Self {
        Self {
            config_service,
            dao,
            imap_client,
            job_dao,
            recipient_dao,
            attachment_dao,
            storage,
            recipient_attachment_dao,
            mail_job_static_attachment_dao,
            static_document_dao,
        }
    }

    async fn load_mail(&self, id: Uuid) -> Result<InboundMail, MailServiceError> {
        self.dao
            .find_by_id(id)
            .await?
            .ok_or(MailServiceError::NotFound)
    }
}

#[async_trait]
impl<C, D, I, J, R, A, St, RA, JSA, SD> InboxService
    for InboxServiceImpl<C, D, I, J, R, A, St, RA, JSA, SD>
where
    C: ConfigService,
    D: InboundMailDao,
    I: InboxImapClient,
    J: MailJobDao,
    R: MailRecipientDao,
    A: InboundMailAttachmentDao,
    St: DocumentStorage + 'static,
    RA: MailRecipientAttachmentDao,
    JSA: MailJobStaticAttachmentDao,
    SD: StaticDocumentDao,
{
    async fn list(&self) -> Result<Arc<[InboundMail]>, MailServiceError> {
        Ok(self.dao.list_active().await?)
    }

    async fn get(&self, id: Uuid) -> Result<InboundMail, MailServiceError> {
        self.load_mail(id).await
    }

    async fn assign_member(
        &self,
        id: Uuid,
        member_id: Uuid,
    ) -> Result<InboundMail, MailServiceError> {
        let mut mail = self.load_mail(id).await?;
        mail.assigned_member_id = Some(member_id);
        mail.version = Uuid::new_v4();
        self.dao.update(&mail).await?;
        Ok(mail)
    }

    async fn unassign(&self, id: Uuid) -> Result<InboundMail, MailServiceError> {
        let mut mail = self.load_mail(id).await?;
        mail.assigned_member_id = None;
        mail.version = Uuid::new_v4();
        self.dao.update(&mail).await?;
        Ok(mail)
    }

    async fn mark_read(&self, id: Uuid) -> Result<InboundMail, MailServiceError> {
        let mail = self.load_mail(id).await?;
        let config = load_imap_config(self.config_service.as_ref()).await?;
        self.imap_client.mark_seen(&config, mail.imap_uid).await?;
        Ok(mail)
    }

    async fn archive(&self, id: Uuid) -> Result<InboundMail, MailServiceError> {
        let mut mail = self.load_mail(id).await?;
        let config = load_imap_config(self.config_service.as_ref()).await?;
        if config.archive_mailbox.is_none() {
            return Err(MailServiceError::ConfigMissing(Arc::from(
                "imap_archive_mailbox",
            )));
        }
        self.imap_client
            .move_to_archive(&config, mail.imap_uid)
            .await?;
        mail.archived = true;
        mail.version = Uuid::new_v4();
        self.dao.update(&mail).await?;
        Ok(mail)
    }

    async fn mark_done(&self, id: Uuid) -> Result<InboundMail, MailServiceError> {
        let mut mail = self.load_mail(id).await?;
        mail.done = true;
        mail.version = Uuid::new_v4();
        self.dao.update(&mail).await?;
        Ok(mail)
    }

    async fn list_folders(&self) -> Result<Vec<String>, MailServiceError> {
        let config = load_imap_config(self.config_service.as_ref()).await?;
        self.imap_client.list_folders(&config).await
    }

    async fn reply(
        &self,
        id: Uuid,
        subject: &str,
        body: &str,
        attachment_inputs: Vec<AttachmentInput>,
        static_document_ids: Vec<Uuid>,
        body_html: Option<String>,
    ) -> Result<MailJob, MailServiceError> {
        let mut mail = self.load_mail(id).await?;

        // Quick 260607-s0s: validate static document existence BEFORE creating the
        // job/recipient rows — mirrors MailServiceImpl::create_job (service.rs:300-308)
        // so we never half-persist a job.
        if !static_document_ids.is_empty() {
            let found = self
                .static_document_dao
                .find_many_by_ids(&static_document_ids)
                .await?;
            if found.len() != static_document_ids.len() {
                return Err(MailServiceError::NotFound);
            }
        }

        let now = time::OffsetDateTime::now_utc();
        let now_primitive = time::PrimitiveDateTime::new(now.date(), now.time());

        let job = MailJob {
            id: Uuid::new_v4(),
            created: now_primitive,
            deleted: None,
            version: Uuid::new_v4(),
            subject: Arc::from(subject),
            body: Arc::from(body),
            status: Arc::from("running"),
            total_count: 1,
            sent_count: 0,
            failed_count: 0,
            reply_to_inbound_mail_id: Some(mail.id),
            // Phase 10: inbox reply is not template/phase-bound.
            template_id: None,
            repayment_phase_id: None,
            // Quick 260603-cz6: inbox reply is not a repayment-bulk send.
            attach_repayment_letter: false,
            // Phase 24 (EDIT-01, D-01): sanitize the optional HTML sibling at
            // the store boundary (Phase 23 D-03 EP wire). None ⇒ None out
            // (text-only reply, pre-Phase-24 behavior).
            body_html: crate::service::sanitize_body_html_opt(body_html.as_deref()).map(Arc::from),
        };
        self.job_dao.create(&job).await?;

        let recipient = MailRecipient {
            id: Uuid::new_v4(),
            created: now_primitive,
            deleted: None,
            version: Uuid::new_v4(),
            mail_job_id: job.id,
            to_address: mail.from_address.clone(),
            member_id: mail.assigned_member_id,
            status: Arc::from("pending"),
            error: None,
            sent_at: None,
            message_id: None,
            rendered_subject: None,
            rendered_body: None,
            rendered_html_body: None,
            rendered_reconstructed: false,
        };
        self.recipient_dao.create(&recipient).await?;

        // Quick 260607-s0s: persist per-recipient MemberDocument attachments
        // (mirrors MailServiceImpl::create_job service.rs:350-359).
        for att in &attachment_inputs {
            let attachment = MailRecipientAttachment {
                recipient_id: recipient.id,
                document_id: att.document_id,
                file_name: Arc::from(att.file_name.as_str()),
                mime_type: Arc::from(att.mime_type.as_str()),
                relative_path: Arc::from(att.relative_path.as_str()),
            };
            self.recipient_attachment_dao.create(&attachment).await?;
        }

        // Quick 260607-s0s: persist job-level StaticDocument joins
        // (mirrors MailServiceImpl::create_job service.rs:362-368).
        for static_document_id in &static_document_ids {
            let join = MailJobStaticAttachment {
                mail_job_id: job.id,
                static_document_id: *static_document_id,
            };
            self.mail_job_static_attachment_dao.create(&join).await?;
        }

        mail.replied = true;
        mail.version = Uuid::new_v4();
        self.dao.update(&mail).await?;

        Ok(job)
    }

    async fn find_attachment(
        &self,
        mail_id: Uuid,
        attachment_id: Uuid,
    ) -> Result<Option<InboundMailAttachment>, MailServiceError> {
        Ok(self
            .attachment_dao
            .find_by_id_and_mail(mail_id, attachment_id)
            .await?)
    }

    async fn list_attachments(
        &self,
        mail_id: Uuid,
    ) -> Result<Arc<[InboundMailAttachment]>, MailServiceError> {
        Ok(self.attachment_dao.find_by_inbound_mail_id(mail_id).await?)
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Inbox worker
// ────────────────────────────────────────────────────────────────────────────

pub async fn start_inbox_worker<C, D, I, A, St>(
    config_service: Arc<C>,
    dao: Arc<D>,
    imap_client: Arc<I>,
    attachment_dao: Arc<A>,
    storage: Arc<St>,
) where
    C: ConfigService,
    D: InboundMailDao,
    I: InboxImapClient,
    A: InboundMailAttachmentDao,
    St: DocumentStorage + 'static,
{
    loop {
        let interval = load_poll_interval(config_service.as_ref()).await;
        if let Err(e) = poll_once(
            config_service.as_ref(),
            dao.as_ref(),
            imap_client.as_ref(),
            attachment_dao.as_ref(),
            storage.as_ref(),
        )
        .await
        {
            tracing::warn!("Inbox worker: poll cycle failed: {:?}", e);
        }
        tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
    }
}

/// One poll cycle: load config, fetch new UIDs, parse, insert mail + attachments.
/// Returns Ok(inserted_count) on success. Attachment persistence is best-effort
/// per D-06: failure to persist a single attachment logs a warning and the
/// cycle continues with the next attachment / next mail.
pub async fn poll_once<C, D, I, A, St>(
    config_service: &C,
    dao: &D,
    imap_client: &I,
    attachment_dao: &A,
    storage: &St,
) -> Result<usize, MailServiceError>
where
    C: ConfigService,
    D: InboundMailDao,
    I: InboxImapClient,
    A: InboundMailAttachmentDao,
    St: DocumentStorage + 'static,
{
    let config = match load_imap_config(config_service).await {
        Ok(c) => c,
        Err(MailServiceError::ConfigMissing(k)) => {
            tracing::debug!("Inbox worker: IMAP not configured ({}), skipping", k);
            return Ok(0);
        }
        Err(e) => return Err(e),
    };

    let uid_validity = imap_client.uid_validity(&config).await?;
    let max_uid = dao.max_uid_for_validity(uid_validity).await?.unwrap_or(0);

    let messages = imap_client.fetch_since(&config, max_uid).await?;
    let mut inserted = 0usize;
    for msg in messages {
        if dao.exists_by_uid(uid_validity, msg.uid).await? {
            continue;
        }
        let parsed = parse_raw_mail(&msg.raw);
        let mail = InboundMail {
            id: Uuid::new_v4(),
            created: now_primitive(),
            version: Uuid::new_v4(),
            uid_validity,
            imap_uid: msg.uid,
            from_address: Arc::from(parsed.from_address.as_str()),
            subject: Arc::from(parsed.subject.as_str()),
            received_at: parsed.received_at,
            body_text: Arc::from(parsed.body_text.as_str()),
            has_attachments: parsed.has_attachments,
            has_html_body: parsed.has_html_body,
            raw_html_body: parsed.raw_html_body.as_deref().map(Arc::from),
            in_reply_to: parsed.in_reply_to.as_deref().map(Arc::from),
            message_id: parsed.message_id.as_deref().map(Arc::from),
            replied: false,
            done: false,
            archived: false,
            assigned_member_id: None,
        };
        if let Err(e) = dao.create(&mail).await {
            tracing::warn!(
                "Inbox worker: failed to store mail uid={}: {:?}",
                msg.uid,
                e
            );
        } else {
            inserted += 1;
            // Phase 19: persist attachments after the parent mail row exists.
            // Best-effort (D-06): a single attachment failure must NOT abort
            // the cycle — log a warn and continue.
            for att in parsed.attachments.iter() {
                if let Err(e) = persist_attachment(
                    storage,
                    attachment_dao,
                    mail.id,
                    &att.file_name,
                    &att.mime_type,
                    &att.bytes,
                    att.declared_size,
                )
                .await
                {
                    tracing::warn!(
                        "inbox_poll: persist_attachment failed for mail {} file '{}': {:?}",
                        mail.id,
                        att.file_name,
                        e
                    );
                }
            }
        }
    }
    if inserted > 0 {
        tracing::info!("Inbox worker: stored {} new inbound mail(s)", inserted);
    }
    Ok(inserted)
}

// ────────────────────────────────────────────────────────────────────────────
// Phase 19 Plan 04 — Attachment backfill (one-shot)
// ────────────────────────────────────────────────────────────────────────────

/// One-shot backfill pass for attachments on legacy inbound mails (mails
/// stored before Phase 19 introduced the attachment pipeline).
///
/// Iterates all active inbound mails, selects those with `has_attachments=true`
/// AND no existing attachment rows (`count_for_mail == 0`), then refetches each
/// candidate from IMAP via `fetch_one_by_uid` (UIDVALIDITY-checked, T-06) and
/// runs the same `persist_attachment` pipeline used by the poll worker.
///
/// Behavior:
/// * **Best-effort (D-05/D-06):** Mails that can no longer be fetched from
///   IMAP (Err) or no longer exist on the server (Ok(None)) are silently
///   skipped — `tracing::warn!` + continue. They will permanently show the
///   "attachment received before Phase 19" hint in the frontend.
/// * **Idempotent on restart:** The `count_for_mail == 0` filter naturally
///   excludes mails that were already backfilled in a previous pass, so
///   re-running on every server start is safe (no double persist).
/// * **One-shot:** No `loop {}` body — exits after walking the candidate
///   list once. Intended to be `tokio::spawn`ed at server start.
/// * **Same 10 MB cap as poll worker** via the shared `persist_attachment`
///   helper (D-02).
///
/// Logging contract:
/// * Start: `inbox_attachment_backfill: starting ({N} candidates)`
/// * End:   `inbox_attachment_backfill: done ({Y} persisted, {Z} skipped)`
pub async fn run_attachment_backfill<C, D, A, St, I>(
    config_service: Arc<C>,
    mail_dao: Arc<D>,
    attachment_dao: Arc<A>,
    storage: Arc<St>,
    imap_client: Arc<I>,
) where
    C: ConfigService + Send + Sync + 'static,
    D: InboundMailDao + Send + Sync + 'static,
    A: InboundMailAttachmentDao + Send + Sync + 'static,
    St: DocumentStorage + Send + Sync + 'static,
    I: InboxImapClient + Send + Sync + 'static,
{
    // 1. Load IMAP config — same code path as start_inbox_worker.
    //    ConfigMissing (no IMAP configured) is a no-op startup case.
    let imap_cfg = match load_imap_config(config_service.as_ref()).await {
        Ok(cfg) => cfg,
        Err(MailServiceError::ConfigMissing(k)) => {
            tracing::debug!(
                "inbox_attachment_backfill: IMAP not configured ({}), skipping",
                k
            );
            return;
        }
        Err(e) => {
            tracing::warn!("inbox_attachment_backfill: config load failed: {:?}", e);
            return;
        }
    };

    // 2. Gather candidates: has_attachments=true AND count_for_mail == 0.
    //    Uses list_active() (in-memory filter on has_attachments). No new DAO
    //    method needed — backfill is a one-shot pass on a small dataset.
    let all = match mail_dao.list_active().await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!("inbox_attachment_backfill: mail dao query failed: {:?}", e);
            return;
        }
    };

    let mut candidates: Vec<InboundMail> = Vec::new();
    for mail in all.iter().filter(|m| m.has_attachments) {
        match attachment_dao.count_for_mail(mail.id).await {
            Ok(0) => candidates.push(mail.clone()),
            Ok(_) => {} // Already backfilled — skip silently for idempotency.
            Err(e) => {
                tracing::warn!(
                    "inbox_attachment_backfill: count_for_mail({}) failed: {:?}",
                    mail.id,
                    e
                );
            }
        }
    }

    tracing::info!(
        "inbox_attachment_backfill: starting ({} candidates)",
        candidates.len()
    );

    let mut persisted: u64 = 0;
    let mut skipped: u64 = 0;
    for mail in candidates.iter() {
        let fetched = match imap_client
            .fetch_one_by_uid(&imap_cfg, mail.uid_validity, mail.imap_uid)
            .await
        {
            Ok(Some(f)) => f,
            Ok(None) => {
                tracing::warn!(
                    "inbox_attachment_backfill: skip mail={} uid={} (validity={}): no message",
                    mail.id,
                    mail.imap_uid,
                    mail.uid_validity
                );
                skipped += 1;
                continue;
            }
            Err(e) => {
                tracing::warn!(
                    "inbox_attachment_backfill: skip mail={} uid={} (validity={}): {:?}",
                    mail.id,
                    mail.imap_uid,
                    mail.uid_validity,
                    e
                );
                skipped += 1;
                continue;
            }
        };
        let parsed = parse_raw_mail(&fetched.raw);
        let mut any_ok = false;
        for att in parsed.attachments.iter() {
            match persist_attachment(
                storage.as_ref(),
                attachment_dao.as_ref(),
                mail.id,
                &att.file_name,
                &att.mime_type,
                &att.bytes,
                att.declared_size,
            )
            .await
            {
                Ok(_) => {
                    any_ok = true;
                }
                Err(e) => {
                    tracing::warn!(
                        "inbox_attachment_backfill: persist failed mail={} file={}: {:?}",
                        mail.id,
                        att.file_name,
                        e
                    );
                }
            }
        }
        if any_ok {
            persisted += 1;
        } else {
            skipped += 1;
        }
    }

    tracing::info!(
        "inbox_attachment_backfill: done ({} persisted, {} skipped)",
        persisted,
        skipped
    );
}

// ────────────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dao::{
        MailDaoError, MockInboundMailAttachmentDao, MockInboundMailDao, MockMailJobDao,
        MockMailJobStaticAttachmentDao, MockMailRecipientAttachmentDao, MockMailRecipientDao,
        MockStaticDocumentDao, StaticDocument,
    };
    use genossi_config::dao::ConfigEntry;
    use genossi_config::service::MockConfigService;
    use genossi_service::document_storage::{MockDocumentStorage, StorageError};

    fn cfg_entry(key: &str, value: &str, ty: &str) -> ConfigEntry {
        ConfigEntry {
            key: Arc::from(key),
            value: Arc::from(value),
            value_type: Arc::from(ty),
        }
    }

    fn mock_config(entries: Vec<ConfigEntry>) -> MockConfigService {
        let mut m = MockConfigService::new();
        m.expect_get_all()
            .returning(move || Ok(entries.clone().into()));
        m
    }

    #[tokio::test]
    async fn load_imap_config_reads_all_keys() {
        let cfg = mock_config(vec![
            cfg_entry("imap_host", "imap.example.com", "string"),
            cfg_entry("imap_port", "993", "int"),
            cfg_entry("imap_user", "me", "string"),
            cfg_entry("imap_pass", "secret", "secret"),
            cfg_entry("imap_tls", "true", "bool"),
            cfg_entry("imap_mailbox", "INBOX", "string"),
            cfg_entry("imap_archive_mailbox", "Archive", "string"),
        ]);
        let c = load_imap_config(&cfg).await.unwrap();
        assert_eq!(c.host, "imap.example.com");
        assert_eq!(c.port, 993);
        assert_eq!(c.user, "me");
        assert_eq!(c.pass, "secret");
        assert!(c.tls);
        assert_eq!(c.mailbox, "INBOX");
        assert_eq!(c.archive_mailbox.as_deref(), Some("Archive"));
    }

    #[tokio::test]
    async fn load_imap_config_missing_host_errors() {
        let cfg = mock_config(vec![
            cfg_entry("imap_user", "me", "string"),
            cfg_entry("imap_pass", "secret", "secret"),
        ]);
        assert!(matches!(
            load_imap_config(&cfg).await,
            Err(MailServiceError::ConfigMissing(_))
        ));
    }

    #[tokio::test]
    async fn load_imap_config_default_mailbox_and_tls() {
        let cfg = mock_config(vec![
            cfg_entry("imap_host", "host", "string"),
            cfg_entry("imap_user", "me", "string"),
            cfg_entry("imap_pass", "secret", "secret"),
        ]);
        let c = load_imap_config(&cfg).await.unwrap();
        assert_eq!(c.mailbox, "INBOX");
        assert!(c.tls);
    }

    #[tokio::test]
    async fn load_poll_interval_default() {
        let cfg = mock_config(vec![]);
        assert_eq!(load_poll_interval(&cfg).await, 300);
    }

    #[tokio::test]
    async fn load_poll_interval_custom() {
        let cfg = mock_config(vec![cfg_entry("imap_poll_interval_seconds", "60", "int")]);
        assert_eq!(load_poll_interval(&cfg).await, 60);
    }

    #[test]
    fn parse_minimal_plain_mail() {
        let raw = b"From: sender@example.com\r\n\
                    To: inbox@example.com\r\n\
                    Subject: Re: Beitrag\r\n\
                    Date: Thu, 09 Apr 2026 14:22:00 +0000\r\n\
                    Message-ID: <reply.123@example.com>\r\n\
                    In-Reply-To: <original.456@example.com>\r\n\
                    \r\n\
                    Hallo, hier meine Antwort.\r\n";
        let p = parse_raw_mail(raw);
        assert_eq!(p.from_address, "sender@example.com");
        assert_eq!(p.subject, "Re: Beitrag");
        assert!(p.body_text.contains("Hallo, hier meine Antwort."));
        assert!(!p.has_attachments);
        assert_eq!(p.in_reply_to.as_deref(), Some("original.456@example.com"));
        assert_eq!(p.message_id.as_deref(), Some("reply.123@example.com"));
    }

    #[test]
    fn parse_mail_without_message_id() {
        let raw = b"From: a@b\r\n\
                    Subject: No ID\r\n\
                    \r\n\
                    body\r\n";
        let p = parse_raw_mail(raw);
        assert!(p.message_id.is_none());
    }

    #[test]
    fn parse_html_only_mail_stores_html_not_body() {
        let raw = b"From: a@b\r\n\
                    Subject: HTML\r\n\
                    MIME-Version: 1.0\r\n\
                    Content-Type: text/html; charset=utf-8\r\n\
                    \r\n\
                    <p>Hallo</p>\r\n";
        let p = parse_raw_mail(raw);
        assert!(p.has_html_body);
        assert!(p.raw_html_body.as_deref().unwrap().contains("<p>Hallo</p>"));
    }

    // ── InboxService unit tests using mocks ────────────────────────────

    fn sample_mail() -> InboundMail {
        InboundMail {
            id: Uuid::new_v4(),
            created: now_primitive(),
            version: Uuid::new_v4(),
            uid_validity: 1,
            imap_uid: 10,
            from_address: Arc::from("a@b"),
            subject: Arc::from("s"),
            received_at: now_primitive(),
            body_text: Arc::from("body"),
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
    async fn assign_member_sets_member() {
        let mail = sample_mail();
        let mail_id = mail.id;
        let member_id = Uuid::new_v4();

        let mut dao = MockInboundMailDao::new();
        let returned = mail.clone();
        dao.expect_find_by_id()
            .returning(move |_| Ok(Some(returned.clone())));
        dao.expect_update().returning(|_| Ok(()));

        let cfg = MockConfigService::new();
        let imap = MockInboxImapClient::new();
        let svc = InboxServiceImpl::new(
            Arc::new(cfg),
            Arc::new(dao),
            Arc::new(imap),
            Arc::new(MockMailJobDao::new()),
            Arc::new(MockMailRecipientDao::new()),
            Arc::new(MockInboundMailAttachmentDao::new()),
            Arc::new(MockDocumentStorage::new()),
            Arc::new(MockMailRecipientAttachmentDao::new()),
            Arc::new(MockMailJobStaticAttachmentDao::new()),
            Arc::new(MockStaticDocumentDao::new()),
        );
        let updated = svc.assign_member(mail_id, member_id).await.unwrap();
        assert_eq!(updated.assigned_member_id, Some(member_id));
    }

    #[tokio::test]
    async fn mark_done_sets_done_flag() {
        let mail = sample_mail();
        let mail_id = mail.id;

        let mut dao = MockInboundMailDao::new();
        let returned = mail.clone();
        dao.expect_find_by_id()
            .returning(move |_| Ok(Some(returned.clone())));
        dao.expect_update().returning(|m| {
            assert!(m.done);
            Ok(())
        });

        let cfg = MockConfigService::new();
        let imap = MockInboxImapClient::new();
        let svc = InboxServiceImpl::new(
            Arc::new(cfg),
            Arc::new(dao),
            Arc::new(imap),
            Arc::new(MockMailJobDao::new()),
            Arc::new(MockMailRecipientDao::new()),
            Arc::new(MockInboundMailAttachmentDao::new()),
            Arc::new(MockDocumentStorage::new()),
            Arc::new(MockMailRecipientAttachmentDao::new()),
            Arc::new(MockMailJobStaticAttachmentDao::new()),
            Arc::new(MockStaticDocumentDao::new()),
        );
        let updated = svc.mark_done(mail_id).await.unwrap();
        assert!(updated.done);
    }

    #[tokio::test]
    async fn archive_requires_archive_mailbox_config() {
        let mail = sample_mail();
        let mail_id = mail.id;

        let mut dao = MockInboundMailDao::new();
        let returned = mail.clone();
        dao.expect_find_by_id()
            .returning(move |_| Ok(Some(returned.clone())));

        let cfg = mock_config(vec![
            cfg_entry("imap_host", "h", "string"),
            cfg_entry("imap_user", "u", "string"),
            cfg_entry("imap_pass", "p", "secret"),
            // no archive mailbox
        ]);
        let imap = MockInboxImapClient::new();
        let svc = InboxServiceImpl::new(
            Arc::new(cfg),
            Arc::new(dao),
            Arc::new(imap),
            Arc::new(MockMailJobDao::new()),
            Arc::new(MockMailRecipientDao::new()),
            Arc::new(MockInboundMailAttachmentDao::new()),
            Arc::new(MockDocumentStorage::new()),
            Arc::new(MockMailRecipientAttachmentDao::new()),
            Arc::new(MockMailJobStaticAttachmentDao::new()),
            Arc::new(MockStaticDocumentDao::new()),
        );
        let res = svc.archive(mail_id).await;
        assert!(matches!(res, Err(MailServiceError::ConfigMissing(_))));
    }

    #[tokio::test]
    async fn poll_once_skips_when_unconfigured() {
        let dao = MockInboundMailDao::new();
        let imap = MockInboxImapClient::new();
        let attachment_dao = MockInboundMailAttachmentDao::new();
        let storage = MockDocumentStorage::new();
        let cfg = mock_config(vec![]); // no imap_host
        let n = poll_once(&cfg, &dao, &imap, &attachment_dao, &storage)
            .await
            .unwrap();
        assert_eq!(n, 0);
    }

    #[tokio::test]
    async fn poll_once_inserts_new_messages() {
        let cfg = mock_config(vec![
            cfg_entry("imap_host", "h", "string"),
            cfg_entry("imap_user", "u", "string"),
            cfg_entry("imap_pass", "p", "secret"),
        ]);

        let raw = b"From: a@b\r\nSubject: Hi\r\n\r\nbody".to_vec();

        let mut imap = MockInboxImapClient::new();
        imap.expect_uid_validity().returning(|_| Ok(42));
        imap.expect_fetch_since().returning(move |_, _min_uid| {
            Ok(vec![FetchedMessage {
                uid: 5,
                raw: raw.clone(),
            }])
        });

        let mut dao = MockInboundMailDao::new();
        dao.expect_max_uid_for_validity().returning(|_| Ok(None));
        dao.expect_exists_by_uid().returning(|_, _| Ok(false));
        dao.expect_create().returning(|_| Ok(()));

        // No attachments in this mail; mocks need no expectations.
        let attachment_dao = MockInboundMailAttachmentDao::new();
        let storage = MockDocumentStorage::new();

        let n = poll_once(&cfg, &dao, &imap, &attachment_dao, &storage)
            .await
            .unwrap();
        assert_eq!(n, 1);
    }

    #[tokio::test]
    async fn poll_once_skips_existing() {
        let cfg = mock_config(vec![
            cfg_entry("imap_host", "h", "string"),
            cfg_entry("imap_user", "u", "string"),
            cfg_entry("imap_pass", "p", "secret"),
        ]);

        let mut imap = MockInboxImapClient::new();
        imap.expect_uid_validity().returning(|_| Ok(1));
        imap.expect_fetch_since().returning(|_, _| {
            Ok(vec![FetchedMessage {
                uid: 5,
                raw: b"From: a@b\r\n\r\n".to_vec(),
            }])
        });

        let mut dao = MockInboundMailDao::new();
        dao.expect_max_uid_for_validity().returning(|_| Ok(Some(4)));
        dao.expect_exists_by_uid().returning(|_, _| Ok(true));

        let attachment_dao = MockInboundMailAttachmentDao::new();
        let storage = MockDocumentStorage::new();

        let n = poll_once(&cfg, &dao, &imap, &attachment_dao, &storage)
            .await
            .unwrap();
        assert_eq!(n, 0);
    }

    #[tokio::test]
    async fn reply_creates_job_and_sets_status() {
        let mail = sample_mail();
        let mail_id = mail.id;
        let from_addr = mail.from_address.to_string();

        let mut dao = MockInboundMailDao::new();
        let returned = mail.clone();
        dao.expect_find_by_id()
            .returning(move |_| Ok(Some(returned.clone())));
        dao.expect_update().returning(|m| {
            assert!(m.replied);
            Ok(())
        });

        let mut job_dao = MockMailJobDao::new();
        job_dao.expect_create().returning(|j| {
            assert!(j.reply_to_inbound_mail_id.is_some());
            assert_eq!(j.total_count, 1);
            Ok(())
        });

        let mut recipient_dao = MockMailRecipientDao::new();
        let expected_addr = from_addr.clone();
        recipient_dao.expect_create().returning(move |r| {
            assert_eq!(r.to_address.as_ref(), expected_addr.as_str());
            assert_eq!(r.status.as_ref(), "pending");
            Ok(())
        });

        let cfg = MockConfigService::new();
        let imap = MockInboxImapClient::new();
        let svc = InboxServiceImpl::new(
            Arc::new(cfg),
            Arc::new(dao),
            Arc::new(imap),
            Arc::new(job_dao),
            Arc::new(recipient_dao),
            Arc::new(MockInboundMailAttachmentDao::new()),
            Arc::new(MockDocumentStorage::new()),
            Arc::new(MockMailRecipientAttachmentDao::new()),
            Arc::new(MockMailJobStaticAttachmentDao::new()),
            Arc::new(MockStaticDocumentDao::new()),
        );
        let job = svc
            .reply(mail_id, "Re: s", "My reply", vec![], vec![], None)
            .await
            .unwrap();
        assert_eq!(job.subject.as_ref(), "Re: s");
        assert_eq!(job.reply_to_inbound_mail_id, Some(mail_id));
    }

    #[tokio::test]
    async fn reply_to_nonexistent_mail_returns_not_found() {
        let mut dao = MockInboundMailDao::new();
        dao.expect_find_by_id().returning(|_| Ok(None));

        let cfg = MockConfigService::new();
        let imap = MockInboxImapClient::new();
        let svc = InboxServiceImpl::new(
            Arc::new(cfg),
            Arc::new(dao),
            Arc::new(imap),
            Arc::new(MockMailJobDao::new()),
            Arc::new(MockMailRecipientDao::new()),
            Arc::new(MockInboundMailAttachmentDao::new()),
            Arc::new(MockDocumentStorage::new()),
            Arc::new(MockMailRecipientAttachmentDao::new()),
            Arc::new(MockMailJobStaticAttachmentDao::new()),
            Arc::new(MockStaticDocumentDao::new()),
        );
        let result = svc
            .reply(Uuid::new_v4(), "Re: x", "body", vec![], vec![], None)
            .await;
        assert!(matches!(result, Err(MailServiceError::NotFound)));
    }

    // ── Quick 260607-s0s: reply-with-attachments tests ─────────────────

    fn sample_attachment_input() -> AttachmentInput {
        AttachmentInput {
            document_id: Uuid::new_v4(),
            file_name: "doc.pdf".to_string(),
            mime_type: "application/pdf".to_string(),
            relative_path: "member_documents/x/y".to_string(),
        }
    }

    /// Quick 260607-s0s: reply with 2 member-doc AttachmentInputs must create
    /// exactly 2 MailRecipientAttachment rows, each carrying the freshly
    /// created `recipient.id`. Verifies the persistence loop mirrors
    /// MailServiceImpl::create_job (service.rs:350-359).
    #[tokio::test]
    async fn reply_creates_attachment_rows_for_member_doc_attachment_ids() {
        let mail = sample_mail();
        let mail_id = mail.id;

        let mut dao = MockInboundMailDao::new();
        let returned = mail.clone();
        dao.expect_find_by_id()
            .returning(move |_| Ok(Some(returned.clone())));
        dao.expect_update().returning(|_| Ok(()));

        let mut job_dao = MockMailJobDao::new();
        job_dao.expect_create().times(1).returning(|_| Ok(()));

        let mut recipient_dao = MockMailRecipientDao::new();
        // Capture the recipient.id assigned by `reply` so we can assert each
        // attachment row references that exact id.
        let captured_recipient_id = Arc::new(std::sync::Mutex::new(None::<Uuid>));
        let captured_for_recipient = captured_recipient_id.clone();
        recipient_dao.expect_create().times(1).returning(move |r| {
            *captured_for_recipient.lock().unwrap() = Some(r.id);
            Ok(())
        });

        // CRITICAL: recipient_attachment_dao.create MUST be called exactly twice,
        // once per AttachmentInput, with the recipient.id from above.
        let mut recipient_attachment_dao = MockMailRecipientAttachmentDao::new();
        let captured_for_attachment = captured_recipient_id.clone();
        recipient_attachment_dao
            .expect_create()
            .times(2)
            .returning(move |att| {
                let expected = captured_for_attachment.lock().unwrap();
                assert_eq!(
                    Some(att.recipient_id),
                    *expected,
                    "attachment.recipient_id must match the newly-created recipient.id"
                );
                Ok(())
            });

        let cfg = MockConfigService::new();
        let imap = MockInboxImapClient::new();
        let svc = InboxServiceImpl::new(
            Arc::new(cfg),
            Arc::new(dao),
            Arc::new(imap),
            Arc::new(job_dao),
            Arc::new(recipient_dao),
            Arc::new(MockInboundMailAttachmentDao::new()),
            Arc::new(MockDocumentStorage::new()),
            Arc::new(recipient_attachment_dao),
            Arc::new(MockMailJobStaticAttachmentDao::new()),
            Arc::new(MockStaticDocumentDao::new()),
        );

        let attachments = vec![sample_attachment_input(), sample_attachment_input()];
        let job = svc
            .reply(mail_id, "Re: with att", "body", attachments, vec![], None)
            .await
            .unwrap();
        assert_eq!(job.reply_to_inbound_mail_id, Some(mail_id));
    }

    /// Quick 260607-s0s: reply with 2 static_document_ids must create exactly
    /// 2 MailJobStaticAttachment join rows, each carrying job.id. Verifies the
    /// static-doc validation runs first (find_many_by_ids must return both)
    /// and the persistence loop mirrors service.rs:362-368.
    #[tokio::test]
    async fn reply_creates_static_doc_joins_for_static_document_ids() {
        let mail = sample_mail();
        let mail_id = mail.id;

        let mut dao = MockInboundMailDao::new();
        let returned = mail.clone();
        dao.expect_find_by_id()
            .returning(move |_| Ok(Some(returned.clone())));
        dao.expect_update().returning(|_| Ok(()));

        let static_a = Uuid::new_v4();
        let static_b = Uuid::new_v4();
        let static_ids = vec![static_a, static_b];

        let mut static_document_dao = MockStaticDocumentDao::new();
        // Must be called exactly once (validation) and return the same count.
        static_document_dao
            .expect_find_many_by_ids()
            .times(1)
            .returning(|ids| {
                let now = time::OffsetDateTime::now_utc();
                let now_primitive = time::PrimitiveDateTime::new(now.date(), now.time());
                let docs: Vec<StaticDocument> = ids
                    .iter()
                    .map(|id| StaticDocument {
                        id: *id,
                        created: now_primitive,
                        deleted: None,
                        version: Uuid::new_v4(),
                        name: Arc::from("name"),
                        filename: Arc::from("f.pdf"),
                        content_type: Arc::from("application/pdf"),
                        size_bytes: 1,
                    })
                    .collect();
                Ok(docs.into())
            });

        let mut job_dao = MockMailJobDao::new();
        // Capture job.id so the static-attachment-join expectation can assert it.
        let captured_job_id = Arc::new(std::sync::Mutex::new(None::<Uuid>));
        let captured_for_job = captured_job_id.clone();
        job_dao.expect_create().times(1).returning(move |j| {
            *captured_for_job.lock().unwrap() = Some(j.id);
            Ok(())
        });

        let mut recipient_dao = MockMailRecipientDao::new();
        recipient_dao.expect_create().times(1).returning(|_| Ok(()));

        let mut mail_job_static_attachment_dao = MockMailJobStaticAttachmentDao::new();
        let captured_for_join = captured_job_id.clone();
        mail_job_static_attachment_dao
            .expect_create()
            .times(2)
            .returning(move |join| {
                let expected = captured_for_join.lock().unwrap();
                assert_eq!(
                    Some(join.mail_job_id),
                    *expected,
                    "join.mail_job_id must match the newly-created job.id"
                );
                Ok(())
            });

        let cfg = MockConfigService::new();
        let imap = MockInboxImapClient::new();
        let svc = InboxServiceImpl::new(
            Arc::new(cfg),
            Arc::new(dao),
            Arc::new(imap),
            Arc::new(job_dao),
            Arc::new(recipient_dao),
            Arc::new(MockInboundMailAttachmentDao::new()),
            Arc::new(MockDocumentStorage::new()),
            Arc::new(MockMailRecipientAttachmentDao::new()),
            Arc::new(mail_job_static_attachment_dao),
            Arc::new(static_document_dao),
        );

        let job = svc
            .reply(mail_id, "Re: static", "body", vec![], static_ids, None)
            .await
            .unwrap();
        assert_eq!(job.reply_to_inbound_mail_id, Some(mail_id));
    }

    /// Quick 260607-s0s: backwards-compat — reply with empty attachment vecs
    /// behaves EXACTLY like before. None of the new DAOs are touched.
    #[tokio::test]
    async fn reply_with_no_attachments_preserves_existing_behavior() {
        let mail = sample_mail();
        let mail_id = mail.id;

        let mut dao = MockInboundMailDao::new();
        let returned = mail.clone();
        dao.expect_find_by_id()
            .returning(move |_| Ok(Some(returned.clone())));
        dao.expect_update().returning(|m| {
            assert!(m.replied);
            Ok(())
        });

        let mut job_dao = MockMailJobDao::new();
        job_dao.expect_create().times(1).returning(|_| Ok(()));

        let mut recipient_dao = MockMailRecipientDao::new();
        recipient_dao.expect_create().times(1).returning(|_| Ok(()));

        // CRITICAL: none of the new DAOs are called for an empty-attachment reply.
        let mut recipient_attachment_dao = MockMailRecipientAttachmentDao::new();
        recipient_attachment_dao.expect_create().times(0);

        let mut mail_job_static_attachment_dao = MockMailJobStaticAttachmentDao::new();
        mail_job_static_attachment_dao.expect_create().times(0);

        let mut static_document_dao = MockStaticDocumentDao::new();
        static_document_dao.expect_find_many_by_ids().times(0);

        let cfg = MockConfigService::new();
        let imap = MockInboxImapClient::new();
        let svc = InboxServiceImpl::new(
            Arc::new(cfg),
            Arc::new(dao),
            Arc::new(imap),
            Arc::new(job_dao),
            Arc::new(recipient_dao),
            Arc::new(MockInboundMailAttachmentDao::new()),
            Arc::new(MockDocumentStorage::new()),
            Arc::new(recipient_attachment_dao),
            Arc::new(mail_job_static_attachment_dao),
            Arc::new(static_document_dao),
        );

        let job = svc
            .reply(mail_id, "Re: plain", "body", vec![], vec![], None)
            .await
            .unwrap();
        assert_eq!(job.reply_to_inbound_mail_id, Some(mail_id));
    }

    /// Phase 24 (EDIT-01, D-01): the sanitize-on-store gate MUST run at the
    /// InboxService::reply entry point. `<script>` tags are stripped by ammonia,
    /// safe markup (`<p>`) survives. Mirrors the pattern established in Phase
    /// 23 Plan 04 for `create_job_sanitizes_body_html`.
    #[tokio::test]
    async fn reply_sanitizes_body_html_on_store() {
        let mail = sample_mail();
        let mail_id = mail.id;

        let mut dao = MockInboundMailDao::new();
        let returned = mail.clone();
        dao.expect_find_by_id()
            .returning(move |_| Ok(Some(returned.clone())));
        dao.expect_update().returning(|_| Ok(()));

        // Capture the persisted MailJob so we can assert on its body_html.
        let captured_body_html: Arc<std::sync::Mutex<Option<Option<Arc<str>>>>> =
            Arc::new(std::sync::Mutex::new(None));
        let captured_for_job = captured_body_html.clone();
        let mut job_dao = MockMailJobDao::new();
        job_dao.expect_create().times(1).returning(move |j| {
            *captured_for_job.lock().unwrap() = Some(j.body_html.clone());
            Ok(())
        });

        let mut recipient_dao = MockMailRecipientDao::new();
        recipient_dao.expect_create().times(1).returning(|_| Ok(()));

        let cfg = MockConfigService::new();
        let imap = MockInboxImapClient::new();
        let svc = InboxServiceImpl::new(
            Arc::new(cfg),
            Arc::new(dao),
            Arc::new(imap),
            Arc::new(job_dao),
            Arc::new(recipient_dao),
            Arc::new(MockInboundMailAttachmentDao::new()),
            Arc::new(MockDocumentStorage::new()),
            Arc::new(MockMailRecipientAttachmentDao::new()),
            Arc::new(MockMailJobStaticAttachmentDao::new()),
            Arc::new(MockStaticDocumentDao::new()),
        );

        let malicious = "<script>alert(1)</script><p>ok</p>".to_string();
        let _job = svc
            .reply(mail_id, "s", "b", vec![], vec![], Some(malicious))
            .await
            .unwrap();

        let persisted = captured_body_html
            .lock()
            .unwrap()
            .clone()
            .expect("job_dao.create must have been called");
        let persisted = persisted.expect("body_html MUST be Some after Some input");
        let persisted_str: &str = persisted.as_ref();
        assert!(
            !persisted_str.contains("<script>"),
            "sanitize gate MUST strip <script>, got: {persisted_str}",
        );
        assert!(
            !persisted_str.contains("alert(1)"),
            "sanitize gate MUST strip the script contents, got: {persisted_str}",
        );
        assert!(
            persisted_str.contains("<p>ok</p>"),
            "safe markup <p> MUST survive, got: {persisted_str}",
        );
    }

    // ── Phase 19: Attachment pipeline tests ────────────────────────────

    /// Test A — parse_raw_mail extracts attachments from a multipart message.
    /// Uses a small inline PNG (1×1 transparent pixel, base64) so the test
    /// stays self-contained.
    #[test]
    fn test_parse_raw_mail_extracts_attachments() {
        // 1×1 transparent PNG (67 bytes). Base64 encoded inline.
        let raw = b"From: sender@example.com\r\n\
                    To: inbox@example.com\r\n\
                    Subject: Mail mit Anhang\r\n\
                    MIME-Version: 1.0\r\n\
                    Content-Type: multipart/mixed; boundary=BOUNDARY\r\n\
                    \r\n\
                    --BOUNDARY\r\n\
                    Content-Type: text/plain\r\n\
                    \r\n\
                    Hallo, im Anhang ist ein Bild.\r\n\
                    --BOUNDARY\r\n\
                    Content-Type: image/png\r\n\
                    Content-Transfer-Encoding: base64\r\n\
                    Content-Disposition: attachment; filename=\"test.png\"\r\n\
                    \r\n\
                    iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=\r\n\
                    --BOUNDARY--\r\n";
        let p = parse_raw_mail(raw);
        assert_eq!(p.attachments.len(), 1, "expected exactly one attachment");
        assert_eq!(p.attachments[0].file_name, "test.png");
        assert_eq!(p.attachments[0].mime_type, "image/png");
        assert!(
            !p.attachments[0].bytes.is_empty(),
            "attachment bytes should be non-empty"
        );
        assert!(p.has_attachments);
        assert!(p.attachments[0].declared_size > 0);
        assert_eq!(
            p.attachments[0].declared_size as usize,
            p.attachments[0].bytes.len(),
            "under the cap, declared_size must match bytes.len()"
        );
    }

    /// Test B — oversized attachment skips DocumentStorage entirely and
    /// persists a metadata-only row (`oversized=true`, `relative_path=None`).
    #[tokio::test]
    async fn test_persist_attachment_oversized_skips_storage() {
        let mut storage = MockDocumentStorage::new();
        // The critical assertion: storage.save MUST NOT be called for an
        // oversized attachment (D-02, T-01).
        storage.expect_save().times(0);

        let mut dao = MockInboundMailAttachmentDao::new();
        dao.expect_create().times(1).returning(|a| {
            assert!(a.oversized, "oversized flag must be true");
            assert!(
                a.relative_path.is_none(),
                "relative_path must be None for oversized"
            );
            Ok(())
        });

        // 1 byte over the 10 MB hard cap.
        let bytes = vec![0u8; (ATTACHMENT_MAX_BYTES as usize) + 1];
        let declared = bytes.len() as u64;
        let mail_id = Uuid::new_v4();
        let result = persist_attachment(
            &storage,
            &dao,
            mail_id,
            "big.bin",
            "image/png",
            &bytes,
            declared,
        )
        .await
        .unwrap();
        assert!(
            result.oversized,
            "returned entity must report oversized=true"
        );
        assert!(
            result.relative_path.is_none(),
            "returned entity must have None relative_path"
        );
    }

    /// Phase 19 Plan 04 — Backfill silently skips candidates when the IMAP
    /// refetch returns Err or Ok(None). The critical assertions are mockall
    /// `expect_*().times(0)` on `attachment_dao.create` and `storage.save`:
    /// when refetch yields no message, the persist pipeline MUST NOT run.
    /// This is the T-06 / D-06 mitigation: IMAP drift → silent skip, never
    /// half-persisted state.
    #[tokio::test]
    async fn test_run_attachment_backfill_silent_skips_imap_error() {
        // Two candidate mails — has_attachments=true, no existing rows.
        let mut mail_a = sample_mail();
        mail_a.has_attachments = true;
        mail_a.uid_validity = 7;
        mail_a.imap_uid = 100;
        let mut mail_b = sample_mail();
        mail_b.has_attachments = true;
        mail_b.uid_validity = 7;
        mail_b.imap_uid = 200;
        let mail_a_id = mail_a.id;
        let mail_b_id = mail_b.id;

        // Config stub: returns enough keys for load_imap_config to succeed.
        let config_service = mock_config(vec![
            cfg_entry("imap_host", "imap.example.com", "string"),
            cfg_entry("imap_user", "me", "string"),
            cfg_entry("imap_pass", "secret", "secret"),
        ]);

        // mail_dao.list_active returns both candidates.
        let mut mail_dao = MockInboundMailDao::new();
        let mails_for_stub: Arc<[InboundMail]> = vec![mail_a.clone(), mail_b.clone()].into();
        mail_dao
            .expect_list_active()
            .times(1)
            .returning(move || Ok(mails_for_stub.clone()));

        // attachment_dao.count_for_mail returns Ok(0) for both → both selected.
        // CRITICAL: attachment_dao.create is NEVER called (mockall verifies on Drop).
        let mut attachment_dao = MockInboundMailAttachmentDao::new();
        attachment_dao
            .expect_count_for_mail()
            .times(2)
            .returning(|_| Ok(0));
        attachment_dao.expect_create().times(0);

        // storage.save is NEVER called.
        let mut storage = MockDocumentStorage::new();
        storage.expect_save().times(0);

        // imap_client.fetch_one_by_uid: first → Err, second → Ok(None).
        // Both responses cause silent-skip; neither triggers persist.
        let mut imap_client = MockInboxImapClient::new();
        imap_client.expect_uid_validity().times(0); // backfill does NOT call uid_validity — only fetch_one_by_uid per candidate
        let mut call_count = 0u32;
        imap_client
            .expect_fetch_one_by_uid()
            .times(2)
            .returning(move |_, _, _| {
                call_count += 1;
                if call_count == 1 {
                    Err(MailServiceError::DataAccess(Arc::from(
                        "simulated IMAP failure",
                    )))
                } else {
                    Ok(None)
                }
            });

        // Run to completion — must not panic, must not persist anything.
        run_attachment_backfill(
            Arc::new(config_service),
            Arc::new(mail_dao),
            Arc::new(attachment_dao),
            Arc::new(storage),
            Arc::new(imap_client),
        )
        .await;

        // Sanity: both mail IDs are non-nil (defensive — verifies the test
        // setup didn't degenerate to a single mail).
        assert_ne!(mail_a_id, mail_b_id);
    }

    /// Test C — when an attachment is already persisted (count_for_mail > 0)
    /// the candidate is NOT refetched. Verifies the idempotency-on-restart
    /// guard (no double persist when backfill runs twice).
    #[tokio::test]
    async fn test_run_attachment_backfill_skips_already_backfilled() {
        let mut mail = sample_mail();
        mail.has_attachments = true;

        let config_service = mock_config(vec![
            cfg_entry("imap_host", "imap.example.com", "string"),
            cfg_entry("imap_user", "me", "string"),
            cfg_entry("imap_pass", "secret", "secret"),
        ]);

        let mut mail_dao = MockInboundMailDao::new();
        let mails_for_stub: Arc<[InboundMail]> = vec![mail.clone()].into();
        mail_dao
            .expect_list_active()
            .times(1)
            .returning(move || Ok(mails_for_stub.clone()));

        // count_for_mail returns 2 → already backfilled.
        let mut attachment_dao = MockInboundMailAttachmentDao::new();
        attachment_dao
            .expect_count_for_mail()
            .times(1)
            .returning(|_| Ok(2));
        attachment_dao.expect_create().times(0);

        let mut storage = MockDocumentStorage::new();
        storage.expect_save().times(0);

        // CRITICAL: imap_client.fetch_one_by_uid MUST NOT be called.
        let mut imap_client = MockInboxImapClient::new();
        imap_client.expect_fetch_one_by_uid().times(0);

        run_attachment_backfill(
            Arc::new(config_service),
            Arc::new(mail_dao),
            Arc::new(attachment_dao),
            Arc::new(storage),
            Arc::new(imap_client),
        )
        .await;
    }

    /// Test C — save-then-DB rollback: when the DB create fails, storage.delete
    /// is invoked to remove the orphaned file (T-07).
    #[tokio::test]
    async fn test_persist_attachment_rollback_on_db_fail() {
        let mut storage = MockDocumentStorage::new();
        storage
            .expect_save()
            .times(1)
            .returning(|_, _| Ok::<(), StorageError>(()));
        storage
            .expect_delete()
            .times(1)
            .returning(|_| Ok::<(), StorageError>(()));

        let mut dao = MockInboundMailAttachmentDao::new();
        dao.expect_create().times(1).returning(|_| {
            Err(MailDaoError::DatabaseError(Arc::from(
                "simulated DB failure",
            )))
        });

        let bytes = vec![1u8; 1024]; // 1 KB, well under the cap
        let declared = bytes.len() as u64;
        let mail_id = Uuid::new_v4();
        let result = persist_attachment(
            &storage,
            &dao,
            mail_id,
            "doc.pdf",
            "application/pdf",
            &bytes,
            declared,
        )
        .await;
        assert!(matches!(result, Err(MailServiceError::DataAccess(_))));
        // mockall verifies expect_save().times(1) and expect_delete().times(1)
        // on Drop — they document the save-then-DB-then-rollback flow.
    }

    /// Phase 19 gap-closure (CR-01): extract_attachments fuehrt einen
    /// Probe-Read durch und allokiert die Bytes NICHT, wenn das Attachment
    /// die 10-MB-Cap (ATTACHMENT_MAX_BYTES) ueberschreitet. Beweist D-02 als
    /// Memory-DoS-Schutz VOR der Heap-Allokation.
    #[test]
    fn test_extract_attachments_oversized_skips_materialization() {
        // Body-part mit > ATTACHMENT_MAX_BYTES roher Payload (kein Base64,
        // damit decoded length == raw length und der probe-read greift).
        let oversized_payload = vec![b'A'; (ATTACHMENT_MAX_BYTES as usize) + 1024];
        let mut raw = Vec::new();
        raw.extend_from_slice(
            b"From: sender@example.com\r\n\
              To: inbox@example.com\r\n\
              Subject: Oversized Anhang\r\n\
              MIME-Version: 1.0\r\n\
              Content-Type: multipart/mixed; boundary=BOUNDARY\r\n\
              \r\n\
              --BOUNDARY\r\n\
              Content-Type: text/plain\r\n\
              \r\n\
              Anhang folgt.\r\n\
              --BOUNDARY\r\n\
              Content-Type: application/octet-stream\r\n\
              Content-Transfer-Encoding: 8bit\r\n\
              Content-Disposition: attachment; filename=\"huge.bin\"\r\n\
              \r\n",
        );
        raw.extend_from_slice(&oversized_payload);
        raw.extend_from_slice(b"\r\n--BOUNDARY--\r\n");

        let parsed = parse_raw_mail(&raw);
        assert_eq!(
            parsed.attachments.len(),
            1,
            "expected exactly one attachment"
        );
        let att = &parsed.attachments[0];
        assert!(
            att.declared_size > ATTACHMENT_MAX_BYTES,
            "declared_size ({}) must exceed cap ({}) — probe-read must record the real size",
            att.declared_size,
            ATTACHMENT_MAX_BYTES
        );
        assert!(
            att.bytes.is_empty(),
            "oversized attachment MUST NOT materialize bytes (got {} bytes; expected 0). \
             This proves part.contents().to_vec() was NOT called above the cap.",
            att.bytes.len()
        );
    }
}
