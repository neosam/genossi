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
    pub shares_mismatches: Arc<[SharesMismatch]>,
    pub missing_entry_actions: Arc<[MissingEntryAction]>,
    pub exit_date_mismatches: Arc<[ExitDateMismatch]>,
    pub active_members_no_shares: Arc<[ActiveMemberNoShares]>,
    pub duplicate_member_numbers: Arc<[DuplicateMemberNumber]>,
    pub exited_members_with_shares: Arc<[ExitedMemberWithShares]>,
    pub migrated_flag_mismatches: Arc<[MigratedFlagMismatch]>,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharesMismatch {
    pub member_id: Uuid,
    pub member_number: i64,
    pub expected: i32,
    pub actual: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissingEntryAction {
    pub member_id: Uuid,
    pub member_number: i64,
    pub actual_count: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExitDateMismatch {
    pub member_id: Uuid,
    pub member_number: i64,
    pub has_exit_date: bool,
    pub has_austritt_action: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActiveMemberNoShares {
    pub member_id: Uuid,
    pub member_number: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DuplicateMemberNumber {
    pub member_number: i64,
    pub member_ids: Arc<[Uuid]>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExitedMemberWithShares {
    pub member_id: Uuid,
    pub member_number: i64,
    pub current_shares: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MigratedFlagMismatch {
    pub member_id: Uuid,
    pub member_number: i64,
    pub flag_value: bool,
    pub computed_status: Arc<str>,
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
