use async_trait::async_trait;
use genossi_dao::application_document::{ApplicationDocumentDao, ApplicationDocumentEntity};
use genossi_dao::DaoError;
use sqlx::SqlitePool;
use std::sync::Arc;
use uuid::Uuid;

use crate::datetime_utils::parse_datetime;
use crate::TransactionImpl;

/// Raw DB row mirror used only for `sqlx::query_as` deserialization.
/// BLOBs come back as `Vec<u8>`, datetimes as ISO8601 `String` — the
/// same convention used across the sqlite DAO layer (`member_document`,
/// `member`, `application`, etc.).
#[derive(Debug, sqlx::FromRow)]
struct ApplicationDocumentDb {
    id: Vec<u8>,
    application_id: Vec<u8>,
    file_name: String,
    mime_type: String,
    relative_path: String,
    size: i64,
    created: String,
    deleted: Option<String>,
    version: Vec<u8>,
}

impl TryFrom<&ApplicationDocumentDb> for ApplicationDocumentEntity {
    type Error = DaoError;

    fn try_from(db: &ApplicationDocumentDb) -> Result<Self, Self::Error> {
        Ok(ApplicationDocumentEntity {
            id: Uuid::from_slice(&db.id)?,
            application_id: Uuid::from_slice(&db.application_id)?,
            file_name: Arc::from(db.file_name.as_str()),
            mime_type: Arc::from(db.mime_type.as_str()),
            relative_path: Arc::from(db.relative_path.as_str()),
            size: db.size,
            created: parse_datetime(&db.created)?,
            deleted: db.deleted.as_ref().map(|d| parse_datetime(d)).transpose()?,
            version: Uuid::from_slice(&db.version)?,
        })
    }
}

pub struct ApplicationDocumentDaoImpl {
    pub pool: Arc<SqlitePool>,
}

impl ApplicationDocumentDaoImpl {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ApplicationDocumentDao for ApplicationDocumentDaoImpl {
    type Transaction = TransactionImpl;

