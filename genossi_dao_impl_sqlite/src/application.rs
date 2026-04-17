use async_trait::async_trait;
use genossi_dao::application::{ApplicationDao, ApplicationEntity, ApplicationStatus};
use genossi_dao::member::Salutation;
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
struct ApplicationDb {
    id: Vec<u8>,
    first_name: String,
    last_name: String,
    salutation: Option<String>,
    title: Option<String>,
    email: Option<String>,
    street: Option<String>,
    house_number: Option<String>,
    postal_code: Option<String>,
    city: Option<String>,
    shares: i32,
    status: String,
    created: String,
    deleted: Option<String>,
    version: Vec<u8>,
}

impl TryFrom<&ApplicationDb> for ApplicationEntity {
    type Error = DaoError;

    fn try_from(db: &ApplicationDb) -> Result<Self, Self::Error> {
        Ok(ApplicationEntity {
            id: Uuid::from_slice(&db.id)?,
            first_name: Arc::from(db.first_name.as_str()),
            last_name: Arc::from(db.last_name.as_str()),
            salutation: db
                .salutation
                .as_deref()
                .map(Salutation::from_str)
                .transpose()?,
            title: db.title.as_deref().map(Arc::from),
            email: db.email.as_deref().map(Arc::from),
            street: db.street.as_deref().map(Arc::from),
            house_number: db.house_number.as_deref().map(Arc::from),
            postal_code: db.postal_code.as_deref().map(Arc::from),
            city: db.city.as_deref().map(Arc::from),
            shares: db.shares,
            status: ApplicationStatus::from_str(&db.status)?,
            created: parse_datetime(&db.created)?,
            deleted: db.deleted.as_ref().map(|d| parse_datetime(d)).transpose()?,
            version: Uuid::from_slice(&db.version)?,
        })
    }
}

pub struct ApplicationDaoImpl {
    pub pool: Arc<SqlitePool>,
}

impl ApplicationDaoImpl {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ApplicationDao for ApplicationDaoImpl {
    type Transaction = TransactionImpl;

    async fn dump_all(&self, tx: Self::Transaction) -> Result<Arc<[ApplicationEntity]>, DaoError> {
        let rows = sqlx::query_as::<_, ApplicationDb>(
            "SELECT id, first_name, last_name, salutation, title, email, street, house_number, \
             postal_code, city, shares, status, created, deleted, version \
             FROM application ORDER BY created DESC",
        )
        .fetch_all(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        rows.iter()
            .map(ApplicationEntity::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into())
    }

    async fn create(
        &self,
        entity: &ApplicationEntity,
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
        let salutation = entity.salutation.as_ref().map(|s| s.as_str().to_string());
        let title = entity.title.as_deref().map(|s| s.to_string());
        let email = entity.email.as_deref().map(|s| s.to_string());
        let street = entity.street.as_deref().map(|s| s.to_string());
        let house_number = entity.house_number.as_deref().map(|s| s.to_string());
        let postal_code = entity.postal_code.as_deref().map(|s| s.to_string());
        let city = entity.city.as_deref().map(|s| s.to_string());
        let status = entity.status.as_str().to_string();

        sqlx::query(
            "INSERT INTO application (id, first_name, last_name, salutation, title, email, street, \
             house_number, postal_code, city, shares, status, created, version) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(first_name)
        .bind(last_name)
        .bind(salutation)
        .bind(title)
        .bind(email)
        .bind(street)
        .bind(house_number)
        .bind(postal_code)
        .bind(city)
        .bind(entity.shares)
        .bind(status)
        .bind(created)
        .bind(version)
        .execute(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(())
    }

    async fn update(
        &self,
        entity: &ApplicationEntity,
        _process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError> {
        let id = entity.id.as_bytes().to_vec();
        let old_version = entity.version.as_bytes().to_vec();
        let new_version = Uuid::new_v4().as_bytes().to_vec();
        let first_name = entity.first_name.to_string();
        let last_name = entity.last_name.to_string();
        let salutation = entity.salutation.as_ref().map(|s| s.as_str().to_string());
        let title = entity.title.as_deref().map(|s| s.to_string());
        let email = entity.email.as_deref().map(|s| s.to_string());
        let street = entity.street.as_deref().map(|s| s.to_string());
        let house_number = entity.house_number.as_deref().map(|s| s.to_string());
        let postal_code = entity.postal_code.as_deref().map(|s| s.to_string());
        let city = entity.city.as_deref().map(|s| s.to_string());
        let status = entity.status.as_str().to_string();

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
            "SELECT COUNT(*) FROM application WHERE id = ? AND deleted IS NULL",
        )
        .bind(id.clone())
        .fetch_one(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        if exists == 0 {
            return Err(DaoError::NotFound);
        }

        let rows_affected = sqlx::query(
            "UPDATE application SET first_name = ?, last_name = ?, salutation = ?, title = ?, email = ?, \
             street = ?, house_number = ?, postal_code = ?, city = ?, shares = ?, \
             status = ?, deleted = ?, version = ? \
             WHERE id = ? AND version = ? AND deleted IS NULL",
        )
        .bind(first_name)
        .bind(last_name)
        .bind(salutation)
        .bind(title)
        .bind(email)
        .bind(street)
        .bind(house_number)
        .bind(postal_code)
        .bind(city)
        .bind(entity.shares)
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
}
