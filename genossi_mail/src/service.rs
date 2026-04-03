use async_trait::async_trait;
use mockall::automock;
use std::sync::Arc;
use uuid::Uuid;

use crate::dao::{MailJob, MailJobDao, MailRecipient, MailRecipientDao};
use genossi_config::dao::ConfigEntry;
use genossi_config::service::ConfigService;

#[derive(Debug, Clone)]
pub enum MailServiceError {
    ConfigMissing(Arc<str>),
    SmtpError(Arc<str>),
    DataAccess(Arc<str>),
    NotFound,
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

#[automock]
#[async_trait]
pub trait MailService: Send + Sync + 'static {
    /// Create a mail job with the given recipients. Returns the created job.
    async fn create_job(
        &self,
        subject: &str,
        body: &str,
        recipients: Vec<RecipientInput>,
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

    /// Get member IDs that were successfully reached (status = "sent") for a given job.
    async fn get_reached_member_ids(
        &self,
        job_id: Uuid,
    ) -> Result<Arc<[Uuid]>, MailServiceError>;
}

pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub pass: String,
    pub from: String,
    pub tls: String,
}

pub async fn load_smtp_config<C: ConfigService>(
    config_service: &C,
) -> Result<SmtpConfig, MailServiceError> {
    let required_keys = ["smtp_host", "smtp_port", "smtp_user", "smtp_pass", "smtp_from"];
    let mut missing = Vec::new();

    let all_config = config_service.get_all().await.map_err(|e| {
        MailServiceError::DataAccess(Arc::from(format!("Failed to load config: {:?}", e)))
    })?;

    let find = |key: &str| -> Option<&ConfigEntry> {
        all_config.iter().find(|e| e.key.as_ref() == key)
    };

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

    let port: u16 = find("smtp_port")
        .unwrap()
        .value
        .parse()
        .map_err(|_| MailServiceError::ConfigMissing(Arc::from("smtp_port is not a valid port number")))?;

    let tls = find("smtp_tls")
        .map(|e| e.value.to_string())
        .unwrap_or_else(|| "starttls".to_string());

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

pub struct MailServiceImpl<C: ConfigService, J: MailJobDao, R: MailRecipientDao> {
    config_service: Arc<C>,
    job_dao: Arc<J>,
    recipient_dao: Arc<R>,
}

impl<C: ConfigService, J: MailJobDao, R: MailRecipientDao> MailServiceImpl<C, J, R> {
    pub fn new(config_service: C, job_dao: J, recipient_dao: R) -> Self {
        Self {
            config_service: Arc::new(config_service),
            job_dao: Arc::new(job_dao),
            recipient_dao: Arc::new(recipient_dao),
        }
    }
}

#[async_trait]
impl<C: ConfigService, J: MailJobDao, R: MailRecipientDao> MailService
    for MailServiceImpl<C, J, R>
{
    async fn create_job(
        &self,
        subject: &str,
        body: &str,
        recipients: Vec<RecipientInput>,
    ) -> Result<MailJob, MailServiceError> {
        if recipients.is_empty() {
            return Err(MailServiceError::DataAccess(Arc::from(
                "Recipients list cannot be empty",
            )));
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
            };
            self.recipient_dao.create(&recipient).await?;
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

    async fn get_reached_member_ids(
        &self,
        job_id: Uuid,
    ) -> Result<Arc<[Uuid]>, MailServiceError> {
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
    use crate::dao::{MockMailJobDao, MockMailRecipientDao};
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
        recipient_dao
            .expect_create()
            .times(2)
            .returning(|_| Ok(()));

        let service = MailServiceImpl::new(config_mock, job_dao, recipient_dao);
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

        let service = MailServiceImpl::new(config_mock, job_dao, recipient_dao);
        let result = service
            .create_job("Test", "Body", vec![])
            .await;

        assert!(matches!(result, Err(MailServiceError::DataAccess(_))));
    }

    #[tokio::test]
    async fn test_get_jobs() {
        let config_mock = MockConfigService::new();
        let mut job_dao = MockMailJobDao::new();
        let recipient_dao = MockMailRecipientDao::new();

        job_dao
            .expect_all()
            .returning(|| Ok(vec![].into()));

        let service = MailServiceImpl::new(config_mock, job_dao, recipient_dao);
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
        };
        let job_clone = job.clone();

        job_dao
            .expect_find_by_id()
            .returning(move |_| Ok(job_clone.clone()));
        recipient_dao
            .expect_find_by_job_id()
            .returning(move |id| {
                assert_eq!(id, job_id_clone);
                Ok(vec![].into())
            });

        let service = MailServiceImpl::new(config_mock, job_dao, recipient_dao);
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
        };
        let recipients: Arc<[MailRecipient]> =
            vec![failed_recipient, sent_recipient].into();
        let recipients_clone = recipients.clone();

        job_dao
            .expect_find_by_id()
            .returning(move |_| Ok(job_clone.clone()));
        recipient_dao
            .expect_find_by_job_id()
            .returning(move |_| Ok(recipients_clone.clone()));
        recipient_dao
            .expect_update()
            .times(1)
            .returning(|r| {
                assert_eq!(r.status.as_ref(), "pending");
                assert!(r.error.is_none());
                Ok(())
            });
        job_dao
            .expect_update()
            .times(1)
            .returning(|j| {
                assert_eq!(j.status.as_ref(), "running");
                assert_eq!(j.failed_count, 0);
                Ok(())
            });

        let service = MailServiceImpl::new(config_mock, job_dao, recipient_dao);
        let result = service.retry_job(job_id).await.unwrap();
        assert_eq!(result.status.as_ref(), "running");
    }

    #[tokio::test]
    async fn test_send_test_mail_missing_config() {
        let mut config_mock = MockConfigService::new();
        config_mock.expect_get_all().returning(|| Ok(vec![].into()));

        let job_dao = MockMailJobDao::new();
        let recipient_dao = MockMailRecipientDao::new();

        let service = MailServiceImpl::new(config_mock, job_dao, recipient_dao);
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

        let service = MailServiceImpl::new(config_mock, job_dao, recipient_dao);
        let result = service.send_test_mail("to@example.com").await;
        // SMTP will fail since no real server, but it should be SmtpError not ConfigMissing
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

        let service = MailServiceImpl::new(config_mock, job_dao, recipient_dao);
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

        let service = MailServiceImpl::new(config_mock, job_dao, recipient_dao);
        let result = service.get_reached_member_ids(Uuid::new_v4()).await;
        assert!(matches!(result, Err(MailServiceError::NotFound)));
    }
}
