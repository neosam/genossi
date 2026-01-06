use async_trait::async_trait;
use sqlx::{FromRow, SqlitePool};
use std::sync::Arc;
use time::PrimitiveDateTime;
use uuid::Uuid;

use crate::TransactionImpl;
use inventurly_dao::{
    product_rack::{ProductRackDao, ProductRackEntity},
    DaoError,
};

#[derive(FromRow)]
struct ProductRackDb {
    product_id: Vec<u8>,
    rack_id: Vec<u8>,
    sort_order: i32,
    created: String,
    deleted: Option<String>,
    version: Vec<u8>,
}

fn parse_datetime(s: &str) -> Result<PrimitiveDateTime, time::error::Parse> {
    // Try ISO8601 format first (used by new records)
    if let Ok(dt) =
        time::OffsetDateTime::parse(s, &time::format_description::well_known::Iso8601::DEFAULT)
    {
        return Ok(dt
            .to_offset(time::UtcOffset::UTC)
            .date()
            .with_time(dt.time()));
    }

    // Try SQLite default format for backward compatibility
    time::PrimitiveDateTime::parse(
        s,
        time::macros::format_description!(
            "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond]"
        ),
    )
}

impl TryFrom<&ProductRackDb> for ProductRackEntity {
    type Error = DaoError;

    fn try_from(db_row: &ProductRackDb) -> Result<Self, Self::Error> {
        let product_id = Uuid::from_slice(&db_row.product_id)?;
        let rack_id = Uuid::from_slice(&db_row.rack_id)?;
        let version = Uuid::from_slice(&db_row.version)?;
        let created = parse_datetime(&db_row.created)?;
        let deleted = match &db_row.deleted {
            Some(deleted_str) => Some(parse_datetime(deleted_str)?),
            None => None,
        };

        Ok(ProductRackEntity {
            product_id,
            rack_id,
            sort_order: db_row.sort_order,
            created,
            deleted,
            version,
        })
    }
}

pub struct ProductRackDaoImpl {
    #[allow(dead_code)] // Used indirectly through transactions
    pool: Arc<SqlitePool>,
}

impl ProductRackDaoImpl {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ProductRackDao for ProductRackDaoImpl {
    type Transaction = TransactionImpl;

