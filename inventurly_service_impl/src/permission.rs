use std::sync::Arc;
use async_trait::async_trait;
use inventurly_dao::permission::{PermissionDao, UserEntity, RoleEntity, PrivilegeEntity};
use inventurly_service::{
    permission::{Authentication, PermissionService, ADMIN_PRIVILEGE},
    user_service::UserService,
    auth_types::{UserTO, RoleTO, PrivilegeTO, UserRole, RolePrivilege, UserResponseTO, RoleResponseTO, PrivilegeResponseTO},
    ServiceError,
};

use crate::gen_service_impl;

gen_service_impl! {
    struct PermissionServiceImpl: PermissionService = PermissionServiceDeps {
        PermissionDao: PermissionDao = permission_dao,
        UserService: UserService<Context = Self::Context> = user_service,
    }
}

#[async_trait]
impl<Deps: PermissionServiceDeps> PermissionService for PermissionServiceImpl<Deps> {
    type Context = Deps::Context;

    async fn check_permission(
        &self,
        privilege: &str,
        context: Authentication<Self::Context>,
    ) -> Result<(), ServiceError> {
        match context {
            Authentication::Full => Ok(()),
            Authentication::Context(ctx) => {
                let current_user = self.user_service.current_user(ctx).await?;
                if self
                    .permission_dao
                    .has_privilege(&current_user, privilege)
                    .await?
                {
                    Ok(())
                } else {
                    Err(ServiceError::PermissionDenied)
                }
            }
        }
    }

    async fn current_user_id(
        &self,
        context: Authentication<Self::Context>,
    ) -> Result<Option<String>, ServiceError> {
        match context {
            Authentication::Full => Ok(None),
            Authentication::Context(ctx) => {
                let current_user = self.user_service.current_user(ctx).await?;
                Ok(Some(current_user))
            }
        }
    }

    // User management methods
    async fn get_all_users(
        &self,
        context: Authentication<Self::Context>,
    ) -> Result<Arc<[UserResponseTO]>, ServiceError> {
        self.check_permission(ADMIN_PRIVILEGE, context).await?;
        
        let users = self.permission_dao.all_users().await?;
        let response_users: Vec<UserResponseTO> = users
            .iter()
            .map(|user| UserResponseTO {
                name: user.name.to_string(),
                update_timestamp: user.update_timestamp,
                update_process: user.update_process.to_string(),
            })
            .collect();
        
        Ok(Arc::from(response_users))
    }

    async fn create_user(
        &self,
        user: UserTO,
        context: Authentication<Self::Context>,
    ) -> Result<(), ServiceError> {
        self.check_permission(ADMIN_PRIVILEGE, context.clone()).await?;
        
        let current_user = self.get_current_user_for_process(context).await?;
        let user_entity = UserEntity {
            name: user.name.into(),
            update_timestamp: None, // Will be set by the DAO layer
            update_process: current_user.clone().into(),
        };
        
        self.permission_dao.create_user(&user_entity, &current_user).await?;
        Ok(())
    }

    async fn delete_user(
        &self,
        username: String,
        context: Authentication<Self::Context>,
    ) -> Result<(), ServiceError> {
        self.check_permission(ADMIN_PRIVILEGE, context).await?;
        
        self.permission_dao.delete_user(&username).await?;
        Ok(())
    }

    // Role management methods
    async fn get_all_roles(
        &self,
        context: Authentication<Self::Context>,
    ) -> Result<Arc<[RoleResponseTO]>, ServiceError> {
        self.check_permission(ADMIN_PRIVILEGE, context).await?;
        
        let roles = self.permission_dao.all_roles().await?;
        let response_roles: Vec<RoleResponseTO> = roles
            .iter()
            .map(|role| RoleResponseTO {
                name: role.name.to_string(),
                update_timestamp: role.update_timestamp,
                update_process: role.update_process.to_string(),
            })
            .collect();
        
        Ok(Arc::from(response_roles))
    }

    async fn create_role(
        &self,
        role: RoleTO,
        context: Authentication<Self::Context>,
    ) -> Result<(), ServiceError> {
        self.check_permission(ADMIN_PRIVILEGE, context.clone()).await?;
        
        let current_user = self.get_current_user_for_process(context).await?;
        let role_entity = RoleEntity {
            name: role.name.into(),
            update_timestamp: None, // Will be set by the DAO layer
            update_process: current_user.clone().into(),
        };
        
        self.permission_dao.create_role(&role_entity, &current_user).await?;
        Ok(())
    }

    async fn delete_role(
        &self,
        role_name: String,
        context: Authentication<Self::Context>,
    ) -> Result<(), ServiceError> {
        self.check_permission(ADMIN_PRIVILEGE, context).await?;
        
        self.permission_dao.delete_role(&role_name).await?;
        Ok(())
    }

    // Privilege management methods
    async fn get_all_privileges(
        &self,
        context: Authentication<Self::Context>,
    ) -> Result<Arc<[PrivilegeResponseTO]>, ServiceError> {
        self.check_permission(ADMIN_PRIVILEGE, context).await?;
        
        let privileges = self.permission_dao.all_privileges().await?;
        let response_privileges: Vec<PrivilegeResponseTO> = privileges
            .iter()
            .map(|privilege| PrivilegeResponseTO {
                name: privilege.name.to_string(),
                update_timestamp: privilege.update_timestamp,
                update_process: privilege.update_process.to_string(),
            })
            .collect();
        
        Ok(Arc::from(response_privileges))
    }

