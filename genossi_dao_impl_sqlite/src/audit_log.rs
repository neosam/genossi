use async_trait::async_trait;
use genossi_dao::audit_log::{AuditLogDao, AuditLogEntry};
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
struct AuditLogDb {
    id: Vec<u8>,
    timestamp: String,
    user_id: String,
    process: String,
    transaction_id: Vec<u8>,
    entity_type: String,
    entity_id: Vec<u8>,
    action: String,
    field_name: String,
    old_value: Option<String>,
    new_value: Option<String>,
    prev_hash: String,
    entry_hash: String,
}

impl TryFrom<&AuditLogDb> for AuditLogEntry {
    type Error = DaoError;

    fn try_from(db: &AuditLogDb) -> Result<Self, Self::Error> {
        Ok(AuditLogEntry {
            id: Uuid::from_slice(&db.id)?,
            timestamp: parse_datetime(&db.timestamp)?,
            user_id: Arc::from(db.user_id.as_str()),
            process: Arc::from(db.process.as_str()),
            transaction_id: Uuid::from_slice(&db.transaction_id)?,
            entity_type: Arc::from(db.entity_type.as_str()),
            entity_id: Uuid::from_slice(&db.entity_id)?,
            action: Arc::from(db.action.as_str()),
            field_name: Arc::from(db.field_name.as_str()),
            old_value: db.old_value.as_deref().map(Arc::from),
            new_value: db.new_value.as_deref().map(Arc::from),
            prev_hash: Arc::from(db.prev_hash.as_str()),
            entry_hash: Arc::from(db.entry_hash.as_str()),
        })
    }
}

pub struct AuditLogDaoImpl {
    pub pool: Arc<SqlitePool>,
}

impl AuditLogDaoImpl {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AuditLogDao for AuditLogDaoImpl {
    type Transaction = TransactionImpl;

