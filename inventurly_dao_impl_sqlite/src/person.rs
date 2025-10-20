use async_trait::async_trait;
use inventurly_dao::{
    person::{PersonDao, PersonEntity},
    DaoError,
};
use sqlx::SqlitePool;
use std::sync::Arc;
use time::PrimitiveDateTime;
use uuid::Uuid;

use crate::TransactionImpl;

#[derive(Debug, sqlx::FromRow)]
struct PersonDb {
    id: Vec<u8>,
    name: String,
    age: i32,
    created: String,
    deleted: Option<String>,
    version: Vec<u8>,
}

impl TryFrom<&PersonDb> for PersonEntity {
    type Error = DaoError;

    fn try_from(db: &PersonDb) -> Result<Self, Self::Error> {
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

        Ok(PersonEntity {
            id: Uuid::from_slice(&db.id)?,
            name: Arc::from(db.name.as_str()),
            age: db.age,
            created: parse_datetime(&db.created)?,
            deleted: db.deleted.as_ref().map(|d| parse_datetime(d)).transpose()?,
            version: Uuid::from_slice(&db.version)?,
        })
    }
}

pub struct PersonDaoImpl {
    pub pool: Arc<SqlitePool>,
}

impl PersonDaoImpl {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PersonDao for PersonDaoImpl {
    type Transaction = TransactionImpl;

    async fn dump_all(&self, tx: Self::Transaction) -> Result<Arc<[PersonEntity]>, DaoError> {
        let rows = sqlx::query_as::<_, PersonDb>(
            "SELECT id, name, age, created, deleted, version FROM person ORDER BY name",
        )
        .fetch_all(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        rows.iter()
            .map(PersonEntity::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into())
    }

    async fn create(
        &self,
        entity: &PersonEntity,
        _process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError> {
        let id = entity.id.as_bytes().to_vec();
        let version = entity.version.as_bytes().to_vec();
        let format = &time::format_description::well_known::Iso8601::DEFAULT;
        let created = entity
            .created
            .assume_utc()
            .format(format)
            .map_err(|e| DaoError::ParseError(Arc::from(e.to_string())))?;
        let name = entity.name.to_string();
        let age = entity.age;

        sqlx::query("INSERT INTO person (id, name, age, created, version) VALUES (?, ?, ?, ?, ?)")
            .bind(id)
            .bind(name)
            .bind(age)
            .bind(created)
            .bind(version)
            .execute(tx.tx.lock().await.as_mut())
            .await
            .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(())
    }

    async fn update(
        &self,
        entity: &PersonEntity,
        _process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError> {
        let id = entity.id.as_bytes().to_vec();
        let old_version = entity.version.as_bytes().to_vec();
        let new_version = Uuid::new_v4().as_bytes().to_vec();
        let name = entity.name.to_string();
        let age = entity.age;

        // Format deleted timestamp if present
        let deleted = match entity.deleted {
            Some(dt) => {
                let format = &time::format_description::well_known::Iso8601::DEFAULT;
                Some(
                    dt.assume_utc()
                        .format(format)
                        .map_err(|e| DaoError::ParseError(Arc::from(e.to_string())))?,
                )
            }
            None => None,
        };

        // First check if the entity exists
        let exists = sqlx::query_scalar::<_, i32>(
            "SELECT COUNT(*) FROM person WHERE id = ? AND deleted IS NULL",
        )
        .bind(id.clone())
        .fetch_one(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        if exists == 0 {
            return Err(DaoError::NotFound);
        }

        let rows_affected = sqlx::query(
            "UPDATE person SET name = ?, age = ?, deleted = ?, version = ? WHERE id = ? AND version = ? AND deleted IS NULL"
        )
        .bind(name)
        .bind(age)
        .bind(deleted)
        .bind(new_version)
        .bind(id)
        .bind(old_version)
        .execute(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?
        .rows_affected();

        if rows_affected == 0 {
            return Err(DaoError::ConflictError(Arc::from("Version mismatch")));
        }

        Ok(())
    }
}
