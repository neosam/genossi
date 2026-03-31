use async_trait::async_trait;
use genossi_dao::member_action::{ActionType, MemberActionDao, MemberActionEntity};
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

fn parse_date(s: &str) -> Result<time::Date, time::error::Parse> {
    let format = time::format_description::parse("[year]-[month]-[day]").unwrap();
    time::Date::parse(s, &format)
}

fn format_date(d: &time::Date) -> String {
    let format = time::format_description::parse("[year]-[month]-[day]").unwrap();
    d.format(&format).unwrap()
}

#[derive(Debug, sqlx::FromRow)]
struct MemberActionDb {
    id: Vec<u8>,
    member_id: Vec<u8>,
    action_type: String,
    date: String,
    shares_change: i32,
    transfer_member_id: Option<Vec<u8>>,
    effective_date: Option<String>,
    comment: Option<String>,
    created: String,
    deleted: Option<String>,
    version: Vec<u8>,
}

impl TryFrom<&MemberActionDb> for MemberActionEntity {
    type Error = DaoError;

    fn try_from(db: &MemberActionDb) -> Result<Self, Self::Error> {
        Ok(MemberActionEntity {
            id: Uuid::from_slice(&db.id)?,
            member_id: Uuid::from_slice(&db.member_id)?,
            action_type: ActionType::from_str(&db.action_type)?,
            date: parse_date(&db.date)?,
            shares_change: db.shares_change,
            transfer_member_id: db
                .transfer_member_id
                .as_ref()
                .map(|b| Uuid::from_slice(b))
                .transpose()?,
            effective_date: db
                .effective_date
                .as_ref()
                .map(|d| parse_date(d))
                .transpose()?,
            comment: db.comment.as_deref().map(Arc::from),
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

pub struct MemberActionDaoImpl {
    pub pool: Arc<SqlitePool>,
}

impl MemberActionDaoImpl {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MemberActionDao for MemberActionDaoImpl {
    type Transaction = TransactionImpl;

    async fn dump_all(
        &self,
        tx: Self::Transaction,
    ) -> Result<Arc<[MemberActionEntity]>, DaoError> {
        let rows = sqlx::query_as::<_, MemberActionDb>(
            "SELECT id, member_id, action_type, date, shares_change, transfer_member_id, \
             effective_date, comment, created, deleted, version \
             FROM member_action ORDER BY date, created",
        )
        .fetch_all(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        rows.iter()
            .map(MemberActionEntity::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into())
    }

    async fn create(
        &self,
        entity: &MemberActionEntity,
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
        let action_type = entity.action_type.as_str().to_string();
        let date = format_date(&entity.date);
        let transfer_member_id = entity
            .transfer_member_id
            .as_ref()
            .map(|u| u.as_bytes().to_vec());
        let effective_date = entity.effective_date.as_ref().map(format_date);
        let comment = entity.comment.as_deref().map(String::from);

        sqlx::query(
            "INSERT INTO member_action (id, member_id, action_type, date, shares_change, \
             transfer_member_id, effective_date, comment, created, version) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(member_id)
        .bind(action_type)
        .bind(date)
        .bind(entity.shares_change)
        .bind(transfer_member_id)
        .bind(effective_date)
        .bind(comment)
        .bind(created)
        .bind(version)
        .execute(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(())
    }

    async fn update(
        &self,
        entity: &MemberActionEntity,
        _process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError> {
        let id = entity.id.as_bytes().to_vec();
        let old_version = entity.version.as_bytes().to_vec();
        let new_version = Uuid::new_v4().as_bytes().to_vec();
        let action_type = entity.action_type.as_str().to_string();
        let date = format_date(&entity.date);
        let transfer_member_id = entity
            .transfer_member_id
            .as_ref()
            .map(|u| u.as_bytes().to_vec());
        let effective_date = entity.effective_date.as_ref().map(format_date);
        let comment = entity.comment.as_deref().map(String::from);

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
            "SELECT COUNT(*) FROM member_action WHERE id = ? AND deleted IS NULL",
        )
        .bind(id.clone())
        .fetch_one(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        if exists == 0 {
            return Err(DaoError::NotFound);
        }

        let rows_affected = sqlx::query(
            "UPDATE member_action SET action_type = ?, date = ?, shares_change = ?, \
             transfer_member_id = ?, effective_date = ?, comment = ?, deleted = ?, version = ? \
             WHERE id = ? AND version = ? AND deleted IS NULL",
        )
        .bind(action_type)
        .bind(date)
        .bind(entity.shares_change)
        .bind(transfer_member_id)
        .bind(effective_date)
        .bind(comment)
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