    async fn create_entries(
        &self,
        entries: &[AuditLogEntry],
        tx: Self::Transaction,
    ) -> Result<(), DaoError> {
        let format = &time::format_description::well_known::Iso8601::DEFAULT;
        for entry in entries {
            let id = entry.id.as_bytes().to_vec();
            let timestamp = entry
                .timestamp
                .assume_utc()
                .format(format)
                .map_err(|e| DaoError::ParseError(Arc::from(e.to_string())))?;
            let transaction_id = entry.transaction_id.as_bytes().to_vec();
            let entity_id = entry.entity_id.as_bytes().to_vec();
            let user_id = entry.user_id.to_string();
            let process = entry.process.to_string();
            let entity_type = entry.entity_type.to_string();
            let action = entry.action.to_string();
            let field_name = entry.field_name.to_string();
            let old_value = entry.old_value.as_deref().map(String::from);
            let new_value = entry.new_value.as_deref().map(String::from);
            let prev_hash = entry.prev_hash.to_string();
            let entry_hash = entry.entry_hash.to_string();

            sqlx::query(
                "INSERT INTO audit_log (id, timestamp, user_id, process, transaction_id, \
                 entity_type, entity_id, action, field_name, old_value, new_value, \
                 prev_hash, entry_hash) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(id)
            .bind(timestamp)
            .bind(user_id)
            .bind(process)
            .bind(transaction_id)
            .bind(entity_type)
            .bind(entity_id)
            .bind(action)
            .bind(field_name)
            .bind(old_value)
            .bind(new_value)
            .bind(prev_hash)
            .bind(entry_hash)
            .execute(tx.tx.lock().await.as_mut())
            .await
            .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;
        }
        Ok(())
    }

    async fn get_latest_hash(
        &self,
        tx: Self::Transaction,
    ) -> Result<Option<String>, DaoError> {
        let result = sqlx::query_scalar::<_, String>(
            "SELECT entry_hash FROM audit_log ORDER BY rowid DESC LIMIT 1",
        )
        .fetch_optional(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(result)
    }

    async fn get_by_entity(
        &self,
        entity_type: &str,
        entity_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Arc<[AuditLogEntry]>, DaoError> {
        let entity_id_bytes = entity_id.as_bytes().to_vec();
        let rows = sqlx::query_as::<_, AuditLogDb>(
            "SELECT id, timestamp, user_id, process, transaction_id, entity_type, entity_id, \
             action, field_name, old_value, new_value, prev_hash, entry_hash \
             FROM audit_log WHERE entity_type = ? AND entity_id = ? ORDER BY rowid ASC",
        )
        .bind(entity_type)
        .bind(entity_id_bytes)
        .fetch_all(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        rows.iter()
            .map(AuditLogEntry::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into())
    }

    async fn get_all_ordered(
        &self,
        tx: Self::Transaction,
    ) -> Result<Arc<[AuditLogEntry]>, DaoError> {
        let rows = sqlx::query_as::<_, AuditLogDb>(
            "SELECT id, timestamp, user_id, process, transaction_id, entity_type, entity_id, \
             action, field_name, old_value, new_value, prev_hash, entry_hash \
             FROM audit_log ORDER BY rowid ASC",
        )
        .fetch_all(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        rows.iter()
            .map(AuditLogEntry::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into())
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
            "CREATE TABLE audit_log (
                id BLOB NOT NULL PRIMARY KEY,
                timestamp TEXT NOT NULL,
                user_id TEXT NOT NULL,
                process TEXT NOT NULL,
                transaction_id BLOB NOT NULL,
                entity_type TEXT NOT NULL,
                entity_id BLOB NOT NULL,
                action TEXT NOT NULL,
                field_name TEXT NOT NULL,
                old_value TEXT,
                new_value TEXT,
                prev_hash TEXT NOT NULL,
                entry_hash TEXT NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .expect("Failed to create table");

        Arc::new(pool)
    }

    fn make_entry(field_name: &str, prev_hash: &str, entry_hash: &str) -> AuditLogEntry {
        let date = time::Date::from_calendar_date(2026, time::Month::April, 15).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        AuditLogEntry {
            id: Uuid::new_v4(),
            timestamp: datetime,
            user_id: Arc::from("testuser"),
            process: Arc::from("test-service"),
            transaction_id: Uuid::new_v4(),
            entity_type: Arc::from("member"),
            entity_id: Uuid::new_v4(),
            action: Arc::from("update"),
            field_name: Arc::from(field_name),
            old_value: Some(Arc::from("old")),
            new_value: Some(Arc::from("new")),
            prev_hash: Arc::from(prev_hash),
            entry_hash: Arc::from(entry_hash),
        }
    }

    #[tokio::test]
    async fn test_create_and_get_all_ordered() {
        let pool = setup_db().await;
        let dao = AuditLogDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let entity_id = Uuid::new_v4();
        let tx_id = Uuid::new_v4();
        let mut entry1 = make_entry("first_name", "", "hash1");
        entry1.entity_id = entity_id;
        entry1.transaction_id = tx_id;
        let mut entry2 = make_entry("last_name", "hash1", "hash2");
        entry2.entity_id = entity_id;
        entry2.transaction_id = tx_id;

        let tx = tx_dao.transaction().await.unwrap();
        dao.create_entries(&[entry1.clone(), entry2.clone()], tx.clone())
            .await
            .unwrap();

        let all = dao.get_all_ordered(tx.clone()).await.unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].field_name.as_ref(), "first_name");
        assert_eq!(all[1].field_name.as_ref(), "last_name");
        assert_eq!(all[0].entry_hash.as_ref(), "hash1");
        assert_eq!(all[1].prev_hash.as_ref(), "hash1");

        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_get_by_entity() {
        let pool = setup_db().await;
        let dao = AuditLogDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let entity_id_1 = Uuid::new_v4();
        let entity_id_2 = Uuid::new_v4();
        let mut entry1 = make_entry("first_name", "", "hash1");
        entry1.entity_id = entity_id_1;
        let mut entry2 = make_entry("email", "hash1", "hash2");
        entry2.entity_id = entity_id_2;
        let mut entry3 = make_entry("last_name", "hash2", "hash3");
        entry3.entity_id = entity_id_1;

        let tx = tx_dao.transaction().await.unwrap();
        dao.create_entries(&[entry1, entry2, entry3], tx.clone())
            .await
            .unwrap();

        let results = dao
            .get_by_entity("member", entity_id_1, tx.clone())
            .await
            .unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].field_name.as_ref(), "first_name");
        assert_eq!(results[1].field_name.as_ref(), "last_name");

        let results2 = dao
            .get_by_entity("member", entity_id_2, tx.clone())
            .await
            .unwrap();
        assert_eq!(results2.len(), 1);

        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_get_latest_hash_empty() {
        let pool = setup_db().await;
        let dao = AuditLogDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let tx = tx_dao.transaction().await.unwrap();
        let result = dao.get_latest_hash(tx.clone()).await.unwrap();
        assert!(result.is_none());

        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_get_latest_hash() {
        let pool = setup_db().await;
        let dao = AuditLogDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let entry1 = make_entry("first_name", "", "aaa");
        let entry2 = make_entry("last_name", "aaa", "bbb");

        let tx = tx_dao.transaction().await.unwrap();
        dao.create_entries(&[entry1, entry2], tx.clone())
            .await
            .unwrap();

        let latest = dao.get_latest_hash(tx.clone()).await.unwrap();
        assert_eq!(latest.as_deref(), Some("bbb"));

        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_create_entry_with_null_values() {
        let pool = setup_db().await;
        let dao = AuditLogDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let mut entry = make_entry("email", "", "hash1");
        entry.old_value = None;
        entry.new_value = Some(Arc::from("new@example.com"));
        entry.action = Arc::from("create");

        let tx = tx_dao.transaction().await.unwrap();
        dao.create_entries(&[entry.clone()], tx.clone())
            .await
            .unwrap();

        let all = dao.get_all_ordered(tx.clone()).await.unwrap();
        assert_eq!(all.len(), 1);
        assert!(all[0].old_value.is_none());
        assert_eq!(all[0].new_value.as_deref(), Some("new@example.com"));

        tx.commit().await.unwrap();
    }
}
