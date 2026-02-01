use async_trait::async_trait;
use inventurly_dao::{
    product::{ProductDao, ProductEntity},
    DaoError,
};
use sqlx::SqlitePool;
use std::sync::Arc;
use time::PrimitiveDateTime;
use uuid::Uuid;

use crate::TransactionImpl;

#[derive(Debug, sqlx::FromRow)]
struct ProductDb {
    id: Vec<u8>,
    ean: String,
    name: String,
    short_name: String,
    sales_unit: String,
    requires_weighing: i32,
    price: i64,
    created: String,
    deleted: Option<String>,
    version: Vec<u8>,
}

impl TryFrom<&ProductDb> for ProductEntity {
    type Error = DaoError;

    fn try_from(db: &ProductDb) -> Result<Self, Self::Error> {
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

        Ok(ProductEntity {
            id: Uuid::from_slice(&db.id)?,
            ean: Arc::from(db.ean.as_str()),
            name: Arc::from(db.name.as_str()),
            short_name: Arc::from(db.short_name.as_str()),
            sales_unit: Arc::from(db.sales_unit.as_str()),
            requires_weighing: db.requires_weighing != 0,
            price: db.price,
            created: parse_datetime(&db.created)?,
            deleted: db.deleted.as_ref().map(|d| parse_datetime(d)).transpose()?,
            version: Uuid::from_slice(&db.version)?,
        })
    }
}

pub struct ProductDaoImpl {
    pub pool: Arc<SqlitePool>,
}

impl ProductDaoImpl {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ProductDao for ProductDaoImpl {
    type Transaction = TransactionImpl;

    async fn dump_all(&self, tx: Self::Transaction) -> Result<Arc<[ProductEntity]>, DaoError> {
        let rows = sqlx::query_as::<_, ProductDb>(
            "SELECT id, ean, name, short_name, sales_unit, requires_weighing, price, created, deleted, version 
             FROM product ORDER BY name"
        )
        .fetch_all(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        rows.iter()
            .map(ProductEntity::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into())
    }

    async fn create(
        &self,
        entity: &ProductEntity,
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
        let ean = entity.ean.to_string();
        let name = entity.name.to_string();
        let short_name = entity.short_name.to_string();
        let sales_unit = entity.sales_unit.to_string();
        let requires_weighing = if entity.requires_weighing { 1 } else { 0 };
        let price = entity.price;

        sqlx::query(
            "INSERT INTO product (id, ean, name, short_name, sales_unit, requires_weighing, price, created, version) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(id)
        .bind(ean)
        .bind(name)
        .bind(short_name)
        .bind(sales_unit)
        .bind(requires_weighing)
        .bind(price)
        .bind(created)
        .bind(version)
        .execute(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(())
    }

    async fn update(
        &self,
        entity: &ProductEntity,
        _process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError> {
        let id = entity.id.as_bytes().to_vec();
        let old_version = entity.version.as_bytes().to_vec();
        let new_version = Uuid::new_v4().as_bytes().to_vec();
        let ean = entity.ean.to_string();
        let name = entity.name.to_string();
        let short_name = entity.short_name.to_string();
        let sales_unit = entity.sales_unit.to_string();
        let requires_weighing = if entity.requires_weighing { 1 } else { 0 };
        let price = entity.price;

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
            "SELECT COUNT(*) FROM product WHERE id = ? AND deleted IS NULL",
        )
        .bind(id.clone())
        .fetch_one(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        if exists == 0 {
            return Err(DaoError::NotFound);
        }

        let rows_affected = sqlx::query(
            "UPDATE product 
             SET ean = ?, name = ?, short_name = ?, sales_unit = ?, requires_weighing = ?, 
                 price = ?, deleted = ?, version = ? 
             WHERE id = ? AND version = ? AND deleted IS NULL",
        )
        .bind(ean)
        .bind(name)
        .bind(short_name)
        .bind(sales_unit)
        .bind(requires_weighing)
        .bind(price)
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

    // Optimized find_by_ean using index lookup instead of full table scan
    async fn find_by_ean(
        &self,
        ean: &str,
        tx: Self::Transaction,
    ) -> Result<Option<ProductEntity>, DaoError> {
        let row = sqlx::query_as::<_, ProductDb>(
            "SELECT id, ean, name, short_name, sales_unit, requires_weighing, price, created, deleted, version
             FROM product
             WHERE ean = ? AND deleted IS NULL"
        )
        .bind(ean)
        .fetch_optional(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        row.as_ref().map(ProductEntity::try_from).transpose()
    }

    // Optimized find_by_id using primary key lookup instead of full table scan
    async fn find_by_id(
        &self,
        id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Option<ProductEntity>, DaoError> {
        let id_bytes = id.as_bytes().to_vec();
        let row = sqlx::query_as::<_, ProductDb>(
            "SELECT id, ean, name, short_name, sales_unit, requires_weighing, price, created, deleted, version
             FROM product
             WHERE id = ? AND deleted IS NULL"
        )
        .bind(id_bytes)
        .fetch_optional(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        row.as_ref().map(ProductEntity::try_from).transpose()
    }

    // Optimized search implementation using SQL LIKE queries
    async fn search(
        &self,
        query: &str,
        limit: Option<usize>,
        tx: Self::Transaction,
    ) -> Result<Arc<[ProductEntity]>, DaoError> {
        let search_pattern = format!("%{}%", query.to_lowercase());
        let limit_clause = match limit {
            Some(l) => format!(" LIMIT {}", l),
            None => String::new(),
        };

        let sql = format!(
            "SELECT id, ean, name, short_name, sales_unit, requires_weighing, price, created, deleted, version 
             FROM product 
             WHERE deleted IS NULL 
               AND (LOWER(name) LIKE ? OR LOWER(ean) LIKE ? OR LOWER(short_name) LIKE ?)
             ORDER BY 
               CASE 
                 WHEN LOWER(name) = LOWER(?) OR LOWER(ean) = LOWER(?) THEN 1
                 WHEN LOWER(name) LIKE LOWER(?) OR LOWER(ean) LIKE LOWER(?) THEN 2
                 ELSE 3
               END,
               name ASC{}",
            limit_clause
        );

        let query_lower = query.to_lowercase();
        let starts_with_pattern = format!("{}%", query_lower);

        let rows: Vec<ProductDb> = sqlx::query_as(&sql)
            .bind(&search_pattern) // For LIKE %query%
            .bind(&search_pattern) // For LIKE %query%
            .bind(&search_pattern) // For LIKE %query%
            .bind(&query_lower) // For exact match comparison
            .bind(&query_lower) // For exact match comparison
            .bind(&starts_with_pattern) // For starts with comparison
            .bind(&starts_with_pattern) // For starts with comparison
            .fetch_all(tx.tx.lock().await.as_mut())
            .await
            .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        let entities: Result<Vec<ProductEntity>, DaoError> =
            rows.iter().map(ProductEntity::try_from).collect();

        Ok(entities?.into())
    }
}
