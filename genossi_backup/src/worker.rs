use std::sync::Arc;

use genossi_config::dao::ConfigEntry;
use genossi_config::service::ConfigService;
use genossi_dao::backup::{BackupDao, BackupDocumentSyncDao};
use genossi_service::document_storage::DocumentStorage;

use crate::generator;
use crate::sync;
use crate::webdav::WebDavClient;

const DEFAULT_INTERVAL_HOURS: u64 = 24;

struct BackupConfig {
    url: String,
    username: String,
    password: String,
    directory: String,
    interval_hours: u64,
}

fn parse_config(entries: &[ConfigEntry]) -> Option<BackupConfig> {
    let find = |key: &str| -> Option<&str> {
        entries
            .iter()
            .find(|e| e.key.as_ref() == key)
            .map(|e| e.value.as_ref())
    };

    let enabled = find("backup_webdav_enabled").unwrap_or("false");
    if enabled != "true" {
        return None;
    }

    let url = find("backup_webdav_url")?.to_string();
    let username = find("backup_webdav_username")?.to_string();
    let password = find("backup_webdav_password")?.to_string();
    let directory = find("backup_webdav_directory")
        .unwrap_or("genossi-export")
        .to_string();
    let interval_hours = find("backup_interval_hours")
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_INTERVAL_HOURS);

    if url.is_empty() || username.is_empty() || password.is_empty() {
        return None;
    }

    Some(BackupConfig {
        url,
        username,
        password,
        directory,
        interval_hours,
    })
}

fn get_interval_hours(entries: &[ConfigEntry]) -> u64 {
    entries
        .iter()
        .find(|e| e.key.as_ref() == "backup_interval_hours")
        .and_then(|e| e.value.parse().ok())
        .unwrap_or(DEFAULT_INTERVAL_HOURS)
}

async fn update_status<C: ConfigService>(config_service: &C, status: &str) {
    let now = time::OffsetDateTime::now_utc();
    let format = time::format_description::well_known::Iso8601::DEFAULT;
    let timestamp = now.format(&format).unwrap_or_default();

    let _ = config_service
        .set(&ConfigEntry {
            key: Arc::from("backup_last_run"),
            value: Arc::from(timestamp.as_str()),
            value_type: Arc::from("string"),
        })
        .await;
    let _ = config_service
        .set(&ConfigEntry {
            key: Arc::from("backup_last_status"),
            value: Arc::from(status),
            value_type: Arc::from("string"),
        })
        .await;
}

async fn run_backup_cycle<C, B, S, D>(
    config: &BackupConfig,
    backup_dao: &B,
    sync_dao: &S,
    document_storage: &D,
    config_service: &C,
) -> Result<String, String>
where
    C: ConfigService,
    B: BackupDao,
    S: BackupDocumentSyncDao,
    D: DocumentStorage,
{
    let webdav = WebDavClient::new(&config.url, &config.username, &config.password);
    let base_dir = &config.directory;

    tracing::info!("Backup cycle starting, target: {}/{}", config.url, base_dir);

    webdav
        .mkcol_recursive(base_dir)
        .await
        .map_err(|e| format!("Failed to create base directory: {}", e))?;

    // 1. Determine year range
    let earliest_year = backup_dao
        .earliest_join_year()
        .await
        .map_err(|e| format!("Failed to get earliest join year: {:?}", e))?;

    let today = time::OffsetDateTime::now_utc().date();
    let current_year = today.year();
    let mut csv_count = 0;

    // 2. Upload yearly member CSVs
    if let Some(start_year) = earliest_year {
        let date_format =
            time::format_description::parse("[year]-[month]-[day]").map_err(|e| e.to_string())?;

        for year in start_year..current_year {
            let date = time::Date::from_calendar_date(year, time::Month::December, 31)
                .map_err(|e| format!("Invalid date for year {}: {}", year, e))?;

            let members = backup_dao
                .members_at_date(date)
                .await
                .map_err(|e| format!("Failed to fetch members for {}: {:?}", year, e))?;

            let csv_data = generator::generate_members_csv(&members)?;
            let path = format!("{}/mitgliederliste-{}.csv", base_dir, year);
            webdav
                .put(&path, csv_data)
                .await
                .map_err(|e| format!("Failed to upload {}: {}", path, e))?;

            csv_count += 1;
            tracing::info!("Uploaded mitgliederliste-{}.csv ({} members)", year, members.len());
        }

        // 3. Upload current member CSV
        let members = backup_dao
            .members_at_date(today)
            .await
            .map_err(|e| format!("Failed to fetch current members: {:?}", e))?;

        let csv_data = generator::generate_members_csv(&members)?;
        let path = format!("{}/mitgliederliste-aktuell.csv", base_dir);
        webdav
            .put(&path, csv_data)
            .await
            .map_err(|e| format!("Failed to upload current member list: {}", e))?;

        csv_count += 1;
        let today_str = today.format(&date_format).unwrap_or_default();
        tracing::info!(
            "Uploaded mitgliederliste-aktuell.csv (date: {}, {} members)",
            today_str,
            members.len()
        );
    }

    // 4. Upload actions CSV
    let actions = backup_dao
        .all_actions()
        .await
        .map_err(|e| format!("Failed to fetch actions: {:?}", e))?;

    let actions_csv = generator::generate_actions_csv(&actions)?;
    let actions_path = format!("{}/aktionen.csv", base_dir);
    webdav
        .put(&actions_path, actions_csv)
        .await
        .map_err(|e| format!("Failed to upload actions: {}", e))?;
    csv_count += 1;
    tracing::info!("Uploaded aktionen.csv ({} actions)", actions.len());

    // 5. Sync documents
    let documents = backup_dao
        .all_documents()
        .await
        .map_err(|e| format!("Failed to fetch documents: {:?}", e))?;

    let sync_stats =
        sync::sync_documents(&webdav, sync_dao, document_storage, &documents, base_dir)
            .await
            .map_err(|e| format!("Document sync failed: {}", e))?;

    tracing::info!(
        "Document sync complete: {} total, {} uploaded, {} skipped, {} failed",
        sync_stats.total,
        sync_stats.uploaded,
        sync_stats.skipped,
        sync_stats.failed
    );

    let status = format!(
        "Erfolgreich: {} CSVs, {} Dokumente synchronisiert ({} übersprungen, {} fehlgeschlagen)",
        csv_count, sync_stats.uploaded, sync_stats.skipped, sync_stats.failed
    );

    Ok(status)
}

