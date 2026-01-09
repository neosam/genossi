use async_trait::async_trait;
use std::sync::Arc;
use time::PrimitiveDateTime;
use uuid::Uuid;

use crate::{permission::Authentication, ServiceError};

#[derive(Debug, Clone, PartialEq)]
pub struct ContainerRack {
    pub container_id: Uuid,
    pub rack_id: Uuid,
    pub sort_order: i32,
    pub created: PrimitiveDateTime,
    pub deleted: Option<PrimitiveDateTime>,
    pub version: Uuid,
}

impl From<&inventurly_dao::container_rack::ContainerRackEntity> for ContainerRack {
    fn from(entity: &inventurly_dao::container_rack::ContainerRackEntity) -> Self {
        Self {
            container_id: entity.container_id,
            rack_id: entity.rack_id,
            sort_order: entity.sort_order,
            created: entity.created,
            deleted: entity.deleted,
            version: entity.version,
        }
    }
}

impl From<&ContainerRack> for inventurly_dao::container_rack::ContainerRackEntity {
    fn from(domain: &ContainerRack) -> Self {
        Self {
            container_id: domain.container_id,
            rack_id: domain.rack_id,
            sort_order: domain.sort_order,
            created: domain.created,
            deleted: domain.deleted,
            version: domain.version,
        }
    }
}

#[async_trait]
pub trait ContainerRackService {
    type Context: Send + Sync + Clone + Eq + std::fmt::Debug + 'static;
    type Transaction;

    /// Add a container to a rack
    async fn add_container_to_rack(
        &self,
        container_id: Uuid,
        rack_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<ContainerRack, ServiceError>;

    /// Remove a container from a rack (soft delete)
    async fn remove_container_from_rack(
        &self,
        container_id: Uuid,
        rack_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<(), ServiceError>;

    /// Get all racks that contain a specific container
    async fn get_racks_for_container(
        &self,
        container_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[ContainerRack]>, ServiceError>;

    /// Get all containers in a specific rack
    async fn get_containers_in_rack(
        &self,
        rack_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[ContainerRack]>, ServiceError>;

    /// Get a specific container-rack relationship
    async fn get_container_rack_relationship(
        &self,
        container_id: Uuid,
        rack_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Option<ContainerRack>, ServiceError>;

    /// Get all active container-rack relationships
    async fn get_all_relationships(
        &self,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[ContainerRack]>, ServiceError>;

    /// Reorder containers within a rack
    /// Takes a list of container_ids in the desired order
    async fn reorder_containers_in_rack(
        &self,
        rack_id: Uuid,
        container_order: Vec<Uuid>,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[ContainerRack]>, ServiceError>;

    /// Set a container's position within a rack
    async fn set_container_position_in_rack(
        &self,
        container_id: Uuid,
        rack_id: Uuid,
        new_position: i32,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<ContainerRack, ServiceError>;
}
