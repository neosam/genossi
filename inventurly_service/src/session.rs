use async_trait::async_trait;
use mockall::automock;
use std::sync::Arc;

use crate::{
    auth_types::{AuthContext, UserSession},
    ServiceError,
};

/// Service for managing user sessions
#[automock]
#[async_trait]
pub trait SessionService: Send + Sync {
    /// Create a new user session
    async fn create_session(
        &self,
        user_id: &str,
        expires_in_seconds: i64,
    ) -> Result<UserSession, ServiceError>;

    /// Create a new user session with claims
    async fn create_session_with_claims(
        &self,
        user_id: &str,
        expires_in_seconds: i64,
        claims: Option<String>,
    ) -> Result<UserSession, ServiceError>;

    /// Verify a session by session ID and return user session info
    async fn verify_user_session(
        &self,
        session_id: &str,
    ) -> Result<Option<UserSession>, ServiceError>;

    /// Invalidate a session
    async fn invalidate_session(&self, session_id: &str) -> Result<(), ServiceError>;

    /// Clean up expired sessions
    async fn cleanup_expired_sessions(&self) -> Result<u64, ServiceError>;

    /// Extract authentication context from session ID  
    async fn extract_auth_context(
        &self,
        session_id: Option<String>,
    ) -> Result<Option<AuthContext>, ServiceError>;

    /// Ensures a user exists and then creates a session for them
    /// Used for OIDC auto-registration flow
    async fn ensure_user_and_create_session(
        &self,
        user_id: &str,
        expires_in_seconds: i64,
    ) -> Result<UserSession, ServiceError> {
        // Default implementation just calls create_session
        // Implementations that need auto-registration should override this
        self.create_session(user_id, expires_in_seconds).await
    }

    /// Ensures a user exists and then creates a session with claims for them
    /// Used for inventur token login auto-registration flow
    async fn ensure_user_and_create_session_with_claims(
        &self,
        user_id: &str,
        expires_in_seconds: i64,
        claims: Option<String>,
    ) -> Result<UserSession, ServiceError> {
        // Default implementation just calls create_session_with_claims
        // Implementations that need auto-registration should override this
        self.create_session_with_claims(user_id, expires_in_seconds, claims).await
    }
}

/// Development session service implementation
pub struct DevSessionService;

#[async_trait]
impl SessionService for DevSessionService {
    async fn create_session(
        &self,
        user_id: &str,
        expires_in_seconds: i64,
    ) -> Result<UserSession, ServiceError> {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        Ok(UserSession {
            session_id: "mock-session".into(),
            user_id: user_id.into(),
            expires_at: now + expires_in_seconds,
            created_at: now,
            claims: None,
        })
    }

    async fn create_session_with_claims(
        &self,
        user_id: &str,
        expires_in_seconds: i64,
        claims: Option<String>,
    ) -> Result<UserSession, ServiceError> {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        Ok(UserSession {
            session_id: "mock-session".into(),
            user_id: user_id.into(),
            expires_at: now + expires_in_seconds,
            created_at: now,
            claims: claims.map(|s| Arc::from(s.as_str())),
        })
    }

    async fn verify_user_session(
        &self,
        _session_id: &str,
    ) -> Result<Option<UserSession>, ServiceError> {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        Ok(Some(UserSession {
            session_id: "mock-session".into(),
            user_id: "DEVUSER".into(),
            expires_at: now + 3600,
            created_at: now,
            claims: None,
        }))
    }

    async fn invalidate_session(&self, _session_id: &str) -> Result<(), ServiceError> {
        Ok(())
    }

    async fn cleanup_expired_sessions(&self) -> Result<u64, ServiceError> {
        Ok(0)
    }

    async fn extract_auth_context(
        &self,
        session_id: Option<String>,
    ) -> Result<Option<AuthContext>, ServiceError> {
        if session_id.is_some() {
            Ok(Some(AuthContext::Mock(
                crate::auth_types::MockContext::default(),
            )))
        } else {
            Ok(None)
        }
    }
}
