use async_trait::async_trait;
use std::sync::Arc;
use time::PrimitiveDateTime;

use crate::DaoError;

#[async_trait]
pub trait PermissionDao: Send + Sync {
    type Transaction: crate::Transaction;

    // Privilege checking
    async fn has_privilege(&self, user: &str, privilege: &str) -> Result<bool, DaoError>;

    // User management
    async fn all_users(&self) -> Result<Arc<[UserEntity]>, DaoError>;
    async fn get_user(&self, name: &str) -> Result<Option<UserEntity>, DaoError>;
    async fn create_user(&self, user: &UserEntity, process: &str) -> Result<(), DaoError>;
    async fn delete_user(&self, username: &str) -> Result<(), DaoError>;

    /// Ensures a user exists in the database, creating it if necessary
    /// Used for auto-registration of OIDC users
    /// Returns true if user was created, false if already existed
    async fn ensure_user_exists(&self, username: &str, process: &str) -> Result<bool, DaoError> {
        // Default implementation - try to create, ignore if exists
        if self.get_user(username).await?.is_some() {
            return Ok(false);
        }

        let user_entity = UserEntity {
            name: username.into(),
            update_timestamp: None,
            update_process: process.into(),
        };

        match self.create_user(&user_entity, process).await {
            Ok(()) => Ok(true),
            Err(DaoError::DatabaseError(msg)) if msg.contains("UNIQUE") => Ok(false),
            Err(e) => Err(e),
        }
    }

    // Role management
    async fn all_roles(&self) -> Result<Arc<[RoleEntity]>, DaoError>;
    async fn get_role(&self, name: &str) -> Result<Option<RoleEntity>, DaoError>;
    async fn create_role(&self, role: &RoleEntity, process: &str) -> Result<(), DaoError>;
    async fn delete_role(&self, role_name: &str) -> Result<(), DaoError>;

    // Privilege management
    async fn all_privileges(&self) -> Result<Arc<[PrivilegeEntity]>, DaoError>;
    async fn get_privilege(&self, name: &str) -> Result<Option<PrivilegeEntity>, DaoError>;
    async fn create_privilege(
        &self,
        privilege: &PrivilegeEntity,
        process: &str,
    ) -> Result<(), DaoError>;
    async fn delete_privilege(&self, privilege_name: &str) -> Result<(), DaoError>;

    // Role-User relationships
    async fn add_user_role(
        &self,
        username: &str,
        role: &str,
        process: &str,
    ) -> Result<(), DaoError>;
    async fn remove_user_role(&self, username: &str, role: &str) -> Result<(), DaoError>;
    async fn get_user_roles(&self, username: &str) -> Result<Arc<[RoleEntity]>, DaoError>;

    // Role-Privilege relationships
    async fn add_role_privilege(
        &self,
        role_name: &str,
        privilege_name: &str,
        process: &str,
    ) -> Result<(), DaoError>;
    async fn remove_role_privilege(
        &self,
        role_name: &str,
        privilege_name: &str,
    ) -> Result<(), DaoError>;
    async fn get_role_privileges(
        &self,
        role_name: &str,
    ) -> Result<Arc<[PrivilegeEntity]>, DaoError>;
    async fn get_user_privileges(&self, username: &str)
        -> Result<Arc<[PrivilegeEntity]>, DaoError>;

    // Session management
    async fn create_session(&self, session: &SessionEntity) -> Result<(), DaoError>;
    async fn get_session(&self, session_id: &str) -> Result<Option<SessionEntity>, DaoError>;
    async fn delete_session(&self, session_id: &str) -> Result<(), DaoError>;
    async fn cleanup_expired_sessions(&self, before_timestamp: i64) -> Result<(), DaoError>;
}

#[derive(Debug, Clone)]
pub struct UserEntity {
    pub name: Arc<str>,
    pub update_timestamp: Option<PrimitiveDateTime>,
    pub update_process: Arc<str>,
}

#[derive(Debug, Clone)]
pub struct RoleEntity {
    pub name: Arc<str>,
    pub update_timestamp: Option<PrimitiveDateTime>,
    pub update_process: Arc<str>,
}

#[derive(Debug, Clone)]
pub struct PrivilegeEntity {
    pub name: Arc<str>,
    pub update_timestamp: Option<PrimitiveDateTime>,
    pub update_process: Arc<str>,
}

