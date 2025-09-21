use async_trait::async_trait;
use inventurly_dao::permission::PermissionDao;
use inventurly_service::{
    permission::{Authentication, PermissionService},
    user_service::UserService,
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
}