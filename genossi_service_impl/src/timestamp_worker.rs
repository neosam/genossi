use genossi_config::dao::ConfigEntry;
use genossi_config::service::ConfigService;
use genossi_service::timestamp::{TimestampError, TimestampService};
use std::sync::Arc;

const DEFAULT_INTERVAL_HOURS: u64 = 168; // 7 days

fn get_interval_hours(entries: &[ConfigEntry]) -> u64 {
    entries
        .iter()
        .find(|e| e.key.as_ref() == "tsa_interval_hours")
        .and_then(|e| e.value.parse().ok())
        .unwrap_or(DEFAULT_INTERVAL_HOURS)
}

fn is_tsa_enabled(entries: &[ConfigEntry]) -> bool {
    entries
        .iter()
        .find(|e| e.key.as_ref() == "tsa_enabled")
        .map(|e| e.value.as_ref() == "true")
        .unwrap_or(false)
}

pub async fn start_timestamp_worker<T, C>(
    timestamp_service: Arc<T>,
    config_service: Arc<C>,
) where
    T: TimestampService,
    C: ConfigService,
{
    tracing::info!("Timestamp worker started");

    loop {
        let entries = match config_service.get_all().await {
            Ok(e) => e,
            Err(e) => {
                tracing::error!("Timestamp worker: failed to read config: {:?}", e);
                tokio::time::sleep(std::time::Duration::from_secs(
                    DEFAULT_INTERVAL_HOURS * 3600,
                ))
                .await;
                continue;
            }
        };

        let interval_hours = get_interval_hours(&entries);

        if is_tsa_enabled(&entries) {
            match timestamp_service.create_timestamp().await {
                Ok(entry) => {
                    tracing::info!(
                        "Timestamp worker: timestamp created, hash={}",
                        entry.audit_hash
                    );
                }
                Err(TimestampError::DuplicateHash) => {
                    tracing::info!("Timestamp worker: no changes since last timestamp, skipping");
                }
                Err(TimestampError::NothingToTimestamp) => {
                    tracing::info!("Timestamp worker: audit log is empty, skipping");
                }
                Err(e) => {
                    tracing::error!("Timestamp worker: failed to create timestamp: {}", e);
                }
            }
        } else {
            tracing::debug!("Timestamp worker: TSA not enabled, skipping");
        }

        tokio::time::sleep(std::time::Duration::from_secs(interval_hours * 3600)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(key: &str, value: &str) -> ConfigEntry {
        ConfigEntry {
            key: Arc::from(key),
            value: Arc::from(value),
            value_type: Arc::from("string"),
        }
    }

    #[test]
    fn test_get_interval_hours_default() {
        let entries = vec![];
        assert_eq!(get_interval_hours(&entries), 168);
    }

    #[test]
    fn test_get_interval_hours_custom() {
        let entries = vec![make_entry("tsa_interval_hours", "48")];
        assert_eq!(get_interval_hours(&entries), 48);
    }

    #[test]
    fn test_get_interval_hours_invalid() {
        let entries = vec![make_entry("tsa_interval_hours", "not_a_number")];
        assert_eq!(get_interval_hours(&entries), 168);
    }

    #[test]
    fn test_is_tsa_enabled_true() {
        let entries = vec![make_entry("tsa_enabled", "true")];
        assert!(is_tsa_enabled(&entries));
    }

    #[test]
    fn test_is_tsa_enabled_false() {
        let entries = vec![make_entry("tsa_enabled", "false")];
        assert!(!is_tsa_enabled(&entries));
    }

    #[test]
    fn test_is_tsa_enabled_missing() {
        let entries = vec![];
        assert!(!is_tsa_enabled(&entries));
    }
}
