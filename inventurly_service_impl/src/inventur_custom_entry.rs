use std::sync::Arc;

use async_trait::async_trait;
use inventurly_dao::{
    inventur::InventurDao, inventur_custom_entry::InventurCustomEntryDao, TransactionDao
};
use inventurly_service::{
    inventur_custom_entry::{InventurCustomEntry, InventurCustomEntryService},
    permission::{Authentication, PermissionService},
    uuid_service::UuidService,
    ServiceError, ValidationFailureItem,
};
use uuid::Uuid;

use crate::gen_service_impl;

gen_service_impl! {
    struct InventurCustomEntryServiceImpl: InventurCustomEntryService = InventurCustomEntryServiceDeps {
        InventurCustomEntryDao: InventurCustomEntryDao<Transaction = Self::Transaction> = inventur_custom_entry_dao,
        InventurDao: InventurDao<Transaction = Self::Transaction> = inventur_dao,
        PermissionService: inventurly_service::permission::PermissionService<Context = Self::Context> = permission_service,
        UuidService: UuidService = uuid_service,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
    }
}

const INVENTUR_CUSTOM_ENTRY_SERVICE_PROCESS: &str = "inventur-custom-entry-service";
const VIEW_INVENTUR_PRIVILEGE: &str = "view_inventur";
const PERFORM_INVENTUR_PRIVILEGE: &str = "perform_inventur";

const STATUS_ACTIVE: &str = "active";

