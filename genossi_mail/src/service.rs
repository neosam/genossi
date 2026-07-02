use async_trait::async_trait;
use mockall::automock;
use std::sync::Arc;
use uuid::Uuid;

use crate::dao::{
    MailJob, MailJobDao, MailJobStaticAttachment, MailJobStaticAttachmentDao, MailRecipient,
    MailRecipientAttachment, MailRecipientAttachmentDao, MailRecipientDao, StaticDocumentDao,
};
use genossi_config::dao::ConfigEntry;
use genossi_config::service::ConfigService;

#[derive(Debug, Clone)]
pub enum MailServiceError {
    ConfigMissing(Arc<str>),
    SmtpError(Arc<str>),
    DataAccess(Arc<str>),
    NotFound,
    TemplateValidation(Arc<str>),
    BadRequest(Arc<str>),
}

impl From<serde_json::Error> for MailServiceError {
    fn from(e: serde_json::Error) -> Self {
        MailServiceError::DataAccess(Arc::from(format!("serialize failed: {}", e)))
    }
}

impl From<crate::dao::MailDaoError> for MailServiceError {
    fn from(e: crate::dao::MailDaoError) -> Self {
        match e {
            crate::dao::MailDaoError::DatabaseError(msg) => MailServiceError::DataAccess(msg),
            crate::dao::MailDaoError::NotFound => MailServiceError::NotFound,
        }
    }
}

pub struct RecipientInput {
    pub address: String,
    pub member_id: Option<Uuid>,
}

pub struct AttachmentInput {
    pub document_id: Uuid,
    pub file_name: String,
    pub mime_type: String,
    pub relative_path: String,
}

#[automock]
#[async_trait]
pub trait MailService: Send + Sync + 'static {
    /// Create a mail job with the given recipients. Returns the created job.
    /// If attachment_inputs is non-empty, recipients must contain exactly one entry.
    /// `static_document_ids` are job-level attachments delivered to every recipient.
    ///
    /// Phase 10:
    /// - `template_id` (D-12): optional reference to MailTemplate used to render this job's
    ///   subject/body. Worker (Plan 10.06) uses this to populate MemberDocument.template_id.
    /// - `repayment_phase_id` (D-03): optional reference to RepaymentPhase. If set, the worker
    ///   merges per-recipient payout context (payout_amount/share_count/fiscal_year) into the
    ///   minijinja render.
    ///
    /// Quick 260603-cz6:
    /// - `attach_repayment_letter`: opt-in. When `true` AND `repayment_phase_id.is_some()`,
    ///   the worker resolves the per-recipient `DocumentType::RepaymentLetter` MemberDocument
    ///   (Description-Fingerprint `"Anschreiben Auszahlung GJ {fiscal_year}"`, Phase 13 D-LETT-04)
    ///   and attaches the file in-memory before send. Recipients with 0 matching letters are
    ///   marked failed with `error="no_repayment_letter"`. REST-layer rejects `true` when
    ///   `repayment_phase_id` is `None` (400 BadRequest).
    async fn create_job(
        &self,
        subject: &str,
        body: &str,
        recipients: Vec<RecipientInput>,
        attachment_inputs: Vec<AttachmentInput>,
        static_document_ids: Vec<Uuid>,
        template_id: Option<Uuid>,
        repayment_phase_id: Option<Uuid>,
        attach_repayment_letter: bool,
    ) -> Result<MailJob, MailServiceError>;

    /// Get all mail jobs ordered by created DESC.
    async fn get_jobs(&self) -> Result<Arc<[MailJob]>, MailServiceError>;

    /// Get a mail job with all its recipients.
    async fn get_job_with_recipients(
        &self,
        job_id: Uuid,
    ) -> Result<(MailJob, Arc<[MailRecipient]>), MailServiceError>;

    /// Retry failed recipients of a job: reset them to pending.
    async fn retry_job(&self, job_id: Uuid) -> Result<MailJob, MailServiceError>;

    /// Send a test email synchronously (no job, direct SMTP).
    async fn send_test_mail(&self, to: &str) -> Result<(), MailServiceError>;

    /// Quick 260603-jtf: Send a test mail with a caller-provided subject and body
    /// synchronously (no job, direct SMTP). Sibling to `send_test_mail` (which uses
    /// a hard-coded constant body for SMTP-config smoke-testing on the Settings
    /// page). This variant is used by the Mail-Template editor's "Test-Template"
    /// flow: the REST handler renders the template against a Member's variables
    /// and then forwards the rendered subject/body here. The `to` argument is the
    /// **explicit** test-recipient address from the request body — NEVER the
    /// resolved Member's email (privacy defense, see
    /// `genossi_mail/src/rest.rs::send_test_mail_with_template`).
    async fn send_test_mail_with_body(
        &self,
        to: &str,
        subject: &str,
        body: &str,
    ) -> Result<(), MailServiceError>;

    /// Get member IDs that were successfully reached (status = "sent") for a given job.
    async fn get_reached_member_ids(&self, job_id: Uuid) -> Result<Arc<[Uuid]>, MailServiceError>;
}

