use async_trait::async_trait;
use sqlx::SqlitePool;
use std::sync::Arc;
use time::PrimitiveDateTime;
use uuid::Uuid;

use inventurly_dao::{
    container::{ContainerDao, ContainerEntity},
    DaoError,
};

use crate::TransactionImpl;

pub struct SqliteContainerDao {
    pool: SqlitePool,
}

impl SqliteContainerDao {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct ContainerDb {
    id: Vec<u8>,
    name: String,
    weight_grams: i64,
    description: Option<String>,
    created: String,
    deleted: Option<String>,
    version: Vec<u8>,
}

impl TryFrom<&ContainerDb> for ContainerEntity {
    type Error = DaoError;

    fn try_from(db: &ContainerDb) -> Result<Self, Self::Error> {
        // Try multiple datetime formats to handle different storage formats
        fn parse_datetime(s: &str) -> Result<PrimitiveDateTime, time::error::Parse> {
            // First try ISO8601 format (what we should be using)
            if let Ok(dt) =
                PrimitiveDateTime::parse(s, &time::format_description::well_known::Iso8601::DEFAULT)
            {
                return Ok(dt);
            }

            // Then try SQLite default format with microseconds
            let sqlite_format = time::format_description::parse(
                "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond]",
            )
            .unwrap(); // This format should always parse correctly
            if let Ok(dt) = PrimitiveDateTime::parse(s, &sqlite_format) {
                return Ok(dt);
            }

            // Try SQLite format without microseconds
            let sqlite_simple =
                time::format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]")
                    .unwrap(); // This format should always parse correctly
            PrimitiveDateTime::parse(s, &sqlite_simple)
        }

        Ok(ContainerEntity {
            id: Uuid::from_slice(&db.id)?,
            name: Arc::from(db.name.as_str()),
            weight_grams: db.weight_grams,
            description: Arc::from(db.description.as_deref().unwrap_or("")),
            created: parse_datetime(&db.created)
                .map_err(|e| DaoError::ParseError(Arc::from(e.to_string())))?,
            deleted: db
                .deleted
                .as_ref()
                .map(|d| parse_datetime(d))
                .transpose()
                .map_err(|e| DaoError::ParseError(Arc::from(e.to_string())))?,
            version: Uuid::from_slice(&db.version)?,
        })
    }
}

#[async_trait]
impl ContainerDao for SqliteContainerDao {
    type Transaction = TransactionImpl;

    async fn dump_all(&self, tx: Self::Transaction) -> Result<Arc<[ContainerEntity]>, DaoError> {
        let rows = sqlx::query_as::<_, ContainerDb>(
            "SELECT id, name, weight_grams, description, created, deleted, version 
             FROM container ORDER BY name",
        )
        .fetch_all(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        rows.iter()
            .map(ContainerEntity::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into())
    }

    async fn create(
        &self,
        entity: &ContainerEntity,
        _process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError> {
        let id_bytes = entity.id.as_bytes().to_vec();
        let version_bytes = entity.version.as_bytes().to_vec();
        let created_str = entity
            .created
            .assume_utc()
            .format(&time::format_description::well_known::Iso8601::DEFAULT)
            .map_err(|e| DaoError::ParseError(Arc::from(e.to_string())))?;
        let deleted_str = entity
            .deleted
            .map(|dt| {
                dt.assume_utc()
                    .format(&time::format_description::well_known::Iso8601::DEFAULT)
                    .map_err(|e| DaoError::ParseError(Arc::from(e.to_string())))
            })
            .transpose()?;

        let description = if entity.description.is_empty() {
            None
        } else {
            Some(entity.description.as_ref())
        };
        let name = entity.name.as_ref();

        sqlx::query!(
            r#"
            INSERT INTO container (id, name, weight_grams, description, created, deleted, version)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            id_bytes,
            name,
            entity.weight_grams,
            description,
            created_str,
            deleted_str,
            version_bytes
        )
        .execute(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(())
    }

    async fn update(
        &self,
        entity: &ContainerEntity,
        _process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError> {
        let id_bytes = entity.id.as_bytes().to_vec();
        let version_bytes = entity.version.as_bytes().to_vec();
        let created_str = entity
            .created
            .assume_utc()
            .format(&time::format_description::well_known::Iso8601::DEFAULT)
            .map_err(|e| DaoError::ParseError(Arc::from(e.to_string())))?;
        let deleted_str = entity
            .deleted
            .map(|dt| {
                dt.assume_utc()
                    .format(&time::format_description::well_known::Iso8601::DEFAULT)
                    .map_err(|e| DaoError::ParseError(Arc::from(e.to_string())))
            })
            .transpose()?;

        let description = if entity.description.is_empty() {
            None
        } else {
            Some(entity.description.as_ref())
        };
        let name = entity.name.as_ref();

        let result = sqlx::query!(
            r#"
            UPDATE container 
            SET name = ?2, weight_grams = ?3, description = ?4, created = ?5, deleted = ?6, version = ?7
            WHERE id = ?1
            "#,
            id_bytes,
            name,
            entity.weight_grams,
            description,
            created_str,
            deleted_str,
            version_bytes
        )
        .execute(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        if result.rows_affected() == 0 {
            return Err(DaoError::NotFound);
        }

        Ok(())
    }
}