pub async fn start_backup_worker<C, B, S, D>(
    config_service: Arc<C>,
    backup_dao: Arc<B>,
    sync_dao: Arc<S>,
    document_storage: Arc<D>,
) where
    C: ConfigService,
    B: BackupDao,
    S: BackupDocumentSyncDao,
    D: DocumentStorage,
{
    tracing::info!("Backup worker started");

    loop {
        let entries = match config_service.get_all().await {
            Ok(e) => e,
            Err(e) => {
                tracing::error!("Backup worker: failed to read config: {:?}", e);
                tokio::time::sleep(std::time::Duration::from_secs(
                    DEFAULT_INTERVAL_HOURS * 3600,
                ))
                .await;
                continue;
            }
        };

        let interval_hours = get_interval_hours(&entries);

        match parse_config(&entries) {
            Some(config) => {
                match run_backup_cycle(
                    &config,
                    backup_dao.as_ref(),
                    sync_dao.as_ref(),
                    document_storage.as_ref(),
                    config_service.as_ref(),
                )
                .await
                {
                    Ok(status) => {
                        tracing::info!("Backup cycle complete: {}", status);
                        update_status(config_service.as_ref(), &status).await;
                    }
                    Err(e) => {
                        tracing::error!("Backup cycle failed: {}", e);
                        update_status(config_service.as_ref(), &format!("Fehler: {}", e)).await;
                    }
                }
            }
            None => {
                tracing::debug!("Backup not configured or disabled, skipping cycle");
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(interval_hours * 3600)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(key: &str, value: &str, value_type: &str) -> ConfigEntry {
        ConfigEntry {
            key: Arc::from(key),
            value: Arc::from(value),
            value_type: Arc::from(value_type),
        }
    }

    #[test]
    fn test_parse_config_enabled() {
        let entries = vec![
            make_entry("backup_webdav_enabled", "true", "bool"),
            make_entry("backup_webdav_url", "https://cloud.example/dav/", "string"),
            make_entry("backup_webdav_username", "user", "string"),
            make_entry("backup_webdav_password", "pass", "secret"),
            make_entry("backup_webdav_directory", "my-backup", "string"),
            make_entry("backup_interval_hours", "12", "int"),
        ];
        let config = parse_config(&entries).unwrap();
        assert_eq!(config.url, "https://cloud.example/dav/");
        assert_eq!(config.username, "user");
        assert_eq!(config.password, "pass");
        assert_eq!(config.directory, "my-backup");
        assert_eq!(config.interval_hours, 12);
    }

    #[test]
    fn test_parse_config_disabled() {
        let entries = vec![
            make_entry("backup_webdav_enabled", "false", "bool"),
            make_entry("backup_webdav_url", "https://cloud.example/dav/", "string"),
            make_entry("backup_webdav_username", "user", "string"),
            make_entry("backup_webdav_password", "pass", "secret"),
        ];
        assert!(parse_config(&entries).is_none());
    }

    #[test]
    fn test_parse_config_missing_enabled() {
        let entries = vec![
            make_entry("backup_webdav_url", "https://cloud.example/dav/", "string"),
            make_entry("backup_webdav_username", "user", "string"),
            make_entry("backup_webdav_password", "pass", "secret"),
        ];
        assert!(parse_config(&entries).is_none());
    }

    #[test]
    fn test_parse_config_missing_url() {
        let entries = vec![
            make_entry("backup_webdav_enabled", "true", "bool"),
            make_entry("backup_webdav_username", "user", "string"),
            make_entry("backup_webdav_password", "pass", "secret"),
        ];
        assert!(parse_config(&entries).is_none());
    }

    #[test]
    fn test_parse_config_defaults() {
        let entries = vec![
            make_entry("backup_webdav_enabled", "true", "bool"),
            make_entry("backup_webdav_url", "https://cloud.example/dav/", "string"),
            make_entry("backup_webdav_username", "user", "string"),
            make_entry("backup_webdav_password", "pass", "secret"),
        ];
        let config = parse_config(&entries).unwrap();
        assert_eq!(config.directory, "genossi-export");
        assert_eq!(config.interval_hours, 24);
    }

    #[test]
    fn test_get_interval_hours_default() {
        let entries = vec![];
        assert_eq!(get_interval_hours(&entries), 24);
    }

    #[test]
    fn test_get_interval_hours_custom() {
        let entries = vec![make_entry("backup_interval_hours", "6", "int")];
        assert_eq!(get_interval_hours(&entries), 6);
    }

    #[test]
    fn test_get_interval_hours_invalid() {
        let entries = vec![make_entry("backup_interval_hours", "not_a_number", "int")];
        assert_eq!(get_interval_hours(&entries), 24);
    }
}
