use async_trait::async_trait;
use inventurly_dao::{
    inventur_measurement::{InventurMeasurementDao, InventurMeasurementEntity},
    DaoError,
};
use sqlx::SqlitePool;
use std::sync::Arc;
use time::PrimitiveDateTime;
use uuid::Uuid;

use crate::TransactionImpl;

#[derive(Debug, sqlx::FromRow)]
struct InventurMeasurementDb {
    id: Vec<u8>,
    inventur_id: Vec<u8>,
    product_id: Vec<u8>,
    rack_id: Option<Vec<u8>>,
    container_id: Option<Vec<u8>>,
    count: Option<i64>,
    weight_grams: Option<i64>,
    measured_by: String,
    measured_at: String,
    notes: Option<String>,
    review_state: String,
    created: String,
    deleted: Option<String>,
    version: Vec<u8>,
}

impl TryFrom<&InventurMeasurementDb> for InventurMeasurementEntity {
    type Error = DaoError;

    fn try_from(db: &InventurMeasurementDb) -> Result<Self, Self::Error> {
        // Try multiple datetime formats to handle different storage formats
        fn parse_datetime(s: &str) -> Result<PrimitiveDateTime, time::error::Parse> {
            // First try ISO8601 format (what we should be using)
            if let Ok(dt) =
                PrimitiveDateTime::parse(s, &time::format_description::well_known::Iso8601::DEFAULT)
            {
                return Ok(dt);
            }

            // Then try SQLite default format with microseconds
            let sqlite_format = time::format_description::parse(
                "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond]",
            )
            .unwrap(); // This format should always parse correctly
            if let Ok(dt) = PrimitiveDateTime::parse(s, &sqlite_format) {
                return Ok(dt);
            }

            // Try SQLite format without microseconds
            let sqlite_simple =
                time::format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]")
                    .unwrap(); // This format should always parse correctly
            PrimitiveDateTime::parse(s, &sqlite_simple)
        }

        Ok(InventurMeasurementEntity {
            id: Uuid::from_slice(&db.id)?,
            inventur_id: Uuid::from_slice(&db.inventur_id)?,
            product_id: Uuid::from_slice(&db.product_id)?,
            rack_id: db
                .rack_id
                .as_ref()
                .map(|bytes| Uuid::from_slice(bytes))
                .transpose()?,
            container_id: db
                .container_id
                .as_ref()
                .map(|bytes| Uuid::from_slice(bytes))
                .transpose()?,
            count: db.count,
            weight_grams: db.weight_grams,
            measured_by: Arc::from(db.measured_by.as_str()),
            measured_at: parse_datetime(&db.measured_at)?,
            notes: db.notes.as_ref().map(|n| Arc::from(n.as_str())),
            review_state: Arc::from(db.review_state.as_str()),
            created: parse_datetime(&db.created)?,
            deleted: db.deleted.as_ref().map(|d| parse_datetime(d)).transpose()?,
            version: Uuid::from_slice(&db.version)?,
        })
    }
}

pub struct InventurMeasurementDaoImpl {
    pub pool: Arc<SqlitePool>,
}

impl InventurMeasurementDaoImpl {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl InventurMeasurementDao for InventurMeasurementDaoImpl {
    type Transaction = TransactionImpl;

