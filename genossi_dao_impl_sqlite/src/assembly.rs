use async_trait::async_trait;
use genossi_dao::assembly::{AssemblyDao, AssemblyEntity, AssemblyStatus};
use genossi_dao::DaoError;
use sqlx::SqlitePool;
use std::sync::Arc;
use uuid::Uuid;

use crate::TransactionImpl;
use crate::datetime_utils::{format_dt, parse_datetime};


#[derive(Debug, sqlx::FromRow)]
struct AssemblyDb {
    id: Vec<u8>,
    name: String,
    date: String,
    location: Option<String>,
    status: String,
    opened_at: Option<String>,
    closed_at: Option<String>,
    created: String,
    deleted: Option<String>,
    version: Vec<u8>,
}

impl TryFrom<&AssemblyDb> for AssemblyEntity {
    type Error = DaoError;

    fn try_from(db: &AssemblyDb) -> Result<Self, Self::Error> {
        Ok(AssemblyEntity {
            id: Uuid::from_slice(&db.id)?,
            name: Arc::from(db.name.as_str()),
            date: parse_datetime(&db.date)?,
            location: db.location.as_deref().map(Arc::from),
            status: AssemblyStatus::from_str(&db.status)?,
            opened_at: db
                .opened_at
                .as_ref()
                .map(|s| parse_datetime(s))
                .transpose()?,
            closed_at: db
                .closed_at
                .as_ref()
                .map(|s| parse_datetime(s))
                .transpose()?,
            created: parse_datetime(&db.created)?,
            deleted: db.deleted.as_ref().map(|d| parse_datetime(d)).transpose()?,
            version: Uuid::from_slice(&db.version)?,
        })
    }
}

pub struct AssemblyDaoImpl {
    pub pool: Arc<SqlitePool>,
}

impl AssemblyDaoImpl {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}


#[async_trait]
impl AssemblyDao for AssemblyDaoImpl {
    type Transaction = TransactionImpl;

