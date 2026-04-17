use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

use genossi_dao::permission::{PermissionDao, SessionEntity};
use genossi_service::{
    auth_types::{AuthContext, MockContext, UserSession},
    session::SessionService,
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
        expires_in_seconds: i64,
    ) -> Result<UserSession, ServiceError> {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        let expires_at = now + expires_in_seconds;
        let session_id = Uuid::new_v4().to_string();

        let session_entity = SessionEntity {
            id: session_id.clone().into(),
            user_id: user_id.into(),
            expires: expires_at,
            created: now,
            claims: None,
            last_used_at: now,
        };

        self.permission_dao.create_session(&session_entity).await?;

        Ok(UserSession {
            session_id: session_id.into(),
            user_id: user_id.into(),
            expires_at,
            created_at: now,
            claims: None,
            last_used_at: now,
        })
    }

    async fn create_session_with_claims(
        &self,
        user_id: &str,
        expires_in_seconds: i64,
        claims: Option<String>,
    ) -> Result<UserSession, ServiceError> {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        let expires_at = now + expires_in_seconds;
        let session_id = Uuid::new_v4().to_string();

        let claims_arc = claims.map(|s| Arc::from(s.as_str()));
        let session_entity = SessionEntity {
            id: session_id.clone().into(),
            user_id: user_id.into(),
            expires: expires_at,
            created: now,
            claims: claims_arc.clone(),
            last_used_at: now,
        };

        self.permission_dao.create_session(&session_entity).await?;

        Ok(UserSession {
            session_id: session_id.into(),
            user_id: user_id.into(),
            expires_at,
            created_at: now,
            claims: claims_arc,
            last_used_at: now,
        })
    }

    async fn verify_user_session(
        &self,
        session_id: &str,
    ) -> Result<Option<UserSession>, ServiceError> {
        let session = self.permission_dao.get_session(session_id).await?;

        if let Some(session_entity) = session {
            let now = time::OffsetDateTime::now_utc().unix_timestamp();

            // Check if session is expired (absolute lifetime)
            if session_entity.expires < now {
                self.permission_dao.delete_session(session_id).await?;
                return Ok(None);
            }

            // Check inactivity timeout (30 days)
            const INACTIVITY_TIMEOUT_SECS: i64 = 30 * 24 * 60 * 60;
            if now - session_entity.last_used_at > INACTIVITY_TIMEOUT_SECS {
                self.permission_dao.delete_session(session_id).await?;
                return Ok(None);
            }

            // Touch session to update last_used_at
            self.permission_dao.touch_session(session_id, now).await?;

            Ok(Some(UserSession {
                session_id: session_entity.id,
                user_id: session_entity.user_id,
                expires_at: session_entity.expires,
                created_at: session_entity.created,
                claims: session_entity.claims,
                last_used_at: now,
            }))
        } else {
            Ok(None)
        }
    }

    async fn invalidate_session(&self, session_id: &str) -> Result<(), ServiceError> {
        self.permission_dao.delete_session(session_id).await?;
        Ok(())
    }

    async fn cleanup_expired_sessions(&self) -> Result<u64, ServiceError> {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        self.permission_dao.cleanup_expired_sessions(now).await?;
        Ok(0)
    }

    async fn revoke_all_for_user(&self, user_id: &str) -> Result<u64, ServiceError> {
        let count = self
            .permission_dao
            .delete_sessions_for_user(user_id)
            .await?;
        Ok(count)
    }

    async fn extract_auth_context(
        &self,
        session_id: Option<String>,
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

    async fn ensure_user_and_create_session(
        &self,
        user_id: &str,
        expires_in_seconds: i64,
    ) -> Result<UserSession, ServiceError> {
        // Ensure user exists for OIDC auto-registration
        self.permission_dao
            .ensure_user_exists(user_id, "oidc-auto-register")
            .await?;

        // Now create the session
        self.create_session(user_id, expires_in_seconds).await
    }

    async fn ensure_user_and_create_session_with_claims(
        &self,
        user_id: &str,
        expires_in_seconds: i64,
        claims: Option<String>,
    ) -> Result<UserSession, ServiceError> {
        // Ensure user exists for inventur token auto-registration
        self.permission_dao
            .ensure_user_exists(user_id, "inventur-token-auto-register")
            .await?;

        // Now create the session with claims
        self.create_session_with_claims(user_id, expires_in_seconds, claims)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use genossi_dao::permission::*;
    use genossi_dao::DaoError;
    use std::sync::Mutex;

    #[derive(Clone, Debug)]
    struct TestTransaction;

    #[async_trait]
    impl genossi_dao::Transaction for TestTransaction {
        async fn begin(&mut self) -> Result<(), DaoError> {
            Ok(())
        }
        async fn commit(self) -> Result<(), DaoError> {
            Ok(())
        }
        async fn rollback(self) -> Result<(), DaoError> {
            Ok(())
        }
    }

    struct TestPermissionDao {
        sessions: Mutex<Vec<SessionEntity>>,
    }

    impl TestPermissionDao {
        fn new() -> Self {
            Self {
                sessions: Mutex::new(Vec::new()),
            }
        }

        fn session_count(&self) -> usize {
            self.sessions.lock().unwrap().len()
        }
    }

    #[async_trait]
    impl PermissionDao for TestPermissionDao {
        type Transaction = TestTransaction;

        async fn has_privilege(&self, _: &str, _: &str) -> Result<bool, DaoError> {
            Ok(true)
        }
        async fn all_users(&self) -> Result<Arc<[UserEntity]>, DaoError> {
            Ok(Arc::new([]))
        }
        async fn get_user(&self, _: &str) -> Result<Option<UserEntity>, DaoError> {
            Ok(None)
        }
        async fn create_user(&self, _: &UserEntity, _: &str) -> Result<(), DaoError> {
            Ok(())
        }
        async fn delete_user(&self, _: &str) -> Result<(), DaoError> {
            Ok(())
        }
        async fn all_roles(&self) -> Result<Arc<[RoleEntity]>, DaoError> {
            Ok(Arc::new([]))
        }
        async fn get_role(&self, _: &str) -> Result<Option<RoleEntity>, DaoError> {
            Ok(None)
        }
        async fn create_role(&self, _: &RoleEntity, _: &str) -> Result<(), DaoError> {
            Ok(())
        }
        async fn delete_role(&self, _: &str) -> Result<(), DaoError> {
            Ok(())
        }
        async fn all_privileges(&self) -> Result<Arc<[PrivilegeEntity]>, DaoError> {
            Ok(Arc::new([]))
        }
        async fn get_privilege(&self, _: &str) -> Result<Option<PrivilegeEntity>, DaoError> {
            Ok(None)
        }
        async fn create_privilege(&self, _: &PrivilegeEntity, _: &str) -> Result<(), DaoError> {
            Ok(())
        }
        async fn delete_privilege(&self, _: &str) -> Result<(), DaoError> {
            Ok(())
        }
        async fn add_user_role(&self, _: &str, _: &str, _: &str) -> Result<(), DaoError> {
            Ok(())
        }
        async fn remove_user_role(&self, _: &str, _: &str) -> Result<(), DaoError> {
            Ok(())
        }
        async fn get_user_roles(&self, _: &str) -> Result<Arc<[RoleEntity]>, DaoError> {
            Ok(Arc::new([]))
        }
        async fn add_role_privilege(&self, _: &str, _: &str, _: &str) -> Result<(), DaoError> {
            Ok(())
        }
        async fn remove_role_privilege(&self, _: &str, _: &str) -> Result<(), DaoError> {
            Ok(())
        }
        async fn get_role_privileges(&self, _: &str) -> Result<Arc<[PrivilegeEntity]>, DaoError> {
            Ok(Arc::new([]))
        }
        async fn get_user_privileges(&self, _: &str) -> Result<Arc<[PrivilegeEntity]>, DaoError> {
            Ok(Arc::new([]))
        }

        async fn create_session(&self, session: &SessionEntity) -> Result<(), DaoError> {
            self.sessions.lock().unwrap().push(session.clone());
            Ok(())
        }

        async fn get_session(&self, session_id: &str) -> Result<Option<SessionEntity>, DaoError> {
            Ok(self
                .sessions
                .lock()
                .unwrap()
                .iter()
                .find(|s| s.id.as_ref() == session_id)
                .cloned())
        }

        async fn delete_session(&self, session_id: &str) -> Result<(), DaoError> {
            self.sessions
                .lock()
                .unwrap()
                .retain(|s| s.id.as_ref() != session_id);
            Ok(())
        }

        async fn cleanup_expired_sessions(&self, before: i64) -> Result<(), DaoError> {
            self.sessions
                .lock()
                .unwrap()
                .retain(|s| s.expires >= before);
            Ok(())
        }

        async fn touch_session(&self, session_id: &str, now: i64) -> Result<(), DaoError> {
            for s in self.sessions.lock().unwrap().iter_mut() {
                if s.id.as_ref() == session_id {
                    s.last_used_at = now;
                }
            }
            Ok(())
        }

        async fn delete_sessions_for_user(&self, user_id: &str) -> Result<u64, DaoError> {
            let mut sessions = self.sessions.lock().unwrap();
            let before = sessions.len();
            sessions.retain(|s| s.user_id.as_ref() != user_id);
            Ok((before - sessions.len()) as u64)
        }
    }

    struct TestDeps;
    impl SessionServiceDeps for TestDeps {
        type Context = genossi_service::permission::MockContext;
        type Transaction = TestTransaction;
        type PermissionDao = TestPermissionDao;
    }

    fn make_service(dao: TestPermissionDao) -> SessionServiceImpl<TestDeps> {
        SessionServiceImpl {
            permission_dao: Arc::new(dao),
        }
    }

    #[tokio::test]
    async fn test_create_session_sets_last_used_at() {
        let dao = TestPermissionDao::new();
        let service = make_service(dao);

        let session = service.create_session("alice", 3600).await.unwrap();
        assert_eq!(session.last_used_at, session.created_at);
    }

    #[tokio::test]
    async fn test_verify_session_updates_last_used_at() {
        let dao = TestPermissionDao::new();
        let service = make_service(dao);

        let session = service.create_session("alice", 3600).await.unwrap();
        let sid = session.session_id.to_string();

        let verified = service.verify_user_session(&sid).await.unwrap().unwrap();
        // last_used_at should be >= created_at (updated to now)
        assert!(verified.last_used_at >= session.created_at);
    }

    #[tokio::test]
    async fn test_session_absolute_lifetime_valid() {
        let dao = TestPermissionDao::new();
        let now = time::OffsetDateTime::now_utc().unix_timestamp();

        // Create session that expires in 14 days, still within absolute lifetime
        let entity = SessionEntity {
            id: "test-session".into(),
            user_id: "alice".into(),
            expires: now + 14 * 24 * 60 * 60 - 1, // 14d - 1s
            created: now,
            claims: None,
            last_used_at: now,
        };
        dao.create_session(&entity).await.unwrap();
        let service = make_service(dao);

        let result = service.verify_user_session("test-session").await.unwrap();
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_session_absolute_lifetime_expired() {
        let dao = TestPermissionDao::new();
        let now = time::OffsetDateTime::now_utc().unix_timestamp();

        // Session created > 14 days ago, already expired
        let entity = SessionEntity {
            id: "test-session".into(),
            user_id: "alice".into(),
            expires: now - 1, // already expired
            created: now - 14 * 24 * 60 * 60 - 1,
            claims: None,
            last_used_at: now,
        };
        dao.create_session(&entity).await.unwrap();
        let service = make_service(dao);

        let result = service.verify_user_session("test-session").await.unwrap();
        assert!(result.is_none());
        // Session should be deleted
        assert_eq!(service.permission_dao.session_count(), 0);
    }

    #[tokio::test]
    async fn test_session_inactivity_valid() {
        let dao = TestPermissionDao::new();
        let now = time::OffsetDateTime::now_utc().unix_timestamp();

        // Session used 29d23h59m59s ago — still valid (within 30d)
        let entity = SessionEntity {
            id: "test-session".into(),
            user_id: "alice".into(),
            expires: now + 365 * 24 * 60 * 60,
            created: now - 3600,
            claims: None,
            last_used_at: now - 30 * 24 * 60 * 60 + 1, // 30d - 1s ago
        };
        dao.create_session(&entity).await.unwrap();
        let service = make_service(dao);

        let result = service.verify_user_session("test-session").await.unwrap();
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_session_inactivity_expired() {
        let dao = TestPermissionDao::new();
        let now = time::OffsetDateTime::now_utc().unix_timestamp();

        // Session used > 30d ago — expired by inactivity
        let entity = SessionEntity {
            id: "test-session".into(),
            user_id: "alice".into(),
            expires: now + 365 * 24 * 60 * 60,
            created: now - 60 * 24 * 60 * 60,
            claims: None,
            last_used_at: now - 30 * 24 * 60 * 60 - 1, // 30d + 1s ago
        };
        dao.create_session(&entity).await.unwrap();
        let service = make_service(dao);

        let result = service.verify_user_session("test-session").await.unwrap();
        assert!(result.is_none());
        assert_eq!(service.permission_dao.session_count(), 0);
    }

    #[tokio::test]
    async fn test_revoke_all_for_user_deletes_only_target_user() {
        let dao = TestPermissionDao::new();
        let now = time::OffsetDateTime::now_utc().unix_timestamp();

        // Alice's sessions
        for i in 0..3 {
            dao.create_session(&SessionEntity {
                id: format!("alice-{i}").into(),
                user_id: "alice".into(),
                expires: now + 3600,
                created: now,
                claims: None,
                last_used_at: now,
            })
            .await
            .unwrap();
        }
        // Bob's session
        dao.create_session(&SessionEntity {
            id: "bob-0".into(),
            user_id: "bob".into(),
            expires: now + 3600,
            created: now,
            claims: None,
            last_used_at: now,
        })
        .await
        .unwrap();

        let service = make_service(dao);
        let count = service.revoke_all_for_user("alice").await.unwrap();

        assert_eq!(count, 3);
        assert_eq!(service.permission_dao.session_count(), 1); // only Bob's session left
    }

    #[tokio::test]
    async fn test_revoke_all_then_verify_returns_none() {
        let dao = TestPermissionDao::new();
        let service = make_service(dao);

        let session = service.create_session("alice", 3600).await.unwrap();
        let sid = session.session_id.to_string();

        service.revoke_all_for_user("alice").await.unwrap();
        let result = service.verify_user_session(&sid).await.unwrap();
        assert!(result.is_none());
    }
}

// Mock implementation for development/testing
pub struct MockSessionServiceImpl;

#[async_trait]
impl SessionService for MockSessionServiceImpl {
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
            last_used_at: now,
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
            last_used_at: now,
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
            created_at: now - 60,
            claims: None,
            last_used_at: now,
        }))
    }

    async fn invalidate_session(&self, _session_id: &str) -> Result<(), ServiceError> {
        Ok(())
    }

    async fn cleanup_expired_sessions(&self) -> Result<u64, ServiceError> {
        Ok(0)
    }

    async fn revoke_all_for_user(&self, _user_id: &str) -> Result<u64, ServiceError> {
        Ok(0)
    }

    async fn extract_auth_context(
        &self,
        session_id: Option<String>,
    ) -> Result<Option<AuthContext>, ServiceError> {
        if session_id.is_some() {
            Ok(Some(AuthContext::Mock(MockContext::default())))
        } else {
            Ok(None)
        }
    }
}
