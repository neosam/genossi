use async_trait::async_trait;
use mockall::automock;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum ConfigDaoError {
    DatabaseError(Arc<str>),
    NotFound,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigEntry {
    pub key: Arc<str>,
    pub value: Arc<str>,
    pub value_type: Arc<str>,
}

#[automock]
#[async_trait]
pub trait ConfigDao: Send + Sync + 'static {
    async fn all(&self) -> Result<Arc<[ConfigEntry]>, ConfigDaoError>;
    async fn get(&self, key: &str) -> Result<Option<ConfigEntry>, ConfigDaoError>;
    async fn set(&self, entry: &ConfigEntry) -> Result<(), ConfigDaoError>;
    async fn delete(&self, key: &str) -> Result<(), ConfigDaoError>;
}