/// Message-content transfer encoding for outgoing SMTP mail bodies.
///
/// Selected via the `smtp_encoding` KV config key (Phase 22, MAIL-03). Default
/// (key absent, empty, or unknown value) is `QuotedPrintable` — production
/// behavior is unchanged unless the operator opts in with `smtp_encoding=8bit`.
///
/// Plan 02 consumes this enum in `send.rs::build_message` to pin the
/// Content-Transfer-Encoding header on outgoing messages.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MailEncoding {
    QuotedPrintable,
    EightBit,
}

pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub pass: String,
    pub from: String,
    pub tls: String,
    pub encoding: MailEncoding,
}

pub async fn load_smtp_config<C: ConfigService>(
    config_service: &C,
) -> Result<SmtpConfig, MailServiceError> {
    let required_keys = [
        "smtp_host",
        "smtp_port",
        "smtp_user",
        "smtp_pass",
        "smtp_from",
    ];
    let mut missing = Vec::new();

    let all_config = config_service.get_all().await.map_err(|e| {
        MailServiceError::DataAccess(Arc::from(format!("Failed to load config: {:?}", e)))
    })?;

    let find =
        |key: &str| -> Option<&ConfigEntry> { all_config.iter().find(|e| e.key.as_ref() == key) };

    for key in &required_keys {
        if find(key).is_none() {
            missing.push(*key);
        }
    }

    if !missing.is_empty() {
        return Err(MailServiceError::ConfigMissing(Arc::from(format!(
            "Missing SMTP config keys: {}",
            missing.join(", ")
        ))));
    }

    let port: u16 = find("smtp_port").unwrap().value.parse().map_err(|_| {
        MailServiceError::ConfigMissing(Arc::from("smtp_port is not a valid port number"))
    })?;

    let tls = find("smtp_tls")
        .map(|e| e.value.to_string())
        .unwrap_or_else(|| "starttls".to_string());

    // MAIL-03 (Phase 22, D-07/D-08): tolerant fallback — unknown values log a
    // warning and revert to QuotedPrintable. Mirrors the smtp_tls policy above
    // (also NOT in `required_keys` — the key is optional). Do NOT hard-error on
    // typos; a rogue value must not disable mail.
    let encoding = match find("smtp_encoding").map(|e| e.value.as_ref()) {
        Some("8bit") => MailEncoding::EightBit,
        Some("quoted-printable") | Some("") | None => MailEncoding::QuotedPrintable,
        Some(other) => {
            tracing::warn!(
                value = %other,
                "Unknown smtp_encoding value — falling back to quoted-printable"
            );
            MailEncoding::QuotedPrintable
        }
    };

    let from_email = find("smtp_from").unwrap().value.to_string();
    let from = match find("smtp_from_name") {
        Some(name) if !name.value.is_empty() => format!("{} <{}>", name.value, from_email),
        _ => from_email,
    };

    Ok(SmtpConfig {
        host: find("smtp_host").unwrap().value.to_string(),
        port,
        user: find("smtp_user").unwrap().value.to_string(),
        pass: find("smtp_pass").unwrap().value.to_string(),
        from,
        tls,
        encoding,
    })
}

pub fn build_transport(
    config: &SmtpConfig,
) -> Result<lettre::AsyncSmtpTransport<lettre::Tokio1Executor>, MailServiceError> {
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::AsyncSmtpTransport;

    let creds = Credentials::new(config.user.clone(), config.pass.clone());

    let transport = match config.tls.as_str() {
        "none" => AsyncSmtpTransport::<lettre::Tokio1Executor>::builder_dangerous(&config.host)
            .port(config.port)
            .credentials(creds)
            .build(),
        "tls" => AsyncSmtpTransport::<lettre::Tokio1Executor>::relay(&config.host)
            .map_err(|e| MailServiceError::SmtpError(Arc::from(e.to_string())))?
            .port(config.port)
            .credentials(creds)
            .build(),
        _ => {
            // Default to starttls
            AsyncSmtpTransport::<lettre::Tokio1Executor>::starttls_relay(&config.host)
                .map_err(|e| MailServiceError::SmtpError(Arc::from(e.to_string())))?
                .port(config.port)
                .credentials(creds)
                .build()
        }
    };

    Ok(transport)
}

pub struct MailServiceImpl<
    C: ConfigService,
    J: MailJobDao,
    R: MailRecipientDao,
    A: MailRecipientAttachmentDao,
    S: StaticDocumentDao,
    M: MailJobStaticAttachmentDao,
> {
    config_service: Arc<C>,
    job_dao: Arc<J>,
    recipient_dao: Arc<R>,
    attachment_dao: Arc<A>,
    static_document_dao: Arc<S>,
    mail_job_static_attachment_dao: Arc<M>,
}

