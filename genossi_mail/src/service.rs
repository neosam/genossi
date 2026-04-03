use async_trait::async_trait;
use mockall::automock;
use std::sync::Arc;
use uuid::Uuid;

use crate::dao::{SentMail, SentMailDao};
use genossi_config::dao::ConfigEntry;
use genossi_config::service::ConfigService;

/// Maximum number of recipients to process per batch before yielding.
const BATCH_SIZE: usize = 10;

#[derive(Debug, Clone)]
pub enum MailServiceError {
    ConfigMissing(Arc<str>),
    SmtpError(Arc<str>),
    DataAccess(Arc<str>),
}

impl From<crate::dao::MailDaoError> for MailServiceError {
    fn from(e: crate::dao::MailDaoError) -> Self {
        match e {
            crate::dao::MailDaoError::DatabaseError(msg) => MailServiceError::DataAccess(msg),
            crate::dao::MailDaoError::NotFound => {
                MailServiceError::DataAccess(Arc::from("Not found"))
            }
        }
    }
}

#[automock]
#[async_trait]
pub trait MailService: Send + Sync + 'static {
    /// Send a single mail. Returns the stored SentMail entry.
    async fn send_mail(
        &self,
        to: &str,
        subject: &str,
        body: &str,
    ) -> Result<SentMail, MailServiceError>;

    /// Send the same mail to multiple recipients individually.
    /// Each recipient gets their own personally addressed email.
    /// Sends in batches of BATCH_SIZE to avoid overwhelming the SMTP server.
    /// Returns one SentMail entry per recipient (some may be failed).
    async fn send_mails(
        &self,
        to_addresses: &[String],
        subject: &str,
        body: &str,
    ) -> Result<Vec<SentMail>, MailServiceError>;

    async fn get_sent_mails(&self) -> Result<Arc<[SentMail]>, MailServiceError>;

    /// Send a test email with a fixed subject and body to verify SMTP configuration.
    async fn send_test_mail(&self, to: &str) -> Result<SentMail, MailServiceError>;
}

struct SmtpConfig {
    host: String,
    port: u16,
    user: String,
    pass: String,
    from: String,
    tls: String,
}

