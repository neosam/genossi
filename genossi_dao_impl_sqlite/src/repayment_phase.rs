use async_trait::async_trait;
use genossi_dao::repayment_phase::{RepaymentPhaseDao, RepaymentPhaseEntity, RepaymentPhaseStatus};
use genossi_dao::DaoError;
use sqlx::SqlitePool;
use std::sync::Arc;
use uuid::Uuid;

use crate::datetime_utils::{format_dt, parse_datetime};
use crate::TransactionImpl;

#[derive(Debug, sqlx::FromRow)]
struct RepaymentPhaseDb {
    id: Vec<u8>,
    // SQLite INTEGER is 8 bytes; sqlx surfaces it as i64. We cast to i32 in
    // TryFrom with a guarded conversion (T-07-02-05) so a corrupt out-of-range
    // value surfaces as a controlled ParseError instead of a panic.
    fiscal_year: i64,
    share_value: i64,
    status: String,
    opened_at: Option<String>,
    closed_at: Option<String>,
    created: String,
    deleted: Option<String>,
    version: Vec<u8>,
}

impl TryFrom<&RepaymentPhaseDb> for RepaymentPhaseEntity {
    type Error = DaoError;

    fn try_from(db: &RepaymentPhaseDb) -> Result<Self, Self::Error> {
        Ok(RepaymentPhaseEntity {
            id: Uuid::from_slice(&db.id)?,
            fiscal_year: i32::try_from(db.fiscal_year).map_err(|e| {
                DaoError::ParseError(Arc::from(format!("fiscal_year out of i32 range: {}", e)))
            })?,
            share_value: db.share_value,
            status: RepaymentPhaseStatus::from_str(&db.status)?,
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

pub struct RepaymentPhaseDaoImpl {
    pub pool: Arc<SqlitePool>,
}

impl RepaymentPhaseDaoImpl {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}


#[async_trait]
impl RepaymentPhaseDao for RepaymentPhaseDaoImpl {
    type Transaction = TransactionImpl;

    async fn dump_all(
        &self,
        tx: Self::Transaction,
    ) -> Result<Arc<[RepaymentPhaseEntity]>, DaoError> {
        // ORDER BY fiscal_year DESC, created DESC — Phase 7 sortiert anders
        // als Assembly (D-08 / CONTEXT.md <specifics>): Frontend zeigt die
        // jeweils aktuellste Phase zuerst, mehrere Phasen pro Geschäftsjahr
        // werden nach Anlage-Zeitpunkt weiter sortiert.
        let rows = sqlx::query_as::<_, RepaymentPhaseDb>(
            "SELECT id, fiscal_year, share_value, status, opened_at, closed_at, created, \
             deleted, version FROM repayment_phase \
             ORDER BY fiscal_year DESC, created DESC",
        )
        .fetch_all(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        rows.iter()
            .map(RepaymentPhaseEntity::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into())
    }

    async fn create(
        &self,
        entity: &RepaymentPhaseEntity,
        _process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError> {
        let id = entity.id.as_bytes().to_vec();
        let version = entity.version.as_bytes().to_vec();
        let fiscal_year = entity.fiscal_year as i64;
        let share_value = entity.share_value;
        let status = entity.status.as_str().to_string();
        let opened_at = entity.opened_at.as_ref().map(format_dt).transpose()?;
        let closed_at = entity.closed_at.as_ref().map(format_dt).transpose()?;
        let created = format_dt(&entity.created)?;
        let deleted = entity.deleted.as_ref().map(format_dt).transpose()?;

        sqlx::query(
            "INSERT INTO repayment_phase (id, fiscal_year, share_value, status, opened_at, \
             closed_at, created, deleted, version) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(fiscal_year)
        .bind(share_value)
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
        entity: &RepaymentPhaseEntity,
        _process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError> {
        let id = entity.id.as_bytes().to_vec();
        let old_version = entity.version.as_bytes().to_vec();
        let new_version = Uuid::new_v4().as_bytes().to_vec();
        let fiscal_year = entity.fiscal_year as i64;
        let share_value = entity.share_value;
        let status = entity.status.as_str().to_string();
        let opened_at = entity.opened_at.as_ref().map(format_dt).transpose()?;
        let closed_at = entity.closed_at.as_ref().map(format_dt).transpose()?;
        let deleted = entity.deleted.as_ref().map(format_dt).transpose()?;

        // Pre-condition: row must exist and not be soft-deleted. Without this
        // check, a missing-id and a version mismatch would both surface as
        // ConflictError, which conflates two distinct error semantics. Pattern
        // 1:1 aus assembly.rs Z. 168-178.
        let exists = sqlx::query_scalar::<_, i32>(
            "SELECT COUNT(*) FROM repayment_phase WHERE id = ? AND deleted IS NULL",
        )
        .bind(id.clone())
        .fetch_one(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        if exists == 0 {
            return Err(DaoError::NotFound);
        }

        let rows_affected = sqlx::query(
            "UPDATE repayment_phase SET fiscal_year = ?, share_value = ?, status = ?, \
             opened_at = ?, closed_at = ?, deleted = ?, version = ? \
             WHERE id = ? AND version = ? AND deleted IS NULL",
        )
        .bind(fiscal_year)
        .bind(share_value)
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

    /// Bring up an in-memory SQLite pool with the repayment_phase schema
    /// applied. Wir kopieren die Migration-DDL hier inline (analog zu
    /// assembly.rs::tests::setup_db) — kein `include_str!` auf die Migration,
    /// weil das Pattern für DAO-Unit-Tests in dieser Crate so etabliert ist.
    async fn setup_db() -> Arc<SqlitePool> {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("create in-memory db");

        sqlx::query(
            "CREATE TABLE repayment_phase (
                id BLOB PRIMARY KEY NOT NULL,
                fiscal_year INTEGER NOT NULL,
                share_value INTEGER NOT NULL,
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
        .expect("create repayment_phase table");

        Arc::new(pool)
    }

    fn make_entity(
        fiscal_year: i32,
        share_value: i64,
        status: RepaymentPhaseStatus,
    ) -> RepaymentPhaseEntity {
        let date = time::Date::from_calendar_date(2026, time::Month::May, 29).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        RepaymentPhaseEntity {
            id: Uuid::new_v4(),
            fiscal_year,
            share_value,
            status,
            opened_at: None,
            closed_at: None,
            created: datetime,
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    #[tokio::test]
    async fn test_create_and_find_repayment_phase() {
        let pool = setup_db().await;
        let dao = RepaymentPhaseDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let entity = make_entity(2026, 12000, RepaymentPhaseStatus::Preparation);
        let entity_id = entity.id;

        let tx = tx_dao.transaction().await.unwrap();
        dao.create(&entity, "test", tx.clone()).await.unwrap();

        let found = dao.find_by_id(entity_id, tx.clone()).await.unwrap();
        let found = found.expect("entity must be found");
        assert_eq!(found.id, entity_id);
        assert_eq!(found.fiscal_year, 2026);
        assert_eq!(found.share_value, 12000);
        assert_eq!(found.status, RepaymentPhaseStatus::Preparation);
        assert_eq!(found.opened_at, None);
        assert_eq!(found.closed_at, None);

        let all = dao.all(tx.clone()).await.unwrap();
        assert_eq!(all.len(), 1);

        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_update_repayment_phase_with_version_mismatch_returns_conflict() {
        let pool = setup_db().await;
        let dao = RepaymentPhaseDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let entity = make_entity(2026, 12000, RepaymentPhaseStatus::Preparation);

        let tx = tx_dao.transaction().await.unwrap();
        dao.create(&entity, "test", tx.clone()).await.unwrap();

        // Build an update with a stale version (random UUID, not the persisted one).
        let mut stale = entity.clone();
        stale.version = Uuid::new_v4();
        stale.status = RepaymentPhaseStatus::Open;

        let result = dao.update(&stale, "test", tx.clone()).await;
        let err = match result {
            Err(DaoError::ConflictError(msg)) => msg,
            other => panic!("expected ConflictError, got: {:?}", other),
        };
        assert!(
            err.contains("Version mismatch"),
            "expected message to contain 'Version mismatch', got: {}",
            err
        );

        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_update_repayment_phase_unknown_id_returns_not_found() {
        let pool = setup_db().await;
        let dao = RepaymentPhaseDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let mut entity = make_entity(2026, 12000, RepaymentPhaseStatus::Preparation);
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
    async fn test_update_repayment_phase_succeeds_then_version_changes() {
        let pool = setup_db().await;
        let dao = RepaymentPhaseDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let entity = make_entity(2026, 12000, RepaymentPhaseStatus::Preparation);
        let entity_id = entity.id;

        let tx = tx_dao.transaction().await.unwrap();
        dao.create(&entity, "test", tx.clone()).await.unwrap();

        let mut update = entity.clone();
        update.status = RepaymentPhaseStatus::Open;
        let opened_at = update.created;
        update.opened_at = Some(opened_at);

        dao.update(&update, "test", tx.clone()).await.unwrap();

        let after = dao
            .find_by_id(entity_id, tx.clone())
            .await
            .unwrap()
            .expect("entity must still exist");
        assert_eq!(after.status, RepaymentPhaseStatus::Open);
        assert_eq!(after.opened_at, Some(opened_at));
        assert_ne!(
            after.version, entity.version,
            "version must rotate on update"
        );

        tx.commit().await.unwrap();
    }
}
