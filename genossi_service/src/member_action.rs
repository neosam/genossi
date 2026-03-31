use async_trait::async_trait;
use genossi_dao::member_action::{ActionType, MemberActionEntity};
use mockall::automock;
use std::fmt::Debug;
use std::sync::Arc;
use uuid::Uuid;

use crate::permission::Authentication;
use crate::ServiceError;

pub use genossi_dao::member_action::ActionType as ServiceActionType;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberAction {
    pub id: Uuid,
    pub member_id: Uuid,
    pub action_type: ActionType,
    pub date: time::Date,
    pub shares_change: i32,
    pub transfer_member_id: Option<Uuid>,
    pub effective_date: Option<time::Date>,
    pub comment: Option<Arc<str>>,
    pub created: time::PrimitiveDateTime,
    pub deleted: Option<time::PrimitiveDateTime>,
    pub version: Uuid,
}

impl From<&MemberActionEntity> for MemberAction {
    fn from(entity: &MemberActionEntity) -> Self {
        Self {
            id: entity.id,
            member_id: entity.member_id,
            action_type: entity.action_type.clone(),
            date: entity.date,
            shares_change: entity.shares_change,
            transfer_member_id: entity.transfer_member_id,
            effective_date: entity.effective_date,
            comment: entity.comment.clone(),
            created: entity.created,
            deleted: entity.deleted,
            version: entity.version,
        }
    }
}

impl From<&MemberAction> for MemberActionEntity {
    fn from(action: &MemberAction) -> Self {
        Self {
            id: action.id,
            member_id: action.member_id,
            action_type: action.action_type.clone(),
            date: action.date,
            shares_change: action.shares_change,
            transfer_member_id: action.transfer_member_id,
            effective_date: action.effective_date,
            comment: action.comment.clone(),
            created: action.created,
            deleted: action.deleted,
            version: action.version,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MigrationStatus {
    pub member_id: Uuid,
    pub status: MigrationState,
    pub expected_shares: i32,
    pub actual_shares: i32,
    pub expected_action_count: i32,
    pub actual_action_count: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MigrationState {
    Migrated,
    Pending,
}

#[automock(type Context=(); type Transaction = genossi_dao::MockTransaction;)]
#[async_trait]
pub trait MemberActionService {
    type Context: Clone + Debug + PartialEq + Eq + Send + Sync + 'static;
    type Transaction: genossi_dao::Transaction;

    async fn get_by_member(
        &self,
        member_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[MemberAction]>, ServiceError>;

    async fn get(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<MemberAction, ServiceError>;

    async fn create(
        &self,
        item: &MemberAction,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<MemberAction, ServiceError>;

    async fn update(
        &self,
        item: &MemberAction,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<MemberAction, ServiceError>;

    async fn delete(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<(), ServiceError>;

    async fn migration_status(
        &self,
        member_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<MigrationStatus, ServiceError>;
}
