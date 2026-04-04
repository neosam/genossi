use async_trait::async_trait;
use genossi_dao::user_preference::UserPreferenceEntity;
use mockall::automock;
use std::fmt::Debug;
use std::sync::Arc;
use uuid::Uuid;

use crate::permission::Authentication;
use crate::ServiceError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserPreference {
    pub id: Uuid,
    pub user_id: Arc<str>,
    pub key: Arc<str>,
    pub value: Arc<str>,
    pub created: time::PrimitiveDateTime,
    pub deleted: Option<time::PrimitiveDateTime>,
    pub version: Uuid,
}

impl From<&UserPreferenceEntity> for UserPreference {
    fn from(entity: &UserPreferenceEntity) -> Self {
        Self {
            id: entity.id,
            user_id: entity.user_id.clone(),
            key: entity.key.clone(),
            value: entity.value.clone(),
            created: entity.created,
            deleted: entity.deleted,
            version: entity.version,
        }
    }
}

impl From<&UserPreference> for UserPreferenceEntity {
    fn from(pref: &UserPreference) -> Self {
        Self {
            id: pref.id,
            user_id: pref.user_id.clone(),
            key: pref.key.clone(),
            value: pref.value.clone(),
            created: pref.created,
            deleted: pref.deleted,
            version: pref.version,
        }
    }
}

#[automock(type Context=(); type Transaction = genossi_dao::MockTransaction;)]
#[async_trait]
pub trait UserPreferenceService {
    type Context: Clone + Debug + PartialEq + Eq + Send + Sync + 'static;
    type Transaction: genossi_dao::Transaction;

    async fn get_by_key(
        &self,
        key: &str,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<UserPreference, ServiceError>;

    async fn upsert(
        &self,
        key: &str,
        value: &str,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<UserPreference, ServiceError>;
}