async fn load_smtp_config<C: ConfigService>(
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

fn build_transport(
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

/// Send a single email and store the result, reusing an already-built transport.
async fn send_single_mail<D: SentMailDao>(
    transport: &lettre::AsyncSmtpTransport<lettre::Tokio1Executor>,
    sent_mail_dao: &D,
    from: &str,
    to: &str,
    subject: &str,
    body: &str,
) -> Result<SentMail, MailServiceError> {
    use lettre::{AsyncTransport, Message};

    let email = Message::builder()
        .from(
            from.parse()
                .map_err(|e: lettre::address::AddressError| {
                    MailServiceError::SmtpError(Arc::from(format!("Invalid from address: {}", e)))
                })?,
        )
        .to(to
            .parse()
            .map_err(|e: lettre::address::AddressError| {
                MailServiceError::SmtpError(Arc::from(format!("Invalid to address: {}", e)))
            })?)
        .subject(subject)
        .body(body.to_string())
        .map_err(|e| MailServiceError::SmtpError(Arc::from(e.to_string())))?;

    let now = time::OffsetDateTime::now_utc();
    let now_primitive = time::PrimitiveDateTime::new(now.date(), now.time());

    let (status, error, sent_at) = match transport.send(email).await {
        Ok(_) => ("sent", None, Some(now_primitive)),
        Err(e) => {
            tracing::error!("SMTP send error to {}: {}", to, e);
            ("failed", Some(Arc::from(e.to_string())), None)
        }
    };

    let sent_mail = SentMail {
        id: Uuid::new_v4(),
        created: now_primitive,
        deleted: None,
        version: Uuid::new_v4(),
        to_address: Arc::from(to),
        subject: Arc::from(subject),
        body: Arc::from(body),
        status: Arc::from(status),
        error,
        sent_at,
    };

    sent_mail_dao.create(&sent_mail).await?;

    Ok(sent_mail)
}

pub struct MailServiceImpl<C: ConfigService, D: SentMailDao> {
    config_service: Arc<C>,
    sent_mail_dao: Arc<D>,
}

impl<C: ConfigService, D: SentMailDao> MailServiceImpl<C, D> {
    pub fn new(config_service: C, sent_mail_dao: D) -> Self {
        Self {
            config_service: Arc::new(config_service),
            sent_mail_dao: Arc::new(sent_mail_dao),
        }
    }
}

#[async_trait]
impl<C: ConfigService, D: SentMailDao> MailService for MailServiceImpl<C, D> {
    async fn send_mail(
        &self,
        to: &str,
        subject: &str,
        body: &str,
    ) -> Result<SentMail, MailServiceError> {
        let smtp_config = load_smtp_config(self.config_service.as_ref()).await?;
        let transport = build_transport(&smtp_config)?;
        send_single_mail(
            &transport,
            self.sent_mail_dao.as_ref(),
            &smtp_config.from,
            to,
            subject,
            body,
        )
        .await
    }

    async fn send_mails(
        &self,
        to_addresses: &[String],
        subject: &str,
        body: &str,
    ) -> Result<Vec<SentMail>, MailServiceError> {
        if to_addresses.is_empty() {
            return Ok(Vec::new());
        }

        let smtp_config = load_smtp_config(self.config_service.as_ref()).await?;
        let transport = build_transport(&smtp_config)?;

        let mut results = Vec::with_capacity(to_addresses.len());

        for (i, to) in to_addresses.iter().enumerate() {
            let sent_mail = send_single_mail(
                &transport,
                self.sent_mail_dao.as_ref(),
                &smtp_config.from,
                to,
                subject,
                body,
            )
            .await?;
            results.push(sent_mail);

            // Yield between batches to avoid blocking and to give SMTP server breathing room
            if (i + 1) % BATCH_SIZE == 0 && i + 1 < to_addresses.len() {
                tracing::info!(
                    "Sent batch {}/{}, pausing briefly",
                    (i + 1) / BATCH_SIZE,
                    (to_addresses.len() + BATCH_SIZE - 1) / BATCH_SIZE
                );
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        }

        Ok(results)
    }

    async fn get_sent_mails(&self) -> Result<Arc<[SentMail]>, MailServiceError> {
        Ok(self.sent_mail_dao.all().await?)
    }

    async fn send_test_mail(&self, to: &str) -> Result<SentMail, MailServiceError> {
        self.send_mail(
            to,
            "Genossi Test-E-Mail",
            "Diese E-Mail bestätigt, dass die SMTP-Konfiguration korrekt ist.\n\nThis email confirms that the SMTP configuration is working correctly.",
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dao::MockSentMailDao;
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
    async fn test_send_mail_missing_config() {
        let mut config_mock = MockConfigService::new();
        config_mock.expect_get_all().returning(|| Ok(vec![].into()));

        let sent_mail_mock = MockSentMailDao::new();

        let service = MailServiceImpl::new(config_mock, sent_mail_mock);
        let result = service.send_mail("to@example.com", "Subject", "Body").await;
        assert!(matches!(result, Err(MailServiceError::ConfigMissing(_))));
    }

    #[tokio::test]
    async fn test_send_mail_partial_config() {
        let mut config_mock = MockConfigService::new();
        config_mock.expect_get_all().returning(|| {
            Ok(vec![ConfigEntry {
                key: Arc::from("smtp_host"),
                value: Arc::from("localhost"),
                value_type: Arc::from("string"),
            }]
            .into())
        });

        let sent_mail_mock = MockSentMailDao::new();

        let service = MailServiceImpl::new(config_mock, sent_mail_mock);
        let result = service.send_mail("to@example.com", "Subject", "Body").await;
        assert!(matches!(result, Err(MailServiceError::ConfigMissing(_))));
    }

    #[tokio::test]
    async fn test_get_sent_mails_empty() {
        let config_mock = MockConfigService::new();
        let mut sent_mail_mock = MockSentMailDao::new();
        sent_mail_mock
            .expect_all()
            .returning(|| Ok(vec![].into()));

        let service = MailServiceImpl::new(config_mock, sent_mail_mock);
        let result = service.get_sent_mails().await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_send_mail_smtp_failure_stores_error() {
        let mut config_mock = MockConfigService::new();
        let config = mock_smtp_config();
        config_mock
            .expect_get_all()
            .returning(move || Ok(config.clone().into()));

        let mut sent_mail_mock = MockSentMailDao::new();
        sent_mail_mock.expect_create().returning(|mail| {
            // The mail should be stored with failed status since SMTP won't connect
            assert_eq!(mail.status.as_ref(), "failed");
            assert!(mail.error.is_some());
            assert!(mail.sent_at.is_none());
            Ok(())
        });

        let service = MailServiceImpl::new(config_mock, sent_mail_mock);
        let result = service
            .send_mail("to@example.com", "Subject", "Body")
            .await;
        // Should succeed (mail stored) even though SMTP failed
        assert!(result.is_ok());
        let sent_mail = result.unwrap();
        assert_eq!(sent_mail.status.as_ref(), "failed");
    }

    #[tokio::test]
    async fn test_send_mails_empty_list() {
        let mut config_mock = MockConfigService::new();
        // Should not even load config for empty list
        config_mock.expect_get_all().never();

        let sent_mail_mock = MockSentMailDao::new();

        let service = MailServiceImpl::new(config_mock, sent_mail_mock);
        let result = service.send_mails(&[], "Subject", "Body").await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_send_mails_missing_config() {
        let mut config_mock = MockConfigService::new();
        config_mock.expect_get_all().returning(|| Ok(vec![].into()));

        let sent_mail_mock = MockSentMailDao::new();

        let service = MailServiceImpl::new(config_mock, sent_mail_mock);
        let result = service
            .send_mails(
                &["a@example.com".into(), "b@example.com".into()],
                "Subject",
                "Body",
            )
            .await;
        assert!(matches!(result, Err(MailServiceError::ConfigMissing(_))));
    }

    #[tokio::test]
    async fn test_send_test_mail_missing_config() {
        let mut config_mock = MockConfigService::new();
        config_mock.expect_get_all().returning(|| Ok(vec![].into()));

        let sent_mail_mock = MockSentMailDao::new();

        let service = MailServiceImpl::new(config_mock, sent_mail_mock);
        let result = service.send_test_mail("to@example.com").await;
        assert!(matches!(result, Err(MailServiceError::ConfigMissing(_))));
    }

    #[tokio::test]
    async fn test_send_test_mail_uses_fixed_subject_and_body() {
        let mut config_mock = MockConfigService::new();
        let config = mock_smtp_config();
        config_mock
            .expect_get_all()
            .returning(move || Ok(config.clone().into()));

        let mut sent_mail_mock = MockSentMailDao::new();
        sent_mail_mock.expect_create().returning(|mail| {
            assert_eq!(mail.subject.as_ref(), "Genossi Test-E-Mail");
            assert!(mail.body.contains("SMTP-Konfiguration"));
            assert_eq!(mail.to_address.as_ref(), "test@example.com");
            Ok(())
        });

        let service = MailServiceImpl::new(config_mock, sent_mail_mock);
        let result = service.send_test_mail("test@example.com").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_mails_multiple_recipients_stores_each() {
        let mut config_mock = MockConfigService::new();
        let config = mock_smtp_config();
        config_mock
            .expect_get_all()
            .returning(move || Ok(config.clone().into()));

        let mut sent_mail_mock = MockSentMailDao::new();
        sent_mail_mock
            .expect_create()
            .times(3)
            .returning(|_mail| Ok(()));

        let service = MailServiceImpl::new(config_mock, sent_mail_mock);
        let result = service
            .send_mails(
                &[
                    "a@example.com".into(),
                    "b@example.com".into(),
                    "c@example.com".into(),
                ],
                "Subject",
                "Body",
            )
            .await
            .unwrap();

        assert_eq!(result.len(), 3);
        // Each result should have the correct to_address
        assert_eq!(result[0].to_address.as_ref(), "a@example.com");
        assert_eq!(result[1].to_address.as_ref(), "b@example.com");
        assert_eq!(result[2].to_address.as_ref(), "c@example.com");
        // All should be "failed" since no real SMTP server
        for mail in &result {
            assert_eq!(mail.status.as_ref(), "failed");
        }
    }
}
