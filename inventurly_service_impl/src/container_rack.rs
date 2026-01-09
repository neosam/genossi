use async_trait::async_trait;
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

use inventurly_dao::{
    container::ContainerDao,
    container_rack::{ContainerRackDao, ContainerRackEntity},
    rack::RackDao,
    TransactionDao,
};
use inventurly_service::{
    permission::{Authentication, PermissionService},
    container_rack::{ContainerRack, ContainerRackService},
    uuid_service::UuidService,
    ServiceError, ValidationFailureItem,
};

const ADMIN_PRIVILEGE: &str = "admin";
const CONTAINER_RACK_SERVICE_PROCESS: &str = "container-rack-service";

use crate::gen_service_impl;

gen_service_impl!(
    struct ContainerRackServiceImpl : ContainerRackService = ContainerRackServiceDependencies {
        ContainerRackDao: ContainerRackDao<Transaction = Self::Transaction> = container_rack_dao,
        ContainerDao: ContainerDao<Transaction = Self::Transaction> = container_dao,
        RackDao: RackDao<Transaction = Self::Transaction> = rack_dao,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
        PermissionService: PermissionService<Context = Self::Context> = permission_service,
        UuidService: UuidService = uuid_service
    }
);

#[async_trait]
impl<Deps: ContainerRackServiceDependencies> ContainerRackService for ContainerRackServiceImpl<Deps> {
    type Context = Deps::Context;
    type Transaction = Deps::Transaction;

    async fn add_container_to_rack(
        &self,
        container_id: Uuid,
        rack_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<ContainerRack, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context)
            .await?;

        // Check if container exists
        let container_exists = self
            .container_dao
            .find_by_id(container_id, tx.clone())
            .await?
            .is_some();
        if !container_exists {
            return Err(ServiceError::EntityNotFound(container_id));
        }

        // Check if rack exists
        let rack_exists = self
            .rack_dao
            .find_by_id(rack_id, tx.clone())
            .await?
            .is_some();
        if !rack_exists {
            return Err(ServiceError::EntityNotFound(rack_id));
        }

        // Check if relationship already exists
        let existing = self
            .container_rack_dao
            .find_by_container_and_rack(container_id, rack_id, tx.clone())
            .await?;

        if let Some(existing_relationship) = existing {
            if existing_relationship.deleted.is_none() {
                return Err(ServiceError::ValidationError(vec![ValidationFailureItem {
                    field: Arc::from("relationship"),
                    message: Arc::from("Container is already assigned to this rack"),
                }]));
            }

            // Reactivate the soft-deleted relationship
            let next_sort_order = self
                .container_rack_dao
                .get_next_sort_order(rack_id, tx.clone())
                .await?;

            let now = OffsetDateTime::now_utc();
            let reactivated = ContainerRack {
                container_id,
                rack_id,
                sort_order: next_sort_order,
                created: time::PrimitiveDateTime::new(now.date(), now.time()),
                deleted: None,
                version: self.uuid_service.new_v4().await,
            };

            let entity = ContainerRackEntity::from(&reactivated);
            self.container_rack_dao
                .reactivate(&entity, CONTAINER_RACK_SERVICE_PROCESS, tx.clone())
                .await?;

            self.transaction_dao.commit(tx).await?;
            return Ok(reactivated);
        }

        // Get next sort_order for this rack
        let next_sort_order = self
            .container_rack_dao
            .get_next_sort_order(rack_id, tx.clone())
            .await?;

        // Create new relationship
        let now = OffsetDateTime::now_utc();
        let new_container_rack = ContainerRack {
            container_id,
            rack_id,
            sort_order: next_sort_order,
            created: time::PrimitiveDateTime::new(now.date(), now.time()),
            deleted: None,
            version: self.uuid_service.new_v4().await,
        };

        let entity = ContainerRackEntity::from(&new_container_rack);
        self.container_rack_dao
            .create(&entity, CONTAINER_RACK_SERVICE_PROCESS, tx.clone())
            .await?;

