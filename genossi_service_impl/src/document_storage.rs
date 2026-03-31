use async_trait::async_trait;
use genossi_service::document_storage::{DocumentStorage, StorageError};
use std::path::PathBuf;
use std::sync::Arc;

pub struct FilesystemDocumentStorage {
    base_path: PathBuf,
}

impl FilesystemDocumentStorage {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    pub fn from_env() -> Self {
        let path = std::env::var("DOCUMENT_STORAGE_PATH").unwrap_or_else(|_| "./documents".into());
        Self::new(PathBuf::from(path))
    }

    fn full_path(&self, relative_path: &str) -> PathBuf {
        self.base_path.join(relative_path)
    }
}

#[async_trait]
impl DocumentStorage for FilesystemDocumentStorage {
    async fn save(&self, relative_path: &str, data: &[u8]) -> Result<(), StorageError> {
        let path = self.full_path(relative_path);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| StorageError::IoError(Arc::from(e.to_string())))?;
        }
        tokio::fs::write(&path, data)
            .await
            .map_err(|e| StorageError::IoError(Arc::from(e.to_string())))
    }

    async fn load(&self, relative_path: &str) -> Result<Vec<u8>, StorageError> {
        let path = self.full_path(relative_path);
        tokio::fs::read(&path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                StorageError::NotFound
            } else {
                StorageError::IoError(Arc::from(e.to_string()))
            }
        })
    }

    async fn delete(&self, relative_path: &str) -> Result<(), StorageError> {
        let path = self.full_path(relative_path);
        tokio::fs::remove_file(&path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                StorageError::NotFound
            } else {
                StorageError::IoError(Arc::from(e.to_string()))
            }
        })
    }
}
