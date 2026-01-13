use std::sync::Arc;

use async_trait::async_trait;
use inventurly_dao::{
    inventur::InventurDao, inventur_measurement::InventurMeasurementDao,  product::ProductDao, TransactionDao
};
use inventurly_service::{
    inventur_measurement::{InventurMeasurement, InventurMeasurementService},
    permission::{Authentication, PermissionService},
    uuid_service::UuidService,
    ServiceError, ValidationFailureItem,
};
use uuid::Uuid;

use crate::gen_service_impl;

gen_service_impl! {
    struct InventurMeasurementServiceImpl: InventurMeasurementService = InventurMeasurementServiceDeps {
        InventurMeasurementDao: InventurMeasurementDao<Transaction = Self::Transaction> = inventur_measurement_dao,
        InventurDao: InventurDao<Transaction = Self::Transaction> = inventur_dao,
        ProductDao: ProductDao<Transaction = Self::Transaction> = product_dao,
        PermissionService: inventurly_service::permission::PermissionService<Context = Self::Context> = permission_service,
        UuidService: UuidService = uuid_service,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
    }
}

const INVENTUR_MEASUREMENT_SERVICE_PROCESS: &str = "inventur-measurement-service";
const VIEW_INVENTUR_PRIVILEGE: &str = "view_inventur";
const PERFORM_INVENTUR_PRIVILEGE: &str = "perform_inventur";

const STATUS_ACTIVE: &str = "active";
const STATUS_POST_PROCESSING: &str = "post_processing";

