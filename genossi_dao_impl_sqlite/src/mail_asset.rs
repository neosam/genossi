use async_trait::async_trait;
use genossi_dao::mail_asset::{MailAssetDao, MailAssetEntity};
use genossi_dao::DaoError;
use sqlx::SqlitePool;
use std::sync::Arc;
use uuid::Uuid;

use crate::datetime_utils::parse_datetime;
use crate::TransactionImpl;

/// Raw DB row mirror used only for `sqlx::query_as` deserialization.
/// BLOBs come back as `Vec<u8>`, datetimes as ISO8601 `String` — the same
/// convention used across the sqlite DAO layer. The `bytes` BLOB maps to
/// `Vec<u8>` natively, exactly like the `id`/`version` BLOB columns.
#[derive(Debug, sqlx::FromRow)]
struct MailAssetDb {
    id: Vec<u8>,
    filename: String,
    mime_type: String,
    size_bytes: i64,
    bytes: Vec<u8>,
    uploaded_by: String,
    created: String,
    deleted: Option<String>,
    version: Vec<u8>,
}

impl TryFrom<&MailAssetDb> for MailAssetEntity {
    type Error = DaoError;

    fn try_from(db: &MailAssetDb) -> Result<Self, Self::Error> {
        Ok(MailAssetEntity {
            id: Uuid::from_slice(&db.id)?,
            filename: Arc::from(db.filename.as_str()),
            mime_type: Arc::from(db.mime_type.as_str()),
            size_bytes: db.size_bytes,
            bytes: db.bytes.clone(),
            uploaded_by: Arc::from(db.uploaded_by.as_str()),
            created: parse_datetime(&db.created)?,
            deleted: db.deleted.as_ref().map(|d| parse_datetime(d)).transpose()?,
            version: Uuid::from_slice(&db.version)?,
        })
    }
}

pub struct MailAssetDaoImpl {
    pub pool: Arc<SqlitePool>,
}

impl MailAssetDaoImpl {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MailAssetDao for MailAssetDaoImpl {
    type Transaction = TransactionImpl;

