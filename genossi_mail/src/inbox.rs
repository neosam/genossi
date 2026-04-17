//! Member inbox: polling IMAP, storing inbound mails, and exposing them for
//! the REST layer. IMAP access is abstracted behind [`InboxImapClient`] so that
//! the service and worker logic can be unit-tested without a real server.

use async_trait::async_trait;
use mockall::automock;
use std::sync::Arc;
use uuid::Uuid;

use crate::dao::{
    InboundMail, InboundMailDao, MailJob, MailJobDao, MailRecipient, MailRecipientDao,
};
use crate::service::MailServiceError;
use genossi_config::service::ConfigService;

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

    let (
        from_address,
        subject,
        received_at,
        body_text,
        html,
        has_attachments,
        in_reply_to,
        message_id,
    ) = if let Some(msg) = parsed {
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

        let has_attachments = msg.attachment_count() > 0;

        let in_reply_to = msg
            .in_reply_to()
            .as_text()
            .map(|s| s.to_string())
            .and_then(|s| crate::dao::normalize_message_id(&s));

        let message_id = msg
            .message_id()
            .and_then(|s| crate::dao::normalize_message_id(s));

        (
            from_address,
            subject,
            received_at,
            body_text,
            html,
            has_attachments,
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
            false,
            None,
            None,
        )
    };

    let has_html_body = html.is_some();
    let raw_html_body = html;

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
    async fn reply(&self, id: Uuid, subject: &str, body: &str)
        -> Result<MailJob, MailServiceError>;
}

pub struct InboxServiceImpl<C, D, I, J, R>
where
    C: ConfigService,
    D: InboundMailDao,
    I: InboxImapClient,
    J: MailJobDao,
    R: MailRecipientDao,
{
    pub config_service: Arc<C>,
    pub dao: Arc<D>,
    pub imap_client: Arc<I>,
    pub job_dao: Arc<J>,
    pub recipient_dao: Arc<R>,
}

impl<C, D, I, J, R> InboxServiceImpl<C, D, I, J, R>
where
    C: ConfigService,
    D: InboundMailDao,
    I: InboxImapClient,
    J: MailJobDao,
    R: MailRecipientDao,
{
    pub fn new(
        config_service: Arc<C>,
        dao: Arc<D>,
        imap_client: Arc<I>,
        job_dao: Arc<J>,
        recipient_dao: Arc<R>,
    ) -> Self {
        Self {
            config_service,
            dao,
            imap_client,
            job_dao,
            recipient_dao,
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
impl<C, D, I, J, R> InboxService for InboxServiceImpl<C, D, I, J, R>
where
    C: ConfigService,
    D: InboundMailDao,
    I: InboxImapClient,
    J: MailJobDao,
    R: MailRecipientDao,
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
    ) -> Result<MailJob, MailServiceError> {
        let mut mail = self.load_mail(id).await?;

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
        };
        self.recipient_dao.create(&recipient).await?;

        mail.replied = true;
        mail.version = Uuid::new_v4();
        self.dao.update(&mail).await?;

        Ok(job)
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Inbox worker
// ────────────────────────────────────────────────────────────────────────────

pub async fn start_inbox_worker<C, D, I>(config_service: Arc<C>, dao: Arc<D>, imap_client: Arc<I>)
where
    C: ConfigService,
    D: InboundMailDao,
    I: InboxImapClient,
{
    loop {
        let interval = load_poll_interval(config_service.as_ref()).await;
        if let Err(e) = poll_once(config_service.as_ref(), dao.as_ref(), imap_client.as_ref()).await
        {
            tracing::warn!("Inbox worker: poll cycle failed: {:?}", e);
        }
        tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
    }
}

/// One poll cycle: load config, fetch new UIDs, parse, insert.
/// Returns Ok(inserted_count) on success.
pub async fn poll_once<C, D, I>(
    config_service: &C,
    dao: &D,
    imap_client: &I,
) -> Result<usize, MailServiceError>
where
    C: ConfigService,
    D: InboundMailDao,
    I: InboxImapClient,
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
        }
    }
    if inserted > 0 {
        tracing::info!("Inbox worker: stored {} new inbound mail(s)", inserted);
    }
    Ok(inserted)
}

// ────────────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dao::{MockInboundMailDao, MockMailJobDao, MockMailRecipientDao};
    use genossi_config::dao::ConfigEntry;
    use genossi_config::service::MockConfigService;

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
        );
        let res = svc.archive(mail_id).await;
        assert!(matches!(res, Err(MailServiceError::ConfigMissing(_))));
    }

    #[tokio::test]
    async fn poll_once_skips_when_unconfigured() {
        let dao = MockInboundMailDao::new();
        let imap = MockInboxImapClient::new();
        let cfg = mock_config(vec![]); // no imap_host
        let n = poll_once(&cfg, &dao, &imap).await.unwrap();
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

        let n = poll_once(&cfg, &dao, &imap).await.unwrap();
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

        let n = poll_once(&cfg, &dao, &imap).await.unwrap();
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
        );
        let job = svc.reply(mail_id, "Re: s", "My reply").await.unwrap();
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
        );
        let result = svc.reply(Uuid::new_v4(), "Re: x", "body").await;
        assert!(matches!(result, Err(MailServiceError::NotFound)));
    }
}
