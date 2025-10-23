use async_trait::async_trait;
use inventurly_service::{
    auth_types::AuthenticatedContext,
    user_service::UserService,
    ServiceError,
};

/// UserService implementation that extracts user from AuthenticatedContext
pub struct AuthContextUserService;

#[async_trait]
impl UserService for AuthContextUserService {
    type Context = AuthenticatedContext;

    async fn current_user(&self, context: Self::Context) -> Result<String, ServiceError> {
        Ok(context.user_id.to_string())
    }
}