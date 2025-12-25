use async_trait::async_trait;
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

use inventurly_dao::{
    product::ProductDao,
    product_rack::{ProductRackDao, ProductRackEntity},
    rack::RackDao,
    TransactionDao,
};
use inventurly_service::{
    permission::{Authentication, PermissionService},
    product_rack::{ProductRack, ProductRackService},
    uuid_service::UuidService,
    ServiceError, ValidationFailureItem,
};

const ADMIN_PRIVILEGE: &str = "admin";
const PRODUCT_RACK_SERVICE_PROCESS: &str = "product-rack-service";

use crate::gen_service_impl;

gen_service_impl!(
    struct ProductRackServiceImpl : ProductRackService = ProductRackServiceDependencies {
        ProductRackDao: ProductRackDao<Transaction = Self::Transaction> = product_rack_dao,
        ProductDao: ProductDao<Transaction = Self::Transaction> = product_dao,
        RackDao: RackDao<Transaction = Self::Transaction> = rack_dao,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
        PermissionService: PermissionService<Context = Self::Context> = permission_service,
        UuidService: UuidService = uuid_service
    }
);

#[async_trait]
impl<Deps: ProductRackServiceDependencies> ProductRackService for ProductRackServiceImpl<Deps> {
    type Context = Deps::Context;
    type Transaction = Deps::Transaction;

    async fn add_product_to_rack(
        &self,
        product_id: Uuid,
        rack_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<ProductRack, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context)
            .await?;

        // Check if product exists
        let product_exists = self
            .product_dao
            .find_by_id(product_id, tx.clone())
            .await?
            .is_some();
        if !product_exists {
            return Err(ServiceError::EntityNotFound(product_id));
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
            .product_rack_dao
            .find_by_product_and_rack(product_id, rack_id, tx.clone())
            .await?;

        if let Some(existing_relationship) = existing {
            if existing_relationship.deleted.is_none() {
                return Err(ServiceError::ValidationError(vec![ValidationFailureItem {
                    field: Arc::from("relationship"),
                    message: Arc::from("Product is already assigned to this rack"),
                }]));
            }
        }

        // Get next sort_order for this rack
        let next_sort_order = self
            .product_rack_dao
            .get_next_sort_order(rack_id, tx.clone())
            .await?;

        // Create new relationship
        let now = OffsetDateTime::now_utc();
        let new_product_rack = ProductRack {
            product_id,
            rack_id,
            sort_order: next_sort_order,
            created: time::PrimitiveDateTime::new(now.date(), now.time()),
            deleted: None,
            version: self.uuid_service.new_v4().await,
        };

        let entity = ProductRackEntity::from(&new_product_rack);
        self.product_rack_dao
            .create(&entity, PRODUCT_RACK_SERVICE_PROCESS, tx.clone())
            .await?;

        self.transaction_dao.commit(tx).await?;
        Ok(new_product_rack)
    }

    async fn remove_product_from_rack(
        &self,
        product_id: Uuid,
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
            .product_rack_dao
            .find_by_product_and_rack(product_id, rack_id, tx.clone())
            .await?;

        let mut relationship = match existing {
            Some(rel) if rel.deleted.is_none() => ProductRack::from(&rel),
            _ => return Err(ServiceError::EntityNotFound(product_id)),
        };

        // Perform soft delete
        let now = OffsetDateTime::now_utc();
        relationship.deleted = Some(time::PrimitiveDateTime::new(now.date(), now.time()));

        let entity = ProductRackEntity::from(&relationship);
        self.product_rack_dao
            .update(&entity, PRODUCT_RACK_SERVICE_PROCESS, tx.clone())
            .await?;

        self.transaction_dao.commit(tx).await?;
        Ok(())
    }

