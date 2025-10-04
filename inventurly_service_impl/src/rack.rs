use std::sync::Arc;

use async_trait::async_trait;
use inventurly_dao::{
    rack::RackDao,
    TransactionDao,
};
use inventurly_service::{
    permission::{Authentication, ADMIN_PRIVILEGE, PermissionService},
    rack::{Rack, RackService},
    uuid_service::UuidService,
    ServiceError, ValidationFailureItem,
};
use uuid::Uuid;

use crate::gen_service_impl;

gen_service_impl! {
    struct RackServiceImpl: RackService = RackServiceDeps {
        RackDao: RackDao<Transaction = Self::Transaction> = rack_dao,
        PermissionService: inventurly_service::permission::PermissionService<Context = Self::Context> = permission_service,
        UuidService: UuidService = uuid_service,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
    }
}

const RACK_SERVICE_PROCESS: &str = "rack-service";

#[async_trait]
impl<Deps: RackServiceDeps> RackService for RackServiceImpl<Deps> {
    type Context = Deps::Context;
    type Transaction = Deps::Transaction;

    async fn get_all(
        &self,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[Rack]>, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;
        
        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context)
            .await?;

        let entities = self.rack_dao.all(tx.clone()).await?;
        let racks: Vec<Rack> = entities.iter().map(Rack::from).collect();
        
        self.transaction_dao.commit(tx).await?;
        Ok(racks.into())
    }

    async fn get_by_id(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Option<Rack>, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;
        
        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context)
            .await?;

        let entity = self.rack_dao.find_by_id(id, tx.clone()).await?;
        let result = entity.map(|e| Rack::from(&e));
        
        self.transaction_dao.commit(tx).await?;
        Ok(result)
    }

    async fn create(
        &self,
        rack: &Rack,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Rack, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;
        
        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context)
            .await?;

        // Validate rack
        self.validate_rack(rack)?;

        // Create new rack with generated ID and version
        let now = time::OffsetDateTime::now_utc();
        let new_rack = Rack {
            id: self.uuid_service.new_v4().await,
            name: rack.name.clone(),
            description: rack.description.clone(),
            created: time::PrimitiveDateTime::new(now.date(), now.time()),
            deleted: None,
            version: self.uuid_service.new_v4().await,
        };

        let entity = inventurly_dao::rack::RackEntity::from(&new_rack);
        self.rack_dao.create(&entity, RACK_SERVICE_PROCESS, tx.clone()).await?;

        self.transaction_dao.commit(tx).await?;
        Ok(new_rack)
    }

    async fn update(
        &self,
        rack: &Rack,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Rack, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;
        
        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context)
            .await?;

        // Validate rack
        self.validate_rack(rack)?;

        // Check if rack exists
        let existing = self.rack_dao.find_by_id(rack.id, tx.clone()).await?;
        if existing.is_none() {
            return Err(ServiceError::EntityNotFound(rack.id));
        }

        // Update rack with new version
        let updated_rack = Rack {
            id: rack.id,
            name: rack.name.clone(),
            description: rack.description.clone(),
            created: rack.created,
            deleted: rack.deleted,
            version: self.uuid_service.new_v4().await,
        };

        let entity = inventurly_dao::rack::RackEntity::from(&updated_rack);
        self.rack_dao.update(&entity, RACK_SERVICE_PROCESS, tx.clone()).await?;

        self.transaction_dao.commit(tx).await?;
        Ok(updated_rack)
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

        // Check if rack exists
        let existing = self.rack_dao.find_by_id(id, tx.clone()).await?;
        if let Some(rack) = existing {
            // Perform soft delete
            let now = time::OffsetDateTime::now_utc();
            let deleted_rack = Rack {
                id: rack.id,
                name: rack.name,
                description: rack.description,
                created: rack.created,
                deleted: Some(time::PrimitiveDateTime::new(now.date(), now.time())),
                version: self.uuid_service.new_v4().await,
            };

            let entity = inventurly_dao::rack::RackEntity::from(&deleted_rack);
            self.rack_dao.update(&entity, RACK_SERVICE_PROCESS, tx.clone()).await?;
        } else {
            return Err(ServiceError::EntityNotFound(id));
        }

        self.transaction_dao.commit(tx).await?;
        Ok(())
    }
}

impl<Deps: RackServiceDeps> RackServiceImpl<Deps> {
    fn validate_rack(&self, rack: &Rack) -> Result<(), ServiceError> {
        let mut errors = Vec::new();

        if rack.name.trim().is_empty() {
            errors.push(ValidationFailureItem {
                field: "name".into(),
                message: "Name cannot be empty".into(),
            });
        }

        if rack.description.trim().is_empty() {
            errors.push(ValidationFailureItem {
                field: "description".into(),
                message: "Description cannot be empty".into(),
            });
        }

        if !errors.is_empty() {
            return Err(ServiceError::ValidationError(errors));
        }

        Ok(())
    }
}