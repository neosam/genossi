use async_trait::async_trait;
use genossi_dao::member::{MemberDao, MemberEntity};
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
struct MemberDb {
    id: Vec<u8>,
    member_number: i64,
    first_name: String,
    last_name: String,
    email: Option<String>,
    company: Option<String>,
    comment: Option<String>,
    street: Option<String>,
    house_number: Option<String>,
    postal_code: Option<String>,
    city: Option<String>,
    join_date: String,
    shares_at_joining: i32,
    current_shares: i32,
    current_balance: i64,
    action_count: i32,
    exit_date: Option<String>,
    bank_account: Option<String>,
    created: String,
    deleted: Option<String>,
    version: Vec<u8>,
}

impl TryFrom<&MemberDb> for MemberEntity {
    type Error = DaoError;

    fn try_from(db: &MemberDb) -> Result<Self, Self::Error> {
        Ok(MemberEntity {
            id: Uuid::from_slice(&db.id)?,
            member_number: db.member_number,
            first_name: Arc::from(db.first_name.as_str()),
            last_name: Arc::from(db.last_name.as_str()),
            email: db.email.as_deref().map(Arc::from),
            company: db.company.as_deref().map(Arc::from),
            comment: db.comment.as_deref().map(Arc::from),
            street: db.street.as_deref().map(Arc::from),
            house_number: db.house_number.as_deref().map(Arc::from),
            postal_code: db.postal_code.as_deref().map(Arc::from),
            city: db.city.as_deref().map(Arc::from),
            join_date: parse_date(&db.join_date)?,
            shares_at_joining: db.shares_at_joining,
            current_shares: db.current_shares,
            current_balance: db.current_balance,
            action_count: db.action_count,
            exit_date: db.exit_date.as_ref().map(|d| parse_date(d)).transpose()?,
            bank_account: db.bank_account.as_deref().map(Arc::from),
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

pub struct MemberDaoImpl {
    pub pool: Arc<SqlitePool>,
}

impl MemberDaoImpl {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MemberDao for MemberDaoImpl {
    type Transaction = TransactionImpl;

    async fn dump_all(&self, tx: Self::Transaction) -> Result<Arc<[MemberEntity]>, DaoError> {
        let rows = sqlx::query_as::<_, MemberDb>(
            "SELECT id, member_number, first_name, last_name, email, company, comment, \
             street, house_number, postal_code, city, join_date, shares_at_joining, \
             current_shares, current_balance, action_count, exit_date, bank_account, created, deleted, version \
             FROM member ORDER BY member_number",
        )
        .fetch_all(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        rows.iter()
            .map(MemberEntity::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into())
    }

    async fn create(
        &self,
        entity: &MemberEntity,
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
        let first_name = entity.first_name.to_string();
        let last_name = entity.last_name.to_string();
        let email = entity.email.as_deref().map(String::from);
        let company = entity.company.as_deref().map(String::from);
        let comment = entity.comment.as_deref().map(String::from);
        let street = entity.street.as_deref().map(String::from);
        let house_number = entity.house_number.as_deref().map(String::from);
        let postal_code = entity.postal_code.as_deref().map(String::from);
        let city = entity.city.as_deref().map(String::from);
        let join_date = format_date(&entity.join_date);
        let exit_date = entity.exit_date.as_ref().map(format_date);
        let bank_account = entity.bank_account.as_deref().map(String::from);

        sqlx::query(
            "INSERT INTO member (id, member_number, first_name, last_name, email, company, comment, \
             street, house_number, postal_code, city, join_date, shares_at_joining, \
             current_shares, current_balance, action_count, exit_date, bank_account, created, version) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(entity.member_number)
        .bind(first_name)
        .bind(last_name)
        .bind(email)
        .bind(company)
        .bind(comment)
        .bind(street)
        .bind(house_number)
        .bind(postal_code)
        .bind(city)
        .bind(join_date)
        .bind(entity.shares_at_joining)
        .bind(entity.current_shares)
        .bind(entity.current_balance)
        .bind(entity.action_count)
        .bind(exit_date)
        .bind(bank_account)
        .bind(created)
        .bind(version)
        .execute(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(())
    }

    async fn update(
        &self,
        entity: &MemberEntity,
        _process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError> {
        let id = entity.id.as_bytes().to_vec();
        let old_version = entity.version.as_bytes().to_vec();
        let new_version = Uuid::new_v4().as_bytes().to_vec();
        let first_name = entity.first_name.to_string();
        let last_name = entity.last_name.to_string();
        let email = entity.email.as_deref().map(String::from);
        let company = entity.company.as_deref().map(String::from);
        let comment = entity.comment.as_deref().map(String::from);
        let street = entity.street.as_deref().map(String::from);
        let house_number = entity.house_number.as_deref().map(String::from);
        let postal_code = entity.postal_code.as_deref().map(String::from);
        let city = entity.city.as_deref().map(String::from);
        let join_date = format_date(&entity.join_date);
        let exit_date = entity.exit_date.as_ref().map(format_date);
        let bank_account = entity.bank_account.as_deref().map(String::from);

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
            "SELECT COUNT(*) FROM member WHERE id = ? AND deleted IS NULL",
        )
        .bind(id.clone())
        .fetch_one(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        if exists == 0 {
            return Err(DaoError::NotFound);
        }

        let rows_affected = sqlx::query(
            "UPDATE member SET member_number = ?, first_name = ?, last_name = ?, email = ?, \
             company = ?, comment = ?, street = ?, house_number = ?, postal_code = ?, city = ?, \
             join_date = ?, shares_at_joining = ?, current_shares = ?, current_balance = ?, \
             action_count = ?, exit_date = ?, bank_account = ?, deleted = ?, version = ? \
             WHERE id = ? AND version = ? AND deleted IS NULL",
        )
        .bind(entity.member_number)
        .bind(first_name)
        .bind(last_name)
        .bind(email)
        .bind(company)
        .bind(comment)
        .bind(street)
        .bind(house_number)
        .bind(postal_code)
        .bind(city)
        .bind(join_date)
        .bind(entity.shares_at_joining)
        .bind(entity.current_shares)
        .bind(entity.current_balance)
        .bind(entity.action_count)
        .bind(exit_date)
        .bind(bank_account)
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
