use async_trait::async_trait;
use genossi_dao::repayment_entry::{RepaymentEntryDao, RepaymentEntryEntity, RepaymentEntryStatus};
use genossi_dao::DaoError;
use sqlx::SqlitePool;
use std::sync::Arc;
use uuid::Uuid;

use crate::datetime_utils::{format_dt, parse_datetime};
use crate::TransactionImpl;

#[derive(Debug, sqlx::FromRow)]
struct RepaymentEntryDb {
    id: Vec<u8>,
    member_id: Vec<u8>,
    phase_id: Vec<u8>,
    // SQLite INTEGER is 8 bytes; sqlx surfaces it as i64. We cast to i32 in
    // TryFrom with a guarded conversion (T-08-02-02, reuse of Phase-7 Plan
    // 07-02 T-07-02-05 pattern) so a corrupt out-of-range value surfaces as a
    // controlled ParseError instead of a panic.
    share_count_to_pay_out: i64,
    status: String,
    created: String,
    deleted: Option<String>,
    version: Vec<u8>,
}

impl TryFrom<&RepaymentEntryDb> for RepaymentEntryEntity {
    type Error = DaoError;

    fn try_from(db: &RepaymentEntryDb) -> Result<Self, Self::Error> {
        Ok(RepaymentEntryEntity {
            id: Uuid::from_slice(&db.id)?,
            member_id: Uuid::from_slice(&db.member_id)?,
            phase_id: Uuid::from_slice(&db.phase_id)?,
            share_count_to_pay_out: i32::try_from(db.share_count_to_pay_out).map_err(|e| {
                DaoError::ParseError(Arc::from(format!(
                    "share_count_to_pay_out out of i32 range: {}",
                    e
                )))
            })?,
            status: RepaymentEntryStatus::from_str(&db.status)?,
            created: parse_datetime(&db.created)?,
            deleted: db.deleted.as_ref().map(|d| parse_datetime(d)).transpose()?,
            version: Uuid::from_slice(&db.version)?,
        })
    }
}

pub struct RepaymentEntryDaoImpl {
    pub pool: Arc<SqlitePool>,
}

impl RepaymentEntryDaoImpl {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}


#[async_trait]
impl RepaymentEntryDao for RepaymentEntryDaoImpl {
    type Transaction = TransactionImpl;

