use async_trait::async_trait;
use genossi_dao::member_document::{MemberDocumentDao, MemberDocumentEntity};
use genossi_dao::DaoError;
use sqlx::SqlitePool;
use std::sync::Arc;
use uuid::Uuid;

use crate::TransactionImpl;
use crate::datetime_utils::parse_datetime;


#[derive(Debug, sqlx::FromRow)]
struct MemberDocumentDb {
    id: Vec<u8>,
    member_id: Vec<u8>,
    document_type: String,
    description: Option<String>,
    file_name: String,
    mime_type: String,
    relative_path: String,
    created: String,
    deleted: Option<String>,
    version: Vec<u8>,
    // Phase 10 D-07: optional mail-tracking columns. NULL for legacy documents.
    template_id: Option<Vec<u8>>,
    mail_recipient_id: Option<Vec<u8>>,
    status: Option<String>,
}

/// Parse an optional UUID from a `Vec<u8>` blob. NULL stays None;
/// invalid bytes become `DaoError::ParseError`.
///
/// Local helper mirroring the Phase-10 convention used in
/// `genossi_mail/src/dao_sqlite.rs::parse_optional_uuid` — duplicated rather
/// than re-exported to keep the genossi_dao_impl_sqlite crate boundary clean.
fn parse_optional_uuid(value: &Option<Vec<u8>>) -> Result<Option<Uuid>, DaoError> {
    match value {
        Some(bytes) => {
            Ok(Some(Uuid::from_slice(bytes).map_err(|e| {
                DaoError::ParseError(Arc::from(e.to_string()))
            })?))
        }
        None => Ok(None),
    }
}

impl TryFrom<&MemberDocumentDb> for MemberDocumentEntity {
    type Error = DaoError;

    fn try_from(db: &MemberDocumentDb) -> Result<Self, Self::Error> {
        Ok(MemberDocumentEntity {
            id: Uuid::from_slice(&db.id)?,
            member_id: Uuid::from_slice(&db.member_id)?,
            document_type: Arc::from(db.document_type.as_str()),
            description: db.description.as_deref().map(Arc::from),
            file_name: Arc::from(db.file_name.as_str()),
            mime_type: Arc::from(db.mime_type.as_str()),
            relative_path: Arc::from(db.relative_path.as_str()),
            created: parse_datetime(&db.created)?,
            deleted: db.deleted.as_ref().map(|d| parse_datetime(d)).transpose()?,
            version: Uuid::from_slice(&db.version)?,
            template_id: parse_optional_uuid(&db.template_id)?,
            mail_recipient_id: parse_optional_uuid(&db.mail_recipient_id)?,
            status: db.status.as_deref().map(Arc::from),
        })
    }
}

pub struct MemberDocumentDaoImpl {
    pub pool: Arc<SqlitePool>,
}

impl MemberDocumentDaoImpl {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MemberDocumentDao for MemberDocumentDaoImpl {
    type Transaction = TransactionImpl;