#[derive(Debug, Clone)]
pub struct SessionEntity {
    pub id: Arc<str>,
    pub user_id: Arc<str>,
    pub expires: i64,
    pub created: i64,
    pub claims: Option<Arc<str>>,  // JSON string containing session claims
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
        Ok(Arc::new([UserEntity {
            name: Arc::from("DEVUSER"),
            update_timestamp: None,
            update_process: Arc::from("mock"),
        }]))
    }

    async fn get_user(&self, _name: &str) -> Result<Option<UserEntity>, DaoError> {
        Ok(Some(UserEntity {
            name: Arc::from("DEVUSER"),
            update_timestamp: None,
            update_process: Arc::from("mock"),
        }))
    }

    async fn create_user(&self, _user: &UserEntity, _process: &str) -> Result<(), DaoError> {
        Ok(())
    }

    async fn delete_user(&self, _username: &str) -> Result<(), DaoError> {
        Ok(())
    }

    async fn all_roles(&self) -> Result<Arc<[RoleEntity]>, DaoError> {
        Ok(Arc::new([RoleEntity {
            name: Arc::from("admin"),
            update_timestamp: None,
            update_process: Arc::from("mock"),
        }]))
    }

    async fn get_role(&self, _name: &str) -> Result<Option<RoleEntity>, DaoError> {
        Ok(Some(RoleEntity {
            name: Arc::from("admin"),
            update_timestamp: None,
            update_process: Arc::from("mock"),
        }))
    }

    async fn create_role(&self, _role: &RoleEntity, _process: &str) -> Result<(), DaoError> {
        Ok(())
    }

    async fn delete_role(&self, _role_name: &str) -> Result<(), DaoError> {
        Ok(())
    }

    async fn all_privileges(&self) -> Result<Arc<[PrivilegeEntity]>, DaoError> {
        Ok(Arc::new([PrivilegeEntity {
            name: Arc::from("admin"),
            update_timestamp: None,
            update_process: Arc::from("mock"),
        }]))
    }

    async fn get_privilege(&self, _name: &str) -> Result<Option<PrivilegeEntity>, DaoError> {
        Ok(Some(PrivilegeEntity {
            name: Arc::from("admin"),
            update_timestamp: None,
            update_process: Arc::from("mock"),
        }))
    }

    async fn create_privilege(
        &self,
        _privilege: &PrivilegeEntity,
        _process: &str,
    ) -> Result<(), DaoError> {
        Ok(())
    }

    async fn delete_privilege(&self, _privilege_name: &str) -> Result<(), DaoError> {
        Ok(())
    }

    async fn add_user_role(
        &self,
        _username: &str,
        _role: &str,
        _process: &str,
    ) -> Result<(), DaoError> {
        Ok(())
    }

    async fn remove_user_role(&self, _username: &str, _role: &str) -> Result<(), DaoError> {
        Ok(())
    }

    async fn get_user_roles(&self, _username: &str) -> Result<Arc<[RoleEntity]>, DaoError> {
        Ok(Arc::new([RoleEntity {
            name: Arc::from("admin"),
            update_timestamp: None,
            update_process: Arc::from("mock"),
        }]))
    }

    async fn add_role_privilege(
        &self,
        _role_name: &str,
        _privilege_name: &str,
        _process: &str,
    ) -> Result<(), DaoError> {
        Ok(())
    }

    async fn remove_role_privilege(
        &self,
        _role_name: &str,
        _privilege_name: &str,
    ) -> Result<(), DaoError> {
        Ok(())
    }

    async fn get_role_privileges(
        &self,
        _role_name: &str,
    ) -> Result<Arc<[PrivilegeEntity]>, DaoError> {
        Ok(Arc::new([PrivilegeEntity {
            name: Arc::from("admin"),
            update_timestamp: None,
            update_process: Arc::from("mock"),
        }]))
    }

    async fn get_user_privileges(
        &self,
        _username: &str,
    ) -> Result<Arc<[PrivilegeEntity]>, DaoError> {
        Ok(Arc::new([PrivilegeEntity {
            name: Arc::from("admin"),
            update_timestamp: None,
            update_process: Arc::from("mock"),
        }]))
    }

    async fn create_session(&self, _session: &SessionEntity) -> Result<(), DaoError> {
        Ok(())
    }

    async fn get_session(&self, _session_id: &str) -> Result<Option<SessionEntity>, DaoError> {
        Ok(Some(SessionEntity {
            id: Arc::from("mock-session"),
            user_id: Arc::from("DEVUSER"),
            expires: time::OffsetDateTime::now_utc().unix_timestamp() + 3600,
            created: time::OffsetDateTime::now_utc().unix_timestamp(),
            claims: None,
        }))
    }

    async fn delete_session(&self, _session_id: &str) -> Result<(), DaoError> {
        Ok(())
    }

    async fn cleanup_expired_sessions(&self, _before_timestamp: i64) -> Result<(), DaoError> {
        Ok(())
    }
}