    async fn dump_all(
        &self,
        tx: Self::Transaction,
    ) -> Result<Arc<[RepaymentEntryEntity]>, DaoError> {
        // ORDER BY created ASC, id ASC — deterministische Audit-Reihenfolge
        // (Plan-Vorgabe Test 6). created-ASC liefert Eintraege in Anlage-
        // Reihenfolge; id-ASC bricht Ties bei gleicher Sekunde deterministisch.
        let rows = sqlx::query_as::<_, RepaymentEntryDb>(
            "SELECT id, member_id, phase_id, share_count_to_pay_out, status, created, \
             deleted, version FROM repayment_entry \
             ORDER BY created ASC, id ASC",
        )
        .fetch_all(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        rows.iter()
            .map(RepaymentEntryEntity::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into())
    }

    async fn create(
        &self,
        entity: &RepaymentEntryEntity,
        _process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError> {
        let id = entity.id.as_bytes().to_vec();
        let member_id = entity.member_id.as_bytes().to_vec();
        let phase_id = entity.phase_id.as_bytes().to_vec();
        let share_count = entity.share_count_to_pay_out as i64;
        let status = entity.status.as_str().to_string();
        let created = format_dt(&entity.created)?;
        let deleted = entity.deleted.as_ref().map(format_dt).transpose()?;
        let version = entity.version.as_bytes().to_vec();

        sqlx::query(
            "INSERT INTO repayment_entry (id, member_id, phase_id, share_count_to_pay_out, \
             status, created, deleted, version) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(member_id)
        .bind(phase_id)
        .bind(share_count)
        .bind(status)
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
        entity: &RepaymentEntryEntity,
        _process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError> {
        let id = entity.id.as_bytes().to_vec();
        let old_version = entity.version.as_bytes().to_vec();
        let new_version = Uuid::new_v4().as_bytes().to_vec();
        let member_id = entity.member_id.as_bytes().to_vec();
        let phase_id = entity.phase_id.as_bytes().to_vec();
        let share_count = entity.share_count_to_pay_out as i64;
        let status = entity.status.as_str().to_string();
        let deleted = entity.deleted.as_ref().map(format_dt).transpose()?;

        // Pre-condition: row must exist and not be soft-deleted. Without this
        // check, a missing-id and a version mismatch would both surface as
        // ConflictError, which conflates two distinct error semantics. Pattern
        // 1:1 aus repayment_phase.rs Z. 162-172 (Phase-7-Plan-07-02-Lektion
        // D-03 in STATE.md).
        let exists = sqlx::query_scalar::<_, i32>(
            "SELECT COUNT(*) FROM repayment_entry WHERE id = ? AND deleted IS NULL",
        )
        .bind(id.clone())
        .fetch_one(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        if exists == 0 {
            return Err(DaoError::NotFound);
        }

        let rows_affected = sqlx::query(
            "UPDATE repayment_entry SET member_id = ?, phase_id = ?, share_count_to_pay_out = ?, \
             status = ?, deleted = ?, version = ? \
             WHERE id = ? AND version = ? AND deleted IS NULL",
        )
        .bind(member_id)
        .bind(phase_id)
        .bind(share_count)
        .bind(status)
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

    async fn find_by_member_and_phase(
        &self,
        member_id: Uuid,
        phase_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Arc<[RepaymentEntryEntity]>, DaoError> {
        // SQL-Override fuer Skalierung (D-14-08): WHERE-Filter direkt im SQLite
        // statt In-Memory-Filter via dump_all. Column-Liste 1:1 aus dump_all
        // uebernommen (Z. 78-81) damit die RepaymentEntryDb-Row-Mapping
        // konsistent bleibt. ORDER BY created ASC, id ASC liefert
        // deterministische Reihenfolge (Phase-8-Plan-08-02-Lektion).
        //
        // Foundation fuer Phase-16-Sum-Check + Auto-Fill-Skip-Pattern
        // (PITFALLS Kat 1, TRSF-06).
        let member_blob = member_id.as_bytes().to_vec();
        let phase_blob = phase_id.as_bytes().to_vec();
        let rows = sqlx::query_as::<_, RepaymentEntryDb>(
            "SELECT id, member_id, phase_id, share_count_to_pay_out, status, created, \
             deleted, version FROM repayment_entry \
             WHERE member_id = ? AND phase_id = ? AND deleted IS NULL \
             ORDER BY created ASC, id ASC",
        )
        .bind(member_blob)
        .bind(phase_blob)
        .fetch_all(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        rows.iter()
            .map(RepaymentEntryEntity::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TransactionDaoImpl;
    use genossi_dao::{Transaction, TransactionDao};

    /// Bring up an in-memory SQLite pool with the repayment_entry schema
    /// applied. Wir kopieren die Migration-DDL hier inline (analog
    /// repayment_phase.rs::tests::setup_db) — kein `include_str!` auf die
    /// Migration, weil das Pattern für DAO-Unit-Tests in dieser Crate so
    /// etabliert ist (Phase-7-Konvention).
    async fn setup_db() -> Arc<SqlitePool> {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("create in-memory db");

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS repayment_entry (
                id BLOB PRIMARY KEY NOT NULL,
                member_id BLOB NOT NULL,
                phase_id BLOB NOT NULL,
                share_count_to_pay_out INTEGER NOT NULL CHECK(share_count_to_pay_out > 0),
                status TEXT NOT NULL DEFAULT 'Open',
                created TEXT NOT NULL,
                deleted TEXT,
                version BLOB NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .expect("create repayment_entry table");

        Arc::new(pool)
    }

    fn sample_entity() -> RepaymentEntryEntity {
        let date = time::Date::from_calendar_date(2026, time::Month::May, 31).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        RepaymentEntryEntity {
            id: Uuid::new_v4(),
            member_id: Uuid::new_v4(),
            phase_id: Uuid::new_v4(),
            share_count_to_pay_out: 5,
            status: RepaymentEntryStatus::Open,
            created: datetime,
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    #[tokio::test]
    async fn test_create_and_find_repayment_entry() {
        let pool = setup_db().await;
        let dao = RepaymentEntryDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let entity = sample_entity();
        let entity_id = entity.id;

        let tx = tx_dao.transaction().await.unwrap();
        dao.create(&entity, "test", tx.clone()).await.unwrap();

        let found = dao
            .find_by_id(entity_id, tx.clone())
            .await
            .unwrap()
            .expect("entity must be found");
        assert_eq!(found.id, entity.id);
        assert_eq!(found.member_id, entity.member_id);
        assert_eq!(found.phase_id, entity.phase_id);
        assert_eq!(found.share_count_to_pay_out, 5);
        assert_eq!(found.status, RepaymentEntryStatus::Open);
        assert_eq!(found.deleted, None);

        let all = dao.all(tx.clone()).await.unwrap();
        assert_eq!(all.len(), 1);

        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_update_repayment_entry_with_version_mismatch_returns_conflict() {
        let pool = setup_db().await;
        let dao = RepaymentEntryDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let entity = sample_entity();

        let tx = tx_dao.transaction().await.unwrap();
        dao.create(&entity, "test", tx.clone()).await.unwrap();

        // Build an update with a stale version (random UUID, not the persisted one).
        let mut stale = entity.clone();
        stale.version = Uuid::new_v4();
        stale.status = RepaymentEntryStatus::Contacted;

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
    async fn test_update_repayment_entry_unknown_id_returns_not_found() {
        let pool = setup_db().await;
        let dao = RepaymentEntryDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let mut entity = sample_entity();
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
    async fn test_update_repayment_entry_succeeds_then_version_changes() {
        let pool = setup_db().await;
        let dao = RepaymentEntryDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let entity = sample_entity();
        let entity_id = entity.id;
        let original_version = entity.version;

        let tx = tx_dao.transaction().await.unwrap();
        dao.create(&entity, "test", tx.clone()).await.unwrap();

        let mut updated = entity.clone();
        updated.status = RepaymentEntryStatus::Contacted;

        dao.update(&updated, "test", tx.clone()).await.unwrap();

        let after = dao
            .find_by_id(entity_id, tx.clone())
            .await
            .unwrap()
            .expect("entity must still exist");
        assert_eq!(after.status, RepaymentEntryStatus::Contacted);
        assert_ne!(
            after.version, original_version,
            "DAO must bump version on update"
        );

        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_dump_all_returns_sorted_entries() {
        // Plan-Vorgabe: ORDER BY created ASC fuer deterministische Audit-
        // Reihenfolge. Wir legen drei Eintraege mit distinkten created-
        // Timestamps an und verifizieren die Sortierung.
        let pool = setup_db().await;
        let dao = RepaymentEntryDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let base_date = time::Date::from_calendar_date(2026, time::Month::May, 31).unwrap();
        let t_early = time::PrimitiveDateTime::new(base_date, time::Time::MIDNIGHT);
        let t_mid =
            time::PrimitiveDateTime::new(base_date, time::Time::from_hms(12, 0, 0).unwrap());
        let t_late =
            time::PrimitiveDateTime::new(base_date, time::Time::from_hms(23, 59, 59).unwrap());

        let mut e_late = sample_entity();
        e_late.created = t_late;
        let mut e_early = sample_entity();
        e_early.created = t_early;
        let mut e_mid = sample_entity();
        e_mid.created = t_mid;

        let tx = tx_dao.transaction().await.unwrap();
        // Anlage in nicht-sortierter Reihenfolge:
        dao.create(&e_late, "test", tx.clone()).await.unwrap();
        dao.create(&e_early, "test", tx.clone()).await.unwrap();
        dao.create(&e_mid, "test", tx.clone()).await.unwrap();

        let dumped = dao.dump_all(tx.clone()).await.unwrap();
        assert_eq!(dumped.len(), 3);
        // Erwartet: created ASC -> early, mid, late
        assert_eq!(dumped[0].id, e_early.id, "first must be the earliest");
        assert_eq!(dumped[1].id, e_mid.id, "second must be the middle one");
        assert_eq!(dumped[2].id, e_late.id, "third must be the latest");

        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_find_by_phase_id_filters_correctly() {
        let pool = setup_db().await;
        let dao = RepaymentEntryDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let phase_a = Uuid::new_v4();
        let phase_b = Uuid::new_v4();

        let mut e1 = sample_entity();
        e1.phase_id = phase_a;
        let mut e2 = sample_entity();
        e2.phase_id = phase_a;
        let mut e3 = sample_entity();
        e3.phase_id = phase_b;

        let tx = tx_dao.transaction().await.unwrap();
        dao.create(&e1, "test", tx.clone()).await.unwrap();
        dao.create(&e2, "test", tx.clone()).await.unwrap();
        dao.create(&e3, "test", tx.clone()).await.unwrap();

        let found_a = dao.find_by_phase_id(phase_a, tx.clone()).await.unwrap();
        assert_eq!(found_a.len(), 2, "phase_a should have exactly 2 entries");
        assert!(found_a.iter().all(|e| e.phase_id == phase_a));

        let found_b = dao.find_by_phase_id(phase_b, tx.clone()).await.unwrap();
        assert_eq!(found_b.len(), 1, "phase_b should have exactly 1 entry");
        assert_eq!(found_b[0].phase_id, phase_b);

        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_find_by_member_and_phase_returns_empty_when_no_match() {
        // Plan 14-02: Empty result wenn weder (member, phase) noch deren
        // Kombination in der DB existiert. Wir legen einen unrelated Eintrag
        // an, um sicherzustellen, dass die WHERE-Klausel wirklich filtert
        // (nicht versehentlich "alle Eintraege" liefert).
        let pool = setup_db().await;
        let dao = RepaymentEntryDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let target_member = Uuid::new_v4();
        let target_phase = Uuid::new_v4();

        // Insert one unrelated entry mit anderen IDs.
        let mut other = sample_entity();
        other.member_id = Uuid::new_v4();
        other.phase_id = Uuid::new_v4();
        let tx = tx_dao.transaction().await.unwrap();
        dao.create(&other, "test", tx.clone()).await.unwrap();

        let result = dao
            .find_by_member_and_phase(target_member, target_phase, tx.clone())
            .await
            .unwrap();
        assert_eq!(
            result.len(),
            0,
            "no matching (member, phase) -> empty result"
        );

        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_find_by_member_and_phase_filters_correctly() {
        // Plan 14-02: Multi-Entry-Filter ueber Member-Phase-Kreuzprodukt.
        // 4 Eintraege:
        //   e1 (m_A, p_X) -> MATCH
        //   e2 (m_A, p_Y) -> phase differs -> exclude
        //   e3 (m_B, p_X) -> member differs -> exclude
        //   e4 (m_A, p_X) -> MATCH
        // Erwartung: genau 2 Eintraege (e1, e4) in created ASC, id ASC.
        let pool = setup_db().await;
        let dao = RepaymentEntryDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let m_a = Uuid::new_v4();
        let m_b = Uuid::new_v4();
        let p_x = Uuid::new_v4();
        let p_y = Uuid::new_v4();

        let mut e1 = sample_entity();
        e1.member_id = m_a;
        e1.phase_id = p_x;
        let mut e2 = sample_entity();
        e2.member_id = m_a;
        e2.phase_id = p_y;
        let mut e3 = sample_entity();
        e3.member_id = m_b;
        e3.phase_id = p_x;
        let mut e4 = sample_entity();
        e4.member_id = m_a;
        e4.phase_id = p_x;

        let tx = tx_dao.transaction().await.unwrap();
        dao.create(&e1, "test", tx.clone()).await.unwrap();
        dao.create(&e2, "test", tx.clone()).await.unwrap();
        dao.create(&e3, "test", tx.clone()).await.unwrap();
        dao.create(&e4, "test", tx.clone()).await.unwrap();

        let found = dao
            .find_by_member_and_phase(m_a, p_x, tx.clone())
            .await
            .unwrap();
        assert_eq!(found.len(), 2, "exactly 2 entries match (m_a, p_x)");
        for entry in found.iter() {
            assert_eq!(entry.member_id, m_a);
            assert_eq!(entry.phase_id, p_x);
            assert!(entry.deleted.is_none());
        }

        tx.commit().await.unwrap();
    }
}