    async fn dump_all(
        &self,
        tx: Self::Transaction,
    ) -> Result<Arc<[ApplicationDocumentEntity]>, DaoError> {
        let rows = sqlx::query_as::<_, ApplicationDocumentDb>(
            "SELECT id, application_id, file_name, mime_type, relative_path, size, \
             created, deleted, version \
             FROM application_documents ORDER BY created",
        )
        .fetch_all(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        rows.iter()
            .map(ApplicationDocumentEntity::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into())
    }

    async fn create(
        &self,
        entity: &ApplicationDocumentEntity,
        _process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError> {
        let id = entity.id.as_bytes().to_vec();
        let application_id = entity.application_id.as_bytes().to_vec();
        let version = entity.version.as_bytes().to_vec();
        let format = &time::format_description::well_known::Iso8601::DEFAULT;
        let created = entity
            .created
            .assume_utc()
            .format(format)
            .map_err(|e| DaoError::ParseError(Arc::from(e.to_string())))?;
        let file_name = entity.file_name.to_string();
        let mime_type = entity.mime_type.to_string();
        let relative_path = entity.relative_path.to_string();
        let size = entity.size;

        sqlx::query(
            "INSERT INTO application_documents (id, application_id, file_name, mime_type, \
             relative_path, size, created, version) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(application_id)
        .bind(file_name)
        .bind(mime_type)
        .bind(relative_path)
        .bind(size)
        .bind(created)
        .bind(version)
        .execute(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(())
    }

    async fn update(
        &self,
        entity: &ApplicationDocumentEntity,
        _process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError> {
        // Contract mirrors MemberDocumentDaoImpl::update:
        //   - entity.version = OLD version (used in WHERE for optimistic lock)
        //   - a fresh v4 is generated here and written as the NEW version
        //   - rows_affected == 0 → ConflictError (version mismatch)
        let id = entity.id.as_bytes().to_vec();
        let old_version = entity.version.as_bytes().to_vec();
        let new_version = Uuid::new_v4().as_bytes().to_vec();
        let file_name = entity.file_name.to_string();
        let mime_type = entity.mime_type.to_string();
        let relative_path = entity.relative_path.to_string();
        let size = entity.size;

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
            "UPDATE application_documents SET file_name = ?, mime_type = ?, \
             relative_path = ?, size = ?, deleted = ?, version = ? \
             WHERE id = ? AND version = ?",
        )
        .bind(file_name)
        .bind(mime_type)
        .bind(relative_path)
        .bind(size)
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

    /// In-memory SQLite pool with the application_documents schema applied
    /// exactly as the migration file writes it (partial unique index +
    /// deleted-index included). We embed the migration directly rather than
    /// re-declaring the DDL, so a divergence between test and prod is
    /// caught by test compilation.
    async fn setup_db() -> Arc<SqlitePool> {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("create in-memory db");

        // The application_documents FK references `application(id)` — we need
        // the parent table to exist so INSERT does not fail with
        // "no such table: main.application". SQLite defers the check to
        // statement execution even when foreign_keys pragma is off.
        // We stub a minimal parent table (only `id BLOB PRIMARY KEY`) — the
        // real column set is irrelevant for DAO tests scoped to
        // application_documents.
        sqlx::query("CREATE TABLE application (id BLOB PRIMARY KEY NOT NULL)")
            .execute(&pool)
            .await
            .expect("create application stub table");

        let migration = include_str!(
            "../../migrations/sqlite/20260703000000_create_application_documents_table.sql"
        );
        for stmt in migration.split(';') {
            let stmt = stmt.trim();
            if stmt.is_empty() {
                continue;
            }
            sqlx::query(stmt)
                .execute(&pool)
                .await
                .unwrap_or_else(|e| panic!("failed to apply migration stmt `{stmt}`: {e}"));
        }

        Arc::new(pool)
    }

    fn sample_entity(application_id: Uuid) -> ApplicationDocumentEntity {
        let date = time::Date::from_calendar_date(2026, time::Month::July, 3).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        ApplicationDocumentEntity {
            id: Uuid::new_v4(),
            application_id,
            file_name: Arc::from("antrag_scan.pdf"),
            mime_type: Arc::from("application/pdf"),
            relative_path: Arc::from("applications/foo/bar.pdf"),
            size: 234_567,
            created: datetime,
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    /// Insert a parent `application(id)` row so the FK on
    /// application_documents does not reject test inserts.
    async fn seed_application(pool: &SqlitePool, app_id: Uuid) {
        sqlx::query("INSERT INTO application (id) VALUES (?)")
            .bind(app_id.as_bytes().to_vec())
            .execute(pool)
            .await
            .expect("seed application row");
    }

    #[tokio::test]
    async fn test_application_document_roundtrip_create_find_softdelete() {
        let pool = setup_db().await;
        let dao = ApplicationDocumentDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool.clone());

        let app_id = Uuid::new_v4();
        seed_application(&pool, app_id).await;
        let entity = sample_entity(app_id);
        let entity_id = entity.id;

        // 1) Create in a transaction and commit.
        let tx = tx_dao.transaction().await.unwrap();
        dao.create(&entity, "test", tx.clone()).await.unwrap();
        tx.commit().await.unwrap();

        // 2) Find by application_id — must return the created row.
        let tx = tx_dao.transaction().await.unwrap();
        let found = dao
            .find_active_by_application_id(app_id, tx.clone())
            .await
            .unwrap()
            .expect("active row must be present");
        assert_eq!(found.id, entity_id);
        assert_eq!(found.application_id, app_id);
        assert_eq!(found.file_name.as_ref(), "antrag_scan.pdf");
        assert_eq!(found.mime_type.as_ref(), "application/pdf");
        assert_eq!(found.size, 234_567);
        assert!(found.deleted.is_none());
        tx.commit().await.unwrap();

        // 3) Soft-delete via update.
        let mut soft_deleted = found.clone();
        soft_deleted.deleted = Some(soft_deleted.created);
        let tx = tx_dao.transaction().await.unwrap();
        dao.update(&soft_deleted, "test", tx.clone()).await.unwrap();
        tx.commit().await.unwrap();

        // 4) find_active_by_application_id now returns None.
        let tx = tx_dao.transaction().await.unwrap();
        let after = dao
            .find_active_by_application_id(app_id, tx.clone())
            .await
            .unwrap();
        assert!(
            after.is_none(),
            "soft-deleted row must not surface via find_active_by_application_id",
        );
        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_application_document_update_version_mismatch_conflict() {
        let pool = setup_db().await;
        let dao = ApplicationDocumentDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool.clone());

        let app_id = Uuid::new_v4();
        seed_application(&pool, app_id).await;
        let entity = sample_entity(app_id);
        let entity_id = entity.id;

        let tx = tx_dao.transaction().await.unwrap();
        dao.create(&entity, "test", tx.clone()).await.unwrap();
        tx.commit().await.unwrap();

        // Build an update with a *stale* version (a fresh v4 that never
        // existed on the row) → optimistic-lock check must fire.
        let mut stale = entity.clone();
        stale.id = entity_id;
        stale.version = Uuid::new_v4();
        stale.file_name = Arc::from("hijack.pdf");

        let tx = tx_dao.transaction().await.unwrap();
        let result = dao.update(&stale, "test", tx.clone()).await;
        match result {
            Err(DaoError::ConflictError(_)) => {}
            other => panic!("expected ConflictError, got {other:?}"),
        }
        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_application_document_partial_unique_index_enforces_single_slot() {
        // Belt-and-suspenders check: the migration's partial unique index
        // must reject a second active row for the same application_id.
        let pool = setup_db().await;
        let dao = ApplicationDocumentDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool.clone());

        let app_id = Uuid::new_v4();
        seed_application(&pool, app_id).await;
        let first = sample_entity(app_id);
        let second = sample_entity(app_id);

        let tx = tx_dao.transaction().await.unwrap();
        dao.create(&first, "test", tx.clone()).await.unwrap();
        let result = dao.create(&second, "test", tx.clone()).await;
        // sqlx bubbles the SQLITE_CONSTRAINT_UNIQUE up as DatabaseError.
        match result {
            Err(DaoError::DatabaseError(_)) => {}
            other => panic!("expected DatabaseError from partial unique index, got {other:?}"),
        }
        // Roll back the transaction — the constraint violation leaves the
        // transaction in a doomed state on some builds; explicitly drop.
        drop(tx);
    }
}
