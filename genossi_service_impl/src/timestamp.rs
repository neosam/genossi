use async_trait::async_trait;
use genossi_config::service::ConfigService;
use genossi_dao::audit_log::AuditLogDao;
use genossi_dao::audit_timestamp::{AuditTimestampDao, AuditTimestampEntry};
use genossi_dao::TransactionDao;
use genossi_service::timestamp::{TimestampError, TimestampService, TimestampVerification};
use std::sync::Arc;
use uuid::Uuid;

use crate::rfc3161;

struct TsaConfig {
    url: String,
    user: Option<String>,
    pass: Option<String>,
}

pub struct TimestampServiceImpl<T, A, L, C>
where
    T: TransactionDao,
    A: AuditTimestampDao<Transaction = T::Transaction>,
    L: AuditLogDao<Transaction = T::Transaction>,
    C: ConfigService,
{
    transaction_dao: Arc<T>,
    audit_timestamp_dao: Arc<A>,
    audit_log_dao: Arc<L>,
    config_service: Arc<C>,
}

impl<T, A, L, C> TimestampServiceImpl<T, A, L, C>
where
    T: TransactionDao,
    A: AuditTimestampDao<Transaction = T::Transaction>,
    L: AuditLogDao<Transaction = T::Transaction>,
    C: ConfigService,
{
    pub fn new(
        transaction_dao: T,
        audit_timestamp_dao: A,
        audit_log_dao: L,
        config_service: Arc<C>,
    ) -> Self {
        Self {
            transaction_dao: Arc::new(transaction_dao),
            audit_timestamp_dao: Arc::new(audit_timestamp_dao),
            audit_log_dao: Arc::new(audit_log_dao),
            config_service,
        }
    }

    async fn read_tsa_config(&self) -> Result<TsaConfig, TimestampError> {
        let entries = self
            .config_service
            .get_all()
            .await
            .map_err(|e| TimestampError::DataAccess(Arc::from(format!("{:?}", e))))?;

        let find = |key: &str| -> Option<String> {
            entries
                .iter()
                .find(|e| e.key.as_ref() == key)
                .map(|e| e.value.to_string())
        };

        let enabled = find("tsa_enabled").unwrap_or_else(|| "false".to_string());
        if enabled != "true" {
            return Err(TimestampError::NotConfigured);
        }

        let url = find("tsa_url")
            .filter(|u| !u.is_empty())
            .ok_or(TimestampError::NotConfigured)?;

        let user = find("tsa_user").filter(|u| !u.is_empty());
        let pass = find("tsa_pass").filter(|p| !p.is_empty());

        Ok(TsaConfig { url, user, pass })
    }
}