    async fn dump_all(
        &self,
        tx: Self::Transaction,
    ) -> Result<Arc<[MailAssetEntity]>, DaoError> {
        let rows = sqlx::query_as::<_, MailAssetDb>(
            "SELECT id, filename, mime_type, size_bytes, bytes, \
             uploaded_by, created, deleted, version \
             FROM mail_assets ORDER BY created",
        )
        .fetch_all(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        rows.iter()
            .map(MailAssetEntity::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into())
    }

    async fn create(
        &self,
        entity: &MailAssetEntity,
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
        let filename = entity.filename.to_string();
        let mime_type = entity.mime_type.to_string();
        let size_bytes = entity.size_bytes;
        let uploaded_by = entity.uploaded_by.to_string();

        sqlx::query(
            "INSERT INTO mail_assets (id, filename, mime_type, size_bytes, bytes, \
             uploaded_by, created, version) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(filename)
        .bind(mime_type)
        .bind(size_bytes)
        .bind(entity.bytes.clone())
        .bind(uploaded_by)
        .bind(created)
        .bind(version)
        .execute(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(())
    }

    async fn update(
        &self,
        entity: &MailAssetEntity,
        _process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError> {
        // Contract mirrors ApplicationDocumentDaoImpl::update:
        //   - entity.version = OLD version (used in WHERE for optimistic lock)
        //   - a fresh v4 is generated here and written as the NEW version
        //   - rows_affected == 0 → ConflictError (version mismatch)
        let id = entity.id.as_bytes().to_vec();
        let old_version = entity.version.as_bytes().to_vec();
        let new_version = Uuid::new_v4().as_bytes().to_vec();
        let filename = entity.filename.to_string();
        let mime_type = entity.mime_type.to_string();
        let size_bytes = entity.size_bytes;
        let uploaded_by = entity.uploaded_by.to_string();

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
            "UPDATE mail_assets SET filename = ?, mime_type = ?, size_bytes = ?, \
             bytes = ?, uploaded_by = ?, deleted = ?, version = ? \
             WHERE id = ? AND version = ?",
        )
        .bind(filename)
        .bind(mime_type)
        .bind(size_bytes)
        .bind(entity.bytes.clone())
        .bind(uploaded_by)
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

    /// In-memory SQLite pool with the mail_assets schema applied exactly as the
    /// migration file writes it. We embed the migration directly via
    /// `include_str!` rather than re-declaring the DDL, so a divergence between
    /// test and prod schema is caught by test compilation / execution
    /// (Phase 25 Plan 02 decision). No stub parent table is needed — mail_assets
    /// has no FK (unlike application_documents).
    async fn setup_db() -> Arc<SqlitePool> {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("create in-memory db");

        let migration =
            include_str!("../../migrations/sqlite/20260723000000_create_mail_assets_table.sql");
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

    fn sample_entity() -> MailAssetEntity {
        let date = time::Date::from_calendar_date(2026, time::Month::July, 23).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        MailAssetEntity {
            id: Uuid::new_v4(),
            filename: Arc::from("logo.png"),
            mime_type: Arc::from("image/png"),
            size_bytes: 8,
            // Non-trivial binary payload incl. NUL + high bytes to prove the
            // BLOB round-trips byte-identically (not truncated at a NUL).
            bytes: vec![0x89, 0x50, 0x4E, 0x47, 0x00, 0xFF, 0x0D, 0x0A],
            uploaded_by: Arc::from("admin-user"),
            created: datetime,
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    /// BLOB round-trip: create() then dump_all()/find_by_id() return the exact
    /// bytes that were inserted (IMG-01 inline BLOB proof).
    #[tokio::test]
    async fn test_mail_asset_blob_roundtrip_create_find() {
        let pool = setup_db().await;
        let dao = MailAssetDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool.clone());

        let entity = sample_entity();
        let entity_id = entity.id;
        let expected_bytes = entity.bytes.clone();

        let tx = tx_dao.transaction().await.unwrap();
        dao.create(&entity, "test", tx.clone()).await.unwrap();
        tx.commit().await.unwrap();

        let tx = tx_dao.transaction().await.unwrap();
        let found = dao
            .find_by_id(entity_id, tx.clone())
            .await
            .unwrap()
            .expect("created row must be present");
        assert_eq!(found.id, entity_id);
        assert_eq!(found.filename.as_ref(), "logo.png");
        assert_eq!(found.mime_type.as_ref(), "image/png");
        assert_eq!(found.size_bytes, 8);
        assert_eq!(found.uploaded_by.as_ref(), "admin-user");
        assert_eq!(
            found.bytes, expected_bytes,
            "BLOB bytes must round-trip byte-identically"
        );
        assert!(found.deleted.is_none());
        tx.commit().await.unwrap();
    }

    /// Soft-delete filter: a row with `deleted = Some(...)` must not surface via
    /// the default `all()`.
    #[tokio::test]
    async fn test_mail_asset_softdelete_filtered_by_all() {
        let pool = setup_db().await;
        let dao = MailAssetDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool.clone());

        let entity = sample_entity();

        let tx = tx_dao.transaction().await.unwrap();
        dao.create(&entity, "test", tx.clone()).await.unwrap();
        tx.commit().await.unwrap();

        // Soft-delete via update.
        let mut soft_deleted = entity.clone();
        soft_deleted.deleted = Some(soft_deleted.created);
        let tx = tx_dao.transaction().await.unwrap();
        dao.update(&soft_deleted, "test", tx.clone()).await.unwrap();
        tx.commit().await.unwrap();

        let tx = tx_dao.transaction().await.unwrap();
        let active = dao.all(tx.clone()).await.unwrap();
        assert!(
            active.is_empty(),
            "soft-deleted rows must be filtered by all()",
        );
        tx.commit().await.unwrap();
    }

    /// Optimistic lock: update() with a stale `version` returns ConflictError.
    #[tokio::test]
    async fn test_mail_asset_update_version_mismatch_conflict() {
        let pool = setup_db().await;
        let dao = MailAssetDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool.clone());

        let entity = sample_entity();
        let entity_id = entity.id;

        let tx = tx_dao.transaction().await.unwrap();
        dao.create(&entity, "test", tx.clone()).await.unwrap();
        tx.commit().await.unwrap();

        // Build an update with a *stale* version (a fresh v4 that never existed
        // on the row) → optimistic-lock check must fire.
        let mut stale = entity.clone();
        stale.id = entity_id;
        stale.version = Uuid::new_v4();
        stale.filename = Arc::from("hijack.png");

        let tx = tx_dao.transaction().await.unwrap();
        let result = dao.update(&stale, "test", tx.clone()).await;
        match result {
            Err(DaoError::ConflictError(_)) => {}
            other => panic!("expected ConflictError, got {other:?}"),
        }
        tx.commit().await.unwrap();
    }
}