    async fn dump_all(&self, tx: Self::Transaction) -> Result<Arc<[ProductRackEntity]>, DaoError> {
        let rows = sqlx::query_as::<_, ProductRackDb>(
            "SELECT product_id, rack_id, sort_order, created, deleted, version FROM product_rack ORDER BY rack_id, sort_order"
        )
        .fetch_all(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        rows.iter()
            .map(ProductRackEntity::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into())
    }

    async fn create(
        &self,
        entity: &ProductRackEntity,
        _process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError> {
        let product_id = entity.product_id.as_bytes().to_vec();
        let rack_id = entity.rack_id.as_bytes().to_vec();
        let version = entity.version.as_bytes().to_vec();
        let format = &time::format_description::well_known::Iso8601::DEFAULT;
        let created = entity
            .created
            .assume_utc()
            .format(format)
            .map_err(|e| DaoError::ParseError(Arc::from(e.to_string())))?;

        sqlx::query(
            "INSERT INTO product_rack (product_id, rack_id, sort_order, created, version) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(product_id)
        .bind(rack_id)
        .bind(entity.sort_order)
        .bind(created)
        .bind(version)
        .execute(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(())
    }

    async fn update(
        &self,
        entity: &ProductRackEntity,
        _process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError> {
        let product_id = entity.product_id.as_bytes().to_vec();
        let rack_id = entity.rack_id.as_bytes().to_vec();
        let old_version = entity.version.as_bytes().to_vec();
        let new_version = Uuid::new_v4().as_bytes().to_vec();

        // Format deleted timestamp if present
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

        // First check if the entity exists
        let exists = sqlx::query_scalar::<_, i32>(
            "SELECT COUNT(*) FROM product_rack WHERE product_id = ? AND rack_id = ? AND deleted IS NULL"
        )
        .bind(product_id.clone())
        .bind(rack_id.clone())
        .fetch_one(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        if exists == 0 {
            return Err(DaoError::NotFound);
        }

        let rows_affected = sqlx::query(
            "UPDATE product_rack SET deleted = ?, sort_order = ?, version = ? WHERE product_id = ? AND rack_id = ? AND version = ? AND deleted IS NULL"
        )
        .bind(deleted)
        .bind(entity.sort_order)
        .bind(new_version)
        .bind(product_id)
        .bind(rack_id)
        .bind(old_version)
        .execute(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        if rows_affected.rows_affected() == 0 {
            return Err(DaoError::ConflictError(Arc::from("Version mismatch")));
        }

        Ok(())
    }

    async fn find_by_product_and_rack(
        &self,
        product_id: Uuid,
        rack_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Option<ProductRackEntity>, DaoError> {
        let product_id_bytes = product_id.as_bytes().to_vec();
        let rack_id_bytes = rack_id.as_bytes().to_vec();

        let row = sqlx::query_as::<_, ProductRackDb>(
            "SELECT product_id, rack_id, sort_order, created, deleted, version FROM product_rack WHERE product_id = ? AND rack_id = ?"
        )
        .bind(product_id_bytes)
        .bind(rack_id_bytes)
        .fetch_optional(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        match row {
            Some(row) => Ok(Some(ProductRackEntity::try_from(&row)?)),
            None => Ok(None),
        }
    }

    async fn find_racks_by_product(
        &self,
        product_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Arc<[ProductRackEntity]>, DaoError> {
        let product_id_bytes = product_id.as_bytes().to_vec();

        let rows = sqlx::query_as::<_, ProductRackDb>(
            "SELECT product_id, rack_id, sort_order, created, deleted, version FROM product_rack WHERE product_id = ? AND deleted IS NULL ORDER BY sort_order"
        )
        .bind(product_id_bytes)
        .fetch_all(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        rows.iter()
            .map(ProductRackEntity::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into())
    }

    async fn find_products_by_rack(
        &self,
        rack_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Arc<[ProductRackEntity]>, DaoError> {
        let rack_id_bytes = rack_id.as_bytes().to_vec();

        let rows = sqlx::query_as::<_, ProductRackDb>(
            "SELECT product_id, rack_id, sort_order, created, deleted, version FROM product_rack WHERE rack_id = ? AND deleted IS NULL ORDER BY sort_order"
        )
        .bind(rack_id_bytes)
        .fetch_all(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        rows.iter()
            .map(ProductRackEntity::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into())
    }

    async fn get_next_sort_order(
        &self,
        rack_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<i32, DaoError> {
        let rack_id_bytes = rack_id.as_bytes().to_vec();

        let max_order = sqlx::query_scalar::<_, Option<i32>>(
            "SELECT MAX(sort_order) FROM product_rack WHERE rack_id = ? AND deleted IS NULL"
        )
        .bind(rack_id_bytes)
        .fetch_one(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(max_order.unwrap_or(0) + 1)
    }

    async fn reactivate(
        &self,
        entity: &ProductRackEntity,
        _process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError> {
        let product_id = entity.product_id.as_bytes().to_vec();
        let rack_id = entity.rack_id.as_bytes().to_vec();
        let new_version = entity.version.as_bytes().to_vec();

        // Format created timestamp
        let format = &time::format_description::well_known::Iso8601::DEFAULT;
        let created = entity
            .created
            .assume_utc()
            .format(format)
            .map_err(|e| DaoError::ParseError(Arc::from(e.to_string())))?;

        // Check if a deleted record exists
        let exists = sqlx::query_scalar::<_, i32>(
            "SELECT COUNT(*) FROM product_rack WHERE product_id = ? AND rack_id = ? AND deleted IS NOT NULL"
        )
        .bind(product_id.clone())
        .bind(rack_id.clone())
        .fetch_one(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        if exists == 0 {
            return Err(DaoError::NotFound);
        }

        // Reactivate by clearing deleted and updating other fields
        sqlx::query(
            "UPDATE product_rack SET deleted = NULL, created = ?, sort_order = ?, version = ? WHERE product_id = ? AND rack_id = ? AND deleted IS NOT NULL"
        )
        .bind(created)
        .bind(entity.sort_order)
        .bind(new_version)
        .bind(product_id)
        .bind(rack_id)
        .execute(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(())
    }
}
