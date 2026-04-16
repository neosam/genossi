use async_trait::async_trait;
use genossi_dao::audit_log::{AuditLogDao, AuditLogEntry, AuditQueryFilter};
use genossi_dao::DaoError;
use sqlx::{QueryBuilder, Sqlite, SqlitePool};
use std::sync::Arc;
use time::PrimitiveDateTime;
use uuid::Uuid;

use crate::TransactionImpl;

/// Append `WHERE` clauses for the given filter to a `QueryBuilder`. The same
/// builder is used by `query` and `count` so the filter semantics stay aligned.
fn push_filter<'a>(builder: &mut QueryBuilder<'a, Sqlite>, filter: &'a AuditQueryFilter) {
    let mut first = true;
    let mut push_where = |builder: &mut QueryBuilder<'a, Sqlite>| {
        if first {
            builder.push(" WHERE ");
            first = false;
        } else {
            builder.push(" AND ");
        }
    };

    if let Some(ref entity_type) = filter.entity_type {
        push_where(builder);
        builder.push("entity_type = ").push_bind(entity_type.clone());
    }
    if let Some(entity_id) = filter.entity_id {
        push_where(builder);
        builder
            .push("entity_id = ")
            .push_bind(entity_id.as_bytes().to_vec());
    }
    if let Some(ref user_id) = filter.user_id {
        push_where(builder);
        builder.push("user_id = ").push_bind(user_id.clone());
    }
    if let Some(ref action) = filter.action {
        push_where(builder);
        builder.push("action = ").push_bind(action.clone());
    }
    if let Some(ref from) = filter.from {
        push_where(builder);
        builder.push("timestamp >= ").push_bind(from.clone());
    }
    if let Some(ref to) = filter.to {
        push_where(builder);
        builder.push("timestamp <= ").push_bind(to.clone());
    }
}

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

    async fn query(
        &self,
        filter: AuditQueryFilter,
        limit: i64,
        offset: i64,
        tx: Self::Transaction,
    ) -> Result<Arc<[AuditLogEntry]>, DaoError> {
        let mut builder: QueryBuilder<Sqlite> = QueryBuilder::new(
            "SELECT id, timestamp, user_id, process, transaction_id, entity_type, entity_id, \
             action, field_name, old_value, new_value, prev_hash, entry_hash \
             FROM audit_log",
        );
        push_filter(&mut builder, &filter);
        // `id` is a stable tiebreaker when timestamps collide so paging produces no
        // duplicates and no skips. Sort direction matches the timestamp DESC order.
        builder.push(" ORDER BY timestamp DESC, id DESC LIMIT ");
        builder.push_bind(limit);
        builder.push(" OFFSET ");
        builder.push_bind(offset);

        let rows = builder
            .build_query_as::<AuditLogDb>()
            .fetch_all(tx.tx.lock().await.as_mut())
            .await
            .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        rows.iter()
            .map(AuditLogEntry::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into())
    }

    async fn count(
        &self,
        filter: AuditQueryFilter,
        tx: Self::Transaction,
    ) -> Result<i64, DaoError> {
        let mut builder: QueryBuilder<Sqlite> = QueryBuilder::new("SELECT COUNT(*) FROM audit_log");
        push_filter(&mut builder, &filter);

        let total: i64 = builder
            .build_query_scalar()
            .fetch_one(tx.tx.lock().await.as_mut())
            .await
            .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(total)
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

    fn make_entry_with(
        field_name: &str,
        entity_type: &str,
        user_id: &str,
        action: &str,
        timestamp: PrimitiveDateTime,
    ) -> AuditLogEntry {
        AuditLogEntry {
            id: Uuid::new_v4(),
            timestamp,
            user_id: Arc::from(user_id),
            process: Arc::from("test-service"),
            transaction_id: Uuid::new_v4(),
            entity_type: Arc::from(entity_type),
            entity_id: Uuid::new_v4(),
            action: Arc::from(action),
            field_name: Arc::from(field_name),
            old_value: Some(Arc::from("old")),
            new_value: Some(Arc::from("new")),
            prev_hash: Arc::from(""),
            entry_hash: Arc::from("hash"),
        }
    }

    fn ts(year: i32, month: time::Month, day: u8, hour: u8, minute: u8, second: u8) -> PrimitiveDateTime {
        let date = time::Date::from_calendar_date(year, month, day).unwrap();
        let time_v = time::Time::from_hms(hour, minute, second).unwrap();
        PrimitiveDateTime::new(date, time_v)
    }

    async fn seed_diverse(
        dao: &AuditLogDaoImpl,
        tx: TransactionImpl,
    ) -> Vec<AuditLogEntry> {
        let entries = vec![
            make_entry_with(
                "first_name",
                "member",
                "alice",
                "update",
                ts(2026, time::Month::January, 1, 8, 0, 0),
            ),
            make_entry_with(
                "email",
                "member",
                "bob",
                "update",
                ts(2026, time::Month::February, 1, 9, 0, 0),
            ),
            make_entry_with(
                "name",
                "application",
                "alice",
                "create",
                ts(2026, time::Month::March, 1, 10, 0, 0),
            ),
            make_entry_with(
                "type",
                "member_action",
                "carol",
                "delete",
                ts(2026, time::Month::April, 1, 11, 0, 0),
            ),
        ];
        dao.create_entries(&entries, tx).await.unwrap();
        entries
    }

    #[tokio::test]
    async fn test_query_no_filter() {
        let pool = setup_db().await;
        let dao = AuditLogDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);
        let tx = tx_dao.transaction().await.unwrap();
        seed_diverse(&dao, tx.clone()).await;

        let result = dao
            .query(AuditQueryFilter::default(), 10, 0, tx.clone())
            .await
            .unwrap();
        assert_eq!(result.len(), 4);
        // Most recent first (April > March > February > January)
        assert_eq!(result[0].field_name.as_ref(), "type");
        assert_eq!(result[1].field_name.as_ref(), "name");
        assert_eq!(result[2].field_name.as_ref(), "email");
        assert_eq!(result[3].field_name.as_ref(), "first_name");

        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_query_filter_by_entity_type() {
        let pool = setup_db().await;
        let dao = AuditLogDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);
        let tx = tx_dao.transaction().await.unwrap();
        seed_diverse(&dao, tx.clone()).await;

        let filter = AuditQueryFilter {
            entity_type: Some("member".into()),
            ..Default::default()
        };
        let result = dao.query(filter.clone(), 10, 0, tx.clone()).await.unwrap();
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|e| e.entity_type.as_ref() == "member"));

        let total = dao.count(filter, tx.clone()).await.unwrap();
        assert_eq!(total, 2);

        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_query_filter_by_user() {
        let pool = setup_db().await;
        let dao = AuditLogDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);
        let tx = tx_dao.transaction().await.unwrap();
        seed_diverse(&dao, tx.clone()).await;

        let filter = AuditQueryFilter {
            user_id: Some("alice".into()),
            ..Default::default()
        };
        let result = dao.query(filter.clone(), 10, 0, tx.clone()).await.unwrap();
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|e| e.user_id.as_ref() == "alice"));

        let total = dao.count(filter, tx.clone()).await.unwrap();
        assert_eq!(total, 2);

        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_query_filter_by_action() {
        let pool = setup_db().await;
        let dao = AuditLogDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);
        let tx = tx_dao.transaction().await.unwrap();
        seed_diverse(&dao, tx.clone()).await;

        let filter = AuditQueryFilter {
            action: Some("create".into()),
            ..Default::default()
        };
        let result = dao.query(filter.clone(), 10, 0, tx.clone()).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].field_name.as_ref(), "name");

        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_query_filter_by_time_range() {
        let pool = setup_db().await;
        let dao = AuditLogDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);
        let tx = tx_dao.transaction().await.unwrap();
        seed_diverse(&dao, tx.clone()).await;

        // ISO8601 strings — match what create_entries writes (with offset).
        // The DB column stores like "2026-02-01T09:00:00.0Z"; lexicographic
        // comparison with "2026-02-01" / "2026-03-31" works because of ISO format.
        let filter = AuditQueryFilter {
            from: Some("2026-02-01".into()),
            to: Some("2026-03-31".into()),
            ..Default::default()
        };
        let result = dao.query(filter.clone(), 10, 0, tx.clone()).await.unwrap();
        assert_eq!(result.len(), 2);

        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_query_combined_filters() {
        let pool = setup_db().await;
        let dao = AuditLogDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);
        let tx = tx_dao.transaction().await.unwrap();
        seed_diverse(&dao, tx.clone()).await;

        let filter = AuditQueryFilter {
            entity_type: Some("member".into()),
            user_id: Some("alice".into()),
            ..Default::default()
        };
        let result = dao.query(filter.clone(), 10, 0, tx.clone()).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].field_name.as_ref(), "first_name");

        let total = dao.count(filter, tx.clone()).await.unwrap();
        assert_eq!(total, 1);

        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_query_pagination_edges() {
        let pool = setup_db().await;
        let dao = AuditLogDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);
        let tx = tx_dao.transaction().await.unwrap();
        seed_diverse(&dao, tx.clone()).await;

        // Page 0, size 2: most recent two
        let p0 = dao
            .query(AuditQueryFilter::default(), 2, 0, tx.clone())
            .await
            .unwrap();
        assert_eq!(p0.len(), 2);
        assert_eq!(p0[0].field_name.as_ref(), "type");
        assert_eq!(p0[1].field_name.as_ref(), "name");

        // Page 1, size 2: next two
        let p1 = dao
            .query(AuditQueryFilter::default(), 2, 2, tx.clone())
            .await
            .unwrap();
        assert_eq!(p1.len(), 2);
        assert_eq!(p1[0].field_name.as_ref(), "email");
        assert_eq!(p1[1].field_name.as_ref(), "first_name");

        // Page beyond total
        let p99 = dao
            .query(AuditQueryFilter::default(), 2, 100, tx.clone())
            .await
            .unwrap();
        assert_eq!(p99.len(), 0);

        // Total reflects unfiltered set
        let total = dao
            .count(AuditQueryFilter::default(), tx.clone())
            .await
            .unwrap();
        assert_eq!(total, 4);

        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_query_empty_db() {
        let pool = setup_db().await;
        let dao = AuditLogDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);
        let tx = tx_dao.transaction().await.unwrap();

        let result = dao
            .query(AuditQueryFilter::default(), 10, 0, tx.clone())
            .await
            .unwrap();
        assert_eq!(result.len(), 0);
        let total = dao
            .count(AuditQueryFilter::default(), tx.clone())
            .await
            .unwrap();
        assert_eq!(total, 0);

        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_query_stable_ordering_with_duplicate_timestamps() {
        let pool = setup_db().await;
        let dao = AuditLogDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);
        let tx = tx_dao.transaction().await.unwrap();

        let same_ts = ts(2026, time::Month::April, 16, 12, 0, 0);
        let mut entries: Vec<AuditLogEntry> = (0..5)
            .map(|i| {
                let field = format!("field_{i}");
                make_entry_with(&field, "member", "alice", "update", same_ts)
            })
            .collect();
        // Force deterministic ids so we can assert ordering by id DESC.
        entries[0].id = Uuid::from_u128(0x0000_0000_0000_0000_0000_0000_0000_0001);
        entries[1].id = Uuid::from_u128(0x0000_0000_0000_0000_0000_0000_0000_0002);
        entries[2].id = Uuid::from_u128(0x0000_0000_0000_0000_0000_0000_0000_0003);
        entries[3].id = Uuid::from_u128(0x0000_0000_0000_0000_0000_0000_0000_0004);
        entries[4].id = Uuid::from_u128(0x0000_0000_0000_0000_0000_0000_0000_0005);

        dao.create_entries(&entries, tx.clone()).await.unwrap();

        // Page 0 size 2 then page 1 size 2 should produce 4 distinct entries
        // and no overlap, all sharing the same timestamp.
        let p0 = dao
            .query(AuditQueryFilter::default(), 2, 0, tx.clone())
            .await
            .unwrap();
        let p1 = dao
            .query(AuditQueryFilter::default(), 2, 2, tx.clone())
            .await
            .unwrap();

        let combined: Vec<Uuid> = p0.iter().chain(p1.iter()).map(|e| e.id).collect();
        // All four are distinct
        let unique: std::collections::HashSet<_> = combined.iter().collect();
        assert_eq!(unique.len(), 4);

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
