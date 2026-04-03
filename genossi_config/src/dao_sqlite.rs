use async_trait::async_trait;
use sqlx::SqlitePool;
use std::sync::Arc;

use crate::dao::{ConfigDao, ConfigDaoError, ConfigEntry};

#[derive(Debug, sqlx::FromRow)]
struct ConfigEntryDb {
    key: String,
    value: String,
    value_type: String,
}

impl From<&ConfigEntryDb> for ConfigEntry {
    fn from(db: &ConfigEntryDb) -> Self {
        Self {
            key: Arc::from(db.key.as_str()),
            value: Arc::from(db.value.as_str()),
            value_type: Arc::from(db.value_type.as_str()),
        }
    }
}

pub struct ConfigDaoSqlite {
    pool: Arc<SqlitePool>,
}

impl ConfigDaoSqlite {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ConfigDao for ConfigDaoSqlite {
    async fn all(&self) -> Result<Arc<[ConfigEntry]>, ConfigDaoError> {
        let rows = sqlx::query_as::<_, ConfigEntryDb>(
            "SELECT key, value, value_type FROM config_entries ORDER BY key",
        )
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| ConfigDaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(rows.iter().map(ConfigEntry::from).collect::<Vec<_>>().into())
    }

    async fn get(&self, key: &str) -> Result<Option<ConfigEntry>, ConfigDaoError> {
        let row = sqlx::query_as::<_, ConfigEntryDb>(
            "SELECT key, value, value_type FROM config_entries WHERE key = ?",
        )
        .bind(key)
        .fetch_optional(self.pool.as_ref())
        .await
        .map_err(|e| ConfigDaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(row.as_ref().map(ConfigEntry::from))
    }

    async fn set(&self, entry: &ConfigEntry) -> Result<(), ConfigDaoError> {
        sqlx::query(
            "INSERT INTO config_entries (key, value, value_type) VALUES (?, ?, ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, value_type = excluded.value_type",
        )
        .bind(entry.key.as_ref())
        .bind(entry.value.as_ref())
        .bind(entry.value_type.as_ref())
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| ConfigDaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<(), ConfigDaoError> {
        sqlx::query("DELETE FROM config_entries WHERE key = ?")
            .bind(key)
            .execute(self.pool.as_ref())
            .await
            .map_err(|e| ConfigDaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_db() -> Arc<SqlitePool> {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create in-memory database");
        sqlx::query(
            "CREATE TABLE config_entries (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                value_type TEXT NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .expect("Failed to create table");
        Arc::new(pool)
    }

    #[tokio::test]
    async fn test_set_and_get() {
        let pool = setup_db().await;
        let dao = ConfigDaoSqlite::new(pool);

        let entry = ConfigEntry {
            key: Arc::from("test_key"),
            value: Arc::from("test_value"),
            value_type: Arc::from("string"),
        };
        dao.set(&entry).await.unwrap();

        let result = dao.get("test_key").await.unwrap();
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.key.as_ref(), "test_key");
        assert_eq!(result.value.as_ref(), "test_value");
        assert_eq!(result.value_type.as_ref(), "string");
    }

    #[tokio::test]
    async fn test_upsert() {
        let pool = setup_db().await;
        let dao = ConfigDaoSqlite::new(pool);

        let entry = ConfigEntry {
            key: Arc::from("key"),
            value: Arc::from("value1"),
            value_type: Arc::from("string"),
        };
        dao.set(&entry).await.unwrap();

        let entry2 = ConfigEntry {
            key: Arc::from("key"),
            value: Arc::from("value2"),
            value_type: Arc::from("int"),
        };
        dao.set(&entry2).await.unwrap();

        let result = dao.get("key").await.unwrap().unwrap();
        assert_eq!(result.value.as_ref(), "value2");
        assert_eq!(result.value_type.as_ref(), "int");
    }

    #[tokio::test]
    async fn test_all() {
        let pool = setup_db().await;
        let dao = ConfigDaoSqlite::new(pool);

        dao.set(&ConfigEntry {
            key: Arc::from("b_key"),
            value: Arc::from("val"),
            value_type: Arc::from("string"),
        })
        .await
        .unwrap();
        dao.set(&ConfigEntry {
            key: Arc::from("a_key"),
            value: Arc::from("val"),
            value_type: Arc::from("string"),
        })
        .await
        .unwrap();

        let all = dao.all().await.unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].key.as_ref(), "a_key");
        assert_eq!(all[1].key.as_ref(), "b_key");
    }

    #[tokio::test]
    async fn test_delete() {
        let pool = setup_db().await;
        let dao = ConfigDaoSqlite::new(pool);

        dao.set(&ConfigEntry {
            key: Arc::from("key"),
            value: Arc::from("val"),
            value_type: Arc::from("string"),
        })
        .await
        .unwrap();

        dao.delete("key").await.unwrap();
        let result = dao.get("key").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_nonexistent() {
        let pool = setup_db().await;
        let dao = ConfigDaoSqlite::new(pool);

        let result = dao.get("nonexistent").await.unwrap();
        assert!(result.is_none());
    }
}
