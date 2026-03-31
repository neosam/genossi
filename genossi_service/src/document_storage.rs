use async_trait::async_trait;
use mockall::automock;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum StorageError {
    IoError(Arc<str>),
    NotFound,
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::IoError(msg) => write!(f, "Storage IO error: {}", msg),
            StorageError::NotFound => write!(f, "File not found in storage"),
        }
    }
}

#[automock]
#[async_trait]
pub trait DocumentStorage: Send + Sync {
    async fn save(&self, relative_path: &str, data: &[u8]) -> Result<(), StorageError>;
    async fn load(&self, relative_path: &str) -> Result<Vec<u8>, StorageError>;
    async fn delete(&self, relative_path: &str) -> Result<(), StorageError>;
}
