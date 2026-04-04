use async_trait::async_trait;
use genossi_dao::user_preference::{UserPreferenceDao, UserPreferenceEntity};
use genossi_dao::DaoError;
use sqlx::SqlitePool;
use std::sync::Arc;
use time::PrimitiveDateTime;
use uuid::Uuid;

use crate::TransactionImpl;

fn parse_datetime(s: &str) -> Result<PrimitiveDateTime, time::error::Parse> {
    if let Ok(dt) =
        PrimitiveDateTime::parse(s, &time::format_description::well_known::Iso8601::DEFAULT)
    {
        return Ok(dt);
    }
    let sqlite_format = time::format_description::parse(
        "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond]",
    )
    .unwrap();
    if let Ok(dt) = PrimitiveDateTime::parse(s, &sqlite_format) {
        return Ok(dt);
    }
    let sqlite_simple =
        time::format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]").unwrap();
    PrimitiveDateTime::parse(s, &sqlite_simple)
}

#[derive(Debug, sqlx::FromRow)]
struct UserPreferenceDb {
    id: Vec<u8>,
    user_id: String,
    key: String,
    value: String,
    created: String,
    deleted: Option<String>,
    version: Vec<u8>,
}

impl TryFrom<&UserPreferenceDb> for UserPreferenceEntity {
    type Error = DaoError;

    fn try_from(db: &UserPreferenceDb) -> Result<Self, Self::Error> {
        Ok(UserPreferenceEntity {
            id: Uuid::from_slice(&db.id)?,
            user_id: Arc::from(db.user_id.as_str()),
            key: Arc::from(db.key.as_str()),
            value: Arc::from(db.value.as_str()),
            created: parse_datetime(&db.created)?,
            deleted: db
                .deleted
                .as_ref()
                .map(|d| parse_datetime(d))
                .transpose()?,
            version: Uuid::from_slice(&db.version)?,
        })
    }
}

pub struct UserPreferenceDaoImpl {
    pub pool: Arc<SqlitePool>,
}

impl UserPreferenceDaoImpl {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserPreferenceDao for UserPreferenceDaoImpl {
    type Transaction = TransactionImpl;

    async fn dump_all(
        &self,
        tx: Self::Transaction,
    ) -> Result<Arc<[UserPreferenceEntity]>, DaoError> {
        let rows = sqlx::query_as::<_, UserPreferenceDb>(
            "SELECT id, user_id, key, value, created, deleted, version \
             FROM user_preferences ORDER BY key",
        )
        .fetch_all(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        rows.iter()
            .map(UserPreferenceEntity::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into())
    }

    async fn create(
        &self,
        entity: &UserPreferenceEntity,
        _process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError> {
        let id = entity.id.as_bytes().to_vec();
        let user_id = entity.user_id.to_string();
        let version = entity.version.as_bytes().to_vec();
        let format = &time::format_description::well_known::Iso8601::DEFAULT;
        let created = entity
            .created
            .assume_utc()
            .format(format)
            .map_err(|e| DaoError::ParseError(Arc::from(e.to_string())))?;
        let key = entity.key.to_string();
        let value = entity.value.to_string();

        sqlx::query(
            "INSERT INTO user_preferences (id, user_id, key, value, created, version) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(user_id)
        .bind(key)
        .bind(value)
        .bind(created)
        .bind(version)
        .execute(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(())
    }

    async fn update(
        &self,
        entity: &UserPreferenceEntity,
        _process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError> {
        let id = entity.id.as_bytes().to_vec();
        let old_version = entity.version.as_bytes().to_vec();
        let new_version = Uuid::new_v4().as_bytes().to_vec();
        let key = entity.key.to_string();
        let value = entity.value.to_string();

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

        let rows_affected = sqlx::query(
            "UPDATE user_preferences SET key = ?, value = ?, deleted = ?, version = ? \
             WHERE id = ? AND version = ? AND deleted IS NULL",
        )
        .bind(key)
        .bind(value)
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