    async fn get_racks_for_product(
        &self,
        product_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[ProductRack]>, ServiceError> {
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
            .product_rack_dao
            .find_racks_by_product(product_id, tx.clone())
            .await?;
        let relationships: Vec<ProductRack> = entities.iter().map(ProductRack::from).collect();

        self.transaction_dao.commit(tx).await?;
        Ok(relationships.into())
    }

    async fn get_products_in_rack(
        &self,
        rack_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[ProductRack]>, ServiceError> {
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
            .product_rack_dao
            .find_products_by_rack(rack_id, tx.clone())
            .await?;
        let relationships: Vec<ProductRack> = entities.iter().map(ProductRack::from).collect();

        self.transaction_dao.commit(tx).await?;
        Ok(relationships.into())
    }

    async fn get_product_rack_relationship(
        &self,
        product_id: Uuid,
        rack_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Option<ProductRack>, ServiceError> {
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
            .product_rack_dao
            .find_by_product_and_rack(product_id, rack_id, tx.clone())
            .await?;

        let result = entity
            .filter(|e| e.deleted.is_none())
            .map(|e| ProductRack::from(&e));

        self.transaction_dao.commit(tx).await?;
        Ok(result)
    }

    async fn get_all_relationships(
        &self,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[ProductRack]>, ServiceError> {
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

        let entities = self.product_rack_dao.all(tx.clone()).await?;
        let relationships: Vec<ProductRack> = entities.iter().map(ProductRack::from).collect();

        self.transaction_dao.commit(tx).await?;
        Ok(relationships.into())
    }

    async fn reorder_products_in_rack(
        &self,
        rack_id: Uuid,
        product_order: Vec<Uuid>,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[ProductRack]>, ServiceError> {
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

        // Get current products in rack
        let current_products = self
            .product_rack_dao
            .find_products_by_rack(rack_id, tx.clone())
            .await?;

        // Validate all products in order list are in the rack
        let current_product_ids: std::collections::HashSet<Uuid> =
            current_products.iter().map(|p| p.product_id).collect();

        for product_id in &product_order {
            if !current_product_ids.contains(product_id) {
                return Err(ServiceError::ValidationError(vec![ValidationFailureItem {
                    field: Arc::from("product_order"),
                    message: Arc::from(format!("Product {} is not in this rack", product_id)),
                }]));
            }
        }

        // Update sort_order for each product
        for (index, product_id) in product_order.iter().enumerate() {
            let product_rack = current_products
                .iter()
                .find(|p| p.product_id == *product_id)
                .unwrap();

            let mut updated = ProductRack::from(product_rack);
            updated.sort_order = (index + 1) as i32;

            let entity = ProductRackEntity::from(&updated);
            self.product_rack_dao
                .update(&entity, PRODUCT_RACK_SERVICE_PROCESS, tx.clone())
                .await?;
        }

        // Fetch updated list
        let updated_products = self
            .product_rack_dao
            .find_products_by_rack(rack_id, tx.clone())
            .await?;
        let relationships: Vec<ProductRack> =
            updated_products.iter().map(ProductRack::from).collect();

        self.transaction_dao.commit(tx).await?;
        Ok(relationships.into())
    }

    async fn set_product_position_in_rack(
        &self,
        product_id: Uuid,
        rack_id: Uuid,
        new_position: i32,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<ProductRack, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context)
            .await?;

        // Find the existing relationship
        let existing = self
            .product_rack_dao
            .find_by_product_and_rack(product_id, rack_id, tx.clone())
            .await?;

        let product_rack = match existing {
            Some(rel) if rel.deleted.is_none() => ProductRack::from(&rel),
            _ => return Err(ServiceError::EntityNotFound(product_id)),
        };

        // Update the position
        let mut updated = product_rack;
        updated.sort_order = new_position;

        let entity = ProductRackEntity::from(&updated);
        self.product_rack_dao
            .update(&entity, PRODUCT_RACK_SERVICE_PROCESS, tx.clone())
            .await?;

        // Fetch updated record
        let updated_entity = self
            .product_rack_dao
            .find_by_product_and_rack(product_id, rack_id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(product_id))?;

        self.transaction_dao.commit(tx).await?;
        Ok(ProductRack::from(&updated_entity))
    }
}