    async fn dump_all(&self, tx: Self::Transaction) -> Result<Arc<[AssemblyEntity]>, DaoError> {
        let rows = sqlx::query_as::<_, AssemblyDb>(
            "SELECT id, name, date, location, status, opened_at, closed_at, created, deleted, version \
             FROM assembly ORDER BY date DESC",
        )
        .fetch_all(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        rows.iter()
            .map(AssemblyEntity::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into())
    }

    async fn create(
        &self,
        entity: &AssemblyEntity,
        _process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError> {
        let id = entity.id.as_bytes().to_vec();
        let version = entity.version.as_bytes().to_vec();
        let name = entity.name.to_string();
        let date = format_dt(&entity.date)?;
        let location = entity.location.as_deref().map(|s| s.to_string());
        let status = entity.status.as_str().to_string();
        let opened_at = entity.opened_at.as_ref().map(format_dt).transpose()?;
        let closed_at = entity.closed_at.as_ref().map(format_dt).transpose()?;
        let created = format_dt(&entity.created)?;
        let deleted = entity.deleted.as_ref().map(format_dt).transpose()?;

        sqlx::query(
            "INSERT INTO assembly (id, name, date, location, status, opened_at, closed_at, \
             created, deleted, version) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(name)
        .bind(date)
        .bind(location)
        .bind(status)
        .bind(opened_at)
        .bind(closed_at)
        .bind(created)
        .bind(deleted)
        .bind(version)
        .execute(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(())
    }

    async fn update(
        &self,
        entity: &AssemblyEntity,
        _process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError> {
        let id = entity.id.as_bytes().to_vec();
        let old_version = entity.version.as_bytes().to_vec();
        let new_version = Uuid::new_v4().as_bytes().to_vec();
        let name = entity.name.to_string();
        let date = format_dt(&entity.date)?;
        let location = entity.location.as_deref().map(|s| s.to_string());
        let status = entity.status.as_str().to_string();
        let opened_at = entity.opened_at.as_ref().map(format_dt).transpose()?;
        let closed_at = entity.closed_at.as_ref().map(format_dt).transpose()?;
        let deleted = entity.deleted.as_ref().map(format_dt).transpose()?;

        // Pre-condition: row must exist and not be soft-deleted. Without this
        // check, a missing-id and a version mismatch would both surface as
        // ConflictError, which conflates two distinct error semantics.
        let exists = sqlx::query_scalar::<_, i32>(
            "SELECT COUNT(*) FROM assembly WHERE id = ? AND deleted IS NULL",
        )
        .bind(id.clone())
        .fetch_one(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        if exists == 0 {
            return Err(DaoError::NotFound);
        }

        let rows_affected = sqlx::query(
            "UPDATE assembly SET name = ?, date = ?, location = ?, status = ?, \
             opened_at = ?, closed_at = ?, deleted = ?, version = ? \
             WHERE id = ? AND version = ? AND deleted IS NULL",
        )
        .bind(name)
        .bind(date)
        .bind(location)
        .bind(status)
        .bind(opened_at)
        .bind(closed_at)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TransactionDaoImpl;
    use genossi_dao::{Transaction, TransactionDao};

    /// Bring up an in-memory SQLite pool with the assembly schema applied.
    /// We don't run the full `migrations/sqlite/` set here because that would
    /// require the entire Member/Application/etc. graph; we only need the
    /// assembly tables for these unit tests.
    async fn setup_db() -> Arc<SqlitePool> {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("create in-memory db");

        sqlx::query(
            "CREATE TABLE assembly (
                id BLOB PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                date TEXT NOT NULL,
                location TEXT,
                status TEXT NOT NULL DEFAULT 'Preparation',
                opened_at TEXT,
                closed_at TEXT,
                created TEXT NOT NULL,
                deleted TEXT,
                version BLOB NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .expect("create assembly table");

        Arc::new(pool)
    }

    fn make_assembly() -> AssemblyEntity {
        let date = time::Date::from_calendar_date(2026, time::Month::May, 15).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        AssemblyEntity {
            id: Uuid::new_v4(),
            name: Arc::from("GV 2026"),
            date: datetime,
            location: Some(Arc::from("Vereinsheim")),
            status: AssemblyStatus::Preparation,
            opened_at: None,
            closed_at: None,
            created: datetime,
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    #[tokio::test]
    async fn test_create_and_find_assembly() {
        let pool = setup_db().await;
        let dao = AssemblyDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let entity = make_assembly();
        let entity_id = entity.id;

        let tx = tx_dao.transaction().await.unwrap();
        dao.create(&entity, "test", tx.clone()).await.unwrap();

        let found = dao.find_by_id(entity_id, tx.clone()).await.unwrap();
        let found = found.expect("entity must be found");
        assert_eq!(found.id, entity_id);
        assert_eq!(found.status, AssemblyStatus::Preparation);
        assert_eq!(found.name.as_ref(), "GV 2026");
        assert_eq!(
            found.location.as_deref().map(|s| s.to_string()),
            Some("Vereinsheim".to_string())
        );

        let all = dao.all(tx.clone()).await.unwrap();
        assert_eq!(all.len(), 1);

        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_update_with_version_mismatch_returns_conflict() {
        let pool = setup_db().await;
        let dao = AssemblyDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let entity = make_assembly();

        let tx = tx_dao.transaction().await.unwrap();
        dao.create(&entity, "test", tx.clone()).await.unwrap();

        // Build an update with a stale version (random UUID, not the persisted one).
        let mut stale = entity.clone();
        stale.version = Uuid::new_v4();
        stale.status = AssemblyStatus::Open;

        let result = dao.update(&stale, "test", tx.clone()).await;
        assert!(
            matches!(result, Err(DaoError::ConflictError(_))),
            "expected ConflictError, got: {:?}",
            result
        );

        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_update_unknown_id_returns_not_found() {
        let pool = setup_db().await;
        let dao = AssemblyDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let mut entity = make_assembly();
        entity.id = Uuid::new_v4(); // never persisted

        let tx = tx_dao.transaction().await.unwrap();
        let result = dao.update(&entity, "test", tx.clone()).await;
        assert!(
            matches!(result, Err(DaoError::NotFound)),
            "expected NotFound, got: {:?}",
            result
        );

        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_update_succeeds_then_version_changes() {
        let pool = setup_db().await;
        let dao = AssemblyDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let entity = make_assembly();
        let entity_id = entity.id;

        let tx = tx_dao.transaction().await.unwrap();
        dao.create(&entity, "test", tx.clone()).await.unwrap();

        let mut update = entity.clone();
        update.status = AssemblyStatus::Open;
        let opened_at = update.date;
        update.opened_at = Some(opened_at);

        dao.update(&update, "test", tx.clone()).await.unwrap();

        let after = dao
            .find_by_id(entity_id, tx.clone())
            .await
            .unwrap()
            .expect("entity must still exist");
        assert_eq!(after.status, AssemblyStatus::Open);
        assert_eq!(after.opened_at, Some(opened_at));
        assert_ne!(
            after.version, entity.version,
            "version must rotate on update"
        );

        tx.commit().await.unwrap();
    }
}
