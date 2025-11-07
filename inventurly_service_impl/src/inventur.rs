use std::sync::Arc;

use async_trait::async_trait;
use inventurly_dao::{inventur::InventurDao, TransactionDao};
use inventurly_service::{
    inventur::{Inventur, InventurService},
    permission::{Authentication, PermissionService},
    uuid_service::UuidService,
    ServiceError, ValidationFailureItem,
};
use uuid::Uuid;

use crate::gen_service_impl;

gen_service_impl! {
    struct InventurServiceImpl: InventurService = InventurServiceDeps {
        InventurDao: InventurDao<Transaction = Self::Transaction> = inventur_dao,
        PermissionService: inventurly_service::permission::PermissionService<Context = Self::Context> = permission_service,
        UuidService: UuidService = uuid_service,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
    }
}

const INVENTUR_SERVICE_PROCESS: &str = "inventur-service";
const VIEW_INVENTUR_PRIVILEGE: &str = "view_inventur";
const MANAGE_INVENTUR_PRIVILEGE: &str = "manage_inventur";

// Valid status transitions
const STATUS_DRAFT: &str = "draft";
const STATUS_ACTIVE: &str = "active";
const STATUS_COMPLETED: &str = "completed";

fn validate_status_transition(current: &str, new: &str) -> Result<(), ServiceError> {
    let valid = match (current, new) {
        (STATUS_DRAFT, STATUS_ACTIVE) => true,
        (STATUS_ACTIVE, STATUS_COMPLETED) => true,
        (current, new) if current == new => true, // No change is OK
        _ => false,
    };

    if !valid {
        return Err(ServiceError::ValidationError(vec![
            ValidationFailureItem {
                field: Arc::from("status"),
                message: Arc::from(format!(
                    "Invalid status transition from '{}' to '{}'",
                    current, new
                )),
            },
        ]));
    }

    Ok(())
}

#[async_trait]
impl<Deps: InventurServiceDeps> InventurService for InventurServiceImpl<Deps> {
    type Context = Deps::Context;
    type Transaction = Deps::Transaction;

    async fn get_all(
        &self,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[Inventur]>, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        self.permission_service
            .check_permission(VIEW_INVENTUR_PRIVILEGE, context)
            .await?;

        let inventurs = self
            .inventur_dao
            .all(tx.clone())
            .await?
            .iter()
            .map(Inventur::from)
            .collect();

        self.transaction_dao.commit(tx).await?;
        Ok(inventurs)
    }

    async fn get_by_id(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Inventur, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        self.permission_service
            .check_permission(VIEW_INVENTUR_PRIVILEGE, context)
            .await?;

        let inventur = self
            .inventur_dao
            .find_by_id(id, tx.clone())
            .await?
            .map(|e| Inventur::from(&e))
            .ok_or(ServiceError::EntityNotFound(id))?;

        self.transaction_dao.commit(tx).await?;
        Ok(inventur)
    }

    async fn get_by_status(
        &self,
        status: &str,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[Inventur]>, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        self.permission_service
            .check_permission(VIEW_INVENTUR_PRIVILEGE, context)
            .await?;

        let inventurs = self
            .inventur_dao
            .find_by_status(status, tx.clone())
            .await?
            .iter()
            .map(Inventur::from)
            .collect();

        self.transaction_dao.commit(tx).await?;
        Ok(inventurs)
    }

    async fn create(
        &self,
        item: &Inventur,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Inventur, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        self.permission_service
            .check_permission(MANAGE_INVENTUR_PRIVILEGE, context.clone())
            .await?;

        // Extract user_id from authentication context
        let user_id = self
            .permission_service
            .current_user_id(context)
            .await?
            .map(|id| Arc::from(id.as_str()))
            .unwrap_or_else(|| Arc::from("SYSTEM"));

        // Validate: new inventur should start in draft status
        if item.status.as_ref() != STATUS_DRAFT {
            return Err(ServiceError::ValidationError(vec![
                ValidationFailureItem {
                    field: Arc::from("status"),
                    message: Arc::from("New inventur must have status 'draft'"),
                },
            ]));
        }

        let mut inventur = item.clone();
        inventur.id = self.uuid_service.new_v4().await;
        inventur.version = self.uuid_service.new_v4().await;
        inventur.created_by = user_id; // Set from authenticated user
        let now = time::OffsetDateTime::now_utc();
        inventur.created = time::PrimitiveDateTime::new(now.date(), now.time());

        let entity = inventurly_dao::inventur::InventurEntity::from(&inventur);
        self.inventur_dao
            .create(&entity, INVENTUR_SERVICE_PROCESS, tx.clone())
            .await?;

        self.transaction_dao.commit(tx).await?;
        Ok(inventur)
    }

    async fn update(
        &self,
        item: &Inventur,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Inventur, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        self.permission_service
            .check_permission(MANAGE_INVENTUR_PRIVILEGE, context)
            .await?;

        // Check if the entity exists and get current status
        let existing = self
            .inventur_dao
            .find_by_id(item.id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(item.id))?;

        // Validate status transition
        validate_status_transition(existing.status.as_ref(), item.status.as_ref())?;

        let entity = inventurly_dao::inventur::InventurEntity::from(item);
        self.inventur_dao
            .update(&entity, INVENTUR_SERVICE_PROCESS, tx.clone())
            .await?;

        self.transaction_dao.commit(tx).await?;
        Ok(item.clone())
    }

    async fn change_status(
        &self,
        id: Uuid,
        new_status: &str,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Inventur, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        self.permission_service
            .check_permission(MANAGE_INVENTUR_PRIVILEGE, context)
            .await?;

        // Get existing entity
        let existing = self
            .inventur_dao
            .find_by_id(id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(id))?;

        // Validate status transition
        validate_status_transition(existing.status.as_ref(), new_status)?;

        // Update status
        let mut inventur = Inventur::from(&existing);
        inventur.status = Arc::from(new_status);

        // If moving to completed, set end_date
        if new_status == STATUS_COMPLETED && inventur.end_date.is_none() {
            let now = time::OffsetDateTime::now_utc();
            inventur.end_date = Some(time::PrimitiveDateTime::new(now.date(), now.time()));
        }

        let entity = inventurly_dao::inventur::InventurEntity::from(&inventur);
        self.inventur_dao
            .update(&entity, INVENTUR_SERVICE_PROCESS, tx.clone())
            .await?;

        self.transaction_dao.commit(tx).await?;
        Ok(inventur)
    }

    async fn delete(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<(), ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        self.permission_service
            .check_permission(MANAGE_INVENTUR_PRIVILEGE, context)
            .await?;

        let existing = self
            .inventur_dao
            .find_by_id(id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(id))?;

        let mut inventur = Inventur::from(&existing);
        let now = time::OffsetDateTime::now_utc();
        inventur.deleted = Some(time::PrimitiveDateTime::new(now.date(), now.time()));

        let entity = inventurly_dao::inventur::InventurEntity::from(&inventur);
        self.inventur_dao
            .update(&entity, INVENTUR_SERVICE_PROCESS, tx.clone())
            .await?;

        self.transaction_dao.commit(tx).await?;
        Ok(())
    }
}