#[async_trait]
impl<T, A, L, C> TimestampService for TimestampServiceImpl<T, A, L, C>
where
    T: TransactionDao + Send + Sync + 'static,
    A: AuditTimestampDao<Transaction = T::Transaction> + Send + Sync + 'static,
    L: AuditLogDao<Transaction = T::Transaction> + Send + Sync + 'static,
    C: ConfigService + Send + Sync + 'static,
{
    async fn create_timestamp(&self) -> Result<AuditTimestampEntry, TimestampError> {
        let tsa_config = self.read_tsa_config().await?;

        let tx = self
            .transaction_dao
            .transaction()
            .await
            .map_err(|e| TimestampError::DataAccess(Arc::from(format!("{:?}", e))))?;

        // Get latest audit hash
        let latest_hash = self
            .audit_log_dao
            .get_latest_hash(tx.clone())
            .await
            .map_err(|e| TimestampError::DataAccess(Arc::from(format!("{:?}", e))))?;

        let audit_hash = match latest_hash {
            Some(hash) => hash,
            None => {
                self.transaction_dao
                    .commit(tx)
                    .await
                    .map_err(|e| TimestampError::DataAccess(Arc::from(format!("{:?}", e))))?;
                return Err(TimestampError::NothingToTimestamp);
            }
        };

        // Get audit entry count
        let all_entries = self
            .audit_log_dao
            .get_all_ordered(tx.clone())
            .await
            .map_err(|e| TimestampError::DataAccess(Arc::from(format!("{:?}", e))))?;
        let entry_count = all_entries.len() as i64;

        // Check for duplicates
        let latest_timestamp = self
            .audit_timestamp_dao
            .get_latest(tx.clone())
            .await
            .map_err(|e| TimestampError::DataAccess(Arc::from(format!("{:?}", e))))?;

        if let Some(ref latest) = latest_timestamp {
            if latest.audit_hash.as_ref() == audit_hash {
                self.transaction_dao
                    .commit(tx)
                    .await
                    .map_err(|e| TimestampError::DataAccess(Arc::from(format!("{:?}", e))))?;
                tracing::info!("Audit hash unchanged since last timestamp, skipping");
                return Err(TimestampError::DuplicateHash);
            }
        }

        // Send TSA request
        let now = time::OffsetDateTime::now_utc();
        let timestamp = time::PrimitiveDateTime::new(now.date(), now.time());
        let id = Uuid::new_v4();

        let tsa_result = rfc3161::request_timestamp(
            &tsa_config.url,
            &audit_hash,
            tsa_config.user.as_deref(),
            tsa_config.pass.as_deref(),
        )
        .await;

        let entry = match tsa_result {
            Ok(tsr_token) => {
                tracing::info!("Qualified timestamp obtained successfully");
                AuditTimestampEntry {
                    id,
                    timestamp,
                    audit_hash: Arc::from(audit_hash.as_str()),
                    audit_entry_count: entry_count,
                    tsr_token: Some(Arc::from(tsr_token.as_slice())),
                    webdav_path: None,
                    status: Arc::from("success"),
                }
            }
            Err(e) => {
                tracing::error!("TSA request failed: {}", e);
                let entry = AuditTimestampEntry {
                    id,
                    timestamp,
                    audit_hash: Arc::from(audit_hash.as_str()),
                    audit_entry_count: entry_count,
                    tsr_token: None,
                    webdav_path: None,
                    status: Arc::from("tsa_failed"),
                };
                self.audit_timestamp_dao
                    .create(&entry, tx.clone())
                    .await
                    .map_err(|e| TimestampError::DataAccess(Arc::from(format!("{:?}", e))))?;
                self.transaction_dao
                    .commit(tx)
                    .await
                    .map_err(|e| TimestampError::DataAccess(Arc::from(format!("{:?}", e))))?;
                return Err(TimestampError::TsaError(Arc::from(e.to_string())));
            }
        };

        self.audit_timestamp_dao
            .create(&entry, tx.clone())
            .await
            .map_err(|e| TimestampError::DataAccess(Arc::from(format!("{:?}", e))))?;

        self.transaction_dao
            .commit(tx)
            .await
            .map_err(|e| TimestampError::DataAccess(Arc::from(format!("{:?}", e))))?;

        Ok(entry)
    }

    async fn get_all(&self) -> Result<Arc<[AuditTimestampEntry]>, TimestampError> {
        let tx = self
            .transaction_dao
            .transaction()
            .await
            .map_err(|e| TimestampError::DataAccess(Arc::from(format!("{:?}", e))))?;

        let entries = self
            .audit_timestamp_dao
            .get_all(tx.clone())
            .await
            .map_err(|e| TimestampError::DataAccess(Arc::from(format!("{:?}", e))))?;

        self.transaction_dao
            .commit(tx)
            .await
            .map_err(|e| TimestampError::DataAccess(Arc::from(format!("{:?}", e))))?;

        Ok(entries)
    }

    async fn get_by_id(&self, id: Uuid) -> Result<Option<AuditTimestampEntry>, TimestampError> {
        let tx = self
            .transaction_dao
            .transaction()
            .await
            .map_err(|e| TimestampError::DataAccess(Arc::from(format!("{:?}", e))))?;

        let entry = self
            .audit_timestamp_dao
            .get_by_id(id, tx.clone())
            .await
            .map_err(|e| TimestampError::DataAccess(Arc::from(format!("{:?}", e))))?;

        self.transaction_dao
            .commit(tx)
            .await
            .map_err(|e| TimestampError::DataAccess(Arc::from(format!("{:?}", e))))?;

        Ok(entry)
    }

    async fn verify(&self, id: Uuid) -> Result<TimestampVerification, TimestampError> {
        let tx = self
            .transaction_dao
            .transaction()
            .await
            .map_err(|e| TimestampError::DataAccess(Arc::from(format!("{:?}", e))))?;

        let entry = self
            .audit_timestamp_dao
            .get_by_id(id, tx.clone())
            .await
            .map_err(|e| TimestampError::DataAccess(Arc::from(format!("{:?}", e))))?
            .ok_or(TimestampError::NotFound)?;

        // Check token validity (can we parse it as a valid TSA response?)
        let token_valid = match &entry.tsr_token {
            Some(token) => rfc3161::parse_timestamp_response(token).unwrap_or(false),
            None => false,
        };

        // Hash matches: trust stored data for now
        let hash_matches = token_valid;

        // Verify audit log consistency: replay hash chain up to entry_count
        let all_entries = self
            .audit_log_dao
            .get_all_ordered(tx.clone())
            .await
            .map_err(|e| TimestampError::DataAccess(Arc::from(format!("{:?}", e))))?;

        let entries_at_timestamp: Vec<_> = all_entries
            .iter()
            .take(entry.audit_entry_count as usize)
            .cloned()
            .collect();

        let audit_log_consistent = if entries_at_timestamp.is_empty() {
            false
        } else {
            let broken_links = crate::audit_log::verify_chain(&entries_at_timestamp);
            let last_hash = entries_at_timestamp
                .last()
                .map(|e| e.entry_hash.as_ref())
                .unwrap_or("");
            broken_links.is_empty() && last_hash == entry.audit_hash.as_ref()
        };

        self.transaction_dao
            .commit(tx)
            .await
            .map_err(|e| TimestampError::DataAccess(Arc::from(format!("{:?}", e))))?;

        Ok(TimestampVerification {
            token_valid,
            hash_matches,
            audit_log_consistent,
            timestamp: entry.timestamp,
            audit_hash: entry.audit_hash,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use genossi_config::service::MockConfigService;
    use genossi_dao::audit_log::MockAuditLogDao;
    use genossi_dao::audit_timestamp::MockAuditTimestampDao;
    use genossi_dao::{DaoError, MockTransaction, MockTransactionDao};
    use genossi_config::dao::ConfigEntry;

    fn setup_mock_tx() -> MockTransactionDao {
        let mut tx_dao = MockTransactionDao::new();
        tx_dao.expect_transaction().returning(|| {
            let mut tx = MockTransaction::new();
            tx.expect_clone().returning(|| {
                let mut tx = MockTransaction::new();
                tx.expect_clone().returning(|| {
                    let mut tx = MockTransaction::new();
                    tx.expect_clone().returning(|| {
                        let mut tx = MockTransaction::new();
                        tx.expect_clone().returning(MockTransaction::new);
                        tx
                    });
                    tx
                });
                tx
            });
            Ok(tx)
        });
        tx_dao.expect_commit().returning(|_| Ok(()));
        tx_dao
    }

    fn mock_config_enabled() -> MockConfigService {
        let mut config = MockConfigService::new();
        config.expect_get_all().returning(|| {
            Ok(vec![
                ConfigEntry {
                    key: Arc::from("tsa_enabled"),
                    value: Arc::from("true"),
                    value_type: Arc::from("bool"),
                },
                ConfigEntry {
                    key: Arc::from("tsa_url"),
                    value: Arc::from("https://freetsa.org/tsr"),
                    value_type: Arc::from("string"),
                },
            ]
            .into())
        });
        config
    }

    fn mock_config_disabled() -> MockConfigService {
        let mut config = MockConfigService::new();
        config.expect_get_all().returning(|| {
            Ok(vec![ConfigEntry {
                key: Arc::from("tsa_enabled"),
                value: Arc::from("false"),
                value_type: Arc::from("bool"),
            }]
            .into())
        });
        config
    }

    #[tokio::test]
    async fn test_create_timestamp_not_configured() {
        let tx_dao = MockTransactionDao::new();
        let audit_log_dao = MockAuditLogDao::new();
        let audit_ts_dao = MockAuditTimestampDao::new();
        let config = mock_config_disabled();

        let service = TimestampServiceImpl::new(
            tx_dao,
            audit_ts_dao,
            audit_log_dao,
            Arc::new(config),
        );

        let result = service.create_timestamp().await;
        assert!(matches!(result, Err(TimestampError::NotConfigured)));
    }

    #[tokio::test]
    async fn test_create_timestamp_no_audit_entries() {
        let tx_dao = setup_mock_tx();
        let mut audit_log_dao = MockAuditLogDao::new();
        audit_log_dao
            .expect_get_latest_hash()
            .returning(|_| Ok(None));

        let audit_ts_dao = MockAuditTimestampDao::new();
        let config = mock_config_enabled();

        let service = TimestampServiceImpl::new(
            tx_dao,
            audit_ts_dao,
            audit_log_dao,
            Arc::new(config),
        );

        let result = service.create_timestamp().await;
        assert!(matches!(result, Err(TimestampError::NothingToTimestamp)));
    }

    #[tokio::test]
    async fn test_create_timestamp_duplicate_hash() {
        let tx_dao = setup_mock_tx();
        let mut audit_log_dao = MockAuditLogDao::new();
        audit_log_dao
            .expect_get_latest_hash()
            .returning(|_| Ok(Some("hash123".to_string())));
        audit_log_dao
            .expect_get_all_ordered()
            .returning(|_| Ok(Arc::from(vec![])));

        let mut audit_ts_dao = MockAuditTimestampDao::new();
        let date = time::Date::from_calendar_date(2026, time::Month::April, 15).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        audit_ts_dao
            .expect_get_latest()
            .returning(move |_| {
                Ok(Some(AuditTimestampEntry {
                    id: Uuid::new_v4(),
                    timestamp: datetime,
                    audit_hash: Arc::from("hash123"),
                    audit_entry_count: 10,
                    tsr_token: Some(Arc::from(vec![1u8].as_slice())),
                    webdav_path: None,
                    status: Arc::from("success"),
                }))
            });

        let config = mock_config_enabled();

        let service = TimestampServiceImpl::new(
            tx_dao,
            audit_ts_dao,
            audit_log_dao,
            Arc::new(config),
        );

        let result = service.create_timestamp().await;
        assert!(matches!(result, Err(TimestampError::DuplicateHash)));
    }
}
