use std::sync::Arc;

use crate::dao::{MailJobDao, MailRecipientDao};
use crate::service::{build_transport, load_smtp_config, MailServiceError};
use genossi_config::service::ConfigService;

const DEFAULT_SEND_INTERVAL_SECONDS: u64 = 36;
const IDLE_POLL_SECONDS: u64 = 5;

async fn get_send_interval<C: ConfigService>(config_service: &C) -> u64 {
    let all_config = match config_service.get_all().await {
        Ok(c) => c,
        Err(_) => return DEFAULT_SEND_INTERVAL_SECONDS,
    };
    all_config
        .iter()
        .find(|e| e.key.as_ref() == "mail_send_interval_seconds")
        .and_then(|e| e.value.parse::<u64>().ok())
        .unwrap_or(DEFAULT_SEND_INTERVAL_SECONDS)
}

pub async fn start_mail_worker<C, J, R>(
    config_service: Arc<C>,
    job_dao: Arc<J>,
    recipient_dao: Arc<R>,
) where
    C: ConfigService,
    J: MailJobDao,
    R: MailRecipientDao,
{
    loop {
        let next = match recipient_dao.next_pending().await {
            Ok(Some(recipient)) => recipient,
            Ok(None) => {
                tokio::time::sleep(std::time::Duration::from_secs(IDLE_POLL_SECONDS)).await;
                continue;
            }
            Err(e) => {
                tracing::error!("Worker: failed to query next pending recipient: {:?}", e);
                tokio::time::sleep(std::time::Duration::from_secs(IDLE_POLL_SECONDS)).await;
                continue;
            }
        };

        // Load job for this recipient
        let mut job = match job_dao.find_by_id(next.mail_job_id).await {
            Ok(j) => j,
            Err(e) => {
                tracing::error!("Worker: failed to find job {}: {:?}", next.mail_job_id, e);
                tokio::time::sleep(std::time::Duration::from_secs(IDLE_POLL_SECONDS)).await;
                continue;
            }
        };

        // Load SMTP config and send
        let send_result = send_mail_for_recipient(
            config_service.as_ref(),
            &next.to_address,
            &job.subject,
            &job.body,
        )
        .await;

        let now = time::OffsetDateTime::now_utc();
        let now_primitive = time::PrimitiveDateTime::new(now.date(), now.time());

        let mut updated_recipient = next.clone();
        updated_recipient.version = uuid::Uuid::new_v4();

        match send_result {
            Ok(()) => {
                updated_recipient.status = Arc::from("sent");
                updated_recipient.sent_at = Some(now_primitive);
                job.sent_count += 1;
                tracing::info!(
                    "Worker: sent mail to {} (job {})",
                    next.to_address,
                    job.id
                );
            }
            Err(e) => {
                let error_msg = format!("{:?}", e);
                updated_recipient.status = Arc::from("failed");
                updated_recipient.error = Some(Arc::from(error_msg.as_str()));
                job.failed_count += 1;
                tracing::error!(
                    "Worker: failed to send mail to {} (job {}): {}",
                    next.to_address,
                    job.id,
                    error_msg
                );
            }
        }

        // Update recipient
        if let Err(e) = recipient_dao.update(&updated_recipient).await {
            tracing::error!("Worker: failed to update recipient {}: {:?}", next.id, e);
        }

        // Check job completion
        if job.sent_count + job.failed_count >= job.total_count {
            if job.failed_count >= job.total_count {
                job.status = Arc::from("failed");
            } else {
                job.status = Arc::from("done");
            }
        }
        job.version = uuid::Uuid::new_v4();

        if let Err(e) = job_dao.update(&job).await {
            tracing::error!("Worker: failed to update job {}: {:?}", job.id, e);
        }

        // Wait configured interval
        let interval = get_send_interval(config_service.as_ref()).await;
        tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
    }
}

async fn send_mail_for_recipient<C: ConfigService>(
    config_service: &C,
    to: &str,
    subject: &str,
    body: &str,
) -> Result<(), MailServiceError> {
    use lettre::{AsyncTransport, Message};

    let smtp_config = load_smtp_config(config_service).await?;
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
        .to(to.parse().map_err(|e: lettre::address::AddressError| {
            MailServiceError::SmtpError(Arc::from(format!("Invalid to address: {}", e)))
        })?)
        .subject(subject)
        .body(body.to_string())
        .map_err(|e| MailServiceError::SmtpError(Arc::from(e.to_string())))?;

    transport
        .send(email)
        .await
        .map_err(|e| MailServiceError::SmtpError(Arc::from(e.to_string())))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dao::{MailDaoError, MailJob, MailRecipient, MockMailJobDao, MockMailRecipientDao};
    use genossi_config::dao::ConfigEntry;
    use genossi_config::service::MockConfigService;

    fn sample_datetime() -> time::PrimitiveDateTime {
        time::PrimitiveDateTime::new(
            time::Date::from_calendar_date(2026, time::Month::April, 3).unwrap(),
            time::Time::from_hms(10, 0, 0).unwrap(),
        )
    }

    #[tokio::test]
    async fn test_get_send_interval_default() {
        let mut config_mock = MockConfigService::new();
        config_mock
            .expect_get_all()
            .returning(|| Ok(vec![].into()));

        let interval = get_send_interval(&config_mock).await;
        assert_eq!(interval, DEFAULT_SEND_INTERVAL_SECONDS);
    }

    #[tokio::test]
    async fn test_get_send_interval_custom() {
        let mut config_mock = MockConfigService::new();
        config_mock.expect_get_all().returning(|| {
            Ok(vec![ConfigEntry {
                key: Arc::from("mail_send_interval_seconds"),
                value: Arc::from("60"),
                value_type: Arc::from("int"),
            }]
            .into())
        });

        let interval = get_send_interval(&config_mock).await;
        assert_eq!(interval, 60);
    }

    #[tokio::test]
    async fn test_get_send_interval_invalid_value() {
        let mut config_mock = MockConfigService::new();
        config_mock.expect_get_all().returning(|| {
            Ok(vec![ConfigEntry {
                key: Arc::from("mail_send_interval_seconds"),
                value: Arc::from("not_a_number"),
                value_type: Arc::from("int"),
            }]
            .into())
        });

        let interval = get_send_interval(&config_mock).await;
        assert_eq!(interval, DEFAULT_SEND_INTERVAL_SECONDS);
    }

    #[tokio::test]
    async fn test_get_send_interval_config_error() {
        let mut config_mock = MockConfigService::new();
        config_mock.expect_get_all().returning(|| {
            Err(genossi_config::service::ConfigServiceError::DataAccess(
                Arc::from("db error"),
            ))
        });

        let interval = get_send_interval(&config_mock).await;
        assert_eq!(interval, DEFAULT_SEND_INTERVAL_SECONDS);
    }
}
