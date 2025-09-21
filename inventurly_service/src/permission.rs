use std::fmt::Debug;
use async_trait::async_trait;
use mockall::automock;
use crate::ServiceError;

#[derive(Clone, Debug)]
pub enum Authentication<Context> {
    Full,
    Context(Context),
}

impl<Context: Clone + Debug + PartialEq + Eq + Send + Sync + 'static> From<Context>
    for Authentication<Context>
{
    fn from(context: Context) -> Self {
        Self::Context(context)
    }
}

pub const ADMIN_PRIVILEGE: &str = "admin";

#[automock(type Context=();)]
#[async_trait]
pub trait PermissionService {
    type Context: Clone + Debug + Send + Sync + 'static;

    async fn check_permission(
        &self,
        privilege: &str,
        context: Authentication<Self::Context>,
    ) -> Result<(), ServiceError>;

    async fn current_user_id(
        &self,
        context: Authentication<Self::Context>,
    ) -> Result<Option<String>, ServiceError>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MockContext;