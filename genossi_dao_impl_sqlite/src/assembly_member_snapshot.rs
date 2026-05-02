use async_trait::async_trait;
use genossi_dao::assembly_member_snapshot::{
    AssemblyMemberSnapshotDao, AssemblyMemberSnapshotEntity,
};
use genossi_dao::DaoError;
use sqlx::SqlitePool;
use std::sync::Arc;
use uuid::Uuid;

use crate::assembly::parse_datetime;
use crate::TransactionImpl;

#[derive(Debug, sqlx::FromRow)]
struct AssemblyMemberSnapshotDb {
    assembly_id: Vec<u8>,
    member_id: Vec<u8>,
    captured_at: String,
}

impl TryFrom<&AssemblyMemberSnapshotDb> for AssemblyMemberSnapshotEntity {
    type Error = DaoError;

    fn try_from(db: &AssemblyMemberSnapshotDb) -> Result<Self, Self::Error> {
        Ok(AssemblyMemberSnapshotEntity {
            assembly_id: Uuid::from_slice(&db.assembly_id)?,
            member_id: Uuid::from_slice(&db.member_id)?,
            captured_at: parse_datetime(&db.captured_at)?,
        })
    }
}

pub struct AssemblyMemberSnapshotDaoImpl {
    pub pool: Arc<SqlitePool>,
}

impl AssemblyMemberSnapshotDaoImpl {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

fn format_dt(dt: &time::PrimitiveDateTime) -> Result<String, DaoError> {
    let format = &time::format_description::well_known::Iso8601::DEFAULT;
    dt.assume_utc()
        .format(format)
        .map_err(|e| DaoError::ParseError(Arc::from(e.to_string())))
}

#[async_trait]
impl AssemblyMemberSnapshotDao for AssemblyMemberSnapshotDaoImpl {
    type Transaction = TransactionImpl;

