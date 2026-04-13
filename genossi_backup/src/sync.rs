use genossi_dao::backup::{
    BackupCommunicationSyncDao, BackupDocumentSyncDao, CommunicationBackupRow,
    DocumentBackupRow,
};
use genossi_service::document_storage::DocumentStorage;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;

use crate::generator;
use crate::webdav::{WebDavClient, WebDavError};

#[derive(Debug)]
pub struct SyncStats {
    pub total: usize,
    pub uploaded: usize,
    pub skipped: usize,
    pub failed: usize,
}

pub async fn sync_documents<S: BackupDocumentSyncDao, D: DocumentStorage>(
    webdav: &WebDavClient,
    sync_dao: &S,
    document_storage: &D,
    documents: &[DocumentBackupRow],
    base_dir: &str,
) -> Result<SyncStats, WebDavError> {
    let docs_dir = format!("{}/dokumente", base_dir);
    webdav.mkcol_recursive(&docs_dir).await?;

    let mut stats = SyncStats {
        total: documents.len(),
        uploaded: 0,
        skipped: 0,
        failed: 0,
    };

    for doc in documents {
        match sync_single_document(webdav, sync_dao, document_storage, doc, &docs_dir).await {
            Ok(true) => stats.uploaded += 1,
            Ok(false) => stats.skipped += 1,
            Err(e) => {
                tracing::warn!(
                    "Failed to sync document {}: {}",
                    doc.relative_path,
                    e
                );
                stats.failed += 1;
            }
        }
    }

    Ok(stats)
}

async fn sync_single_document<S: BackupDocumentSyncDao, D: DocumentStorage>(
    webdav: &WebDavClient,
    sync_dao: &S,
    document_storage: &D,
    doc: &DocumentBackupRow,
    docs_dir: &str,
) -> Result<bool, String> {
    let data = document_storage
        .load(&doc.relative_path)
        .await
        .map_err(|e| format!("Failed to load document: {}", e))?;

    let hash = compute_sha256(&data);

    let stored_hash = sync_dao
        .get_hash(&doc.relative_path)
        .await
        .map_err(|e| format!("Failed to get hash: {:?}", e))?;

    if let Some(stored) = stored_hash {
        if stored.as_ref() == hash.as_str() {
            return Ok(false);
        }
    }

    let member_dir = format!(
        "{}/{:03}_{}_{}",
        docs_dir, doc.member_number, doc.last_name, doc.first_name
    );
    webdav
        .mkcol(&member_dir)
        .await
        .map_err(|e| format!("Failed to create directory: {}", e))?;

    let file_path = format!(
        "{}/{}_{}", member_dir, doc.document_type, doc.file_name
    );
    webdav
        .put(&file_path, data)
        .await
        .map_err(|e| format!("Failed to upload: {}", e))?;

    let now = time::OffsetDateTime::now_utc();
    let format = time::format_description::well_known::Iso8601::DEFAULT;
    let timestamp = now.format(&format).unwrap_or_default();

    sync_dao
        .upsert_hash(&doc.relative_path, &hash, &timestamp)
        .await
        .map_err(|e| format!("Failed to update hash: {:?}", e))?;

    Ok(true)
}

pub async fn sync_communications<CS: BackupCommunicationSyncDao>(
    webdav: &WebDavClient,
    comm_sync_dao: &CS,
    communications: &[CommunicationBackupRow],
    base_dir: &str,
) -> Result<SyncStats, WebDavError> {
    let mut stats = SyncStats {
        total: communications.len(),
        uploaded: 0,
        skipped: 0,
        failed: 0,
    };

    // Track filenames for collision detection
    let mut filename_counts: HashMap<String, u32> = HashMap::new();

    for comm in communications {
        // Check if already synced
        let is_synced = comm_sync_dao
            .is_synced(&comm.mail_type, comm.mail_id)
            .await
            .map_err(|e| WebDavError::RequestFailed(Arc::from(format!("DB error: {:?}", e))))?;

        if is_synced {
            stats.skipped += 1;
            continue;
        }

        let member_dir = format!(
            "{}/kommunikation/{:03}_{}_{}",
            base_dir, comm.member_number, comm.last_name, comm.first_name
        );

        let base_filename =
            generator::generate_communication_filename(&comm.date, &comm.direction, &comm.subject, None);

        let count = filename_counts
            .entry(format!("{}/{}", member_dir, base_filename))
            .or_insert(0);
        *count += 1;

        let filename = if *count > 1 {
            let suffix = &comm.mail_id.to_string()[..8];
            generator::generate_communication_filename(&comm.date, &comm.direction, &comm.subject, Some(suffix))
        } else {
            base_filename
        };

        let txt_content = generator::generate_communication_txt(comm);

        // Create member kommunikation directory
        match webdav.mkcol(&member_dir).await {
            Ok(_) => {}
            Err(e) => {
                tracing::warn!("Failed to create dir {}: {}", member_dir, e);
                stats.failed += 1;
                continue;
            }
        }

        let file_path = format!("{}/{}.txt", member_dir, filename);
        match webdav.put(&file_path, txt_content.into_bytes()).await {
            Ok(_) => {
                if let Err(e) = comm_sync_dao
                    .mark_synced(&comm.mail_type, comm.mail_id)
                    .await
                {
                    tracing::warn!("Failed to mark synced {}: {:?}", comm.mail_id, e);
                }
                stats.uploaded += 1;
            }
            Err(e) => {
                tracing::warn!("Failed to upload {}: {}", file_path, e);
                stats.failed += 1;
            }
        }
    }

    Ok(stats)
}

