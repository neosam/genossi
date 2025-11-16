use std::sync::Arc;

use async_trait::async_trait;
use inventurly_dao::{inventur::InventurDao, TransactionDao};
use inventurly_service::{
    inventur::{Inventur, InventurService},
    permission::{Authentication, PermissionService},
    uuid_service::UuidService,
    ServiceError, ValidationFailureItem,
};
use rand::Rng;
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

/// Generate a random 32-character token for inventur access
fn generate_token() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                              abcdefghijklmnopqrstuvwxyz\
                              0123456789";
    let mut rng = rand::thread_rng();

    (0..32)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

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

        // Check if user has claims (token-based auth)
        let claimed_inventur_id = match &context {
            Authentication::Full => None,
            Authentication::Context(ctx) => self.permission_service.get_claimed_inventur_id(ctx).await?,
        };

        // If user has claims, filter to only their claimed inventur
        // If no claims, check global permission
        if claimed_inventur_id.is_none() {
            self.permission_service
                .check_permission(VIEW_INVENTUR_PRIVILEGE, context)
                .await?;
        }

        let inventurs: Arc<[Inventur]> = self
            .inventur_dao
            .all(tx.clone())
            .await?
            .iter()
            .map(Inventur::from)
            .filter(|inv| {
                // Filter based on claims if present
                match claimed_inventur_id {
                    Some(id) => inv.id == id,
                    None => true, // Global access
                }
            })
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
            .check_inventur_permission(VIEW_INVENTUR_PRIVILEGE, id, context)
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

        // Check if user has claims (token-based auth)
        let claimed_inventur_id = match &context {
            Authentication::Full => None,
            Authentication::Context(ctx) => self.permission_service.get_claimed_inventur_id(ctx).await?,
        };

        // If user has claims, filter to only their claimed inventur
        // If no claims, check global permission
        if claimed_inventur_id.is_none() {
            self.permission_service
                .check_permission(VIEW_INVENTUR_PRIVILEGE, context)
                .await?;
        }

        let inventurs: Arc<[Inventur]> = self
            .inventur_dao
            .find_by_status(status, tx.clone())
            .await?
            .iter()
            .map(Inventur::from)
            .filter(|inv| {
                // Filter based on claims if present
                match claimed_inventur_id {
                    Some(id) => inv.id == id,
                    None => true, // Global access
                }
            })
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
            .check_inventur_permission(MANAGE_INVENTUR_PRIVILEGE, item.id, context)
            .await?;

        // Check if the entity exists and get current status
        let existing = self
            .inventur_dao
            .find_by_id(item.id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(item.id))?;

        // Validate status transition
        validate_status_transition(existing.status.as_ref(), item.status.as_ref())?;

        // Generate token if transitioning to active and no token exists
        let mut updated_item = item.clone();
        if item.status.as_ref() == STATUS_ACTIVE
            && existing.status.as_ref() != STATUS_ACTIVE
            && item.token.is_none()
        {
            updated_item.token = Some(Arc::from(generate_token()));
        }

        let entity = inventurly_dao::inventur::InventurEntity::from(&updated_item);
        self.inventur_dao
            .update(&entity, INVENTUR_SERVICE_PROCESS, tx.clone())
            .await?;

        // Fetch the updated entity to ensure response matches database state
        let updated = self
            .inventur_dao
            .find_by_id(item.id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(item.id))?;

        self.transaction_dao.commit(tx).await?;
        Ok(Inventur::from(&updated))
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
            .check_inventur_permission(MANAGE_INVENTUR_PRIVILEGE, id, context)
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

        // If moving to active, generate token if not already present
        if new_status == STATUS_ACTIVE && inventur.token.is_none() {
            inventur.token = Some(Arc::from(generate_token()));
        }

        // If moving to completed, set end_date
        if new_status == STATUS_COMPLETED && inventur.end_date.is_none() {
            let now = time::OffsetDateTime::now_utc();
            inventur.end_date = Some(time::PrimitiveDateTime::new(now.date(), now.time()));
        }

        let entity = inventurly_dao::inventur::InventurEntity::from(&inventur);
        self.inventur_dao
            .update(&entity, INVENTUR_SERVICE_PROCESS, tx.clone())
            .await?;

        // Fetch the updated entity to ensure response matches database state
        let updated = self
            .inventur_dao
            .find_by_id(id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(id))?;

        self.transaction_dao.commit(tx).await?;
        Ok(Inventur::from(&updated))
    }

    async fn delete(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<(), ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        self.permission_service
            .check_inventur_permission(MANAGE_INVENTUR_PRIVILEGE, id, context)
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

    async fn find_by_token(
        &self,
        token: &str,
        tx: Option<Self::Transaction>,
    ) -> Result<Option<Inventur>, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        let inventur_entity = self
            .inventur_dao
            .find_by_token(token, tx.clone())
            .await?;

        self.transaction_dao.commit(tx).await?;

        Ok(inventur_entity.as_ref().map(Inventur::from))
    }
}