    async fn create_privilege(
        &self,
        privilege: PrivilegeTO,
        context: Authentication<Self::Context>,
    ) -> Result<(), ServiceError> {
        self.check_permission(ADMIN_PRIVILEGE, context.clone()).await?;
        
        let current_user = self.get_current_user_for_process(context).await?;
        let privilege_entity = PrivilegeEntity {
            name: privilege.name.into(),
            update_timestamp: None, // Will be set by the DAO layer
            update_process: current_user.clone().into(),
        };
        
        self.permission_dao.create_privilege(&privilege_entity, &current_user).await?;
        Ok(())
    }

    async fn delete_privilege(
        &self,
        privilege_name: String,
        context: Authentication<Self::Context>,
    ) -> Result<(), ServiceError> {
        self.check_permission(ADMIN_PRIVILEGE, context).await?;
        
        self.permission_dao.delete_privilege(&privilege_name).await?;
        Ok(())
    }

    // User-Role relationship management
    async fn assign_user_role(
        &self,
        user_role: UserRole,
        context: Authentication<Self::Context>,
    ) -> Result<(), ServiceError> {
        self.check_permission(ADMIN_PRIVILEGE, context.clone()).await?;
        
        let current_user = self.get_current_user_for_process(context).await?;
        self.permission_dao.add_user_role(&user_role.user, &user_role.role, &current_user).await?;
        Ok(())
    }

    async fn remove_user_role(
        &self,
        user_role: UserRole,
        context: Authentication<Self::Context>,
    ) -> Result<(), ServiceError> {
        self.check_permission(ADMIN_PRIVILEGE, context).await?;
        
        self.permission_dao.remove_user_role(&user_role.user, &user_role.role).await?;
        Ok(())
    }

    async fn get_user_roles(
        &self,
        username: String,
        context: Authentication<Self::Context>,
    ) -> Result<Arc<[RoleResponseTO]>, ServiceError> {
        self.check_permission(ADMIN_PRIVILEGE, context).await?;
        
        let roles = self.permission_dao.get_user_roles(&username).await?;
        let response_roles: Vec<RoleResponseTO> = roles
            .iter()
            .map(|role| RoleResponseTO {
                name: role.name.to_string(),
                update_timestamp: role.update_timestamp,
                update_process: role.update_process.to_string(),
            })
            .collect();
        
        Ok(Arc::from(response_roles))
    }

    // Role-Privilege relationship management
    async fn assign_role_privilege(
        &self,
        role_privilege: RolePrivilege,
        context: Authentication<Self::Context>,
    ) -> Result<(), ServiceError> {
        self.check_permission(ADMIN_PRIVILEGE, context.clone()).await?;
        
        let current_user = self.get_current_user_for_process(context).await?;
        self.permission_dao.add_role_privilege(&role_privilege.role, &role_privilege.privilege, &current_user).await?;
        Ok(())
    }

    async fn remove_role_privilege(
        &self,
        role_privilege: RolePrivilege,
        context: Authentication<Self::Context>,
    ) -> Result<(), ServiceError> {
        self.check_permission(ADMIN_PRIVILEGE, context).await?;
        
        self.permission_dao.remove_role_privilege(&role_privilege.role, &role_privilege.privilege).await?;
        Ok(())
    }

    async fn get_role_privileges(
        &self,
        role_name: String,
        context: Authentication<Self::Context>,
    ) -> Result<Arc<[PrivilegeResponseTO]>, ServiceError> {
        self.check_permission(ADMIN_PRIVILEGE, context).await?;
        
        let privileges = self.permission_dao.get_role_privileges(&role_name).await?;
        let response_privileges: Vec<PrivilegeResponseTO> = privileges
            .iter()
            .map(|privilege| PrivilegeResponseTO {
                name: privilege.name.to_string(),
                update_timestamp: privilege.update_timestamp,
                update_process: privilege.update_process.to_string(),
            })
            .collect();
        
        Ok(Arc::from(response_privileges))
    }

    async fn get_user_privileges(
        &self,
        username: String,
        context: Authentication<Self::Context>,
    ) -> Result<Arc<[PrivilegeResponseTO]>, ServiceError> {
        self.check_permission(ADMIN_PRIVILEGE, context).await?;
        
        let privileges = self.permission_dao.get_user_privileges(&username).await?;
        let response_privileges: Vec<PrivilegeResponseTO> = privileges
            .iter()
            .map(|privilege| PrivilegeResponseTO {
                name: privilege.name.to_string(),
                update_timestamp: privilege.update_timestamp,
                update_process: privilege.update_process.to_string(),
            })
            .collect();
        
        Ok(Arc::from(response_privileges))
    }
}

// Helper methods for PermissionServiceImpl
impl<Deps: PermissionServiceDeps> PermissionServiceImpl<Deps> {
    /// Get the current user ID for audit processes
    async fn get_current_user_for_process(
        &self,
        context: Authentication<Deps::Context>,
    ) -> Result<String, ServiceError> {
        match context {
            Authentication::Full => Ok("system".to_string()),
            Authentication::Context(ctx) => {
                self.user_service.current_user(ctx).await
            }
        }
    }
}