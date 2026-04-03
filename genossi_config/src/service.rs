use async_trait::async_trait;
use mockall::automock;
use std::sync::Arc;

use crate::dao::ConfigEntry;

#[derive(Debug, Clone)]
pub enum ConfigServiceError {
    DataAccess(Arc<str>),
    NotFound,
    ValidationError(Arc<str>),
}

impl From<crate::dao::ConfigDaoError> for ConfigServiceError {
    fn from(e: crate::dao::ConfigDaoError) -> Self {
        match e {
            crate::dao::ConfigDaoError::NotFound => ConfigServiceError::NotFound,
            crate::dao::ConfigDaoError::DatabaseError(msg) => ConfigServiceError::DataAccess(msg),
        }
    }
}

#[automock]
#[async_trait]
pub trait ConfigService: Send + Sync + 'static {
    async fn get_all(&self) -> Result<Arc<[ConfigEntry]>, ConfigServiceError>;
    async fn get(&self, key: &str) -> Result<ConfigEntry, ConfigServiceError>;
    async fn set(&self, entry: &ConfigEntry) -> Result<(), ConfigServiceError>;
    async fn delete(&self, key: &str) -> Result<(), ConfigServiceError>;
}

pub struct ConfigServiceImpl<D: crate::dao::ConfigDao> {
    dao: Arc<D>,
}

impl<D: crate::dao::ConfigDao> ConfigServiceImpl<D> {
    pub fn new(dao: D) -> Self {
        Self {
            dao: Arc::new(dao),
        }
    }
}

fn validate_value(value: &str, value_type: &str) -> Result<(), ConfigServiceError> {
    match value_type {
        "string" | "secret" => Ok(()),
        "int" => value.parse::<i64>().map(|_| ()).map_err(|_| {
            ConfigServiceError::ValidationError(
                Arc::from(format!("Value '{}' is not a valid integer", value)),
            )
        }),
        "bool" => match value {
            "true" | "false" => Ok(()),
            _ => Err(ConfigServiceError::ValidationError(Arc::from(format!(
                "Value '{}' is not a valid boolean (must be 'true' or 'false')",
                value
            )))),
        },
        _ => Err(ConfigServiceError::ValidationError(Arc::from(format!(
            "Unknown value_type '{}'",
            value_type
        )))),
    }
}

#[async_trait]
impl<D: crate::dao::ConfigDao> ConfigService for ConfigServiceImpl<D> {
    async fn get_all(&self) -> Result<Arc<[ConfigEntry]>, ConfigServiceError> {
        Ok(self.dao.all().await?)
    }

    async fn get(&self, key: &str) -> Result<ConfigEntry, ConfigServiceError> {
        self.dao
            .get(key)
            .await?
            .ok_or(ConfigServiceError::NotFound)
    }

    async fn set(&self, entry: &ConfigEntry) -> Result<(), ConfigServiceError> {
        validate_value(&entry.value, &entry.value_type)?;
        Ok(self.dao.set(entry).await?)
    }

    async fn delete(&self, key: &str) -> Result<(), ConfigServiceError> {
        let existing = self.dao.get(key).await?;
        if existing.is_none() {
            return Err(ConfigServiceError::NotFound);
        }
        Ok(self.dao.delete(key).await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dao::MockConfigDao;

    #[tokio::test]
    async fn test_set_valid_int() {
        let mut mock = MockConfigDao::new();
        mock.expect_set().returning(|_| Ok(()));

        let service = ConfigServiceImpl::new(mock);
        let entry = ConfigEntry {
            key: Arc::from("smtp_port"),
            value: Arc::from("587"),
            value_type: Arc::from("int"),
        };
        assert!(service.set(&entry).await.is_ok());
    }

    #[tokio::test]
    async fn test_set_invalid_int() {
        let mock = MockConfigDao::new();
        let service = ConfigServiceImpl::new(mock);
        let entry = ConfigEntry {
            key: Arc::from("smtp_port"),
            value: Arc::from("not_a_number"),
            value_type: Arc::from("int"),
        };
        assert!(matches!(
            service.set(&entry).await,
            Err(ConfigServiceError::ValidationError(_))
        ));
    }

    #[tokio::test]
    async fn test_set_valid_bool() {
        let mut mock = MockConfigDao::new();
        mock.expect_set().returning(|_| Ok(()));

        let service = ConfigServiceImpl::new(mock);
        let entry = ConfigEntry {
            key: Arc::from("some_flag"),
            value: Arc::from("true"),
            value_type: Arc::from("bool"),
        };
        assert!(service.set(&entry).await.is_ok());
    }

    #[tokio::test]
    async fn test_set_invalid_bool() {
        let mock = MockConfigDao::new();
        let service = ConfigServiceImpl::new(mock);
        let entry = ConfigEntry {
            key: Arc::from("some_flag"),
            value: Arc::from("yes"),
            value_type: Arc::from("bool"),
        };
        assert!(matches!(
            service.set(&entry).await,
            Err(ConfigServiceError::ValidationError(_))
        ));
    }

    #[tokio::test]
    async fn test_set_valid_string() {
        let mut mock = MockConfigDao::new();
        mock.expect_set().returning(|_| Ok(()));

        let service = ConfigServiceImpl::new(mock);
        let entry = ConfigEntry {
            key: Arc::from("smtp_host"),
            value: Arc::from("mail.example.com"),
            value_type: Arc::from("string"),
        };
        assert!(service.set(&entry).await.is_ok());
    }

    #[tokio::test]
    async fn test_set_valid_secret() {
        let mut mock = MockConfigDao::new();
        mock.expect_set().returning(|_| Ok(()));

        let service = ConfigServiceImpl::new(mock);
        let entry = ConfigEntry {
            key: Arc::from("smtp_pass"),
            value: Arc::from("supersecret"),
            value_type: Arc::from("secret"),
        };
        assert!(service.set(&entry).await.is_ok());
    }

    #[tokio::test]
    async fn test_set_unknown_type() {
        let mock = MockConfigDao::new();
        let service = ConfigServiceImpl::new(mock);
        let entry = ConfigEntry {
            key: Arc::from("key"),
            value: Arc::from("value"),
            value_type: Arc::from("unknown"),
        };
        assert!(matches!(
            service.set(&entry).await,
            Err(ConfigServiceError::ValidationError(_))
        ));
    }

    #[tokio::test]
    async fn test_delete_existing() {
        let mut mock = MockConfigDao::new();
        mock.expect_get().returning(|_| {
            Ok(Some(ConfigEntry {
                key: Arc::from("key"),
                value: Arc::from("value"),
                value_type: Arc::from("string"),
            }))
        });
        mock.expect_delete().returning(|_| Ok(()));

        let service = ConfigServiceImpl::new(mock);
        assert!(service.delete("key").await.is_ok());
    }

    #[tokio::test]
    async fn test_delete_nonexisting() {
        let mut mock = MockConfigDao::new();
        mock.expect_get().returning(|_| Ok(None));

        let service = ConfigServiceImpl::new(mock);
        assert!(matches!(
            service.delete("key").await,
            Err(ConfigServiceError::NotFound)
        ));
    }

    #[tokio::test]
    async fn test_get_all() {
        let mut mock = MockConfigDao::new();
        mock.expect_all().returning(|| {
            Ok(vec![ConfigEntry {
                key: Arc::from("key"),
                value: Arc::from("value"),
                value_type: Arc::from("string"),
            }]
            .into())
        });

        let service = ConfigServiceImpl::new(mock);
        let result = service.get_all().await.unwrap();
        assert_eq!(result.len(), 1);
    }
}
