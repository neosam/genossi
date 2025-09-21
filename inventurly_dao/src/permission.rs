use async_trait::async_trait;
use std::sync::Arc;

use crate::DaoError;

#[async_trait]
pub trait PermissionDao: Send + Sync {
    type Transaction: crate::Transaction;

    async fn has_privilege(&self, user: &str, privilege: &str) -> Result<bool, DaoError>;
    async fn all_users(&self) -> Result<Arc<[UserEntity]>, DaoError>;
    async fn create_user(&self, user: &UserEntity, process: &str) -> Result<(), DaoError>;
    async fn add_user_role(&self, username: &str, role: &str, process: &str) -> Result<(), DaoError>;
}

#[derive(Debug, Clone)]
pub struct UserEntity {
    pub name: Arc<str>,
}

// Mock implementation for testing
pub struct MockPermissionDao;

#[async_trait]
impl PermissionDao for MockPermissionDao {
    type Transaction = crate::MockTransaction;

    async fn has_privilege(&self, _user: &str, _privilege: &str) -> Result<bool, DaoError> {
        Ok(true) // Always grant permission for mock
    }

    async fn all_users(&self) -> Result<Arc<[UserEntity]>, DaoError> {
        Ok(Arc::new([]))
    }

    async fn create_user(&self, _user: &UserEntity, _process: &str) -> Result<(), DaoError> {
        Ok(())
    }

    async fn add_user_role(&self, _username: &str, _role: &str, _process: &str) -> Result<(), DaoError> {
        Ok(())
    }
}