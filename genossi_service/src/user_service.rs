use crate::ServiceError;
use async_trait::async_trait;

#[async_trait]
pub trait UserService: Send + Sync {
    type Context: Send + Sync + Clone + 'static;

    async fn current_user(&self, context: Self::Context) -> Result<String, ServiceError>;
}

// Mock implementation for development
pub struct MockUserService;

#[async_trait]
impl UserService for MockUserService {
    type Context = crate::permission::MockContext;

    async fn current_user(&self, _context: Self::Context) -> Result<String, ServiceError> {
        Ok("DEVUSER".to_string())
    }
}