fn compute_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use genossi_dao::backup::BackupDocumentSyncDao;
    use genossi_dao::DaoError;
    use genossi_service::document_storage::{DocumentStorage, StorageError};

    // Manual mock for BackupDocumentSyncDao
    struct MockSyncDao {
        hash: std::sync::Mutex<Option<Arc<str>>>,
        upserted: std::sync::Mutex<Vec<(String, String)>>,
    }

    impl MockSyncDao {
        fn new(hash: Option<&str>) -> Self {
            Self {
                hash: std::sync::Mutex::new(hash.map(|h| Arc::from(h))),
                upserted: std::sync::Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait::async_trait]
    impl BackupDocumentSyncDao for MockSyncDao {
        async fn get_hash(&self, _relative_path: &str) -> Result<Option<Arc<str>>, DaoError> {
            Ok(self.hash.lock().unwrap().clone())
        }
        async fn upsert_hash(
            &self,
            relative_path: &str,
            content_hash: &str,
            _last_uploaded: &str,
        ) -> Result<(), DaoError> {
            self.upserted
                .lock()
                .unwrap()
                .push((relative_path.to_string(), content_hash.to_string()));
            Ok(())
        }
    }

    struct MockStorage {
        data: Vec<u8>,
    }

    #[async_trait::async_trait]
    impl DocumentStorage for MockStorage {
        async fn save(&self, _path: &str, _data: &[u8]) -> Result<(), StorageError> {
            Ok(())
        }
        async fn load(&self, _path: &str) -> Result<Vec<u8>, StorageError> {
            Ok(self.data.clone())
        }
        async fn delete(&self, _path: &str) -> Result<(), StorageError> {
            Ok(())
        }
    }

    #[test]
    fn test_compute_sha256() {
        let hash = compute_sha256(b"hello world");
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[tokio::test]
    async fn test_sync_new_document() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("MKCOL"))
            .respond_with(wiremock::ResponseTemplate::new(201))
            .mount(&server)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("PUT"))
            .respond_with(wiremock::ResponseTemplate::new(201))
            .mount(&server)
            .await;

        let webdav = WebDavClient::new(&server.uri(), "user", "pass");
        let sync_dao = MockSyncDao::new(None);
        let storage = MockStorage {
            data: b"pdf content".to_vec(),
        };

        let doc = DocumentBackupRow {
            member_number: 1,
            first_name: Arc::from("Hans"),
            last_name: Arc::from("Müller"),
            document_type: Arc::from("Beitrittserklärung"),
            file_name: Arc::from("beitritt.pdf"),
            relative_path: Arc::from("docs/1/beitritt.pdf"),
        };

        let result =
            sync_single_document(&webdav, &sync_dao, &storage, &doc, "backup/dokumente").await;
        assert!(result.unwrap());
        assert_eq!(sync_dao.upserted.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_sync_unchanged_document() {
        let server = wiremock::MockServer::start().await;

        let data = b"pdf content";
        let hash = compute_sha256(data);

        let webdav = WebDavClient::new(&server.uri(), "user", "pass");
        let sync_dao = MockSyncDao::new(Some(&hash));
        let storage = MockStorage {
            data: data.to_vec(),
        };

        let doc = DocumentBackupRow {
            member_number: 1,
            first_name: Arc::from("Hans"),
            last_name: Arc::from("Müller"),
            document_type: Arc::from("Beitrittserklärung"),
            file_name: Arc::from("beitritt.pdf"),
            relative_path: Arc::from("docs/1/beitritt.pdf"),
        };

        let result =
            sync_single_document(&webdav, &sync_dao, &storage, &doc, "backup/dokumente").await;
        assert!(!result.unwrap()); // skipped
        assert!(sync_dao.upserted.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_sync_changed_document() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("MKCOL"))
            .respond_with(wiremock::ResponseTemplate::new(201))
            .mount(&server)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("PUT"))
            .respond_with(wiremock::ResponseTemplate::new(201))
            .mount(&server)
            .await;

        let webdav = WebDavClient::new(&server.uri(), "user", "pass");
        let sync_dao = MockSyncDao::new(Some("old_hash_value"));
        let storage = MockStorage {
            data: b"new pdf content".to_vec(),
        };

        let doc = DocumentBackupRow {
            member_number: 1,
            first_name: Arc::from("Hans"),
            last_name: Arc::from("Müller"),
            document_type: Arc::from("Beitrittserklärung"),
            file_name: Arc::from("beitritt.pdf"),
            relative_path: Arc::from("docs/1/beitritt.pdf"),
        };

        let result =
            sync_single_document(&webdav, &sync_dao, &storage, &doc, "backup/dokumente").await;
        assert!(result.unwrap()); // uploaded
        assert_eq!(sync_dao.upserted.lock().unwrap().len(), 1);
    }

    // ─── Communication sync mocks ──────────────────────────────────────

    struct MockCommSyncDao {
        synced_ids: std::sync::Mutex<Vec<(String, uuid::Uuid)>>,
        pre_synced: std::sync::Mutex<Vec<uuid::Uuid>>,
    }

    impl MockCommSyncDao {
        fn new() -> Self {
            Self {
                synced_ids: std::sync::Mutex::new(Vec::new()),
                pre_synced: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn with_pre_synced(ids: Vec<uuid::Uuid>) -> Self {
            Self {
                synced_ids: std::sync::Mutex::new(Vec::new()),
                pre_synced: std::sync::Mutex::new(ids),
            }
        }
    }

    #[async_trait::async_trait]
    impl BackupCommunicationSyncDao for MockCommSyncDao {
        async fn is_synced(&self, _mail_type: &str, mail_id: uuid::Uuid) -> Result<bool, DaoError> {
            Ok(self.pre_synced.lock().unwrap().contains(&mail_id))
        }
        async fn mark_synced(&self, mail_type: &str, mail_id: uuid::Uuid) -> Result<(), DaoError> {
            self.synced_ids
                .lock()
                .unwrap()
                .push((mail_type.to_string(), mail_id));
            Ok(())
        }
    }

    fn sample_communication(mail_id: uuid::Uuid) -> CommunicationBackupRow {
        CommunicationBackupRow {
            member_number: 1,
            first_name: Arc::from("Hans"),
            last_name: Arc::from("Müller"),
            direction: Arc::from("ausgehend"),
            date: Arc::from("2026-03-15 14:30:00"),
            subject: Arc::from("Willkommen"),
            body: Arc::from("Hallo Hans!"),
            from_address: None,
            to_address: Some(Arc::from("hans@example.com")),
            mail_id,
            mail_type: Arc::from("outbound"),
        }
    }

    #[tokio::test]
    async fn test_sync_new_communication() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("MKCOL"))
            .respond_with(wiremock::ResponseTemplate::new(201))
            .mount(&server)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("PUT"))
            .respond_with(wiremock::ResponseTemplate::new(201))
            .mount(&server)
            .await;

        let webdav = WebDavClient::new(&server.uri(), "user", "pass");
        let comm_sync_dao = MockCommSyncDao::new();
        let mail_id = uuid::Uuid::new_v4();

        let comms = vec![sample_communication(mail_id)];
        let stats = sync_communications(&webdav, &comm_sync_dao, &comms, "backup")
            .await
            .unwrap();

        assert_eq!(stats.uploaded, 1);
        assert_eq!(stats.skipped, 0);
        assert_eq!(comm_sync_dao.synced_ids.lock().unwrap().len(), 1);
        assert_eq!(comm_sync_dao.synced_ids.lock().unwrap()[0].1, mail_id);
    }

    #[tokio::test]
    async fn test_sync_already_synced_communication_skipped() {
        let server = wiremock::MockServer::start().await;

        let webdav = WebDavClient::new(&server.uri(), "user", "pass");
        let mail_id = uuid::Uuid::new_v4();
        let comm_sync_dao = MockCommSyncDao::with_pre_synced(vec![mail_id]);

        let comms = vec![sample_communication(mail_id)];
        let stats = sync_communications(&webdav, &comm_sync_dao, &comms, "backup")
            .await
            .unwrap();

        assert_eq!(stats.uploaded, 0);
        assert_eq!(stats.skipped, 1);
        assert!(comm_sync_dao.synced_ids.lock().unwrap().is_empty());
    }
}