    async fn dump_all(
        &self,
        tx: Self::Transaction,
    ) -> Result<Arc<[MemberDocumentEntity]>, DaoError> {
        let rows = sqlx::query_as::<_, MemberDocumentDb>(
            "SELECT id, member_id, document_type, description, file_name, mime_type, \
             relative_path, created, deleted, version, \
             template_id, mail_recipient_id, status \
             FROM member_document ORDER BY created",
        )
        .fetch_all(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        rows.iter()
            .map(MemberDocumentEntity::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into())
    }

    async fn create(
        &self,
        entity: &MemberDocumentEntity,
        _process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError> {
        let id = entity.id.as_bytes().to_vec();
        let member_id = entity.member_id.as_bytes().to_vec();
        let version = entity.version.as_bytes().to_vec();
        let format = &time::format_description::well_known::Iso8601::DEFAULT;
        let created = entity
            .created
            .assume_utc()
            .format(format)
            .map_err(|e| DaoError::ParseError(Arc::from(e.to_string())))?;
        let document_type = entity.document_type.to_string();
        let description = entity.description.as_deref().map(String::from);
        let file_name = entity.file_name.to_string();
        let mime_type = entity.mime_type.to_string();
        let relative_path = entity.relative_path.to_string();
        // Phase 10 D-07: persist the 3 optional mail-tracking columns.
        let template_id = entity.template_id.map(|u| u.as_bytes().to_vec());
        let mail_recipient_id = entity.mail_recipient_id.map(|u| u.as_bytes().to_vec());
        let status = entity.status.as_deref().map(String::from);

        sqlx::query(
            "INSERT INTO member_document (id, member_id, document_type, description, file_name, \
             mime_type, relative_path, created, version, \
             template_id, mail_recipient_id, status) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(member_id)
        .bind(document_type)
        .bind(description)
        .bind(file_name)
        .bind(mime_type)
        .bind(relative_path)
        .bind(created)
        .bind(version)
        .bind(template_id)
        .bind(mail_recipient_id)
        .bind(status)
        .execute(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(())
    }

    async fn update(
        &self,
        entity: &MemberDocumentEntity,
        _process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError> {
        let id = entity.id.as_bytes().to_vec();
        let old_version = entity.version.as_bytes().to_vec();
        let new_version = Uuid::new_v4().as_bytes().to_vec();
        let document_type = entity.document_type.to_string();
        let description = entity.description.as_deref().map(String::from);
        let file_name = entity.file_name.to_string();
        let mime_type = entity.mime_type.to_string();
        let relative_path = entity.relative_path.to_string();
        // Phase 10 D-07: persist updates to the 3 optional mail-tracking columns
        // (e.g. retry-flow flipping status sent→failed). Worker writes via Final-
        // State `audited_create!` (Plan 10.06), but D-08 demands update-path
        // coverage so Auditable diff sees changes if any future code calls update.
        let template_id = entity.template_id.map(|u| u.as_bytes().to_vec());
        let mail_recipient_id = entity.mail_recipient_id.map(|u| u.as_bytes().to_vec());
        let status = entity.status.as_deref().map(String::from);

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

        let exists = sqlx::query_scalar::<_, i32>(
            "SELECT COUNT(*) FROM member_document WHERE id = ? AND deleted IS NULL",
        )
        .bind(id.clone())
        .fetch_one(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        if exists == 0 {
            return Err(DaoError::NotFound);
        }

        let rows_affected = sqlx::query(
            "UPDATE member_document SET document_type = ?, description = ?, file_name = ?, \
             mime_type = ?, relative_path = ?, deleted = ?, version = ?, \
             template_id = ?, mail_recipient_id = ?, status = ? \
             WHERE id = ? AND version = ? AND deleted IS NULL",
        )
        .bind(document_type)
        .bind(description)
        .bind(file_name)
        .bind(mime_type)
        .bind(relative_path)
        .bind(deleted)
        .bind(new_version)
        .bind(template_id)
        .bind(mail_recipient_id)
        .bind(status)
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

    /// In-memory SQLite pool with the member_document schema applied.
    /// Mirrors the Phase-7 `repayment_entry.rs::tests::setup_db` convention:
    /// the DDL is duplicated inline so DAO unit tests do not depend on the
    /// migration runner.
    ///
    /// **Phase 10 D-07:** schema INCLUDES the 3 new mail-tracking columns
    /// (template_id, mail_recipient_id, status) so roundtrip tests can verify
    /// the SQLite impl persists them.
    async fn setup_db() -> Arc<SqlitePool> {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("create in-memory db");

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS member_document (
                id BLOB PRIMARY KEY NOT NULL,
                member_id BLOB NOT NULL,
                document_type TEXT NOT NULL,
                description TEXT,
                file_name TEXT NOT NULL,
                mime_type TEXT NOT NULL,
                relative_path TEXT NOT NULL,
                created TEXT NOT NULL,
                deleted TEXT,
                version BLOB NOT NULL,
                template_id BLOB NULL,
                mail_recipient_id BLOB NULL,
                status TEXT NULL
            )",
        )
        .execute(&pool)
        .await
        .expect("create member_document table");

        Arc::new(pool)
    }

    fn sample_entity_with_phase10_fields(
        template_id: Option<Uuid>,
        mail_recipient_id: Option<Uuid>,
        status: Option<&str>,
    ) -> MemberDocumentEntity {
        let date = time::Date::from_calendar_date(2026, time::Month::June, 1).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        MemberDocumentEntity {
            id: Uuid::new_v4(),
            member_id: Uuid::new_v4(),
            document_type: Arc::from("repayment_mail"),
            description: Some(Arc::from("Subject")),
            file_name: Arc::from(""),
            mime_type: Arc::from("text/plain"),
            relative_path: Arc::from(""),
            created: datetime,
            deleted: None,
            version: Uuid::new_v4(),
            template_id,
            mail_recipient_id,
            status: status.map(Arc::from),
        }
    }

    #[tokio::test]
    async fn test_member_document_roundtrip_with_phase10_fields_some() {
        let pool = setup_db().await;
        let dao = MemberDocumentDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let template_id = Uuid::new_v4();
        let mail_recipient_id = Uuid::new_v4();
        let entity = sample_entity_with_phase10_fields(
            Some(template_id),
            Some(mail_recipient_id),
            Some("sent"),
        );
        let entity_id = entity.id;

        let tx = tx_dao.transaction().await.unwrap();
        dao.create(&entity, "test", tx.clone()).await.unwrap();

        let found = dao
            .find_by_id(entity_id, tx.clone())
            .await
            .unwrap()
            .expect("entity must be found after create");
        assert_eq!(found.id, entity.id);
        assert_eq!(
            found.template_id,
            Some(template_id),
            "template_id roundtrip preserved"
        );
        assert_eq!(
            found.mail_recipient_id,
            Some(mail_recipient_id),
            "mail_recipient_id roundtrip preserved"
        );
        assert_eq!(
            found.status.as_deref(),
            Some("sent"),
            "status roundtrip preserved"
        );

        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_member_document_roundtrip_with_phase10_fields_none_backward_compat() {
        // Backward-compat: a legacy MemberDocument (e.g. JoinDeclaration) has NULL
        // in all 3 new columns. Create + find must keep them None — no auto-fill,
        // no spurious empty-string values for `status`.
        let pool = setup_db().await;
        let dao = MemberDocumentDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let entity = sample_entity_with_phase10_fields(None, None, None);
        let entity_id = entity.id;

        let tx = tx_dao.transaction().await.unwrap();
        dao.create(&entity, "test", tx.clone()).await.unwrap();

        let found = dao
            .find_by_id(entity_id, tx.clone())
            .await
            .unwrap()
            .expect("entity must be found after create");
        assert!(
            found.template_id.is_none(),
            "template_id NULL bleibt None nach roundtrip"
        );
        assert!(
            found.mail_recipient_id.is_none(),
            "mail_recipient_id NULL bleibt None nach roundtrip"
        );
        assert!(
            found.status.is_none(),
            "status NULL bleibt None nach roundtrip"
        );

        tx.commit().await.unwrap();
    }
}
