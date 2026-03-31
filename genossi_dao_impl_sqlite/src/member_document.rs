use async_trait::async_trait;
use genossi_dao::member_document::{MemberDocumentDao, MemberDocumentEntity};
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
            deleted: db
                .deleted
                .as_ref()
                .map(|d| parse_datetime(d))
                .transpose()?,
            version: Uuid::from_slice(&db.version)?,
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
             relative_path, created, deleted, version \
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

        sqlx::query(
            "INSERT INTO member_document (id, member_id, document_type, description, file_name, \
             mime_type, relative_path, created, version) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
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
             mime_type = ?, relative_path = ?, deleted = ?, version = ? \
             WHERE id = ? AND version = ? AND deleted IS NULL",
        )
        .bind(document_type)
        .bind(description)
        .bind(file_name)
        .bind(mime_type)
        .bind(relative_path)
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