    async fn dump_all(
        &self,
        tx: Self::Transaction,
    ) -> Result<Arc<[InventurMeasurementEntity]>, DaoError> {
        let rows = sqlx::query_as::<_, InventurMeasurementDb>(
            "SELECT id, inventur_id, product_id, rack_id, container_id, count, weight_grams,
                    measured_by, measured_at, notes, review_state, created, deleted, version
             FROM inventur_measurement ORDER BY measured_at DESC"
        )
        .fetch_all(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        rows.iter()
            .map(InventurMeasurementEntity::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into())
    }

    async fn create(
        &self,
        entity: &InventurMeasurementEntity,
        _process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError> {
        let id = entity.id.as_bytes().to_vec();
        let inventur_id = entity.inventur_id.as_bytes().to_vec();
        let product_id = entity.product_id.as_bytes().to_vec();
        let rack_id = entity.rack_id.map(|uuid| uuid.as_bytes().to_vec());
        let container_id = entity.container_id.map(|uuid| uuid.as_bytes().to_vec());
        let version = entity.version.as_bytes().to_vec();
        let format = &time::format_description::well_known::Iso8601::DEFAULT;
        let created = entity
            .created
            .assume_utc()
            .format(format)
            .map_err(|e| DaoError::ParseError(Arc::from(e.to_string())))?;
        let measured_at = entity
            .measured_at
            .assume_utc()
            .format(format)
            .map_err(|e| DaoError::ParseError(Arc::from(e.to_string())))?;
        let measured_by = entity.measured_by.to_string();
        let notes = entity.notes.as_ref().map(|n| n.to_string());
        let review_state = entity.review_state.to_string();

        sqlx::query(
            "INSERT INTO inventur_measurement
             (id, inventur_id, product_id, rack_id, container_id, count, weight_grams,
              measured_by, measured_at, notes, review_state, created, version)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(id)
        .bind(inventur_id)
        .bind(product_id)
        .bind(rack_id)
        .bind(container_id)
        .bind(entity.count)
        .bind(entity.weight_grams)
        .bind(measured_by)
        .bind(measured_at)
        .bind(notes)
        .bind(review_state)
        .bind(created)
        .bind(version)
        .execute(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(())
    }

    async fn update(
        &self,
        entity: &InventurMeasurementEntity,
        _process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError> {
        let id = entity.id.as_bytes().to_vec();
        let old_version = entity.version.as_bytes().to_vec();
        let new_version = Uuid::new_v4().as_bytes().to_vec();
        let inventur_id = entity.inventur_id.as_bytes().to_vec();
        let product_id = entity.product_id.as_bytes().to_vec();
        let rack_id = entity.rack_id.map(|uuid| uuid.as_bytes().to_vec());
        let container_id = entity.container_id.map(|uuid| uuid.as_bytes().to_vec());
        let format = &time::format_description::well_known::Iso8601::DEFAULT;
        let measured_at = entity
            .measured_at
            .assume_utc()
            .format(format)
            .map_err(|e| DaoError::ParseError(Arc::from(e.to_string())))?;
        let measured_by = entity.measured_by.to_string();
        let notes = entity.notes.as_ref().map(|n| n.to_string());
        let review_state = entity.review_state.to_string();

        // Format deleted timestamp if present
        let deleted = match entity.deleted {
            Some(dt) => Some(
                dt.assume_utc()
                    .format(format)
                    .map_err(|e| DaoError::ParseError(Arc::from(e.to_string())))?,
            ),
            None => None,
        };

        // First check if the entity exists
        let exists = sqlx::query_scalar::<_, i32>(
            "SELECT COUNT(*) FROM inventur_measurement WHERE id = ? AND deleted IS NULL",
        )
        .bind(id.clone())
        .fetch_one(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        if exists == 0 {
            return Err(DaoError::NotFound);
        }

        let rows_affected = sqlx::query(
            "UPDATE inventur_measurement
             SET inventur_id = ?, product_id = ?, rack_id = ?, container_id = ?,
                 count = ?, weight_grams = ?, measured_by = ?, measured_at = ?,
                 notes = ?, review_state = ?, deleted = ?, version = ?
             WHERE id = ? AND version = ? AND deleted IS NULL",
        )
        .bind(inventur_id)
        .bind(product_id)
        .bind(rack_id)
        .bind(container_id)
        .bind(entity.count)
        .bind(entity.weight_grams)
        .bind(measured_by)
        .bind(measured_at)
        .bind(notes)
        .bind(review_state)
        .bind(deleted)
        .bind(new_version)
        .bind(id)
        .bind(old_version)
        .execute(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        if rows_affected.rows_affected() == 0 {
            return Err(DaoError::ConflictError(Arc::from(
                "Version mismatch - entity was modified by another transaction",
            )));
        }

        Ok(())
    }
}