#[async_trait]
impl<Deps: InventurMeasurementServiceDeps> InventurMeasurementService
    for InventurMeasurementServiceImpl<Deps>
{
    type Context = Deps::Context;
    type Transaction = Deps::Transaction;

    async fn get_all(
        &self,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[InventurMeasurement]>, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        // Check if user has claims (token-based auth)
        let claimed_inventur_id = match &context {
            Authentication::Full => None,
            Authentication::Context(ctx) => self.permission_service.get_claimed_inventur_id(ctx).await?,
        };

        // If no claims, check global permission
        if claimed_inventur_id.is_none() {
            self.permission_service
                .check_permission(VIEW_INVENTUR_PRIVILEGE, context)
                .await?;
        }

        let measurements: Arc<[InventurMeasurement]> = self
            .inventur_measurement_dao
            .all(tx.clone())
            .await?
            .iter()
            .map(InventurMeasurement::from)
            .filter(|m| {
                // Filter based on claims if present
                match claimed_inventur_id {
                    Some(id) => m.inventur_id == id,
                    None => true, // Global access
                }
            })
            .collect();

        self.transaction_dao.commit(tx).await?;
        Ok(measurements)
    }

    async fn get_by_id(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<InventurMeasurement, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        let measurement = self
            .inventur_measurement_dao
            .find_by_id(id, tx.clone())
            .await?
            .map(|e| InventurMeasurement::from(&e))
            .ok_or(ServiceError::EntityNotFound(id))?;

        // Check permission based on the measurement's inventur_id
        self.permission_service
            .check_inventur_permission(VIEW_INVENTUR_PRIVILEGE, measurement.inventur_id, context)
            .await?;

        self.transaction_dao.commit(tx).await?;
        Ok(measurement)
    }

    async fn get_by_inventur_id(
        &self,
        inventur_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[InventurMeasurement]>, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        self.permission_service
            .check_inventur_permission(VIEW_INVENTUR_PRIVILEGE, inventur_id, context)
            .await?;

        let measurements = self
            .inventur_measurement_dao
            .find_by_inventur_id(inventur_id, tx.clone())
            .await?
            .iter()
            .map(InventurMeasurement::from)
            .collect();

        self.transaction_dao.commit(tx).await?;
        Ok(measurements)
    }

    async fn get_by_product_and_inventur(
        &self,
        product_id: Uuid,
        inventur_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[InventurMeasurement]>, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        self.permission_service
            .check_inventur_permission(VIEW_INVENTUR_PRIVILEGE, inventur_id, context)
            .await?;

        let measurements = self
            .inventur_measurement_dao
            .find_by_product_and_inventur(product_id, inventur_id, tx.clone())
            .await?
            .iter()
            .map(InventurMeasurement::from)
            .collect();

        self.transaction_dao.commit(tx).await?;
        Ok(measurements)
    }

    async fn create(
        &self,
        item: &InventurMeasurement,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<InventurMeasurement, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        self.permission_service
            .check_inventur_permission(PERFORM_INVENTUR_PRIVILEGE, item.inventur_id, context.clone())
            .await?;

        // Validate: inventur must be in active or post_processing status
        let inventur = self
            .inventur_dao
            .find_by_id(item.inventur_id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(item.inventur_id))?;

        let is_active = inventur.status.as_ref() == STATUS_ACTIVE;
        let is_post_processing = inventur.status.as_ref() == STATUS_POST_PROCESSING;

        if !is_active && !is_post_processing {
            return Err(ServiceError::ValidationError(vec![
                ValidationFailureItem {
                    field: Arc::from("inventur_id"),
                    message: Arc::from(format!(
                        "Cannot record measurements for inventur with status '{}'",
                        inventur.status
                    )),
                },
            ]));
        }

        // In post_processing state, only role-based users can create (not token-based)
        if is_post_processing {
            let has_claims = match &context {
                Authentication::Full => false,
                Authentication::Context(ctx) => {
                    self.permission_service.has_claims(ctx).await?
                }
            };
            if has_claims {
                return Err(ServiceError::PermissionDenied);
            }
        }

        // Extract user_id from authentication context
        let user_id = self
            .permission_service
            .current_user_id(context)
            .await?
            .map(|id| Arc::from(id.as_str()))
            .unwrap_or_else(|| Arc::from("SYSTEM"));

        // Validate: product must exist
        let product = self
            .product_dao
            .find_by_id(item.product_id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(item.product_id))?;

        // Validate: either count or weight_grams must be set based on product.requires_weighing
        let mut validation_errors = Vec::new();

        if product.requires_weighing && item.weight_grams.is_none() {
            validation_errors.push(ValidationFailureItem {
                field: Arc::from("weight_grams"),
                message: Arc::from("Weight is required for products that require weighing"),
            });
        }

        if !product.requires_weighing && item.count.is_none() {
            validation_errors.push(ValidationFailureItem {
                field: Arc::from("count"),
                message: Arc::from("Count is required for products that don't require weighing"),
            });
        }

        if !validation_errors.is_empty() {
            return Err(ServiceError::ValidationError(validation_errors));
        }

        // Create measurement
        let mut measurement = item.clone();
        measurement.id = self.uuid_service.new_v4().await;
        measurement.version = self.uuid_service.new_v4().await;
        measurement.measured_by = user_id; // Set from authenticated user
        let now = time::OffsetDateTime::now_utc();
        measurement.created = time::PrimitiveDateTime::new(now.date(), now.time());

        // Set measured_at to now if not provided
        if measurement.measured_at == time::PrimitiveDateTime::MIN {
            measurement.measured_at = measurement.created;
        }

        let entity =
            inventurly_dao::inventur_measurement::InventurMeasurementEntity::from(&measurement);
        self.inventur_measurement_dao
            .create(&entity, INVENTUR_MEASUREMENT_SERVICE_PROCESS, tx.clone())
            .await?;

        self.transaction_dao.commit(tx).await?;
        Ok(measurement)
    }

    async fn update(
        &self,
        item: &InventurMeasurement,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<InventurMeasurement, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        self.permission_service
            .check_inventur_permission(PERFORM_INVENTUR_PRIVILEGE, item.inventur_id, context.clone())
            .await?;

        // Check if the entity exists
        let _existing = self
            .inventur_measurement_dao
            .find_by_id(item.id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(item.id))?;

        // Validate: inventur must still be in active or post_processing status
        let inventur = self
            .inventur_dao
            .find_by_id(item.inventur_id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(item.inventur_id))?;

        let is_active = inventur.status.as_ref() == STATUS_ACTIVE;
        let is_post_processing = inventur.status.as_ref() == STATUS_POST_PROCESSING;

        if !is_active && !is_post_processing {
            return Err(ServiceError::ValidationError(vec![
                ValidationFailureItem {
                    field: Arc::from("inventur_id"),
                    message: Arc::from(format!(
                        "Cannot update measurements for inventur with status '{}'",
                        inventur.status
                    )),
                },
            ]));
        }

        // In post_processing state, only role-based users can update (not token-based)
        if is_post_processing {
            let has_claims = match &context {
                Authentication::Full => false,
                Authentication::Context(ctx) => {
                    self.permission_service.has_claims(ctx).await?
                }
            };
            if has_claims {
                return Err(ServiceError::PermissionDenied);
            }
        }

        let entity =
            inventurly_dao::inventur_measurement::InventurMeasurementEntity::from(item);
        self.inventur_measurement_dao
            .update(&entity, INVENTUR_MEASUREMENT_SERVICE_PROCESS, tx.clone())
            .await?;

        self.transaction_dao.commit(tx).await?;
        Ok(item.clone())
    }

    async fn delete(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<(), ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        let existing = self
            .inventur_measurement_dao
            .find_by_id(id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(id))?;

        // Check permission based on the measurement's inventur_id
        self.permission_service
            .check_inventur_permission(PERFORM_INVENTUR_PRIVILEGE, existing.inventur_id, context.clone())
            .await?;

        // Validate: inventur must be in active or post_processing status
        let inventur = self
            .inventur_dao
            .find_by_id(existing.inventur_id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(existing.inventur_id))?;

        let is_active = inventur.status.as_ref() == STATUS_ACTIVE;
        let is_post_processing = inventur.status.as_ref() == STATUS_POST_PROCESSING;

        if !is_active && !is_post_processing {
            return Err(ServiceError::ValidationError(vec![
                ValidationFailureItem {
                    field: Arc::from("inventur_id"),
                    message: Arc::from(format!(
                        "Cannot delete measurements for inventur with status '{}'",
                        inventur.status
                    )),
                },
            ]));
        }

        // In post_processing state, only role-based users can delete (not token-based)
        if is_post_processing {
            let has_claims = match &context {
                Authentication::Full => false,
                Authentication::Context(ctx) => {
                    self.permission_service.has_claims(ctx).await?
                }
            };
            if has_claims {
                return Err(ServiceError::PermissionDenied);
            }
        }

        let mut measurement = InventurMeasurement::from(&existing);
        let now = time::OffsetDateTime::now_utc();
        measurement.deleted = Some(time::PrimitiveDateTime::new(now.date(), now.time()));

        let entity =
            inventurly_dao::inventur_measurement::InventurMeasurementEntity::from(&measurement);
        self.inventur_measurement_dao
            .update(&entity, INVENTUR_MEASUREMENT_SERVICE_PROCESS, tx.clone())
            .await?;

        self.transaction_dao.commit(tx).await?;
        Ok(())
    }
}
