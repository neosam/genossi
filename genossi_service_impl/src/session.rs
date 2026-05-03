use async_trait::async_trait;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use genossi_dao::assembly::{AssemblyDao, AssemblyStatus};
use genossi_dao::permission::{PermissionDao, SessionEntity};
use genossi_dao::TransactionDao;
use genossi_service::{
    auth_types::{AuthContext, MockContext, UserSession},
    session::SessionService,
    ServiceError,
};

use crate::gen_service_impl;

/// JSON shape of `session.claims` for helper-token sessions (D-16).
///
/// Concrete schema: `{"kind":"helper","assembly_id":"<uuid-string>"}`.
///
/// Used by `SessionServiceImpl::extract_auth_context` (Plan 02-06 Task 1,
/// RESEARCH §Pattern 2) to discriminate Helper-Sessions from regular
/// User-Sessions. Other claim shapes (no `kind` field, or `kind != "helper"`,
/// or invalid JSON) fall through to the user-session branch — backward
/// compatibility for any legacy sessions that lack the discriminator.
#[derive(Deserialize)]
struct HelperClaims {
    kind: String,
    assembly_id: Uuid,
}

gen_service_impl! {
    struct SessionServiceImpl: SessionService = SessionServiceDeps {
        PermissionDao: PermissionDao = permission_dao,
        AssemblyDao: AssemblyDao<Transaction = Self::Transaction> = assembly_dao,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
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
        // Plan 02-06 (D-15/D-16/D-18/D-19): the wire-point for the Assembly-
        // status-check is here in the Service layer (not in the REST auth-
        // middleware). Three reasons:
        //  1. Schicht-Trennung: lifecycle/permission logic belongs in the
        //     Service layer, not in the REST middleware.
        //  2. Mockability: SessionServiceDeps already has AssemblyDao +
        //     TransactionDao injected — tests can mock them directly.
        //  3. Minimal-invasiv: auth_middleware.rs already delegates to
        //     extract_auth_context; no changes needed there.
        let Some(sid) = session_id else {
            return Ok(None);
        };
        let Some(session) = self.verify_user_session(&sid).await? else {
            return Ok(None);
        };

        // Pitfall 2: early-return when the session has no claims so regular
        // OIDC/User sessions don't pay an extra DB roundtrip on the hot path.
        if let Some(claims_str) = session.claims.as_deref() {
            // Try to parse the helper-claims discriminator (D-16). If the JSON
            // matches the helper schema AND `kind == "helper"`, take the helper
            // branch with D-18 status-check. Anything else (parse error,
            // missing kind, kind != helper) falls through to the user-session
            // path — backward compatibility for legacy sessions.
            if let Ok(parsed) = serde_json::from_str::<HelperClaims>(claims_str) {
                if parsed.kind == "helper" {
                    // D-18: helper-session is only valid while the bound
                    // assembly is Open. Any other state (Preparation, Closed,
                    // missing) means the session must be invalidated.
                    let tx = self.transaction_dao.use_transaction(None).await?;
                    let assembly =
                        self.assembly_dao.find_by_id(parsed.assembly_id, tx.clone()).await?;
                    self.transaction_dao.commit(tx).await?;
                    return match assembly {
                        Some(a) if a.status == AssemblyStatus::Open => {
                            Ok(Some(AuthContext::Helper {
                                session_id: session.session_id,
                                assembly_id: parsed.assembly_id,
                            }))
                        }
                        _ => {
                            // HLPR-05 SC#4: kill the session server-side so
                            // the cookie is useless even if the browser still
                            // has it. We swallow delete errors because the
                            // primary outcome (reject the request) is already
                            // happening — logging would be appropriate in a
                            // production hardening pass.
                            self.permission_dao.delete_session(&sid).await.ok();
                            Ok(None)
                        }
                    };
                }
            }
            // Fall-through (claims existed but did not match helper schema):
            // treat as user-session. Future claim discriminators (e.g.
            // "vorstand-impersonation") would extend the if-let above.
        }

        // Default: user-session path (mock_auth-build always returns Mock;
        // OIDC-build keeps the same shape because Phase-1 already returned
        // Mock here — extending to AuthContext::Oidc(...) is a separate
        // refactor outside Plan 02-06's scope).
        Ok(Some(AuthContext::Mock(MockContext {
            user_id: session.user_id,
        })))
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

    /// Test double for `TransactionDao` — returns a fresh `TestTransaction`
    /// without touching any DB. Used by `extract_auth_context` for the
    /// D-18 status-check transaction (`use_transaction(None)`).
    #[derive(Clone, Debug)]
    struct TestTransactionDao;

    #[async_trait]
    impl genossi_dao::TransactionDao for TestTransactionDao {
        type Transaction = TestTransaction;
        async fn transaction(&self) -> Result<Self::Transaction, DaoError> {
            Ok(TestTransaction)
        }
        async fn use_transaction(
            &self,
            tx: Option<Self::Transaction>,
        ) -> Result<Self::Transaction, DaoError> {
            Ok(tx.unwrap_or(TestTransaction))
        }
        async fn commit(&self, _tx: Self::Transaction) -> Result<(), DaoError> {
            Ok(())
        }
    }

    /// Test double for `AssemblyDao` — programmable to return a specific
    /// assembly (or `None`) from `find_by_id`. Other DAO methods are stubbed
    /// since `extract_auth_context` only calls `find_by_id`.
    struct TestAssemblyDao {
        assembly: Mutex<Option<genossi_dao::assembly::AssemblyEntity>>,
    }

    impl TestAssemblyDao {
        fn empty() -> Self {
            Self {
                assembly: Mutex::new(None),
            }
        }

        fn with_assembly(entity: genossi_dao::assembly::AssemblyEntity) -> Self {
            Self {
                assembly: Mutex::new(Some(entity)),
            }
        }
    }

    #[async_trait]
    impl genossi_dao::assembly::AssemblyDao for TestAssemblyDao {
        type Transaction = TestTransaction;

        async fn dump_all(
            &self,
            _tx: Self::Transaction,
        ) -> Result<Arc<[genossi_dao::assembly::AssemblyEntity]>, DaoError> {
            let guard = self.assembly.lock().unwrap();
            match guard.as_ref() {
                Some(entity) => Ok(Arc::from(vec![entity.clone()])),
                None => Ok(Arc::from(Vec::<genossi_dao::assembly::AssemblyEntity>::new())),
            }
        }

        async fn create(
            &self,
            _entity: &genossi_dao::assembly::AssemblyEntity,
            _process: &str,
            _tx: Self::Transaction,
        ) -> Result<(), DaoError> {
            Ok(())
        }

        async fn update(
            &self,
            _entity: &genossi_dao::assembly::AssemblyEntity,
            _process: &str,
            _tx: Self::Transaction,
        ) -> Result<(), DaoError> {
            Ok(())
        }
    }

    fn make_assembly(
        id: Uuid,
        status: AssemblyStatus,
    ) -> genossi_dao::assembly::AssemblyEntity {
        let date = time::Date::from_calendar_date(2026, time::Month::May, 15).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        genossi_dao::assembly::AssemblyEntity {
            id,
            name: Arc::from("Test-GV"),
            date: datetime,
            location: None,
            status,
            opened_at: None,
            closed_at: None,
            created: datetime,
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    struct TestDeps;
    impl SessionServiceDeps for TestDeps {
        type Context = genossi_service::permission::MockContext;
        type Transaction = TestTransaction;
        type PermissionDao = TestPermissionDao;
        type AssemblyDao = TestAssemblyDao;
        type TransactionDao = TestTransactionDao;
    }

    fn make_service(dao: TestPermissionDao) -> SessionServiceImpl<TestDeps> {
        SessionServiceImpl {
            permission_dao: Arc::new(dao),
            assembly_dao: Arc::new(TestAssemblyDao::empty()),
            transaction_dao: Arc::new(TestTransactionDao),
        }
    }

    /// Builder variant for tests that need a programmable assembly
    /// (Helper-Claims-Discriminator tests).
    fn make_service_with_assembly(
        dao: TestPermissionDao,
        assembly_dao: TestAssemblyDao,
    ) -> SessionServiceImpl<TestDeps> {
        SessionServiceImpl {
            permission_dao: Arc::new(dao),
            assembly_dao: Arc::new(assembly_dao),
            transaction_dao: Arc::new(TestTransactionDao),
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

    // ========================================================================
    // Helper-Claims-Discriminator Tests (Plan 02-06 Task 1, D-15/D-16/D-18)
    //
    // These verify that `extract_auth_context` correctly discriminates between
    // user-sessions (no claims, or non-helper claims) and helper-sessions
    // (claims = `{"kind":"helper","assembly_id":"..."}`), and that helper-
    // sessions are gated by the assembly-status check (D-18).
    // ========================================================================

    fn make_helper_claims(assembly_id: Uuid) -> String {
        format!(r#"{{"kind":"helper","assembly_id":"{}"}}"#, assembly_id)
    }

    /// Insert a helper-claims-bearing session directly into the test DAO.
    /// Bypasses the public API because we want a stable session id for
    /// the discriminator tests, and we don't need to exercise create_session
    /// in this test scope.
    fn insert_helper_session(dao: &TestPermissionDao, sid: &str, assembly_id: Uuid) {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        let entity = SessionEntity {
            id: sid.into(),
            user_id: "helper-user".into(),
            expires: now + 24 * 60 * 60,
            created: now,
            claims: Some(Arc::from(make_helper_claims(assembly_id).as_str())),
            last_used_at: now,
        };
        dao.sessions.lock().unwrap().push(entity);
    }

    #[tokio::test]
    async fn test_extract_auth_context_helper_claims_returns_helper_context_when_assembly_open()
    {
        let dao = TestPermissionDao::new();
        let assembly_id = Uuid::new_v4();
        insert_helper_session(&dao, "helper-sid", assembly_id);
        let assembly_dao =
            TestAssemblyDao::with_assembly(make_assembly(assembly_id, AssemblyStatus::Open));
        let service = make_service_with_assembly(dao, assembly_dao);

        let result = service
            .extract_auth_context(Some("helper-sid".to_string()))
            .await
            .unwrap();

        match result {
            Some(AuthContext::Helper {
                session_id,
                assembly_id: parsed,
            }) => {
                assert_eq!(session_id.as_ref(), "helper-sid");
                assert_eq!(parsed, assembly_id);
            }
            other => panic!("expected AuthContext::Helper, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_extract_auth_context_helper_claims_invalidates_when_assembly_closed() {
        let dao = TestPermissionDao::new();
        let assembly_id = Uuid::new_v4();
        insert_helper_session(&dao, "helper-sid", assembly_id);
        let assembly_dao =
            TestAssemblyDao::with_assembly(make_assembly(assembly_id, AssemblyStatus::Closed));
        let service = make_service_with_assembly(dao, assembly_dao);

        let result = service
            .extract_auth_context(Some("helper-sid".to_string()))
            .await
            .unwrap();

        assert!(
            result.is_none(),
            "expected None for closed assembly, got {:?}",
            result
        );
        // HLPR-05 SC#4: session is invalidated server-side so the cookie
        // becomes useless even if the browser still presents it.
        assert_eq!(service.permission_dao.session_count(), 0);
    }

    #[tokio::test]
    async fn test_extract_auth_context_helper_claims_invalidates_when_assembly_missing() {
        let dao = TestPermissionDao::new();
        let assembly_id = Uuid::new_v4();
        insert_helper_session(&dao, "helper-sid", assembly_id);
        // No assembly registered → find_by_id returns None.
        let service = make_service_with_assembly(dao, TestAssemblyDao::empty());

        let result = service
            .extract_auth_context(Some("helper-sid".to_string()))
            .await
            .unwrap();

        assert!(result.is_none());
        assert_eq!(
            service.permission_dao.session_count(),
            0,
            "missing assembly must invalidate the helper session"
        );
    }

    #[tokio::test]
    async fn test_extract_auth_context_no_claims_returns_mock_context_backward_compat() {
        let dao = TestPermissionDao::new();
        let service = make_service(dao);

        // Standard user-session via create_session (claims = None).
        let session = service.create_session("alice", 3600).await.unwrap();
        let sid = session.session_id.to_string();

        let result = service.extract_auth_context(Some(sid)).await.unwrap();

        match result {
            Some(AuthContext::Mock(ctx)) => {
                assert_eq!(ctx.user_id.as_ref(), "alice");
            }
            other => panic!(
                "expected AuthContext::Mock for backward-compat, got {:?}",
                other
            ),
        }
    }

    #[tokio::test]
    async fn test_extract_auth_context_invalid_json_claims_falls_through_to_mock() {
        let dao = TestPermissionDao::new();
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        // Session with claims that do NOT match the helper-claims schema.
        let entity = SessionEntity {
            id: "weird-sid".into(),
            user_id: "alice".into(),
            expires: now + 3600,
            created: now,
            claims: Some(Arc::from("not-valid-json")),
            last_used_at: now,
        };
        dao.sessions.lock().unwrap().push(entity);
        let service = make_service(dao);

        let result = service
            .extract_auth_context(Some("weird-sid".to_string()))
            .await
            .unwrap();

        // Backward-compat: parse failure → fall through to user-session path.
        match result {
            Some(AuthContext::Mock(ctx)) => {
                assert_eq!(ctx.user_id.as_ref(), "alice");
            }
            other => panic!(
                "expected fall-through to AuthContext::Mock, got {:?}",
                other
            ),
        }
    }
}

/// Probe used by `MockSessionServiceImpl` to perform the D-18 status-check
/// in the `mock_auth` build (Plan 02-06 Task 2 + Plan 02-08 Task 2). Plan 07
/// wires an adapter that holds an `AssemblyDao` + `TransactionDao` internally
/// and answers `is_open` via a real DB lookup. Default tests can pass `None`
/// to keep backward-compat (helper-cookies are accepted unconditionally).
#[async_trait]
pub trait AssemblyStatusProbe: Send + Sync {
    async fn is_open(&self, assembly_id: uuid::Uuid) -> bool;
}

/// Mock implementation of `SessionService` for development/testing.
///
/// **Backward-compat:** `MockSessionServiceImpl::default()` (or
/// `::new()`) constructs an instance without a probe — all existing
/// Phase-1 tests, Phase-2-Plan-01-tests etc. pass through unchanged.
/// Helper-cookies are still recognised and return `AuthContext::Helper`,
/// but the D-18 cascade is **not** exercised (no assembly lookup).
///
/// **Plan 02-08 Task 2 (HLPR-05 cascade) usage:**
/// `MockSessionServiceImpl::with_probe(probe)` wires an adapter that
/// asks the probe whether the bound assembly is still open. When the
/// probe answers `false`, the helper-cookie is rejected — exactly the
/// D-18 behaviour that the real `SessionServiceImpl` implements.
///
/// Plan 07 wires the production `mock_auth`-build constructor in
/// `genossi_bin/src/lib.rs` so the cascade is observable end-to-end.
#[derive(Default, Clone)]
pub struct MockSessionServiceImpl {
    assembly_status_probe: Option<Arc<dyn AssemblyStatusProbe>>,
}

impl MockSessionServiceImpl {
    /// Backward-compat constructor — equivalent to `Default::default()`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct a `MockSessionServiceImpl` with the given assembly-status
    /// probe. Plan 02-08 Task 2 (HLPR-05 cascade) and Plan 07 (DI-Wiring)
    /// use this variant.
    pub fn with_probe(probe: Arc<dyn AssemblyStatusProbe>) -> Self {
        Self {
            assembly_status_probe: Some(probe),
        }
    }
}

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
        let Some(sid) = session_id else {
            return Ok(None);
        };

        // Plan 02-06 Task 2 (RESEARCH-A3 + Pitfall 5): E2E-tests in Plan 02-08
        // exercise the helper-pfad only if the mock recognises helper-cookies.
        // Convention from RESEARCH Open Q1 / 02-PATTERNS.md: helper-cookies
        // use the format `helper:<assembly_uuid>:<token_id>`. Other cookie
        // shapes (regular UUID, anything without the `helper:` prefix, or a
        // `helper:` prefix with malformed UUID) fall through to the existing
        // mock behaviour (AuthContext::Mock). This preserves backward compat
        // for every Phase-1 test that uses arbitrary session-id strings.
        if let Some(rest) = sid.strip_prefix("helper:") {
            if let Some((assembly_id_str, _token_id_str)) = rest.split_once(':') {
                if let Ok(assembly_id) = uuid::Uuid::parse_str(assembly_id_str) {
                    // D-18 cascade in mock_auth: if a probe is wired, ask it
                    // whether the bound assembly is still Open. The probe
                    // returning `false` short-circuits the helper-pfad — Plan
                    // 02-08 Task 2 uses this to assert HLPR-05 end-to-end.
                    if let Some(probe) = &self.assembly_status_probe {
                        if !probe.is_open(assembly_id).await {
                            return Ok(None);
                        }
                    }
                    return Ok(Some(AuthContext::Helper {
                        session_id: Arc::from(sid.as_str()),
                        assembly_id,
                    }));
                }
            }
            // helper:-Prefix but format invalid → fall through to MockContext.
        }
        // Existing mock behaviour for non-helper cookies.
        Ok(Some(AuthContext::Mock(MockContext::default())))
    }
}

#[cfg(test)]
mod mock_session_helper_tests {
    //! Tests for `MockSessionServiceImpl` helper-cookie-format recognition
    //! (Plan 02-06 Task 2, RESEARCH-A3 + Pitfall 5). Cookie format convention:
    //! `helper:<assembly_uuid>:<token_id>`. The probe-based variant
    //! exercises the D-18 cascade in `mock_auth` builds (Plan 02-08 Task 2).

    use super::*;

    struct AlwaysOpenProbe;
    #[async_trait]
    impl AssemblyStatusProbe for AlwaysOpenProbe {
        async fn is_open(&self, _: uuid::Uuid) -> bool {
            true
        }
    }

    struct AlwaysClosedProbe;
    #[async_trait]
    impl AssemblyStatusProbe for AlwaysClosedProbe {
        async fn is_open(&self, _: uuid::Uuid) -> bool {
            false
        }
    }

    #[tokio::test]
    async fn test_mock_helper_cookie_format_returns_helper_context() {
        let svc = MockSessionServiceImpl::default();
        let assembly_id = "550e8400-e29b-41d4-a716-446655440000";
        let cookie = format!("helper:{}:tok-abc", assembly_id);
        let result = svc
            .extract_auth_context(Some(cookie.clone()))
            .await
            .unwrap();
        match result {
            Some(AuthContext::Helper {
                session_id,
                assembly_id: parsed,
            }) => {
                assert_eq!(session_id.as_ref(), cookie.as_str());
                assert_eq!(parsed.to_string(), assembly_id);
            }
            other => panic!("expected AuthContext::Helper, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_mock_normal_cookie_returns_mock_context() {
        let svc = MockSessionServiceImpl::default();
        let result = svc
            .extract_auth_context(Some("regular-session-uuid".to_string()))
            .await
            .unwrap();
        match result {
            Some(AuthContext::Mock(_)) => {}
            other => panic!("expected AuthContext::Mock, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_mock_helper_cookie_with_invalid_uuid_falls_back_to_mock() {
        let svc = MockSessionServiceImpl::default();
        let result = svc
            .extract_auth_context(Some("helper:not-a-uuid:tok".to_string()))
            .await
            .unwrap();
        match result {
            Some(AuthContext::Mock(_)) => {}
            other => panic!("expected fall-back to Mock, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_mock_no_cookie_returns_none() {
        let svc = MockSessionServiceImpl::default();
        let result = svc.extract_auth_context(None).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_mock_helper_cookie_with_open_probe_returns_helper() {
        let svc = MockSessionServiceImpl::with_probe(Arc::new(AlwaysOpenProbe));
        let aid = "550e8400-e29b-41d4-a716-446655440000";
        let cookie = format!("helper:{}:tok-x", aid);
        let result = svc.extract_auth_context(Some(cookie)).await.unwrap();
        assert!(matches!(result, Some(AuthContext::Helper { .. })));
    }

    #[tokio::test]
    async fn test_mock_helper_cookie_with_closed_probe_returns_none() {
        // HLPR-05 cascade in mock_auth: probe says assembly is closed →
        // helper cookie is rejected. Plan 02-08 Task 2 uses this exact
        // wiring to assert the end-to-end cascade behaviour.
        let svc = MockSessionServiceImpl::with_probe(Arc::new(AlwaysClosedProbe));
        let aid = "550e8400-e29b-41d4-a716-446655440000";
        let cookie = format!("helper:{}:tok-x", aid);
        let result = svc.extract_auth_context(Some(cookie)).await.unwrap();
        assert!(result.is_none(), "closed probe must invalidate helper cookie");
    }
}
