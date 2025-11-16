use async_trait::async_trait;
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

use inventurly_dao::{container::ContainerDao, TransactionDao};
use inventurly_service::{
    container::{Container, ContainerService},
    permission::{Authentication, PermissionService, ADMIN_PRIVILEGE},
    uuid_service::UuidService,
    ServiceError, ValidationFailureItem,
};

use crate::gen_service_impl;

gen_service_impl! {
    struct ContainerServiceImpl: ContainerService = ContainerServiceDeps {
        ContainerDao: ContainerDao<Transaction = Self::Transaction> = container_dao,
        PermissionService: PermissionService<Context = Self::Context> = permission_service,
        UuidService: UuidService = uuid_service,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
    }
}

const CONTAINER_SERVICE_PROCESS: &str = "container-service";

#[async_trait]
impl<Deps: ContainerServiceDeps> ContainerService for ContainerServiceImpl<Deps> {
    type Context = Deps::Context;
    type Transaction = Deps::Transaction;

    async fn get_all(
        &self,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[Container]>, ServiceError> {
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

        let entities = self.container_dao.all(tx.clone()).await?;
        let containers: Vec<Container> = entities.iter().map(Container::from).collect();

        self.transaction_dao.commit(tx).await?;
        Ok(containers.into())
    }

    async fn get_by_id(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Container, ServiceError> {
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
            .container_dao
            .find_by_id(id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(id))?;

        self.transaction_dao.commit(tx).await?;
        Ok(Container::from(&entity))
    }

    async fn get_by_name(
        &self,
        name: &str,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Container, ServiceError> {
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
            .container_dao
            .find_by_name(name, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(Uuid::nil()))?;

        self.transaction_dao.commit(tx).await?;
        Ok(Container::from(&entity))
    }

    async fn create(
        &self,
        item: &Container,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Container, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context)
            .await?;

        // Validate the container
        let mut validation_errors = Vec::new();

        if item.name.trim().is_empty() {
            validation_errors.push(ValidationFailureItem {
                field: Arc::from("name"),
                message: Arc::from("Container name cannot be empty"),
            });
        }

        if item.weight_grams <= 0 {
            validation_errors.push(ValidationFailureItem {
                field: Arc::from("weight_grams"),
                message: Arc::from("Container weight must be positive"),
            });
        }

        if !validation_errors.is_empty() {
            return Err(ServiceError::ValidationError(validation_errors));
        }

        // Generate new UUID and version if not provided
        let id = if item.id == Uuid::nil() {
            self.uuid_service.new_v4().await
        } else {
            item.id
        };

        let version = self.uuid_service.new_v4().await;
        let now = OffsetDateTime::now_utc();
        let created = time::PrimitiveDateTime::new(now.date(), now.time());

        let mut new_container = item.clone();
        new_container.id = id;
        new_container.version = version;
        new_container.created = created;
        new_container.deleted = None;

        let entity = inventurly_dao::container::ContainerEntity::from(&new_container);
        self.container_dao
            .create(&entity, CONTAINER_SERVICE_PROCESS, tx.clone())
            .await?;

        self.transaction_dao.commit(tx).await?;
        Ok(new_container)
    }

    async fn update(
        &self,
        item: &Container,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Container, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context)
            .await?;

        // Validate the container
        let mut validation_errors = Vec::new();

        if item.name.trim().is_empty() {
            validation_errors.push(ValidationFailureItem {
                field: Arc::from("name"),
                message: Arc::from("Container name cannot be empty"),
            });
        }

        if item.weight_grams <= 0 {
            validation_errors.push(ValidationFailureItem {
                field: Arc::from("weight_grams"),
                message: Arc::from("Container weight must be positive"),
            });
        }

        if !validation_errors.is_empty() {
            return Err(ServiceError::ValidationError(validation_errors));
        }

        // Check if container exists
        let existing = self
            .container_dao
            .find_by_id(item.id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(item.id))?;

        // Generate new version for optimistic locking
        let new_version = self.uuid_service.new_v4().await;

        let mut updated_container = item.clone();
        updated_container.version = new_version;
        updated_container.created = existing.created; // Preserve original creation time

        let entity = inventurly_dao::container::ContainerEntity::from(&updated_container);
        self.container_dao
            .update(&entity, CONTAINER_SERVICE_PROCESS, tx.clone())
            .await?;

        self.transaction_dao.commit(tx).await?;
        Ok(updated_container)
    }

    async fn delete(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<(), ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context)
            .await?;

        // Check if container exists
        let existing = self
            .container_dao
            .find_by_id(id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(id))?;

        // Soft delete by setting deleted timestamp
        let now = OffsetDateTime::now_utc();
        let deleted = time::PrimitiveDateTime::new(now.date(), now.time());
        let new_version = self.uuid_service.new_v4().await;

        let mut deleted_container = Container::from(&existing);
        deleted_container.deleted = Some(deleted);
        deleted_container.version = new_version;

        let entity = inventurly_dao::container::ContainerEntity::from(&deleted_container);
        self.container_dao
            .update(&entity, CONTAINER_SERVICE_PROCESS, tx.clone())
            .await?;

        self.transaction_dao.commit(tx).await?;
        Ok(())
    }

    async fn search(
        &self,
        query: &str,
        limit: Option<usize>,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[Container]>, ServiceError> {
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

        let entities = self.container_dao.search(query, limit, tx.clone()).await?;
        let containers: Vec<Container> = entities.iter().map(Container::from).collect();

        self.transaction_dao.commit(tx).await?;
        Ok(containers.into())
    }
}
