use async_trait::async_trait;
use genossi_dao::user_preference::UserPreferenceDao;
use genossi_dao::TransactionDao;
use genossi_service::permission::{Authentication, PermissionService};
use genossi_service::user_preference::{UserPreference, UserPreferenceService};
use genossi_service::uuid_service::UuidService;
use genossi_service::ServiceError;
use std::sync::Arc;

use crate::gen_service_impl;

const USER_PREFERENCE_SERVICE_PROCESS: &str = "user-preference-service";
const VIEW_MEMBERS_PRIVILEGE: &str = "view_members";

gen_service_impl! {
    struct UserPreferenceServiceImpl: UserPreferenceService = UserPreferenceServiceDeps {
        UserPreferenceDao: UserPreferenceDao<Transaction = Self::Transaction> = user_preference_dao,
        PermissionService: PermissionService<Context = Self::Context> = permission_service,
        UuidService: UuidService = uuid_service,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
    }
}

#[async_trait]
impl<Deps: UserPreferenceServiceDeps> UserPreferenceService
    for UserPreferenceServiceImpl<Deps>
{
    type Context = Deps::Context;
    type Transaction = Deps::Transaction;

    async fn get_by_key(
        &self,
        key: &str,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<UserPreference, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        self.permission_service
            .check_permission(VIEW_MEMBERS_PRIVILEGE, context.clone())
            .await?;

        let user_id = self
            .permission_service
            .current_user_id(context)
            .await?
            .ok_or(ServiceError::Unauthorized)?;

        let entity = self
            .user_preference_dao
            .find_by_user_and_key(&user_id, key, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(uuid::Uuid::nil()))?;

        self.transaction_dao.commit(tx).await?;
        Ok(UserPreference::from(&entity))
    }

    async fn upsert(
        &self,
        key: &str,
        value: &str,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<UserPreference, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        self.permission_service
            .check_permission(VIEW_MEMBERS_PRIVILEGE, context.clone())
            .await?;

        let user_id = self
            .permission_service
            .current_user_id(context)
            .await?
            .ok_or(ServiceError::Unauthorized)?;

        let existing = self
            .user_preference_dao
            .find_by_user_and_key(&user_id, key, tx.clone())
            .await?;

        let result = match existing {
            Some(mut entity) => {
                entity.value = Arc::from(value);
                self.user_preference_dao
                    .update(&entity, USER_PREFERENCE_SERVICE_PROCESS, tx.clone())
                    .await?;
                // Re-fetch to get updated version
                self.user_preference_dao
                    .find_by_user_and_key(&user_id, key, tx.clone())
                    .await?
                    .ok_or(ServiceError::EntityNotFound(entity.id))?
            }
            None => {
                let now = time::OffsetDateTime::now_utc();
                let created = time::PrimitiveDateTime::new(now.date(), now.time());
                let entity = genossi_dao::user_preference::UserPreferenceEntity {
                    id: self.uuid_service.new_v4().await,
                    user_id: Arc::from(user_id.as_str()),
                    key: Arc::from(key),
                    value: Arc::from(value),
                    created,
                    deleted: None,
                    version: self.uuid_service.new_v4().await,
                };
                self.user_preference_dao
                    .create(&entity, USER_PREFERENCE_SERVICE_PROCESS, tx.clone())
                    .await?;
                entity
            }
        };

        self.transaction_dao.commit(tx).await?;
        Ok(UserPreference::from(&result))
    }
}