impl<
        C: ConfigService,
        J: MailJobDao,
        R: MailRecipientDao,
        A: MailRecipientAttachmentDao,
        S: StaticDocumentDao,
        M: MailJobStaticAttachmentDao,
    > MailServiceImpl<C, J, R, A, S, M>
{
    pub fn new(
        config_service: C,
        job_dao: J,
        recipient_dao: R,
        attachment_dao: A,
        static_document_dao: S,
        mail_job_static_attachment_dao: M,
    ) -> Self {
        Self {
            config_service: Arc::new(config_service),
            job_dao: Arc::new(job_dao),
            recipient_dao: Arc::new(recipient_dao),
            attachment_dao: Arc::new(attachment_dao),
            static_document_dao: Arc::new(static_document_dao),
            mail_job_static_attachment_dao: Arc::new(mail_job_static_attachment_dao),
        }
    }
}

#[async_trait]
impl<
        C: ConfigService,
        J: MailJobDao,
        R: MailRecipientDao,
        A: MailRecipientAttachmentDao,
        S: StaticDocumentDao,
        M: MailJobStaticAttachmentDao,
    > MailService for MailServiceImpl<C, J, R, A, S, M>
{
    async fn create_job(
        &self,
        subject: &str,
        body: &str,
        recipients: Vec<RecipientInput>,
        attachment_inputs: Vec<AttachmentInput>,
        static_document_ids: Vec<Uuid>,
        template_id: Option<Uuid>,
        repayment_phase_id: Option<Uuid>,
        attach_repayment_letter: bool,
    ) -> Result<MailJob, MailServiceError> {
        if recipients.is_empty() {
            return Err(MailServiceError::DataAccess(Arc::from(
                "Recipients list cannot be empty",
            )));
        }

        if !attachment_inputs.is_empty() && recipients.len() > 1 {
            return Err(MailServiceError::DataAccess(Arc::from(
                "Attachments are only supported for single-recipient sends",
            )));
        }

        // Quick 260603-cz6: opt-in requires a phase reference — otherwise the worker
        // has no fiscal_year to filter MemberDocuments by.
        if attach_repayment_letter && repayment_phase_id.is_none() {
            return Err(MailServiceError::DataAccess(Arc::from(
                "attach_repayment_letter requires repayment_phase_id",
            )));
        }

        // Validate that every referenced static document exists and is not soft-deleted.
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
            total_count: recipients.len() as i64,
            sent_count: 0,
            failed_count: 0,
            reply_to_inbound_mail_id: None,
            // Phase 10 (Plan 10.03): real values flow in via the extended create_job signature.
            template_id,        // D-12: optional MailTemplate reference (job-wide)
            repayment_phase_id, // D-03: optional RepaymentPhase reference (job-wide)
            // Quick 260603-cz6: opt-in worker-side per-recipient RepaymentLetter attachment.
            attach_repayment_letter,
        };

        self.job_dao.create(&job).await?;

        for input in &recipients {
            let recipient = MailRecipient {
                id: Uuid::new_v4(),
                created: now_primitive,
                deleted: None,
                version: Uuid::new_v4(),
                mail_job_id: job.id,
                to_address: Arc::from(input.address.as_str()),
                member_id: input.member_id,
                status: Arc::from("pending"),
                error: None,
                sent_at: None,
                message_id: None,
                rendered_subject: None,
                rendered_body: None,
                rendered_reconstructed: false,
            };
            self.recipient_dao.create(&recipient).await?;

            for att in &attachment_inputs {
                let attachment = MailRecipientAttachment {
                    recipient_id: recipient.id,
                    document_id: att.document_id,
                    file_name: Arc::from(att.file_name.as_str()),
                    mime_type: Arc::from(att.mime_type.as_str()),
                    relative_path: Arc::from(att.relative_path.as_str()),
                };
                self.attachment_dao.create(&attachment).await?;
            }
        }

        for static_document_id in &static_document_ids {
            let join = MailJobStaticAttachment {
                mail_job_id: job.id,
                static_document_id: *static_document_id,
            };
            self.mail_job_static_attachment_dao.create(&join).await?;
        }

        Ok(job)
    }

    async fn get_jobs(&self) -> Result<Arc<[MailJob]>, MailServiceError> {
        Ok(self.job_dao.all().await?)
    }

    async fn get_job_with_recipients(
        &self,
        job_id: Uuid,
    ) -> Result<(MailJob, Arc<[MailRecipient]>), MailServiceError> {
        let job = self.job_dao.find_by_id(job_id).await?;
        let recipients = self.recipient_dao.find_by_job_id(job_id).await?;
        Ok((job, recipients))
    }

    async fn retry_job(&self, job_id: Uuid) -> Result<MailJob, MailServiceError> {
        let mut job = self.job_dao.find_by_id(job_id).await?;
        let recipients = self.recipient_dao.find_by_job_id(job_id).await?;

        let mut retry_count = 0i64;
        for r in recipients.iter() {
            if r.status.as_ref() == "failed" {
                let mut updated = r.clone();
                updated.status = Arc::from("pending");
                updated.error = None;
                updated.version = Uuid::new_v4();
                self.recipient_dao.update(&updated).await?;
                retry_count += 1;
            }
        }

        if retry_count > 0 {
            job.failed_count = 0;
            job.status = Arc::from("running");
            job.version = Uuid::new_v4();
            self.job_dao.update(&job).await?;
        }

        Ok(job)
    }

    async fn send_test_mail(&self, to: &str) -> Result<(), MailServiceError> {
        use lettre::{AsyncTransport, Message};

        let smtp_config = load_smtp_config(self.config_service.as_ref()).await?;
        let transport = build_transport(&smtp_config)?;

        let email = Message::builder()
            .from(
                smtp_config
                    .from
                    .parse()
                    .map_err(|e: lettre::address::AddressError| {
                        MailServiceError::SmtpError(Arc::from(format!("Invalid from address: {}", e)))
                    })?,
            )
            .to(to
                .parse()
                .map_err(|e: lettre::address::AddressError| {
                    MailServiceError::SmtpError(Arc::from(format!("Invalid to address: {}", e)))
                })?)
            .subject("Genossi Test-E-Mail")
            .body("Diese E-Mail bestätigt, dass die SMTP-Konfiguration korrekt ist.\n\nThis email confirms that the SMTP configuration is working correctly.".to_string())
            .map_err(|e| MailServiceError::SmtpError(Arc::from(e.to_string())))?;

        transport
            .send(email)
            .await
            .map_err(|e| MailServiceError::SmtpError(Arc::from(e.to_string())))?;

        Ok(())
    }

    async fn send_test_mail_with_body(
        &self,
        to: &str,
        subject: &str,
        body: &str,
    ) -> Result<(), MailServiceError> {
        // Quick 260603-jtf: This is the template-test sibling of `send_test_mail`.
        // The REST handler renders subject+body against a Member's template
        // variables and forwards them here; `to` is ALWAYS the explicit
        // test-recipient address from the request body (NEVER the Member's
        // email — privacy defense).
        use lettre::{AsyncTransport, Message};

        let smtp_config = load_smtp_config(self.config_service.as_ref()).await?;
        let transport = build_transport(&smtp_config)?;

        let email = Message::builder()
            .from(
                smtp_config
                    .from
                    .parse()
                    .map_err(|e: lettre::address::AddressError| {
                        MailServiceError::SmtpError(Arc::from(format!(
                            "Invalid from address: {}",
                            e
                        )))
                    })?,
            )
            .to(to.parse().map_err(|e: lettre::address::AddressError| {
                MailServiceError::SmtpError(Arc::from(format!("Invalid to address: {}", e)))
            })?)
            .subject(subject.to_string())
            .body(body.to_string())
            .map_err(|e| MailServiceError::SmtpError(Arc::from(e.to_string())))?;

        transport
            .send(email)
            .await
            .map_err(|e| MailServiceError::SmtpError(Arc::from(e.to_string())))?;

        Ok(())
    }

    async fn get_reached_member_ids(&self, job_id: Uuid) -> Result<Arc<[Uuid]>, MailServiceError> {
        // Verify job exists
        self.job_dao.find_by_id(job_id).await?;
        Ok(self
            .recipient_dao
            .find_sent_member_ids_by_job_id(job_id)
            .await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dao::{
        MockMailJobDao, MockMailJobStaticAttachmentDao, MockMailRecipientAttachmentDao,
        MockMailRecipientDao, MockStaticDocumentDao,
    };

    fn empty_static_mocks() -> (MockStaticDocumentDao, MockMailJobStaticAttachmentDao) {
        (
            MockStaticDocumentDao::new(),
            MockMailJobStaticAttachmentDao::new(),
        )
    }
    use genossi_config::dao::ConfigEntry;
    use genossi_config::service::MockConfigService;

    fn mock_smtp_config() -> Vec<ConfigEntry> {
        vec![
            ConfigEntry {
                key: Arc::from("smtp_host"),
                value: Arc::from("localhost"),
                value_type: Arc::from("string"),
            },
            ConfigEntry {
                key: Arc::from("smtp_port"),
                value: Arc::from("587"),
                value_type: Arc::from("int"),
            },
            ConfigEntry {
                key: Arc::from("smtp_user"),
                value: Arc::from("user"),
                value_type: Arc::from("string"),
            },
            ConfigEntry {
                key: Arc::from("smtp_pass"),
                value: Arc::from("pass"),
                value_type: Arc::from("secret"),
            },
            ConfigEntry {
                key: Arc::from("smtp_from"),
                value: Arc::from("sender@example.com"),
                value_type: Arc::from("string"),
            },
            ConfigEntry {
                key: Arc::from("smtp_tls"),
                value: Arc::from("none"),
                value_type: Arc::from("string"),
            },
        ]
    }

    #[tokio::test]
    async fn test_create_job() {
        let config_mock = MockConfigService::new();
        let mut job_dao = MockMailJobDao::new();
        let mut recipient_dao = MockMailRecipientDao::new();

        job_dao.expect_create().returning(|_| Ok(()));
        recipient_dao.expect_create().times(2).returning(|_| Ok(()));

        let (sd_mock, msa_mock) = empty_static_mocks();
        let service = MailServiceImpl::new(
            config_mock,
            job_dao,
            recipient_dao,
            MockMailRecipientAttachmentDao::new(),
            sd_mock,
            msa_mock,
        );
        let result = service
            .create_job(
                "Test Subject",
                "Test Body",
                vec![
                    RecipientInput {
                        address: "a@example.com".into(),
                        member_id: None,
                    },
                    RecipientInput {
                        address: "b@example.com".into(),
                        member_id: None,
                    },
                ],
                vec![],
                vec![],
                None,
                None,
                false,
            )
            .await
            .unwrap();

        assert_eq!(result.subject.as_ref(), "Test Subject");
        assert_eq!(result.status.as_ref(), "running");
        assert_eq!(result.total_count, 2);
        assert_eq!(result.sent_count, 0);
    }

    #[tokio::test]
    async fn test_create_job_empty_recipients() {
        let config_mock = MockConfigService::new();
        let job_dao = MockMailJobDao::new();
        let recipient_dao = MockMailRecipientDao::new();

        let (sd_mock, msa_mock) = empty_static_mocks();
        let service = MailServiceImpl::new(
            config_mock,
            job_dao,
            recipient_dao,
            MockMailRecipientAttachmentDao::new(),
            sd_mock,
            msa_mock,
        );
        let result = service
            .create_job("Test", "Body", vec![], vec![], vec![], None, None, false)
            .await;

        assert!(matches!(result, Err(MailServiceError::DataAccess(_))));
    }

    #[tokio::test]
    async fn test_get_jobs() {
        let config_mock = MockConfigService::new();
        let mut job_dao = MockMailJobDao::new();
        let recipient_dao = MockMailRecipientDao::new();

        job_dao.expect_all().returning(|| Ok(vec![].into()));

        let (sd_mock, msa_mock) = empty_static_mocks();
        let service = MailServiceImpl::new(
            config_mock,
            job_dao,
            recipient_dao,
            MockMailRecipientAttachmentDao::new(),
            sd_mock,
            msa_mock,
        );
        let result = service.get_jobs().await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_get_job_with_recipients() {
        let config_mock = MockConfigService::new();
        let mut job_dao = MockMailJobDao::new();
        let mut recipient_dao = MockMailRecipientDao::new();

        let job_id = Uuid::new_v4();
        let job_id_clone = job_id;

        let now = time::PrimitiveDateTime::new(
            time::Date::from_calendar_date(2026, time::Month::April, 3).unwrap(),
            time::Time::from_hms(10, 0, 0).unwrap(),
        );

        let job = MailJob {
            id: job_id,
            created: now,
            deleted: None,
            version: Uuid::new_v4(),
            subject: Arc::from("Test"),
            body: Arc::from("Body"),
            status: Arc::from("running"),
            total_count: 1,
            sent_count: 0,
            failed_count: 0,
            reply_to_inbound_mail_id: None,
            template_id: None,
            repayment_phase_id: None,
            attach_repayment_letter: false,
        };
        let job_clone = job.clone();

        job_dao
            .expect_find_by_id()
            .returning(move |_| Ok(job_clone.clone()));
        recipient_dao.expect_find_by_job_id().returning(move |id| {
            assert_eq!(id, job_id_clone);
            Ok(vec![].into())
        });

        let (sd_mock, msa_mock) = empty_static_mocks();
        let service = MailServiceImpl::new(
            config_mock,
            job_dao,
            recipient_dao,
            MockMailRecipientAttachmentDao::new(),
            sd_mock,
            msa_mock,
        );
        let (found_job, recipients) = service.get_job_with_recipients(job_id).await.unwrap();
        assert_eq!(found_job.id, job_id);
        assert!(recipients.is_empty());
    }

    #[tokio::test]
    async fn test_retry_job() {
        let config_mock = MockConfigService::new();
        let mut job_dao = MockMailJobDao::new();
        let mut recipient_dao = MockMailRecipientDao::new();

        let job_id = Uuid::new_v4();
        let now = time::PrimitiveDateTime::new(
            time::Date::from_calendar_date(2026, time::Month::April, 3).unwrap(),
            time::Time::from_hms(10, 0, 0).unwrap(),
        );

        let job = MailJob {
            id: job_id,
            created: now,
            deleted: None,
            version: Uuid::new_v4(),
            subject: Arc::from("Test"),
            body: Arc::from("Body"),
            status: Arc::from("done"),
            total_count: 2,
            sent_count: 1,
            failed_count: 1,
            reply_to_inbound_mail_id: None,
            template_id: None,
            repayment_phase_id: None,
            attach_repayment_letter: false,
        };
        let job_clone = job.clone();

        let failed_recipient = MailRecipient {
            id: Uuid::new_v4(),
            created: now,
            deleted: None,
            version: Uuid::new_v4(),
            mail_job_id: job_id,
            to_address: Arc::from("fail@example.com"),
            member_id: None,
            status: Arc::from("failed"),
            error: Some(Arc::from("Connection refused")),
            sent_at: None,
            message_id: None,
            rendered_subject: None,
            rendered_body: None,
            rendered_reconstructed: false,
        };
        let sent_recipient = MailRecipient {
            id: Uuid::new_v4(),
            created: now,
            deleted: None,
            version: Uuid::new_v4(),
            mail_job_id: job_id,
            to_address: Arc::from("ok@example.com"),
            member_id: None,
            status: Arc::from("sent"),
            error: None,
            sent_at: Some(now),
            message_id: Some(Arc::from("abc@example.com")),
            rendered_subject: None,
            rendered_body: None,
            rendered_reconstructed: false,
        };
        let recipients: Arc<[MailRecipient]> = vec![failed_recipient, sent_recipient].into();
        let recipients_clone = recipients.clone();

        job_dao
            .expect_find_by_id()
            .returning(move |_| Ok(job_clone.clone()));
        recipient_dao
            .expect_find_by_job_id()
            .returning(move |_| Ok(recipients_clone.clone()));
        recipient_dao.expect_update().times(1).returning(|r| {
            assert_eq!(r.status.as_ref(), "pending");
            assert!(r.error.is_none());
            Ok(())
        });
        job_dao.expect_update().times(1).returning(|j| {
            assert_eq!(j.status.as_ref(), "running");
            assert_eq!(j.failed_count, 0);
            Ok(())
        });

        let (sd_mock, msa_mock) = empty_static_mocks();
        let service = MailServiceImpl::new(
            config_mock,
            job_dao,
            recipient_dao,
            MockMailRecipientAttachmentDao::new(),
            sd_mock,
            msa_mock,
        );
        let result = service.retry_job(job_id).await.unwrap();
        assert_eq!(result.status.as_ref(), "running");
    }

    #[tokio::test]
    async fn test_send_test_mail_missing_config() {
        let mut config_mock = MockConfigService::new();
        config_mock.expect_get_all().returning(|| Ok(vec![].into()));

        let job_dao = MockMailJobDao::new();
        let recipient_dao = MockMailRecipientDao::new();

        let (sd_mock, msa_mock) = empty_static_mocks();
        let service = MailServiceImpl::new(
            config_mock,
            job_dao,
            recipient_dao,
            MockMailRecipientAttachmentDao::new(),
            sd_mock,
            msa_mock,
        );
        let result = service.send_test_mail("to@example.com").await;
        assert!(matches!(result, Err(MailServiceError::ConfigMissing(_))));
    }

    #[tokio::test]
    async fn test_send_test_mail_smtp_failure() {
        let mut config_mock = MockConfigService::new();
        let config = mock_smtp_config();
        config_mock
            .expect_get_all()
            .returning(move || Ok(config.clone().into()));

        let job_dao = MockMailJobDao::new();
        let recipient_dao = MockMailRecipientDao::new();

        let (sd_mock, msa_mock) = empty_static_mocks();
        let service = MailServiceImpl::new(
            config_mock,
            job_dao,
            recipient_dao,
            MockMailRecipientAttachmentDao::new(),
            sd_mock,
            msa_mock,
        );
        let result = service.send_test_mail("to@example.com").await;
        // SMTP will fail since no real server, but it should be SmtpError not ConfigMissing
        assert!(matches!(result, Err(MailServiceError::SmtpError(_))));
    }

    /// Quick 260603-jtf: `send_test_mail_with_body` without any SMTP config
    /// returns `ConfigMissing` — symmetry with `test_send_test_mail_missing_config`.
    #[tokio::test]
    async fn test_send_test_mail_with_body_missing_config() {
        let mut config_mock = MockConfigService::new();
        config_mock.expect_get_all().returning(|| Ok(vec![].into()));

        let job_dao = MockMailJobDao::new();
        let recipient_dao = MockMailRecipientDao::new();

        let (sd_mock, msa_mock) = empty_static_mocks();
        let service = MailServiceImpl::new(
            config_mock,
            job_dao,
            recipient_dao,
            MockMailRecipientAttachmentDao::new(),
            sd_mock,
            msa_mock,
        );
        let result = service
            .send_test_mail_with_body("to@example.com", "Hallo Welt", "Body-Inhalt")
            .await;
        assert!(matches!(result, Err(MailServiceError::ConfigMissing(_))));
    }

    /// Quick 260603-jtf: with SMTP config present but unreachable server,
    /// `send_test_mail_with_body` propagates an `SmtpError` (NOT
    /// `ConfigMissing`). Subject/body are passed through `Message::builder()`;
    /// the unique subject/body strings used here are a regression guard
    /// against future refactors that accidentally hard-code constants like
    /// the sibling `send_test_mail` does.
    #[tokio::test]
    async fn test_send_test_mail_with_body_smtp_failure() {
        let mut config_mock = MockConfigService::new();
        // Re-use the existing mock_smtp_config helper (port 587 on localhost,
        // matching `test_send_test_mail_smtp_failure`).
        let config = mock_smtp_config();
        config_mock
            .expect_get_all()
            .returning(move || Ok(config.clone().into()));

        let job_dao = MockMailJobDao::new();
        let recipient_dao = MockMailRecipientDao::new();

        let (sd_mock, msa_mock) = empty_static_mocks();
        let service = MailServiceImpl::new(
            config_mock,
            job_dao,
            recipient_dao,
            MockMailRecipientAttachmentDao::new(),
            sd_mock,
            msa_mock,
        );
        let result = service
            .send_test_mail_with_body(
                "to@example.com",
                "X-CUSTOM-SUBJECT",
                "X-CUSTOM-BODY with template-rendered content {{ first_name }}",
            )
            .await;
        // No real SMTP server reachable on the mocked localhost:587 — must surface
        // as SmtpError, not ConfigMissing.
        assert!(matches!(result, Err(MailServiceError::SmtpError(_))));
    }

    #[tokio::test]
    async fn test_get_reached_member_ids() {
        let config_mock = MockConfigService::new();
        let mut job_dao = MockMailJobDao::new();
        let mut recipient_dao = MockMailRecipientDao::new();

        let job_id = Uuid::new_v4();
        let now = time::PrimitiveDateTime::new(
            time::Date::from_calendar_date(2026, time::Month::April, 3).unwrap(),
            time::Time::from_hms(10, 0, 0).unwrap(),
        );

        let job = MailJob {
            id: job_id,
            created: now,
            deleted: None,
            version: Uuid::new_v4(),
            subject: Arc::from("Test"),
            body: Arc::from("Body"),
            status: Arc::from("done"),
            total_count: 3,
            sent_count: 2,
            failed_count: 1,
            reply_to_inbound_mail_id: None,
            template_id: None,
            repayment_phase_id: None,
            attach_repayment_letter: false,
        };
        let job_clone = job.clone();

        let member1 = Uuid::new_v4();
        let member2 = Uuid::new_v4();
        let sent_ids: Arc<[Uuid]> = vec![member1, member2].into();
        let sent_ids_clone = sent_ids.clone();

        job_dao
            .expect_find_by_id()
            .returning(move |_| Ok(job_clone.clone()));
        recipient_dao
            .expect_find_sent_member_ids_by_job_id()
            .returning(move |_| Ok(sent_ids_clone.clone()));

        let (sd_mock, msa_mock) = empty_static_mocks();
        let service = MailServiceImpl::new(
            config_mock,
            job_dao,
            recipient_dao,
            MockMailRecipientAttachmentDao::new(),
            sd_mock,
            msa_mock,
        );
        let result = service.get_reached_member_ids(job_id).await.unwrap();
        assert_eq!(result.len(), 2);
        assert!(result.contains(&member1));
        assert!(result.contains(&member2));
    }

    #[tokio::test]
    async fn test_get_reached_member_ids_not_found() {
        let config_mock = MockConfigService::new();
        let mut job_dao = MockMailJobDao::new();
        let recipient_dao = MockMailRecipientDao::new();

        job_dao
            .expect_find_by_id()
            .returning(|_| Err(crate::dao::MailDaoError::NotFound));

        let (sd_mock, msa_mock) = empty_static_mocks();
        let service = MailServiceImpl::new(
            config_mock,
            job_dao,
            recipient_dao,
            MockMailRecipientAttachmentDao::new(),
            sd_mock,
            msa_mock,
        );
        let result = service.get_reached_member_ids(Uuid::new_v4()).await;
        assert!(matches!(result, Err(MailServiceError::NotFound)));
    }

    #[tokio::test]
    async fn test_create_job_with_attachments_single_recipient() {
        let config_mock = MockConfigService::new();
        let mut job_dao = MockMailJobDao::new();
        let mut recipient_dao = MockMailRecipientDao::new();
        let mut attachment_dao = MockMailRecipientAttachmentDao::new();

        job_dao.expect_create().returning(|_| Ok(()));
        recipient_dao.expect_create().times(1).returning(|_| Ok(()));
        attachment_dao
            .expect_create()
            .times(2)
            .returning(|_| Ok(()));

        let (sd_mock, msa_mock) = empty_static_mocks();
        let service = MailServiceImpl::new(
            config_mock,
            job_dao,
            recipient_dao,
            attachment_dao,
            sd_mock,
            msa_mock,
        );
        let result = service
            .create_job(
                "Subject",
                "Body",
                vec![RecipientInput {
                    address: "a@example.com".into(),
                    member_id: Some(Uuid::new_v4()),
                }],
                vec![
                    AttachmentInput {
                        document_id: Uuid::new_v4(),
                        file_name: "doc1.pdf".into(),
                        mime_type: "application/pdf".into(),
                        relative_path: "aaa.pdf".into(),
                    },
                    AttachmentInput {
                        document_id: Uuid::new_v4(),
                        file_name: "doc2.pdf".into(),
                        mime_type: "application/pdf".into(),
                        relative_path: "bbb.pdf".into(),
                    },
                ],
                vec![],
                None,
                None,
                false,
            )
            .await
            .unwrap();

        assert_eq!(result.total_count, 1);
    }

    #[tokio::test]
    async fn test_create_job_attachments_rejected_for_multiple_recipients() {
        let config_mock = MockConfigService::new();
        let job_dao = MockMailJobDao::new();
        let recipient_dao = MockMailRecipientDao::new();
        let attachment_dao = MockMailRecipientAttachmentDao::new();

        let (sd_mock, msa_mock) = empty_static_mocks();
        let service = MailServiceImpl::new(
            config_mock,
            job_dao,
            recipient_dao,
            attachment_dao,
            sd_mock,
            msa_mock,
        );
        let result = service
            .create_job(
                "Subject",
                "Body",
                vec![
                    RecipientInput {
                        address: "a@example.com".into(),
                        member_id: None,
                    },
                    RecipientInput {
                        address: "b@example.com".into(),
                        member_id: None,
                    },
                ],
                vec![AttachmentInput {
                    document_id: Uuid::new_v4(),
                    file_name: "doc.pdf".into(),
                    mime_type: "application/pdf".into(),
                    relative_path: "aaa.pdf".into(),
                }],
                vec![],
                None,
                None,
                false,
            )
            .await;

        assert!(matches!(result, Err(MailServiceError::DataAccess(_))));
    }

    #[test]
    fn from_serde_json_error_maps_to_data_access() {
        let err = serde_json::from_str::<u32>("not a number").unwrap_err();
        let svc_err: MailServiceError = err.into();
        assert!(
            matches!(&svc_err, MailServiceError::DataAccess(msg) if msg.as_ref().contains("serialize failed")),
            "expected MailServiceError::DataAccess with 'serialize failed'"
        );
    }

    // Phase 10 Plan 10.03: create_job persists template_id + repayment_phase_id (D-03, D-12).
    #[tokio::test]
    async fn test_create_job_persists_template_id_and_repayment_phase_id() {
        let config_mock = MockConfigService::new();
        let mut job_dao = MockMailJobDao::new();
        let mut recipient_dao = MockMailRecipientDao::new();

        let template_id = Uuid::new_v4();
        let phase_id = Uuid::new_v4();

        // Capture the MailJob that gets persisted so we can assert on it.
        let captured_template_id = std::sync::Arc::new(std::sync::Mutex::new(None::<Option<Uuid>>));
        let captured_phase_id = std::sync::Arc::new(std::sync::Mutex::new(None::<Option<Uuid>>));
        let cap_t = captured_template_id.clone();
        let cap_p = captured_phase_id.clone();
        job_dao.expect_create().returning(move |job| {
            *cap_t.lock().unwrap() = Some(job.template_id);
            *cap_p.lock().unwrap() = Some(job.repayment_phase_id);
            Ok(())
        });
        recipient_dao.expect_create().times(1).returning(|_| Ok(()));

        let (sd_mock, msa_mock) = empty_static_mocks();
        let service = MailServiceImpl::new(
            config_mock,
            job_dao,
            recipient_dao,
            MockMailRecipientAttachmentDao::new(),
            sd_mock,
            msa_mock,
        );

        let result = service
            .create_job(
                "Subject",
                "Body",
                vec![RecipientInput {
                    address: "a@example.com".into(),
                    member_id: Some(Uuid::new_v4()),
                }],
                vec![],
                vec![],
                Some(template_id),
                Some(phase_id),
                false,
            )
            .await
            .unwrap();

        assert_eq!(result.template_id, Some(template_id));
        assert_eq!(result.repayment_phase_id, Some(phase_id));
        assert_eq!(
            *captured_template_id.lock().unwrap(),
            Some(Some(template_id)),
            "MailJob persisted via DAO must carry template_id"
        );
        assert_eq!(
            *captured_phase_id.lock().unwrap(),
            Some(Some(phase_id)),
            "MailJob persisted via DAO must carry repayment_phase_id"
        );
    }

    // Phase 10 Plan 10.03: create_job with None,None keeps fields NULL (D-03, D-12 edge case).
    #[tokio::test]
    async fn test_create_job_with_none_template_and_phase_keeps_null() {
        let config_mock = MockConfigService::new();
        let mut job_dao = MockMailJobDao::new();
        let mut recipient_dao = MockMailRecipientDao::new();

        job_dao.expect_create().returning(|_| Ok(()));
        recipient_dao.expect_create().times(1).returning(|_| Ok(()));

        let (sd_mock, msa_mock) = empty_static_mocks();
        let service = MailServiceImpl::new(
            config_mock,
            job_dao,
            recipient_dao,
            MockMailRecipientAttachmentDao::new(),
            sd_mock,
            msa_mock,
        );

        let result = service
            .create_job(
                "Subject",
                "Body",
                vec![RecipientInput {
                    address: "a@example.com".into(),
                    member_id: Some(Uuid::new_v4()),
                }],
                vec![],
                vec![],
                None,
                None,
                false,
            )
            .await
            .unwrap();

        assert_eq!(result.template_id, None);
        assert_eq!(result.repayment_phase_id, None);
    }
}
