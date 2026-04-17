use async_trait::async_trait;
use genossi_dao::audit_timestamp::{AuditTimestampDao, AuditTimestampEntry};
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
struct AuditTimestampDb {
    id: Vec<u8>,
    timestamp: String,
    audit_hash: String,
    audit_entry_count: i64,
    tsr_token: Option<Vec<u8>>,
    webdav_path: Option<String>,
    status: String,
}

impl TryFrom<&AuditTimestampDb> for AuditTimestampEntry {
    type Error = DaoError;

    fn try_from(db: &AuditTimestampDb) -> Result<Self, Self::Error> {
        Ok(AuditTimestampEntry {
            id: Uuid::from_slice(&db.id)?,
            timestamp: parse_datetime(&db.timestamp)?,
            audit_hash: Arc::from(db.audit_hash.as_str()),
            audit_entry_count: db.audit_entry_count,
            tsr_token: db.tsr_token.as_deref().map(Arc::from),
            webdav_path: db.webdav_path.as_deref().map(Arc::from),
            status: Arc::from(db.status.as_str()),
        })
    }
}

pub struct AuditTimestampDaoImpl {
    pub pool: Arc<SqlitePool>,
}

impl AuditTimestampDaoImpl {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AuditTimestampDao for AuditTimestampDaoImpl {
    type Transaction = TransactionImpl;

