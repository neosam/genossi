use async_trait::async_trait;
use genossi_service::document_storage::{DocumentStorage, StorageError};
use path_clean::PathClean;
use std::path::PathBuf;
use std::sync::Arc;

pub struct FilesystemDocumentStorage {
    base_path: PathBuf,
}

impl FilesystemDocumentStorage {
    pub fn new(base_path: PathBuf) -> Self {
        // Ensure base_path exists
        if !base_path.exists() {
            std::fs::create_dir_all(&base_path).expect("Failed to create document storage dir");
        }
        Self { base_path }
    }

    pub fn from_env() -> Self {
        let path = std::env::var("DOCUMENT_STORAGE_PATH").unwrap_or_else(|_| "./documents".into());
        Self::new(PathBuf::from(path))
    }

    fn full_path(&self, relative_path: &str) -> Result<PathBuf, StorageError> {
        let joined = self.base_path.join(relative_path);
        // Normalize the path (resolve `.` and `..` without filesystem access)
        let normalized = if self.base_path.is_absolute() {
            joined.clean()
        } else {
            // Make base_path absolute for reliable comparison
            let abs_base = std::env::current_dir()
                .map_err(|e| StorageError::IoError(Arc::from(e.to_string())))?
                .join(&self.base_path);
            let abs_joined = abs_base.join(relative_path);
            abs_joined.clean()
        };

        let canonical_base = if self.base_path.is_absolute() {
            self.base_path.clean()
        } else {
            std::env::current_dir()
                .map_err(|e| StorageError::IoError(Arc::from(e.to_string())))?
                .join(&self.base_path)
                .clean()
        };

        if !normalized.starts_with(&canonical_base) {
            return Err(StorageError::ValidationError(Arc::from(
                "Path traversal detected: resolved path is outside storage directory",
            )));
        }

        // Den NORMALISIERTEN Pfad zurückgeben, nicht den rohen `joined`: sonst
        // wird zwar gegen die normalisierte Variante validiert, aber ein Pfad mit
        // `.`/`..`-Segmenten unverändert an die fs-Operationen weitergereicht
        // (document-storage-normalized-path, Defense-in-Depth).
        Ok(normalized)
    }
}

#[async_trait]
impl DocumentStorage for FilesystemDocumentStorage {
    async fn save(&self, relative_path: &str, data: &[u8]) -> Result<(), StorageError> {
        let path = self.full_path(relative_path)?;
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
        let path = self.full_path(relative_path)?;
        tokio::fs::read(&path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                StorageError::NotFound
            } else {
                StorageError::IoError(Arc::from(e.to_string()))
            }
        })
    }

    async fn delete(&self, relative_path: &str) -> Result<(), StorageError> {
        let path = self.full_path(relative_path)?;
        tokio::fs::remove_file(&path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                StorageError::NotFound
            } else {
                StorageError::IoError(Arc::from(e.to_string()))
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_save_and_load_normal_path() {
        let dir = tempfile::tempdir().unwrap();
        let storage = FilesystemDocumentStorage::new(dir.path().to_path_buf());

        storage
            .save("normal-uuid.pdf", b"hello")
            .await
            .expect("save should succeed");
        let data = storage
            .load("normal-uuid.pdf")
            .await
            .expect("load should succeed");
        assert_eq!(data, b"hello");
    }

    #[tokio::test]
    async fn test_path_traversal_dotdot() {
        let dir = tempfile::tempdir().unwrap();
        let storage = FilesystemDocumentStorage::new(dir.path().to_path_buf());

        let result = storage.save("../evil", b"bad").await;
        assert!(matches!(result, Err(StorageError::ValidationError(_))));
    }

    #[tokio::test]
    async fn test_full_path_returns_normalized_path() {
        use std::path::Component;
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().to_path_buf();
        let storage = FilesystemDocumentStorage::new(base.clone());

        // Interior `..`, das innerhalb der Base bleibt → erlaubt, MUSS aber
        // normalisiert zurückkommen (kein `..`-Segment mehr im Rückgabepfad).
        let resolved = storage
            .full_path("sub/../keep.pdf")
            .expect("path within base must be allowed");

        assert!(
            !resolved.components().any(|c| c == Component::ParentDir),
            "full_path muss einen normalisierten Pfad ohne `..` liefern, war: {}",
            resolved.display()
        );
        assert_eq!(resolved, base.clean().join("keep.pdf"));
    }

    #[tokio::test]
    async fn test_path_traversal_absolute() {
        let dir = tempfile::tempdir().unwrap();
        let storage = FilesystemDocumentStorage::new(dir.path().to_path_buf());

        let result = storage.save("/etc/passwd", b"bad").await;
        assert!(matches!(result, Err(StorageError::ValidationError(_))));
    }

    #[tokio::test]
    async fn test_delete_normal_path() {
        let dir = tempfile::tempdir().unwrap();
        let storage = FilesystemDocumentStorage::new(dir.path().to_path_buf());

        storage.save("to-delete.pdf", b"data").await.unwrap();
        storage
            .delete("to-delete.pdf")
            .await
            .expect("delete should succeed");

        let result = storage.load("to-delete.pdf").await;
        assert!(matches!(result, Err(StorageError::NotFound)));
    }

    #[tokio::test]
    async fn test_load_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let storage = FilesystemDocumentStorage::new(dir.path().to_path_buf());

        let result = storage.load("nonexistent.pdf").await;
        assert!(matches!(result, Err(StorageError::NotFound)));
    }
}
