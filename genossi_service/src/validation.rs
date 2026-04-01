use async_trait::async_trait;
use genossi_dao::member_action::ActionType;
use mockall::automock;
use std::fmt::Debug;
use std::sync::Arc;
use uuid::Uuid;

use crate::permission::Authentication;
use crate::ServiceError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationResult {
    pub member_number_gaps: Arc<[i64]>,
    pub unmatched_transfers: Arc<[UnmatchedTransfer]>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnmatchedTransfer {
    pub action_id: Uuid,
    pub member_id: Uuid,
    pub member_number: i64,
    pub action_type: ActionType,
    pub transfer_member_id: Uuid,
    pub transfer_member_number: i64,
    pub shares_change: i32,
    pub date: time::Date,
}

#[automock(type Context=();)]
#[async_trait]
pub trait ValidationService {
    type Context: Clone + Debug + PartialEq + Eq + Send + Sync + 'static;

    async fn validate(
        &self,
        context: Authentication<Self::Context>,
    ) -> Result<ValidationResult, ServiceError>;
}
