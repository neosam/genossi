use crate::{
    auth_types::{
        PrivilegeResponseTO, PrivilegeTO, RolePrivilege, RoleResponseTO, RoleTO, UserResponseTO,
        UserRole, UserTO,
    },
    ServiceError,
};
use async_trait::async_trait;
use mockall::automock;
use std::fmt::Debug;
use std::sync::Arc;

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

    // Core authentication methods
    async fn check_permission(
        &self,
        privilege: &str,
        context: Authentication<Self::Context>,
    ) -> Result<(), ServiceError>;

    async fn current_user_id(
        &self,
        context: Authentication<Self::Context>,
    ) -> Result<Option<String>, ServiceError>;

    // User management methods
    async fn get_all_users(
        &self,
        context: Authentication<Self::Context>,
    ) -> Result<Arc<[UserResponseTO]>, ServiceError>;

    async fn create_user(
        &self,
        user: UserTO,
        context: Authentication<Self::Context>,
    ) -> Result<(), ServiceError>;

    async fn delete_user(
        &self,
        username: String,
        context: Authentication<Self::Context>,
    ) -> Result<(), ServiceError>;

    // Role management methods
    async fn get_all_roles(
        &self,
        context: Authentication<Self::Context>,
    ) -> Result<Arc<[RoleResponseTO]>, ServiceError>;

    async fn create_role(
        &self,
        role: RoleTO,
        context: Authentication<Self::Context>,
    ) -> Result<(), ServiceError>;

    async fn delete_role(
        &self,
        role_name: String,
        context: Authentication<Self::Context>,
    ) -> Result<(), ServiceError>;

    // Privilege management methods
    async fn get_all_privileges(
        &self,
        context: Authentication<Self::Context>,
    ) -> Result<Arc<[PrivilegeResponseTO]>, ServiceError>;

    async fn create_privilege(
        &self,
        privilege: PrivilegeTO,
        context: Authentication<Self::Context>,
    ) -> Result<(), ServiceError>;

    async fn delete_privilege(
        &self,
        privilege_name: String,
        context: Authentication<Self::Context>,
    ) -> Result<(), ServiceError>;

    // User-Role relationship management
    async fn assign_user_role(
        &self,
        user_role: UserRole,
        context: Authentication<Self::Context>,
    ) -> Result<(), ServiceError>;

    async fn remove_user_role(
        &self,
        user_role: UserRole,
        context: Authentication<Self::Context>,
    ) -> Result<(), ServiceError>;

    async fn get_user_roles(
        &self,
        username: String,
        context: Authentication<Self::Context>,
    ) -> Result<Arc<[RoleResponseTO]>, ServiceError>;

    // Role-Privilege relationship management
    async fn assign_role_privilege(
        &self,
        role_privilege: RolePrivilege,
        context: Authentication<Self::Context>,
    ) -> Result<(), ServiceError>;

    async fn remove_role_privilege(
        &self,
        role_privilege: RolePrivilege,
        context: Authentication<Self::Context>,
    ) -> Result<(), ServiceError>;

    async fn get_role_privileges(
        &self,
        role_name: String,
        context: Authentication<Self::Context>,
    ) -> Result<Arc<[PrivilegeResponseTO]>, ServiceError>;

    async fn get_user_privileges(
        &self,
        username: String,
        context: Authentication<Self::Context>,
    ) -> Result<Arc<[PrivilegeResponseTO]>, ServiceError>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MockContext;
