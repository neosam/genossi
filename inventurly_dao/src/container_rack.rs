use async_trait::async_trait;
use std::sync::Arc;
use time::PrimitiveDateTime;
use uuid::Uuid;

use crate::DaoError;

#[derive(Debug, Clone, PartialEq)]
pub struct ContainerRackEntity {
    pub container_id: Uuid,
    pub rack_id: Uuid,
    pub sort_order: i32,
    pub created: PrimitiveDateTime,
    pub deleted: Option<PrimitiveDateTime>,
    pub version: Uuid,
}

#[async_trait]
pub trait ContainerRackDao {
    type Transaction: Send + Sync + Clone;

    /// Get all container-rack relationships (includes soft-deleted)
    async fn dump_all(&self, tx: Self::Transaction) -> Result<Arc<[ContainerRackEntity]>, DaoError>;

    /// Create a new container-rack relationship
    async fn create(
        &self,
        entity: &ContainerRackEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    /// Update an existing container-rack relationship
    async fn update(
        &self,
        entity: &ContainerRackEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    /// Find container-rack relationship by container and rack IDs
    async fn find_by_container_and_rack(
        &self,
        container_id: Uuid,
        rack_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Option<ContainerRackEntity>, DaoError>;

    /// Get all racks for a specific container (active relationships only)
    async fn find_racks_by_container(
        &self,
        container_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Arc<[ContainerRackEntity]>, DaoError>;

    /// Get all containers in a specific rack (active relationships only)
    async fn find_containers_by_rack(
        &self,
        rack_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Arc<[ContainerRackEntity]>, DaoError>;

    /// Get all active container-rack relationships
    async fn all(&self, tx: Self::Transaction) -> Result<Arc<[ContainerRackEntity]>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        let active_entities: Vec<ContainerRackEntity> = all_entities
            .iter()
            .filter(|entity| entity.deleted.is_none())
            .cloned()
            .collect();
        Ok(active_entities.into())
    }

    /// Get the next available sort_order for a rack
    async fn get_next_sort_order(
        &self,
        rack_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<i32, DaoError>;

    /// Reactivate a soft-deleted container-rack relationship
    async fn reactivate(
        &self,
        entity: &ContainerRackEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;
}
