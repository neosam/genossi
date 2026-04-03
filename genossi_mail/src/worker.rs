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

async fn update_job_with_retry<J: MailJobDao>(job_dao: &J, job: &crate::dao::MailJob) -> bool {
    for attempt in 1..=3 {
        match job_dao.update(job).await {
            Ok(()) => return true,
            Err(e) => {
                tracing::error!(
                    "Worker: failed to update job {} (attempt {}/3): {:?}",
                    job.id,
                    attempt,
                    e
                );
                if attempt < 3 {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            }
        }
    }
    tracing::error!(
        "Worker: giving up on job update for {} after 3 attempts",
        job.id
    );
    false
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

        update_job_with_retry(job_dao.as_ref(), &job).await;

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
    use crate::dao::{MailDaoError, MailJob, MockMailJobDao};
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

    fn sample_job() -> MailJob {
        MailJob {
            id: uuid::Uuid::new_v4(),
            created: sample_datetime(),
            deleted: None,
            version: uuid::Uuid::new_v4(),
            subject: Arc::from("Test"),
            body: Arc::from("Body"),
            status: Arc::from("running"),
            total_count: 1,
            sent_count: 0,
            failed_count: 1,
        }
    }

    #[tokio::test]
    async fn test_update_job_with_retry_succeeds_on_second_attempt() {
        let mut job_dao = MockMailJobDao::new();
        let call_count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        job_dao.expect_update().times(2).returning(move |_| {
            let count = call_count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if count == 0 {
                Err(MailDaoError::DatabaseError(Arc::from("transient error")))
            } else {
                Ok(())
            }
        });

        let job = sample_job();
        let result = update_job_with_retry(&job_dao, &job).await;
        assert!(result);
        assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_update_job_with_retry_fails_after_3_attempts() {
        let mut job_dao = MockMailJobDao::new();
        let call_count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        job_dao.expect_update().times(3).returning(move |_| {
            call_count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Err(MailDaoError::DatabaseError(Arc::from("persistent error")))
        });

        let job = sample_job();
        let result = update_job_with_retry(&job_dao, &job).await;
        assert!(!result);
        assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 3);
    }
}
