use async_trait::async_trait;
use sqlx::{FromRow, SqlitePool};
use std::sync::Arc;
use time::PrimitiveDateTime;
use uuid::Uuid;

use crate::TransactionImpl;
use inventurly_dao::{
    container_rack::{ContainerRackDao, ContainerRackEntity},
    DaoError,
};

#[derive(FromRow)]
struct ContainerRackDb {
    container_id: Vec<u8>,
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

impl TryFrom<&ContainerRackDb> for ContainerRackEntity {
    type Error = DaoError;

    fn try_from(db_row: &ContainerRackDb) -> Result<Self, Self::Error> {
        let container_id = Uuid::from_slice(&db_row.container_id)?;
        let rack_id = Uuid::from_slice(&db_row.rack_id)?;
        let version = Uuid::from_slice(&db_row.version)?;
        let created = parse_datetime(&db_row.created)?;
        let deleted = match &db_row.deleted {
            Some(deleted_str) => Some(parse_datetime(deleted_str)?),
            None => None,
        };

        Ok(ContainerRackEntity {
            container_id,
            rack_id,
            sort_order: db_row.sort_order,
            created,
            deleted,
            version,
        })
    }
}

pub struct ContainerRackDaoImpl {
    #[allow(dead_code)] // Used indirectly through transactions
    pool: Arc<SqlitePool>,
}

impl ContainerRackDaoImpl {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ContainerRackDao for ContainerRackDaoImpl {
    type Transaction = TransactionImpl;

    async fn dump_all(&self, tx: Self::Transaction) -> Result<Arc<[ContainerRackEntity]>, DaoError> {
        let rows = sqlx::query_as::<_, ContainerRackDb>(
            "SELECT container_id, rack_id, sort_order, created, deleted, version FROM container_rack ORDER BY rack_id, sort_order"
        )
        .fetch_all(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        rows.iter()
            .map(ContainerRackEntity::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into())
    }

    async fn create(
        &self,
        entity: &ContainerRackEntity,
        _process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError> {
        let container_id = entity.container_id.as_bytes().to_vec();
        let rack_id = entity.rack_id.as_bytes().to_vec();
        let version = entity.version.as_bytes().to_vec();
        let format = &time::format_description::well_known::Iso8601::DEFAULT;
        let created = entity
            .created
            .assume_utc()
            .format(format)
            .map_err(|e| DaoError::ParseError(Arc::from(e.to_string())))?;

        sqlx::query(
            "INSERT INTO container_rack (container_id, rack_id, sort_order, created, version) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(container_id)
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
        entity: &ContainerRackEntity,
        _process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError> {
        let container_id = entity.container_id.as_bytes().to_vec();
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
            "SELECT COUNT(*) FROM container_rack WHERE container_id = ? AND rack_id = ? AND deleted IS NULL"
        )
        .bind(container_id.clone())
        .bind(rack_id.clone())
        .fetch_one(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        if exists == 0 {
            return Err(DaoError::NotFound);
        }

        let rows_affected = sqlx::query(
            "UPDATE container_rack SET deleted = ?, sort_order = ?, version = ? WHERE container_id = ? AND rack_id = ? AND version = ? AND deleted IS NULL"
        )
        .bind(deleted)
        .bind(entity.sort_order)
        .bind(new_version)
        .bind(container_id)
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

    async fn find_by_container_and_rack(
        &self,
        container_id: Uuid,
        rack_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Option<ContainerRackEntity>, DaoError> {
        let container_id_bytes = container_id.as_bytes().to_vec();
        let rack_id_bytes = rack_id.as_bytes().to_vec();

        let row = sqlx::query_as::<_, ContainerRackDb>(
            "SELECT container_id, rack_id, sort_order, created, deleted, version FROM container_rack WHERE container_id = ? AND rack_id = ?"
        )
        .bind(container_id_bytes)
        .bind(rack_id_bytes)
        .fetch_optional(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        match row {
            Some(row) => Ok(Some(ContainerRackEntity::try_from(&row)?)),
            None => Ok(None),
        }
    }

    async fn find_racks_by_container(
        &self,
        container_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Arc<[ContainerRackEntity]>, DaoError> {
        let container_id_bytes = container_id.as_bytes().to_vec();

        let rows = sqlx::query_as::<_, ContainerRackDb>(
            "SELECT container_id, rack_id, sort_order, created, deleted, version FROM container_rack WHERE container_id = ? AND deleted IS NULL ORDER BY sort_order"
        )
        .bind(container_id_bytes)
        .fetch_all(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        rows.iter()
            .map(ContainerRackEntity::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into())
    }

    async fn find_containers_by_rack(
        &self,
        rack_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Arc<[ContainerRackEntity]>, DaoError> {
        let rack_id_bytes = rack_id.as_bytes().to_vec();

        let rows = sqlx::query_as::<_, ContainerRackDb>(
            "SELECT container_id, rack_id, sort_order, created, deleted, version FROM container_rack WHERE rack_id = ? AND deleted IS NULL ORDER BY sort_order"
        )
        .bind(rack_id_bytes)
        .fetch_all(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        rows.iter()
            .map(ContainerRackEntity::try_from)
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
            "SELECT MAX(sort_order) FROM container_rack WHERE rack_id = ? AND deleted IS NULL"
        )
        .bind(rack_id_bytes)
        .fetch_one(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(max_order.unwrap_or(0) + 1)
    }

    async fn reactivate(
        &self,
        entity: &ContainerRackEntity,
        _process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError> {
        let container_id = entity.container_id.as_bytes().to_vec();
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
            "SELECT COUNT(*) FROM container_rack WHERE container_id = ? AND rack_id = ? AND deleted IS NOT NULL"
        )
        .bind(container_id.clone())
        .bind(rack_id.clone())
        .fetch_one(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        if exists == 0 {
            return Err(DaoError::NotFound);
        }

        // Reactivate by clearing deleted and updating other fields
        sqlx::query(
            "UPDATE container_rack SET deleted = NULL, created = ?, sort_order = ?, version = ? WHERE container_id = ? AND rack_id = ? AND deleted IS NOT NULL"
        )
        .bind(created)
        .bind(entity.sort_order)
        .bind(new_version)
        .bind(container_id)
        .bind(rack_id)
        .execute(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(())
    }
}