        self.transaction_dao.commit(tx).await?;
        Ok(new_container_rack)
    }

    async fn remove_container_from_rack(
        &self,
        container_id: Uuid,
        rack_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<(), ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context)
            .await?;

        // Find existing relationship
        let existing = self
            .container_rack_dao
            .find_by_container_and_rack(container_id, rack_id, tx.clone())
            .await?;

        let mut relationship = match existing {
            Some(rel) if rel.deleted.is_none() => ContainerRack::from(&rel),
            _ => return Err(ServiceError::EntityNotFound(container_id)),
        };

        // Perform soft delete
        let now = OffsetDateTime::now_utc();
        relationship.deleted = Some(time::PrimitiveDateTime::new(now.date(), now.time()));

        let entity = ContainerRackEntity::from(&relationship);
        self.container_rack_dao
            .update(&entity, CONTAINER_RACK_SERVICE_PROCESS, tx.clone())
            .await?;

        self.transaction_dao.commit(tx).await?;
        Ok(())
    }

    async fn get_racks_for_container(
        &self,
        container_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[ContainerRack]>, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        // Allow access if user has claims (inventur token) or admin privilege
        match &context {
            Authentication::Full => {}
            Authentication::Context(ctx) => {
                if !self.permission_service.has_claims(ctx).await? {
                    // No claims, check for admin privilege
                    self.permission_service
                        .check_permission(ADMIN_PRIVILEGE, context)
                        .await?;
                }
            }
        }

        let entities = self
            .container_rack_dao
            .find_racks_by_container(container_id, tx.clone())
            .await?;
        let relationships: Vec<ContainerRack> = entities.iter().map(ContainerRack::from).collect();

        self.transaction_dao.commit(tx).await?;
        Ok(relationships.into())
    }

    async fn get_containers_in_rack(
        &self,
        rack_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[ContainerRack]>, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        // Allow access if user has claims (inventur token) or admin privilege
        match &context {
            Authentication::Full => {}
            Authentication::Context(ctx) => {
                if !self.permission_service.has_claims(ctx).await? {
                    // No claims, check for admin privilege
                    self.permission_service
                        .check_permission(ADMIN_PRIVILEGE, context)
                        .await?;
                }
            }
        }

        let entities = self
            .container_rack_dao
            .find_containers_by_rack(rack_id, tx.clone())
            .await?;
        let relationships: Vec<ContainerRack> = entities.iter().map(ContainerRack::from).collect();

        self.transaction_dao.commit(tx).await?;
        Ok(relationships.into())
    }

    async fn get_container_rack_relationship(
        &self,
        container_id: Uuid,
        rack_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Option<ContainerRack>, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        // Allow access if user has claims (inventur token) or admin privilege
        match &context {
            Authentication::Full => {}
            Authentication::Context(ctx) => {
                if !self.permission_service.has_claims(ctx).await? {
                    // No claims, check for admin privilege
                    self.permission_service
                        .check_permission(ADMIN_PRIVILEGE, context)
                        .await?;
                }
            }
        }

        let entity = self
            .container_rack_dao
            .find_by_container_and_rack(container_id, rack_id, tx.clone())
            .await?;

        let result = entity
            .filter(|e| e.deleted.is_none())
            .map(|e| ContainerRack::from(&e));

        self.transaction_dao.commit(tx).await?;
        Ok(result)
    }

    async fn get_all_relationships(
        &self,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[ContainerRack]>, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        // Allow access if user has claims (inventur token) or admin privilege
        match &context {
            Authentication::Full => {}
            Authentication::Context(ctx) => {
                if !self.permission_service.has_claims(ctx).await? {
                    // No claims, check for admin privilege
                    self.permission_service
                        .check_permission(ADMIN_PRIVILEGE, context)
                        .await?;
                }
            }
        }

        let entities = self.container_rack_dao.all(tx.clone()).await?;
        let relationships: Vec<ContainerRack> = entities.iter().map(ContainerRack::from).collect();

        self.transaction_dao.commit(tx).await?;
        Ok(relationships.into())
    }

    async fn reorder_containers_in_rack(
        &self,
        rack_id: Uuid,
        container_order: Vec<Uuid>,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[ContainerRack]>, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context)
            .await?;

        // Validate rack exists
        let rack_exists = self
            .rack_dao
            .find_by_id(rack_id, tx.clone())
            .await?
            .is_some();
        if !rack_exists {
            return Err(ServiceError::EntityNotFound(rack_id));
        }

        // Get current containers in rack
        let current_containers = self
            .container_rack_dao
            .find_containers_by_rack(rack_id, tx.clone())
            .await?;

        // Validate all containers in order list are in the rack
        let current_container_ids: std::collections::HashSet<Uuid> =
            current_containers.iter().map(|c| c.container_id).collect();

        for container_id in &container_order {
            if !current_container_ids.contains(container_id) {
                return Err(ServiceError::ValidationError(vec![ValidationFailureItem {
                    field: Arc::from("container_order"),
                    message: Arc::from(format!("Container {} is not in this rack", container_id)),
                }]));
            }
        }

        // Update sort_order for each container
        for (index, container_id) in container_order.iter().enumerate() {
            let container_rack = current_containers
                .iter()
                .find(|c| c.container_id == *container_id)
                .unwrap();

            let mut updated = ContainerRack::from(container_rack);
            updated.sort_order = (index + 1) as i32;

            let entity = ContainerRackEntity::from(&updated);
            self.container_rack_dao
                .update(&entity, CONTAINER_RACK_SERVICE_PROCESS, tx.clone())
                .await?;
        }

        // Fetch updated list
        let updated_containers = self
            .container_rack_dao
            .find_containers_by_rack(rack_id, tx.clone())
            .await?;
        let relationships: Vec<ContainerRack> =
            updated_containers.iter().map(ContainerRack::from).collect();

        self.transaction_dao.commit(tx).await?;
        Ok(relationships.into())
    }

    async fn set_container_position_in_rack(
        &self,
        container_id: Uuid,
        rack_id: Uuid,
        new_position: i32,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<ContainerRack, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context)
            .await?;

        // Find the existing relationship
        let existing = self
            .container_rack_dao
            .find_by_container_and_rack(container_id, rack_id, tx.clone())
            .await?;

        let container_rack = match existing {
            Some(rel) if rel.deleted.is_none() => ContainerRack::from(&rel),
            _ => return Err(ServiceError::EntityNotFound(container_id)),
        };

        // Update the position
        let mut updated = container_rack;
        updated.sort_order = new_position;

        let entity = ContainerRackEntity::from(&updated);
        self.container_rack_dao
            .update(&entity, CONTAINER_RACK_SERVICE_PROCESS, tx.clone())
            .await?;

        // Fetch updated record
        let updated_entity = self
            .container_rack_dao
            .find_by_container_and_rack(container_id, rack_id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(container_id))?;

        self.transaction_dao.commit(tx).await?;
        Ok(ContainerRack::from(&updated_entity))
    }
}