    async fn create(
        &self,
        entry: &AuditTimestampEntry,
        tx: Self::Transaction,
    ) -> Result<(), DaoError> {
        let format = &time::format_description::well_known::Iso8601::DEFAULT;
        let id = entry.id.as_bytes().to_vec();
        let timestamp = entry
            .timestamp
            .assume_utc()
            .format(format)
            .map_err(|e| DaoError::ParseError(Arc::from(e.to_string())))?;
        let audit_hash = entry.audit_hash.to_string();
        let tsr_token = entry.tsr_token.as_deref().map(|t| t.to_vec());
        let webdav_path = entry.webdav_path.as_deref().map(String::from);
        let status = entry.status.to_string();

        sqlx::query(
            "INSERT INTO audit_timestamp (id, timestamp, audit_hash, audit_entry_count, \
             tsr_token, webdav_path, status) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(timestamp)
        .bind(audit_hash)
        .bind(entry.audit_entry_count)
        .bind(tsr_token)
        .bind(webdav_path)
        .bind(status)
        .execute(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(())
    }

    async fn get_latest(
        &self,
        tx: Self::Transaction,
    ) -> Result<Option<AuditTimestampEntry>, DaoError> {
        let row = sqlx::query_as::<_, AuditTimestampDb>(
            "SELECT id, timestamp, audit_hash, audit_entry_count, tsr_token, webdav_path, status \
             FROM audit_timestamp ORDER BY rowid DESC LIMIT 1",
        )
        .fetch_optional(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        match row {
            Some(ref db) => Ok(Some(AuditTimestampEntry::try_from(db)?)),
            None => Ok(None),
        }
    }

    async fn get_all(&self, tx: Self::Transaction) -> Result<Arc<[AuditTimestampEntry]>, DaoError> {
        let rows = sqlx::query_as::<_, AuditTimestampDb>(
            "SELECT id, timestamp, audit_hash, audit_entry_count, tsr_token, webdav_path, status \
             FROM audit_timestamp ORDER BY rowid DESC",
        )
        .fetch_all(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        rows.iter()
            .map(AuditTimestampEntry::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into())
    }

    async fn get_by_id(
        &self,
        id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Option<AuditTimestampEntry>, DaoError> {
        let id_bytes = id.as_bytes().to_vec();
        let row = sqlx::query_as::<_, AuditTimestampDb>(
            "SELECT id, timestamp, audit_hash, audit_entry_count, tsr_token, webdav_path, status \
             FROM audit_timestamp WHERE id = ?",
        )
        .bind(id_bytes)
        .fetch_optional(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        match row {
            Some(ref db) => Ok(Some(AuditTimestampEntry::try_from(db)?)),
            None => Ok(None),
        }
    }

    async fn get_pending_upload(
        &self,
        tx: Self::Transaction,
    ) -> Result<Arc<[AuditTimestampEntry]>, DaoError> {
        let rows = sqlx::query_as::<_, AuditTimestampDb>(
            "SELECT id, timestamp, audit_hash, audit_entry_count, tsr_token, webdav_path, status \
             FROM audit_timestamp WHERE webdav_path IS NULL AND status = 'success' ORDER BY rowid ASC",
        )
        .fetch_all(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        rows.iter()
            .map(AuditTimestampEntry::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into())
    }

    async fn update_webdav_path(
        &self,
        id: Uuid,
        path: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError> {
        let id_bytes = id.as_bytes().to_vec();
        sqlx::query("UPDATE audit_timestamp SET webdav_path = ? WHERE id = ?")
            .bind(path)
            .bind(id_bytes)
            .execute(tx.tx.lock().await.as_mut())
            .await
            .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TransactionDaoImpl;
    use genossi_dao::{Transaction, TransactionDao};

    async fn setup_db() -> Arc<SqlitePool> {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create in-memory database");

        sqlx::query(
            "CREATE TABLE audit_timestamp (
                id BLOB NOT NULL PRIMARY KEY,
                timestamp TEXT NOT NULL,
                audit_hash TEXT NOT NULL,
                audit_entry_count INTEGER NOT NULL,
                tsr_token BLOB,
                webdav_path TEXT,
                status TEXT NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .expect("Failed to create table");

        Arc::new(pool)
    }

    fn make_entry(audit_hash: &str, status: &str) -> AuditTimestampEntry {
        let date = time::Date::from_calendar_date(2026, time::Month::April, 15).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        AuditTimestampEntry {
            id: Uuid::new_v4(),
            timestamp: datetime,
            audit_hash: Arc::from(audit_hash),
            audit_entry_count: 42,
            tsr_token: Some(Arc::from(vec![1, 2, 3, 4].as_slice())),
            webdav_path: Some(Arc::from("audit-timestamps/test.tsr")),
            status: Arc::from(status),
        }
    }

    #[tokio::test]
    async fn test_create_and_get_by_id() {
        let pool = setup_db().await;
        let dao = AuditTimestampDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let entry = make_entry("abc123", "success");
        let tx = tx_dao.transaction().await.unwrap();
        dao.create(&entry, tx.clone()).await.unwrap();

        let result = dao.get_by_id(entry.id, tx.clone()).await.unwrap();
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.id, entry.id);
        assert_eq!(result.audit_hash.as_ref(), "abc123");
        assert_eq!(result.audit_entry_count, 42);
        assert_eq!(result.status.as_ref(), "success");
        assert!(result.tsr_token.is_some());
        assert_eq!(
            result.webdav_path.as_deref(),
            Some("audit-timestamps/test.tsr")
        );

        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_get_latest() {
        let pool = setup_db().await;
        let dao = AuditTimestampDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let tx = tx_dao.transaction().await.unwrap();

        let latest = dao.get_latest(tx.clone()).await.unwrap();
        assert!(latest.is_none());

        let entry1 = make_entry("hash1", "success");
        dao.create(&entry1, tx.clone()).await.unwrap();

        let entry2 = make_entry("hash2", "success");
        dao.create(&entry2, tx.clone()).await.unwrap();

        let latest = dao.get_latest(tx.clone()).await.unwrap();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().audit_hash.as_ref(), "hash2");

        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_get_all() {
        let pool = setup_db().await;
        let dao = AuditTimestampDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let tx = tx_dao.transaction().await.unwrap();

        let entry1 = make_entry("hash1", "success");
        dao.create(&entry1, tx.clone()).await.unwrap();
        let entry2 = make_entry("hash2", "tsa_failed");
        dao.create(&entry2, tx.clone()).await.unwrap();

        let all = dao.get_all(tx.clone()).await.unwrap();
        assert_eq!(all.len(), 2);
        // Ordered by rowid DESC, so newest first
        assert_eq!(all[0].audit_hash.as_ref(), "hash2");
        assert_eq!(all[1].audit_hash.as_ref(), "hash1");

        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_create_with_null_token() {
        let pool = setup_db().await;
        let dao = AuditTimestampDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let mut entry = make_entry("hash1", "tsa_failed");
        entry.tsr_token = None;
        entry.webdav_path = None;

        let tx = tx_dao.transaction().await.unwrap();
        dao.create(&entry, tx.clone()).await.unwrap();

        let result = dao.get_by_id(entry.id, tx.clone()).await.unwrap().unwrap();
        assert!(result.tsr_token.is_none());
        assert!(result.webdav_path.is_none());
        assert_eq!(result.status.as_ref(), "tsa_failed");

        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_get_by_id_not_found() {
        let pool = setup_db().await;
        let dao = AuditTimestampDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let tx = tx_dao.transaction().await.unwrap();
        let result = dao.get_by_id(Uuid::new_v4(), tx.clone()).await.unwrap();
        assert!(result.is_none());

        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_get_pending_upload() {
        let pool = setup_db().await;
        let dao = AuditTimestampDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let tx = tx_dao.transaction().await.unwrap();

        // Entry with webdav_path set (already uploaded)
        let entry1 = make_entry("hash1", "success");
        dao.create(&entry1, tx.clone()).await.unwrap();

        // Entry without webdav_path (pending)
        let mut entry2 = make_entry("hash2", "success");
        entry2.webdav_path = None;
        dao.create(&entry2, tx.clone()).await.unwrap();

        // Failed entry without webdav_path (should NOT be included)
        let mut entry3 = make_entry("hash3", "tsa_failed");
        entry3.tsr_token = None;
        entry3.webdav_path = None;
        dao.create(&entry3, tx.clone()).await.unwrap();

        let pending = dao.get_pending_upload(tx.clone()).await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].audit_hash.as_ref(), "hash2");

        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_update_webdav_path() {
        let pool = setup_db().await;
        let dao = AuditTimestampDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let mut entry = make_entry("hash1", "success");
        entry.webdav_path = None;

        let tx = tx_dao.transaction().await.unwrap();
        dao.create(&entry, tx.clone()).await.unwrap();

        // Verify initially no webdav_path
        let result = dao.get_by_id(entry.id, tx.clone()).await.unwrap().unwrap();
        assert!(result.webdav_path.is_none());

        // Update
        dao.update_webdav_path(entry.id, "audit-timestamps/test.tsr", tx.clone())
            .await
            .unwrap();

        // Verify updated
        let result = dao.get_by_id(entry.id, tx.clone()).await.unwrap().unwrap();
        assert_eq!(
            result.webdav_path.as_deref(),
            Some("audit-timestamps/test.tsr")
        );

        // Should no longer be pending
        let pending = dao.get_pending_upload(tx.clone()).await.unwrap();
        assert!(pending.is_empty());

        tx.commit().await.unwrap();
    }
}
