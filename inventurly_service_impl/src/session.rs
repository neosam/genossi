use async_trait::async_trait;
use uuid::Uuid;

use inventurly_dao::permission::{PermissionDao, SessionEntity};
use inventurly_service::{
    session::SessionService,
    auth_types::{UserSession, AuthContext, MockContext},
    ServiceError,
};

use crate::gen_service_impl;

gen_service_impl! {
    struct SessionServiceImpl: SessionService = SessionServiceDeps {
        PermissionDao: PermissionDao = permission_dao,
    }
}

#[async_trait]
impl<Deps: SessionServiceDeps> SessionService for SessionServiceImpl<Deps> {
    async fn create_session(
        &self, 
        user_id: &str, 
        expires_in_seconds: i64
    ) -> Result<UserSession, ServiceError> {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        let expires_at = now + expires_in_seconds;
        let session_id = Uuid::new_v4().to_string();
        
        let session_entity = SessionEntity {
            id: session_id.clone().into(),
            user_id: user_id.into(),
            expires: expires_at,
            created: now,
        };
        
        self.permission_dao.create_session(&session_entity).await?;
        
        Ok(UserSession {
            session_id: session_id.into(),
            user_id: user_id.into(),
            expires_at,
            created_at: now,
        })
    }

    async fn verify_user_session(
        &self, 
        session_id: &str
    ) -> Result<Option<UserSession>, ServiceError> {
        let session = self.permission_dao.get_session(session_id).await?;
        
        if let Some(session_entity) = session {
            let now = time::OffsetDateTime::now_utc().unix_timestamp();
            
            // Check if session is expired
            if session_entity.expires < now {
                // Clean up expired session
                self.permission_dao.delete_session(session_id).await?;
                return Ok(None);
            }
            
            Ok(Some(UserSession {
                session_id: session_entity.id,
                user_id: session_entity.user_id,
                expires_at: session_entity.expires,
                created_at: session_entity.created,
            }))
        } else {
            Ok(None)
        }
    }

    async fn invalidate_session(
        &self, 
        session_id: &str
    ) -> Result<(), ServiceError> {
        self.permission_dao.delete_session(session_id).await?;
        Ok(())
    }

    async fn cleanup_expired_sessions(&self) -> Result<u64, ServiceError> {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        self.permission_dao.cleanup_expired_sessions(now).await?;
        // Note: PermissionDao doesn't return count, so we return 0 for now
        // In a real implementation, this could be enhanced to return actual count
        Ok(0)
    }

    async fn extract_auth_context(
        &self, 
        session_id: Option<String>
    ) -> Result<Option<AuthContext>, ServiceError> {
        match session_id {
            Some(sid) => {
                if let Some(session) = self.verify_user_session(&sid).await? {
                    // For now, return Mock context with the user ID
                    // In a real implementation, this would determine the context type based on config
                    Ok(Some(AuthContext::Mock(MockContext {
                        user_id: session.user_id,
                    })))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }
}

// Mock implementation for development/testing
pub struct MockSessionServiceImpl;

#[async_trait]
impl SessionService for MockSessionServiceImpl {
    async fn create_session(
        &self, 
        user_id: &str, 
        expires_in_seconds: i64
    ) -> Result<UserSession, ServiceError> {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        Ok(UserSession {
            session_id: "mock-session".into(),
            user_id: user_id.into(),
            expires_at: now + expires_in_seconds,
            created_at: now,
        })
    }

    async fn verify_user_session(
        &self, 
        _session_id: &str
    ) -> Result<Option<UserSession>, ServiceError> {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        Ok(Some(UserSession {
            session_id: "mock-session".into(),
            user_id: "DEVUSER".into(),
            expires_at: now + 3600, // 1 hour from now
            created_at: now - 60,   // Created 1 minute ago
        }))
    }

    async fn invalidate_session(
        &self, 
        _session_id: &str
    ) -> Result<(), ServiceError> {
        Ok(())
    }

    async fn cleanup_expired_sessions(&self) -> Result<u64, ServiceError> {
        Ok(0)
    }

    async fn extract_auth_context(
        &self, 
        session_id: Option<String>
    ) -> Result<Option<AuthContext>, ServiceError> {
        if session_id.is_some() {
            Ok(Some(AuthContext::Mock(MockContext::default())))
        } else {
            Ok(None)
        }
    }
}