    async fn create(
        &self,
        entity: &AssemblyMemberSnapshotEntity,
        _process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError> {
        let assembly_id = entity.assembly_id.as_bytes().to_vec();
        let member_id = entity.member_id.as_bytes().to_vec();
        let captured_at = format_dt(&entity.captured_at)?;

        sqlx::query(
            "INSERT INTO assembly_member_snapshot (assembly_id, member_id, captured_at) \
             VALUES (?, ?, ?)",
        )
        .bind(assembly_id)
        .bind(member_id)
        .bind(captured_at)
        .execute(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(())
    }

    async fn create_batch(
        &self,
        entities: &[AssemblyMemberSnapshotEntity],
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError> {
        // Empty input must be a no-op (Plan 03 will call this when a GV opens
        // with zero active members; the call should not error).
        if entities.is_empty() {
            return Ok(());
        }
        // Loop variant — keeps the bind list trivially correct, and the
        // Composite-PK constraint surfaces on the offending row in the same
        // way it would for a single insert. A multi-row INSERT would be a
        // micro-optimisation; the snapshot is captured once per GV, not in a
        // hot loop.
        for entity in entities {
            self.create(entity, process, tx.clone()).await?;
        }
        Ok(())
    }

    async fn find_by_assembly_id(
        &self,
        assembly_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Arc<[AssemblyMemberSnapshotEntity]>, DaoError> {
        let id = assembly_id.as_bytes().to_vec();
        let rows = sqlx::query_as::<_, AssemblyMemberSnapshotDb>(
            "SELECT assembly_id, member_id, captured_at \
             FROM assembly_member_snapshot WHERE assembly_id = ?",
        )
        .bind(id)
        .fetch_all(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        rows.iter()
            .map(AssemblyMemberSnapshotEntity::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into())
    }

    async fn count_by_assembly_id(
        &self,
        assembly_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<u64, DaoError> {
        let id = assembly_id.as_bytes().to_vec();
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM assembly_member_snapshot WHERE assembly_id = ?",
        )
        .bind(id)
        .fetch_one(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(count as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TransactionDaoImpl;
    use genossi_dao::{Transaction, TransactionDao};

    /// In-memory schema for the snapshot DAO. We deliberately omit the FK
    /// constraints here (PRAGMA foreign_keys is off by default in SQLite),
    /// since we want to exercise the Composite-PK path without bringing up
    /// the full Member+Assembly schema graph.
    async fn setup_db() -> Arc<SqlitePool> {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("create in-memory db");
        sqlx::query(
            "CREATE TABLE assembly_member_snapshot (
                assembly_id BLOB NOT NULL,
                member_id BLOB NOT NULL,
                captured_at TEXT NOT NULL,
                PRIMARY KEY (assembly_id, member_id)
            )",
        )
        .execute(&pool)
        .await
        .expect("create snapshot table");
        Arc::new(pool)
    }

    fn make_snapshot(assembly_id: Uuid, member_id: Uuid) -> AssemblyMemberSnapshotEntity {
        let date = time::Date::from_calendar_date(2026, time::Month::May, 15).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        AssemblyMemberSnapshotEntity {
            assembly_id,
            member_id,
            captured_at: datetime,
        }
    }

    #[tokio::test]
    async fn test_create_snapshot_then_count() {
        let pool = setup_db().await;
        let dao = AssemblyMemberSnapshotDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let assembly_id = Uuid::new_v4();
        let member_id = Uuid::new_v4();
        let entity = make_snapshot(assembly_id, member_id);

        let tx = tx_dao.transaction().await.unwrap();
        dao.create(&entity, "test", tx.clone()).await.unwrap();

        let count = dao
            .count_by_assembly_id(assembly_id, tx.clone())
            .await
            .unwrap();
        assert_eq!(count, 1);

        let found = dao
            .find_by_assembly_id(assembly_id, tx.clone())
            .await
            .unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].assembly_id, assembly_id);
        assert_eq!(found[0].member_id, member_id);

        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_create_duplicate_snapshot_returns_db_error() {
        let pool = setup_db().await;
        let dao = AssemblyMemberSnapshotDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let assembly_id = Uuid::new_v4();
        let member_id = Uuid::new_v4();
        let entity = make_snapshot(assembly_id, member_id);

        let tx = tx_dao.transaction().await.unwrap();
        dao.create(&entity, "test", tx.clone()).await.unwrap();

        // Second insert with identical (assembly_id, member_id) MUST fail
        // due to the composite primary key constraint (Pitfall 5).
        let result = dao.create(&entity, "test", tx.clone()).await;
        assert!(
            matches!(result, Err(DaoError::DatabaseError(_))),
            "expected DatabaseError on PK violation, got: {:?}",
            result
        );

        // The transaction is now poisoned by the failed statement; rollback.
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn test_count_unknown_assembly_returns_zero() {
        let pool = setup_db().await;
        let dao = AssemblyMemberSnapshotDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let unknown = Uuid::new_v4();

        let tx = tx_dao.transaction().await.unwrap();
        let count = dao.count_by_assembly_id(unknown, tx.clone()).await.unwrap();
        assert_eq!(count, 0);

        let found = dao.find_by_assembly_id(unknown, tx.clone()).await.unwrap();
        assert_eq!(found.len(), 0);

        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_create_batch_empty_is_noop() {
        let pool = setup_db().await;
        let dao = AssemblyMemberSnapshotDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let tx = tx_dao.transaction().await.unwrap();
        // Empty slice must not error.
        dao.create_batch(&[], "test", tx.clone()).await.unwrap();
        // And must not have inserted anything.
        let count = dao
            .count_by_assembly_id(Uuid::new_v4(), tx.clone())
            .await
            .unwrap();
        assert_eq!(count, 0);

        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_create_batch_inserts_all() {
        let pool = setup_db().await;
        let dao = AssemblyMemberSnapshotDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let assembly_id = Uuid::new_v4();
        let entities: Vec<AssemblyMemberSnapshotEntity> = (0..3)
            .map(|_| make_snapshot(assembly_id, Uuid::new_v4()))
            .collect();

        let tx = tx_dao.transaction().await.unwrap();
        dao.create_batch(&entities, "test", tx.clone())
            .await
            .unwrap();

        let count = dao
            .count_by_assembly_id(assembly_id, tx.clone())
            .await
            .unwrap();
        assert_eq!(count, 3);

        tx.commit().await.unwrap();
    }
}