#[async_trait]
impl<Deps: InventurCustomEntryServiceDeps> InventurCustomEntryService
    for InventurCustomEntryServiceImpl<Deps>
{
    type Context = Deps::Context;
    type Transaction = Deps::Transaction;

    async fn get_all(
        &self,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[InventurCustomEntry]>, ServiceError> {
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

        let entries: Arc<[InventurCustomEntry]> = self
            .inventur_custom_entry_dao
            .all(tx.clone())
            .await?
            .iter()
            .map(InventurCustomEntry::from)
            .filter(|e| {
                // Filter based on claims if present
                match claimed_inventur_id {
                    Some(id) => e.inventur_id == id,
                    None => true, // Global access
                }
            })
            .collect();

        self.transaction_dao.commit(tx).await?;
        Ok(entries)
    }

    async fn get_by_id(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<InventurCustomEntry, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        let entry = self
            .inventur_custom_entry_dao
            .find_by_id(id, tx.clone())
            .await?
            .map(|e| InventurCustomEntry::from(&e))
            .ok_or(ServiceError::EntityNotFound(id))?;

        // Check permission based on the entry's inventur_id
        self.permission_service
            .check_inventur_permission(VIEW_INVENTUR_PRIVILEGE, entry.inventur_id, context)
            .await?;

        self.transaction_dao.commit(tx).await?;
        Ok(entry)
    }

    async fn get_by_inventur_id(
        &self,
        inventur_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[InventurCustomEntry]>, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        self.permission_service
            .check_inventur_permission(VIEW_INVENTUR_PRIVILEGE, inventur_id, context)
            .await?;

        let entries = self
            .inventur_custom_entry_dao
            .find_by_inventur_id(inventur_id, tx.clone())
            .await?
            .iter()
            .map(InventurCustomEntry::from)
            .collect();

        self.transaction_dao.commit(tx).await?;
        Ok(entries)
    }

    async fn create(
        &self,
        item: &InventurCustomEntry,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<InventurCustomEntry, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        self.permission_service
            .check_inventur_permission(PERFORM_INVENTUR_PRIVILEGE, item.inventur_id, context.clone())
            .await?;

        // Extract user_id from authentication context
        let user_id = self
            .permission_service
            .current_user_id(context)
            .await?
            .map(|id| Arc::from(id.as_str()))
            .unwrap_or_else(|| Arc::from("SYSTEM"));

        // Validate: inventur must be in active status
        let inventur = self
            .inventur_dao
            .find_by_id(item.inventur_id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(item.inventur_id))?;

        if inventur.status.as_ref() != STATUS_ACTIVE {
            return Err(ServiceError::ValidationError(vec![
                ValidationFailureItem {
                    field: Arc::from("inventur_id"),
                    message: Arc::from(format!(
                        "Cannot record custom entries for inventur with status '{}'",
                        inventur.status
                    )),
                },
            ]));
        }

        // Validate: at least one of count or weight_grams must be provided
        let mut validation_errors = Vec::new();

        if item.count.is_none() && item.weight_grams.is_none() {
            validation_errors.push(ValidationFailureItem {
                field: Arc::from("count"),
                message: Arc::from("At least one of 'count' or 'weight_grams' must be provided"),
            });
            validation_errors.push(ValidationFailureItem {
                field: Arc::from("weight_grams"),
                message: Arc::from("At least one of 'count' or 'weight_grams' must be provided"),
            });
        }

        // Validate: custom_product_name must not be empty
        if item.custom_product_name.is_empty() {
            validation_errors.push(ValidationFailureItem {
                field: Arc::from("custom_product_name"),
                message: Arc::from("Product name cannot be empty"),
            });
        }

        if !validation_errors.is_empty() {
            return Err(ServiceError::ValidationError(validation_errors));
        }

        // Create custom entry
        let mut entry = item.clone();
        entry.id = self.uuid_service.new_v4().await;
        entry.version = self.uuid_service.new_v4().await;
        entry.measured_by = user_id; // Set from authenticated user
        let now = time::OffsetDateTime::now_utc();
        entry.created = time::PrimitiveDateTime::new(now.date(), now.time());

        // Set measured_at to now if not provided
        if entry.measured_at == time::PrimitiveDateTime::MIN {
            entry.measured_at = entry.created;
        }

        let entity =
            inventurly_dao::inventur_custom_entry::InventurCustomEntryEntity::from(&entry);
        self.inventur_custom_entry_dao
            .create(&entity, INVENTUR_CUSTOM_ENTRY_SERVICE_PROCESS, tx.clone())
            .await?;

        self.transaction_dao.commit(tx).await?;
        Ok(entry)
    }

    async fn update(
        &self,
        item: &InventurCustomEntry,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<InventurCustomEntry, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        self.permission_service
            .check_inventur_permission(PERFORM_INVENTUR_PRIVILEGE, item.inventur_id, context)
            .await?;

        // Check if the entity exists
        let _existing = self
            .inventur_custom_entry_dao
            .find_by_id(item.id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(item.id))?;

        // Validate: inventur must still be in active status
        let inventur = self
            .inventur_dao
            .find_by_id(item.inventur_id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(item.inventur_id))?;

        if inventur.status.as_ref() != STATUS_ACTIVE {
            return Err(ServiceError::ValidationError(vec![
                ValidationFailureItem {
                    field: Arc::from("inventur_id"),
                    message: Arc::from(format!(
                        "Cannot update custom entries for inventur with status '{}'",
                        inventur.status
                    )),
                },
            ]));
        }

        // Validate: at least one of count or weight_grams must be provided
        let mut validation_errors = Vec::new();

        if item.count.is_none() && item.weight_grams.is_none() {
            validation_errors.push(ValidationFailureItem {
                field: Arc::from("count"),
                message: Arc::from("At least one of 'count' or 'weight_grams' must be provided"),
            });
            validation_errors.push(ValidationFailureItem {
                field: Arc::from("weight_grams"),
                message: Arc::from("At least one of 'count' or 'weight_grams' must be provided"),
            });
        }

        // Validate: custom_product_name must not be empty
        if item.custom_product_name.is_empty() {
            validation_errors.push(ValidationFailureItem {
                field: Arc::from("custom_product_name"),
                message: Arc::from("Product name cannot be empty"),
            });
        }

        if !validation_errors.is_empty() {
            return Err(ServiceError::ValidationError(validation_errors));
        }

        let entity =
            inventurly_dao::inventur_custom_entry::InventurCustomEntryEntity::from(item);
        self.inventur_custom_entry_dao
            .update(&entity, INVENTUR_CUSTOM_ENTRY_SERVICE_PROCESS, tx.clone())
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
            .inventur_custom_entry_dao
            .find_by_id(id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(id))?;

        // Check permission based on the entry's inventur_id
        self.permission_service
            .check_inventur_permission(PERFORM_INVENTUR_PRIVILEGE, existing.inventur_id, context)
            .await?;

        let mut entry = InventurCustomEntry::from(&existing);
        let now = time::OffsetDateTime::now_utc();
        entry.deleted = Some(time::PrimitiveDateTime::new(now.date(), now.time()));

        let entity =
            inventurly_dao::inventur_custom_entry::InventurCustomEntryEntity::from(&entry);
        self.inventur_custom_entry_dao
            .update(&entity, INVENTUR_CUSTOM_ENTRY_SERVICE_PROCESS, tx.clone())
            .await?;

        self.transaction_dao.commit(tx).await?;
        Ok(())
    }
}
