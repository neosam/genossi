use std::sync::Arc;

use async_trait::async_trait;
use inventurly_dao::{
    product::ProductDao,
    TransactionDao,
};
use inventurly_service::{
    permission::{Authentication, ADMIN_PRIVILEGE, PermissionService},
    product::{Product, ProductService},
    uuid_service::UuidService,
    ServiceError, ValidationFailureItem,
};
use uuid::Uuid;

use crate::gen_service_impl;

gen_service_impl! {
    struct ProductServiceImpl: ProductService = ProductServiceDeps {
        ProductDao: ProductDao<Transaction = Self::Transaction> = product_dao,
        PermissionService: inventurly_service::permission::PermissionService<Context = Self::Context> = permission_service,
        UuidService: UuidService = uuid_service,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
    }
}

const PRODUCT_SERVICE_PROCESS: &str = "product-service";

#[async_trait]
impl<Deps: ProductServiceDeps> ProductService for ProductServiceImpl<Deps> {
    type Context = Deps::Context;
    type Transaction = Deps::Transaction;

    async fn get_all(
        &self,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[Product]>, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;
        
        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context)
            .await?;
        
        let products = self
            .product_dao
            .all(tx.clone())
            .await?
            .iter()
            .map(Product::from)
            .collect();
        
        self.transaction_dao.commit(tx).await?;
        Ok(products)
    }

    async fn get_by_ean(
        &self,
        ean: &str,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Product, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;
        
        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context)
            .await?;
        
        let product = self
            .product_dao
            .find_by_ean(ean, tx.clone())
            .await?
            .map(|e| Product::from(&e))
            .ok_or(ServiceError::EntityNotFound(Uuid::nil()))?;
        
        self.transaction_dao.commit(tx).await?;
        Ok(product)
    }

    async fn get_by_id(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Product, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;
        
        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context)
            .await?;
        
        let product = self
            .product_dao
            .find_by_id(id, tx.clone())
            .await?
            .map(|e| Product::from(&e))
            .ok_or(ServiceError::EntityNotFound(id))?;
        
        self.transaction_dao.commit(tx).await?;
        Ok(product)
    }

    async fn create(
        &self,
        item: &Product,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Product, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;
        
        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context.clone())
            .await?;

        let mut validation_errors = Vec::new();
        
        // Validate EAN
        if item.ean.is_empty() {
            validation_errors.push(ValidationFailureItem {
                field: Arc::from("ean"),
                message: Arc::from("EAN cannot be empty"),
            });
        }
        
        // Validate name
        if item.name.is_empty() {
            validation_errors.push(ValidationFailureItem {
                field: Arc::from("name"),
                message: Arc::from("Name cannot be empty"),
            });
        }
        
        // Validate short_name
        if item.short_name.is_empty() {
            validation_errors.push(ValidationFailureItem {
                field: Arc::from("short_name"),
                message: Arc::from("Short name cannot be empty"),
            });
        }
        
        // Validate sales_unit
        if item.sales_unit.is_empty() {
            validation_errors.push(ValidationFailureItem {
                field: Arc::from("sales_unit"),
                message: Arc::from("Sales unit cannot be empty"),
            });
        }
        
        // Validate price
        if item.price.to_cents() < 0 {
            validation_errors.push(ValidationFailureItem {
                field: Arc::from("price"),
                message: Arc::from("Price cannot be negative"),
            });
        }
        
        if !validation_errors.is_empty() {
            return Err(ServiceError::ValidationError(validation_errors));
        }

        // Check if EAN already exists
        if let Some(_) = self.product_dao.find_by_ean(&item.ean, tx.clone()).await? {
            validation_errors.push(ValidationFailureItem {
                field: Arc::from("ean"),
                message: Arc::from("EAN already exists"),
            });
            return Err(ServiceError::ValidationError(validation_errors));
        }

        let now = time::OffsetDateTime::now_utc();
        let new_product = Product {
            id: self.uuid_service.new_v4().await,
            ean: item.ean.clone(),
            name: item.name.clone(),
            short_name: item.short_name.clone(),
            sales_unit: item.sales_unit.clone(),
            requires_weighing: item.requires_weighing,
            price: item.price,
            created: time::PrimitiveDateTime::new(now.date(), now.time()),
            deleted: None,
            version: self.uuid_service.new_v4().await,
        };

        self.product_dao
            .create(&(&new_product).into(), PRODUCT_SERVICE_PROCESS, tx.clone())
            .await?;
        
        self.transaction_dao.commit(tx).await?;
        Ok(new_product)
    }

    async fn update(
        &self,
        item: &Product,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Product, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;
        
        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context.clone())
            .await?;

        // First check if the product exists
        let existing = self
            .product_dao
            .find_by_id(item.id, tx.clone())
            .await?;
        
        if existing.is_none() {
            return Err(ServiceError::EntityNotFound(item.id));
        }

        let mut validation_errors = Vec::new();
        
        // Validate EAN
        if item.ean.is_empty() {
            validation_errors.push(ValidationFailureItem {
                field: Arc::from("ean"),
                message: Arc::from("EAN cannot be empty"),
            });
        }
        
        // Validate name
        if item.name.is_empty() {
            validation_errors.push(ValidationFailureItem {
                field: Arc::from("name"),
                message: Arc::from("Name cannot be empty"),
            });
        }
        
        // Validate short_name
        if item.short_name.is_empty() {
            validation_errors.push(ValidationFailureItem {
                field: Arc::from("short_name"),
                message: Arc::from("Short name cannot be empty"),
            });
        }
        
        // Validate sales_unit
        if item.sales_unit.is_empty() {
            validation_errors.push(ValidationFailureItem {
                field: Arc::from("sales_unit"),
                message: Arc::from("Sales unit cannot be empty"),
            });
        }
        
        // Validate price
        if item.price.to_cents() < 0 {
            validation_errors.push(ValidationFailureItem {
                field: Arc::from("price"),
                message: Arc::from("Price cannot be negative"),
            });
        }
        
        if !validation_errors.is_empty() {
            return Err(ServiceError::ValidationError(validation_errors));
        }

        // Check if new EAN already exists (if EAN changed)
        if existing.as_ref().unwrap().ean != item.ean {
            if let Some(_) = self.product_dao.find_by_ean(&item.ean, tx.clone()).await? {
                validation_errors.push(ValidationFailureItem {
                    field: Arc::from("ean"),
                    message: Arc::from("EAN already exists"),
                });
                return Err(ServiceError::ValidationError(validation_errors));
            }
        }

        self.product_dao
            .update(&item.into(), PRODUCT_SERVICE_PROCESS, tx.clone())
            .await?;
        
        let updated = self
            .product_dao
            .find_by_id(item.id, tx.clone())
            .await?
            .map(|e| Product::from(&e))
            .ok_or(ServiceError::EntityNotFound(item.id))?;
        
        self.transaction_dao.commit(tx).await?;
        Ok(updated)
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
        
        // Fetch the existing entity
        let existing = self
            .product_dao
            .find_by_id(id, tx.clone())
            .await?;
        
        match existing {
            Some(mut entity) => {
                // Set deleted timestamp
                let now = time::OffsetDateTime::now_utc();
                entity.deleted = Some(time::PrimitiveDateTime::new(now.date(), now.time()));
                
                // Update the entity with deleted timestamp
                self.product_dao
                    .update(&entity, PRODUCT_SERVICE_PROCESS, tx.clone())
                    .await?;
                
                self.transaction_dao.commit(tx).await?;
                Ok(())
            }
            None => Err(ServiceError::EntityNotFound(id))
        }
    }
}