//! Service-layer implementation of the helper_token aggregate (Phase 2).
//!
//! Lifecycle: Open -> Used (atomic redeem) | Open -> Revoked (vorstand revoke).
//! Status is DERIVED from columns (D-02): no Status field on the entity.
//!
//! Audit (D-07/D-08):
//!   - create: audited via `audited_create!` with process "helper_token.create"
//!   - update (revoke / set_session_id): NOT audited (D-08)
//!
//! Token codegen + storage:
//!   - 10-char Crockford-Base32 plaintext, OsRng (D-09/D-10)
//!   - Stored as SHA256(code) hex-lowercase (D-11)
//!   - Plaintext returned ONCE in HelperTokenCreated.code; never logged
//!
//! Atomic redeem (D-25, Pitfall 3):
//!   1. validate_code_format -> 400
//!   2. sha256_hex(code)
//!   3. helper_token_dao.atomic_redeem in tx -> Some/None
//!   4. on None: lookup_status -> 404 / 410 / 403 (D-24)
//!   5. assembly_dao.find_by_id -> status==Open or 403
//!   6. permission_dao.ensure_user_exists(helper:<token_id>, HELPER_USER_PROCESS="helper-token-redeem")
//!      + session_service.create_session_with_claims (claims kind=helper)
//!   7. helper_token_dao.set_session_id IN SAME TX (Pitfall 3)
//!   8. commit -> return HelperRedeemSuccess

use std::sync::Arc;

use async_trait::async_trait;
use qrcode::render::svg;
use qrcode::{EcLevel, QrCode};
use rand::{rngs::OsRng, RngCore};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use genossi_dao::assembly::{AssemblyDao, AssemblyStatus};
use genossi_dao::audit_log::AuditLogDao;
use genossi_dao::helper_token::{HelperTokenDao, HelperTokenEntity};
use genossi_dao::permission::PermissionDao;
use genossi_dao::TransactionDao;
use genossi_service::helper_token::{
    HelperRedeemSuccess, HelperToken, HelperTokenCreated, HelperTokenService, HelperTokenSubmission,
};
use genossi_service::permission::{Authentication, PermissionService};
use genossi_service::session::SessionService;
use genossi_service::uuid_service::UuidService;
use genossi_service::{ServiceError, ValidationFailureItem};

use crate::gen_service_impl;

/// Process identifier for the audit hash chain (D-07, dot-notation per Phase-1-D-11).
const HELPER_TOKEN_PROCESS_CREATE: &str = "helper_token.create";

/// Process identifier for the (un-audited) revoke DAO update.
const HELPER_TOKEN_PROCESS_REVOKE: &str = "helper_token.revoke";

/// Required privilege for token-management endpoints (D-21).
const ADMIN_PRIVILEGE: &str = "admin";

/// Auto-register process tag for the synthetic helper user (D-17).
/// Deliberately distinct from the inventur-token-auto-register tag used by
/// the SessionService-wrapper in `session.rs` -- keeps the helper-token
/// redemption forensically separable in the user table.
const HELPER_USER_PROCESS: &str = "helper-token-redeem";

/// Crockford-Base32 alphabet (D-09): excludes I, L, O, U for human readability.
const CROCKFORD_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Plaintext code length (D-09): fixed 10 chars => 50 bit entropy.
const CODE_LENGTH: usize = 10;

/// Session lifetime in seconds (D-18): 24h ab redeem.
const HELPER_SESSION_LIFETIME_SECS: i64 = 24 * 60 * 60;

/// Generate a fresh 10-char Crockford-Base32 plaintext code.
/// Uses OsRng (cryptographically secure, D-10).
/// Bias-Analyse: `b & 0x1f` partitioniert 256 source-bytes auf 32 buckets gleichmaessig.
pub fn generate_crockford_code() -> String {
    let mut buf = [0u8; CODE_LENGTH];
    OsRng.fill_bytes(&mut buf);
    buf.iter()
        .map(|&b| CROCKFORD_ALPHABET[(b & 0x1f) as usize] as char)
        .collect()
}

/// SHA256 of input, lowercase hex (D-11). Used as token_hash.
/// Salting NOT required: 50-bit pre-image entropy makes rainbow tables irrelevant.
pub fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Render a QR-Code-SVG with `EcLevel::Q` (high error-correction for printed codes; D-13).
pub fn render_qr_svg(payload: &str) -> Result<String, ServiceError> {
    let code =
        QrCode::with_error_correction_level(payload.as_bytes(), EcLevel::Q).map_err(|e| {
            ServiceError::InternalError(Arc::from(format!("QR generate failed: {}", e)))
        })?;
    Ok(code.render::<svg::Color>().build())
}

/// Validate that a redeem-code matches the D-09 format.
/// Returns `ServiceError::ValidationError` mapped to HTTP 400 (D-24).
pub fn validate_code_format(code: &str) -> Result<(), ServiceError> {
    let mut errors: Vec<ValidationFailureItem> = Vec::new();
    if code.chars().count() != CODE_LENGTH {
        errors.push(ValidationFailureItem {
            field: Arc::from("code"),
            message: Arc::from(format!("invalid_length (expected {})", CODE_LENGTH)),
        });
    } else if !code
        .chars()
        .all(|c| (CROCKFORD_ALPHABET as &[u8]).contains(&(c as u8)))
    {
        errors.push(ValidationFailureItem {
            field: Arc::from("code"),
            message: Arc::from("invalid_alphabet (use Crockford base32 uppercase)"),
        });
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(ServiceError::ValidationError(errors))
    }
}

/// Read APP_URL with mock-friendly default (D-12, RESEARCH-A4).
/// In OIDC build APP_URL is required at server start (Plan 07 wires fail-fast); in
/// mock_auth (Tests) we accept the default to keep e2e-tests hermetic.
fn app_url() -> String {
    std::env::var("APP_URL").unwrap_or_else(|_| "http://localhost:3000/".to_string())
}

// ============================================================================
// Service-Impl + 4 Methoden (Task 2)
// ============================================================================

gen_service_impl! {
    struct HelperTokenServiceImpl: HelperTokenService = HelperTokenServiceDeps {
        HelperTokenDao: HelperTokenDao<Transaction = Self::Transaction> = helper_token_dao,
        AssemblyDao: AssemblyDao<Transaction = Self::Transaction> = assembly_dao,
        AuditLogDao: AuditLogDao<Transaction = Self::Transaction> = audit_log_dao,
        PermissionService: PermissionService<Context = Self::Context> = permission_service,
        PermissionDao: PermissionDao<Transaction = Self::Transaction> = permission_dao,
        SessionService: SessionService = session_service,
        UuidService: UuidService = uuid_service,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
    }
}

#[async_trait]
impl<Deps: HelperTokenServiceDeps> HelperTokenService for HelperTokenServiceImpl<Deps> {
    type Context = Deps::Context;
    type Transaction = Deps::Transaction;

    async fn create_helper_token(
        &self,
        assembly_id: Uuid,
        submission: &HelperTokenSubmission,
        context: Authentication<Self::Context>,
    ) -> Result<HelperTokenCreated, ServiceError> {
        let tx = self.transaction_dao.use_transaction(None).await?;

        // Permission + user_id (Phase-1-Pattern aus assembly.rs:67-110).
        let user_id = self
            .permission_service
            .current_user_id(context.clone())
            .await?
            .unwrap_or_else(|| "SYSTEM".to_string());
        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context)
            .await?;

        // Assembly-Lifecycle-Guard: token erzeugen ist nur in Preparation oder Open zulaessig.
        let assembly = self
            .assembly_dao
            .find_by_id(assembly_id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(assembly_id))?;
        if assembly.status == AssemblyStatus::Closed {
            return Err(ServiceError::Conflict(Arc::from(
                "Cannot create helper_token: assembly status is Closed",
            )));
        }

        // Codegen + hash + qr_svg.
        // KEIN tracing::debug!(code) -- D-11: Klartext-Code darf nicht geloggt werden.
        let code = generate_crockford_code();
        let token_hash = sha256_hex(&code);
        let payload = format!("{}/helper?code={}", app_url().trim_end_matches('/'), code);
        let qr_svg = render_qr_svg(&payload)?;

        // Build entity.
        let now = time::OffsetDateTime::now_utc();
        let created = time::PrimitiveDateTime::new(now.date(), now.time());
        let entity = HelperTokenEntity {
            id: self.uuid_service.new_v4().await,
            assembly_id,
            memo: submission.memo.clone(),
            token_hash: Arc::from(token_hash.as_str()),
            created,
            used_at: None,
            session_id: None,
            revoked_at: None,
            deleted: None,
            version: self.uuid_service.new_v4().await,
        };

        crate::audited_create!(
            self,
            self.helper_token_dao,
            &entity,
            HELPER_TOKEN_PROCESS_CREATE,
            &user_id,
            tx
        );

        self.transaction_dao.commit(tx).await?;

        Ok(HelperTokenCreated {
            token: HelperToken::from(&entity),
            code: Arc::from(code.as_str()),
            qr_svg: Arc::from(qr_svg.as_str()),
        })
    }

    async fn list_for_assembly(
        &self,
        assembly_id: Uuid,
        context: Authentication<Self::Context>,
    ) -> Result<Arc<[HelperToken]>, ServiceError> {
        let tx = self.transaction_dao.use_transaction(None).await?;
        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context)
            .await?;
        let entities = self
            .helper_token_dao
            .all_for_assembly(assembly_id, tx.clone())
            .await?;
        self.transaction_dao.commit(tx).await?;
        let domain: Vec<HelperToken> = entities.iter().map(HelperToken::from).collect();
        Ok(Arc::from(domain))
    }

    async fn revoke_helper_token(
        &self,
        assembly_id: Uuid,
        token_id: Uuid,
        context: Authentication<Self::Context>,
    ) -> Result<HelperToken, ServiceError> {
        let tx = self.transaction_dao.use_transaction(None).await?;
        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context)
            .await?;

        // Assembly-Status-Guard (D-23).
        let assembly = self
            .assembly_dao
            .find_by_id(assembly_id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(assembly_id))?;
        if assembly.status == AssemblyStatus::Closed {
            return Err(ServiceError::Conflict(Arc::from(
                "Cannot revoke helper_token: assembly status is Closed",
            )));
        }

        // Token-Existenz.
        let mut token = self
            .helper_token_dao
            .find_by_id(token_id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(token_id))?;

        // Path-Konsistenz: token muss zu der genannten Assembly gehoeren.
        if token.assembly_id != assembly_id {
            return Err(ServiceError::EntityNotFound(token_id));
        }

        // D-03: Revoke nur wenn used_at IS NULL.
        if token.used_at.is_some() {
            return Err(ServiceError::Conflict(Arc::from("already_used")));
        }
        // Idempotenz: revoke eines bereits revoked Tokens schlaegt fehl mit Conflict.
        if token.revoked_at.is_some() {
            return Err(ServiceError::Conflict(Arc::from("already_revoked")));
        }

        // Mutation. Note: do NOT bump `token.version` here -- the SQLite
        // DAO `update` reads `entity.version` as the WHERE-clause guard
        // (optimistic-lock against the DB row's existing version) and
        // generates a fresh `new_version` internally. Setting
        // `token.version = new_v4()` would cause the WHERE to match the
        // *new* version against the DB's *old* version -> 0 rows
        // affected -> ConflictError("Version mismatch") on the very
        // first revoke. Plan 02-08 Task 2 e2e listing-test caught this.
        let now = time::OffsetDateTime::now_utc();
        token.revoked_at = Some(time::PrimitiveDateTime::new(now.date(), now.time()));

        // D-08: revoke wird NICHT auditiert. Direct DAO-Update.
        self.helper_token_dao
            .update(&token, HELPER_TOKEN_PROCESS_REVOKE, tx.clone())
            .await?;

        self.transaction_dao.commit(tx).await?;
        Ok(HelperToken::from(&token))
    }

    async fn redeem_helper_token(&self, code: &str) -> Result<HelperRedeemSuccess, ServiceError> {
        // 1. Format-validation (D-24-400).
        validate_code_format(code)?;

        // 2. Hash.
        let token_hash = sha256_hex(code);

        // ----------------------------------------------------------------
        // PHASE 1: Atomic redeem + assembly-status-check inside a TX.
        //   The TX is committed BEFORE we touch any DAO that uses its own
        //   pool connection (permission_dao.create_session,
        //   permission_dao.ensure_user_exists). If the redeem-TX is still
        //   open while a parallel pool-acquire is requested in the same
        //   async task, sqlx-sqlite serialises pool acquires and the task
        //   deadlocks (an open BEGIN holds its connection; the next
        //   acquire waits indefinitely).
        // ----------------------------------------------------------------
        let tx = self.transaction_dao.use_transaction(None).await?;
        let now = time::OffsetDateTime::now_utc();
        let now_pdt = time::PrimitiveDateTime::new(now.date(), now.time());

        // 3. Atomic redeem (D-25).
        let redeem_result = self
            .helper_token_dao
            .atomic_redeem(&token_hash, now_pdt, tx.clone())
            .await?;

        let (token_id, assembly_id) = match redeem_result {
            Some(t) => t,
            None => {
                // 4. Differential lookup_status (D-24).
                let status = self
                    .helper_token_dao
                    .lookup_status(&token_hash, tx.clone())
                    .await?;
                self.transaction_dao.commit(tx).await?; // close TX cleanly
                return match status {
                    None => Err(ServiceError::EntityNotFound(Uuid::nil())), // 404
                    Some((_, Some(_))) => Err(ServiceError::Conflict(Arc::from("revoked"))), // 403
                    Some((Some(_), None)) => {
                        Err(ServiceError::Conflict(Arc::from("already_used"))) // 410
                    }
                    Some((None, None)) => {
                        // Should not happen -- atomic_redeem would have succeeded.
                        // Defensive: treat as unknown.
                        Err(ServiceError::EntityNotFound(Uuid::nil()))
                    }
                };
            }
        };

        // 5. Assembly-Status-Check (D-18/D-24-403).
        let assembly = self
            .assembly_dao
            .find_by_id(assembly_id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(assembly_id))?;
        if assembly.status != AssemblyStatus::Open {
            // Token wurde bereits "verbrannt" durch atomic_redeem -- D-18 akzeptiert das.
            // RESEARCH Pitfall 6: D-18 garantiert Session-Invalidate; verbrannter Token ist OK.
            self.transaction_dao.commit(tx).await?;
            return Err(ServiceError::Conflict(Arc::from("assembly_not_open")));
        }

        // Commit the redeem-TX — assembly-Open is confirmed. We split the
        // session-creation and set_session_id into a follow-up step (see
        // PHASE 2 below). RESEARCH Pitfall 3 warned about a 2-step
        // commit-window; the inconsistency mode is a "burned" token whose
        // session_id remains NULL on a crash between step 6 and step 9.
        // That is functionally identical to a token whose session was
        // immediately invalidated by D-18 (Pitfall 6) and is acceptable.
        self.transaction_dao.commit(tx).await?;

        // ----------------------------------------------------------------
        // PHASE 2: register synthetic user + create session + persist
        //   session_id. Each of these uses an independent pool connection
        //   (permission_dao + session_service own a SqlitePool, no shared
        //   tx with the redeem-TX). After they finish, we open a fresh
        //   short-lived TX for the set_session_id update.
        // ----------------------------------------------------------------

        // 6. Synthetischer User pro Token (D-17).
        let helper_user_id = format!("helper:{}", token_id);

        // 7. Claims-JSON (D-16).
        let claims_json = serde_json::json!({
            "kind": "helper",
            "assembly_id": assembly_id.to_string(),
        })
        .to_string();

        // 8a. Synthetischen Helper-User mit eigenem Process-Tag registrieren (D-17).
        //     Der wrapper session_service.ensure_user_and_create_session_with_claims
        //     verwendet hardcoded "inventur-token-auto-register" (session.rs:248-256).
        //     Wir wollen forensisch separierbar bleiben -> eigener Process-Tag.
        self.permission_dao
            .ensure_user_exists(&helper_user_id, HELPER_USER_PROCESS)
            .await?;

        // 8b. Session erzeugen mit claims (low-level, D-15/D-16/D-18).
        let session = self
            .session_service
            .create_session_with_claims(
                &helper_user_id,
                HELPER_SESSION_LIFETIME_SECS,
                Some(claims_json),
            )
            .await?;

        // 9. set_session_id in einer fresh short-lived TX (Pitfall 3 split).
        let set_tx = self.transaction_dao.use_transaction(None).await?;
        self.helper_token_dao
            .set_session_id(token_id, &session.session_id, set_tx.clone())
            .await?;
        self.transaction_dao.commit(set_tx).await?;

        Ok(HelperRedeemSuccess {
            session_id: session.session_id.clone(),
            assembly_id,
            expires_at: session.expires_at,
        })
    }
}

#[cfg(test)]
mod helper_fn_tests {
    use super::*;

    #[test]
    fn test_generate_crockford_code_length_and_alphabet() {
        for _ in 0..100 {
            let code = generate_crockford_code();
            assert_eq!(code.len(), 10, "code must be 10 chars; got {}", code);
            for c in code.chars() {
                assert!(
                    (CROCKFORD_ALPHABET as &[u8]).contains(&(c as u8)),
                    "char {} not in Crockford alphabet",
                    c
                );
            }
        }
    }

    #[test]
    fn test_generate_crockford_code_is_random() {
        let a = generate_crockford_code();
        let b = generate_crockford_code();
        assert_ne!(
            a, b,
            "two consecutive codes equal -- RNG broken? a={}, b={}",
            a, b
        );
    }

    #[test]
    fn test_sha256_hex_deterministic() {
        assert_eq!(sha256_hex("ABC1234567"), sha256_hex("ABC1234567"));
        assert_ne!(sha256_hex("ABC1234567"), sha256_hex("ABC1234568"));
        assert_eq!(sha256_hex("anything").len(), 64);
        assert!(sha256_hex("X")
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()));
    }

    #[test]
    fn test_render_qr_svg_starts_with_svg_tag() {
        let svg_str = render_qr_svg("https://example.com/helper?code=ABC1234567").unwrap();
        assert!(
            svg_str.starts_with("<?xml") || svg_str.starts_with("<svg"),
            "qr svg must start with xml or svg tag; got: {}",
            &svg_str[..50.min(svg_str.len())]
        );
        assert!(
            svg_str.contains("</svg>"),
            "qr svg must contain closing tag"
        );
    }

    #[test]
    fn test_validate_code_format_accepts_valid() {
        assert!(validate_code_format("ABC1234567").is_ok());
        assert!(validate_code_format("0123456789").is_ok());
        assert!(validate_code_format("ZYXWVTSRQP").is_ok());
    }

    #[test]
    fn test_validate_code_format_rejects_wrong_length() {
        assert!(matches!(
            validate_code_format("ABC"),
            Err(ServiceError::ValidationError(_))
        ));
        assert!(matches!(
            validate_code_format("ABCDEFGHIJK"),
            Err(ServiceError::ValidationError(_))
        ));
        assert!(matches!(
            validate_code_format(""),
            Err(ServiceError::ValidationError(_))
        ));
    }

    #[test]
    fn test_validate_code_format_rejects_invalid_alphabet() {
        assert!(matches!(
            validate_code_format("abc1234567"),
            Err(ServiceError::ValidationError(_))
        ));
        assert!(matches!(
            validate_code_format("ABCI234567"),
            Err(ServiceError::ValidationError(_))
        ));
        assert!(matches!(
            validate_code_format("ABCL234567"),
            Err(ServiceError::ValidationError(_))
        ));
        assert!(matches!(
            validate_code_format("ABCO234567"),
            Err(ServiceError::ValidationError(_))
        ));
        assert!(matches!(
            validate_code_format("ABCU234567"),
            Err(ServiceError::ValidationError(_))
        ));
    }
}

#[cfg(test)]
mod service_tests {
    //! Service-Method-Tests with full Mockall-stubs against TestTransaction.
    //! Pattern follows genossi_service_impl/src/assembly.rs:351+ (TestDeps wiring).
    //!
    //! We test 4 essentials (per Plan 02-05 Task 2 fallback note):
    //!   1. redeem rejects invalid code format (no DAO calls)
    //!   2. redeem returns 410 ("already_used") when atomic_redeem returns None and lookup_status indicates used
    //!   3. redeem returns 403 ("revoked") when atomic_redeem returns None and lookup_status indicates revoked
    //!   4. redeem returns 404 (EntityNotFound) when atomic_redeem returns None and lookup_status returns None
    //!
    //! These cover the four most security/correctness-critical D-24 mappings.
    //! Full-orchestration tests (create + revoke + happy-path-redeem) are deferred
    //! to e2e-tests in Plan 02-08 where real DAOs replace mocks.

    use super::*;
    use async_trait::async_trait;
    use genossi_dao::audit_log::{AuditLogEntry, AuditQueryFilter};
    use genossi_dao::{DaoError, Transaction};
    use mockall::mock;

    /// Test-local Transaction with Debug -- MockTransaction does not derive Debug.
    #[derive(Clone, Debug)]
    pub struct TestTransaction;

    #[async_trait]
    impl Transaction for TestTransaction {
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

    mock! {
        pub TestTxDao {}
        #[async_trait]
        impl TransactionDao for TestTxDao {
            type Transaction = TestTransaction;
            async fn transaction(&self) -> Result<TestTransaction, DaoError>;
            async fn use_transaction(
                &self,
                tx: Option<TestTransaction>,
            ) -> Result<TestTransaction, DaoError>;
            async fn commit(&self, tx: TestTransaction) -> Result<(), DaoError>;
        }
    }

    mock! {
        pub TestHelperTokenDao {}
        #[async_trait]
        impl HelperTokenDao for TestHelperTokenDao {
            type Transaction = TestTransaction;
            async fn dump_all(&self, tx: TestTransaction) -> Result<Arc<[HelperTokenEntity]>, DaoError>;
            async fn create(&self, entity: &HelperTokenEntity, process: &str, tx: TestTransaction) -> Result<(), DaoError>;
            async fn update(&self, entity: &HelperTokenEntity, process: &str, tx: TestTransaction) -> Result<(), DaoError>;
            async fn all(&self, tx: TestTransaction) -> Result<Arc<[HelperTokenEntity]>, DaoError>;
            async fn find_by_id(&self, id: Uuid, tx: TestTransaction) -> Result<Option<HelperTokenEntity>, DaoError>;
            async fn atomic_redeem(
                &self,
                token_hash: &str,
                used_at: time::PrimitiveDateTime,
                tx: TestTransaction,
            ) -> Result<Option<(Uuid, Uuid)>, DaoError>;
            async fn set_session_id(
                &self,
                token_id: Uuid,
                session_id: &str,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn lookup_status(
                &self,
                token_hash: &str,
                tx: TestTransaction,
            ) -> Result<
                Option<(Option<time::PrimitiveDateTime>, Option<time::PrimitiveDateTime>)>,
                DaoError,
            >;
            async fn all_for_assembly(
                &self,
                assembly_id: Uuid,
                tx: TestTransaction,
            ) -> Result<Arc<[HelperTokenEntity]>, DaoError>;
        }
    }

    mock! {
        pub TestAssemblyDao {}
        #[async_trait]
        impl AssemblyDao for TestAssemblyDao {
            type Transaction = TestTransaction;
            async fn dump_all(&self, tx: TestTransaction) -> Result<Arc<[genossi_dao::assembly::AssemblyEntity]>, DaoError>;
            async fn create(&self, entity: &genossi_dao::assembly::AssemblyEntity, process: &str, tx: TestTransaction) -> Result<(), DaoError>;
            async fn update(&self, entity: &genossi_dao::assembly::AssemblyEntity, process: &str, tx: TestTransaction) -> Result<(), DaoError>;
            async fn all(&self, tx: TestTransaction) -> Result<Arc<[genossi_dao::assembly::AssemblyEntity]>, DaoError>;
            async fn find_by_id(&self, id: Uuid, tx: TestTransaction) -> Result<Option<genossi_dao::assembly::AssemblyEntity>, DaoError>;
        }
    }

    mock! {
        pub TestAuditLogDao {}
        #[async_trait]
        impl AuditLogDao for TestAuditLogDao {
            type Transaction = TestTransaction;
            async fn create_entries(
                &self,
                entries: &[AuditLogEntry],
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn get_latest_hash(&self, tx: TestTransaction) -> Result<Option<String>, DaoError>;
            async fn get_by_entity(
                &self,
                entity_type: &str,
                entity_id: Uuid,
                tx: TestTransaction,
            ) -> Result<Arc<[AuditLogEntry]>, DaoError>;
            async fn get_all_ordered(
                &self,
                tx: TestTransaction,
            ) -> Result<Arc<[AuditLogEntry]>, DaoError>;
            async fn query(
                &self,
                filter: AuditQueryFilter,
                limit: i64,
                offset: i64,
                tx: TestTransaction,
            ) -> Result<Arc<[AuditLogEntry]>, DaoError>;
            async fn count(
                &self,
                filter: AuditQueryFilter,
                tx: TestTransaction,
            ) -> Result<i64, DaoError>;
        }
    }

    mock! {
        pub TestPermissionDao {}
        #[async_trait]
        impl PermissionDao for TestPermissionDao {
            type Transaction = TestTransaction;
            async fn has_privilege(&self, user: &str, privilege: &str) -> Result<bool, DaoError>;
            async fn all_users(&self) -> Result<Arc<[genossi_dao::permission::UserEntity]>, DaoError>;
            async fn get_user(&self, name: &str) -> Result<Option<genossi_dao::permission::UserEntity>, DaoError>;
            async fn create_user(&self, user: &genossi_dao::permission::UserEntity, process: &str) -> Result<(), DaoError>;
            async fn delete_user(&self, username: &str) -> Result<(), DaoError>;
            async fn ensure_user_exists(&self, username: &str, process: &str) -> Result<bool, DaoError>;
            async fn all_roles(&self) -> Result<Arc<[genossi_dao::permission::RoleEntity]>, DaoError>;
            async fn get_role(&self, name: &str) -> Result<Option<genossi_dao::permission::RoleEntity>, DaoError>;
            async fn create_role(&self, role: &genossi_dao::permission::RoleEntity, process: &str) -> Result<(), DaoError>;
            async fn delete_role(&self, role_name: &str) -> Result<(), DaoError>;
            async fn all_privileges(&self) -> Result<Arc<[genossi_dao::permission::PrivilegeEntity]>, DaoError>;
            async fn get_privilege(&self, name: &str) -> Result<Option<genossi_dao::permission::PrivilegeEntity>, DaoError>;
            async fn create_privilege(&self, privilege: &genossi_dao::permission::PrivilegeEntity, process: &str) -> Result<(), DaoError>;
            async fn delete_privilege(&self, privilege_name: &str) -> Result<(), DaoError>;
            async fn add_user_role(&self, username: &str, role: &str, process: &str) -> Result<(), DaoError>;
            async fn remove_user_role(&self, username: &str, role: &str) -> Result<(), DaoError>;
            async fn get_user_roles(&self, username: &str) -> Result<Arc<[genossi_dao::permission::RoleEntity]>, DaoError>;
            async fn add_role_privilege(&self, role_name: &str, privilege_name: &str, process: &str) -> Result<(), DaoError>;
            async fn remove_role_privilege(&self, role_name: &str, privilege_name: &str) -> Result<(), DaoError>;
            async fn get_role_privileges(&self, role_name: &str) -> Result<Arc<[genossi_dao::permission::PrivilegeEntity]>, DaoError>;
            async fn get_user_privileges(&self, username: &str) -> Result<Arc<[genossi_dao::permission::PrivilegeEntity]>, DaoError>;
            async fn create_session(&self, session: &genossi_dao::permission::SessionEntity) -> Result<(), DaoError>;
            async fn get_session(&self, session_id: &str) -> Result<Option<genossi_dao::permission::SessionEntity>, DaoError>;
            async fn delete_session(&self, session_id: &str) -> Result<(), DaoError>;
            async fn cleanup_expired_sessions(&self, before_timestamp: i64) -> Result<(), DaoError>;
            async fn touch_session(&self, session_id: &str, now: i64) -> Result<(), DaoError>;
            async fn delete_sessions_for_user(&self, user_id: &str) -> Result<u64, DaoError>;
        }
    }

    mock! {
        pub TestPermissionService {}
        #[async_trait]
        impl PermissionService for TestPermissionService {
            type Context = genossi_service::permission::MockContext;
            async fn check_permission(
                &self,
                privilege: &str,
                context: Authentication<genossi_service::permission::MockContext>,
            ) -> Result<(), ServiceError>;
            async fn current_user_id(
                &self,
                context: Authentication<genossi_service::permission::MockContext>,
            ) -> Result<Option<String>, ServiceError>;
            async fn get_all_users(
                &self,
                context: Authentication<genossi_service::permission::MockContext>,
            ) -> Result<Arc<[genossi_service::auth_types::UserResponseTO]>, ServiceError>;
            async fn create_user(
                &self,
                user: genossi_service::auth_types::UserTO,
                context: Authentication<genossi_service::permission::MockContext>,
            ) -> Result<(), ServiceError>;
            async fn delete_user(
                &self,
                username: String,
                context: Authentication<genossi_service::permission::MockContext>,
            ) -> Result<(), ServiceError>;
            async fn get_all_roles(
                &self,
                context: Authentication<genossi_service::permission::MockContext>,
            ) -> Result<Arc<[genossi_service::auth_types::RoleResponseTO]>, ServiceError>;
            async fn create_role(
                &self,
                role: genossi_service::auth_types::RoleTO,
                context: Authentication<genossi_service::permission::MockContext>,
            ) -> Result<(), ServiceError>;
            async fn delete_role(
                &self,
                role_name: String,
                context: Authentication<genossi_service::permission::MockContext>,
            ) -> Result<(), ServiceError>;
            async fn get_all_privileges(
                &self,
                context: Authentication<genossi_service::permission::MockContext>,
            ) -> Result<Arc<[genossi_service::auth_types::PrivilegeResponseTO]>, ServiceError>;
            async fn create_privilege(
                &self,
                privilege: genossi_service::auth_types::PrivilegeTO,
                context: Authentication<genossi_service::permission::MockContext>,
            ) -> Result<(), ServiceError>;
            async fn delete_privilege(
                &self,
                privilege_name: String,
                context: Authentication<genossi_service::permission::MockContext>,
            ) -> Result<(), ServiceError>;
            async fn assign_user_role(
                &self,
                user_role: genossi_service::auth_types::UserRole,
                context: Authentication<genossi_service::permission::MockContext>,
            ) -> Result<(), ServiceError>;
            async fn remove_user_role(
                &self,
                user_role: genossi_service::auth_types::UserRole,
                context: Authentication<genossi_service::permission::MockContext>,
            ) -> Result<(), ServiceError>;
            async fn get_user_roles(
                &self,
                username: String,
                context: Authentication<genossi_service::permission::MockContext>,
            ) -> Result<Arc<[genossi_service::auth_types::RoleResponseTO]>, ServiceError>;
            async fn assign_role_privilege(
                &self,
                role_privilege: genossi_service::auth_types::RolePrivilege,
                context: Authentication<genossi_service::permission::MockContext>,
            ) -> Result<(), ServiceError>;
            async fn remove_role_privilege(
                &self,
                role_privilege: genossi_service::auth_types::RolePrivilege,
                context: Authentication<genossi_service::permission::MockContext>,
            ) -> Result<(), ServiceError>;
            async fn get_role_privileges(
                &self,
                role_name: String,
                context: Authentication<genossi_service::permission::MockContext>,
            ) -> Result<Arc<[genossi_service::auth_types::PrivilegeResponseTO]>, ServiceError>;
            async fn get_user_privileges(
                &self,
                username: String,
                context: Authentication<genossi_service::permission::MockContext>,
            ) -> Result<Arc<[genossi_service::auth_types::PrivilegeResponseTO]>, ServiceError>;
            async fn has_claims(
                &self,
                context: &genossi_service::permission::MockContext,
            ) -> Result<bool, ServiceError>;
        }
    }

    mock! {
        pub TestSessionService {}
        #[async_trait]
        impl SessionService for TestSessionService {
            async fn create_session(
                &self,
                user_id: &str,
                expires_in_seconds: i64,
            ) -> Result<genossi_service::auth_types::UserSession, ServiceError>;
            async fn create_session_with_claims(
                &self,
                user_id: &str,
                expires_in_seconds: i64,
                claims: Option<String>,
            ) -> Result<genossi_service::auth_types::UserSession, ServiceError>;
            async fn verify_user_session(
                &self,
                session_id: &str,
            ) -> Result<Option<genossi_service::auth_types::UserSession>, ServiceError>;
            async fn invalidate_session(&self, session_id: &str) -> Result<(), ServiceError>;
            async fn cleanup_expired_sessions(&self) -> Result<u64, ServiceError>;
            async fn revoke_all_for_user(&self, user_id: &str) -> Result<u64, ServiceError>;
            async fn extract_auth_context(
                &self,
                session_id: Option<String>,
            ) -> Result<Option<genossi_service::auth_types::AuthContext>, ServiceError>;
        }
    }

    #[derive(Clone)]
    struct StaticUuidService;
    #[async_trait]
    impl UuidService for StaticUuidService {
        async fn new_v4(&self) -> Uuid {
            Uuid::new_v4()
        }
    }

    /// Wires the local mocks into the gen_service_impl!-generated Deps trait.
    struct TestDeps;
    impl HelperTokenServiceDeps for TestDeps {
        type Context = genossi_service::permission::MockContext;
        type Transaction = TestTransaction;
        type HelperTokenDao = MockTestHelperTokenDao;
        type AssemblyDao = MockTestAssemblyDao;
        type AuditLogDao = MockTestAuditLogDao;
        type PermissionService = MockTestPermissionService;
        type PermissionDao = MockTestPermissionDao;
        type SessionService = MockTestSessionService;
        type UuidService = StaticUuidService;
        type TransactionDao = MockTestTxDao;
    }

    fn setup_mock_tx_dao() -> MockTestTxDao {
        let mut tx_dao = MockTestTxDao::new();
        tx_dao
            .expect_use_transaction()
            .returning(|_| Ok(TestTransaction));
        tx_dao.expect_commit().returning(|_| Ok(()));
        tx_dao
    }

    fn build_service_for_redeem(
        helper_token_dao: MockTestHelperTokenDao,
    ) -> HelperTokenServiceImpl<TestDeps> {
        HelperTokenServiceImpl {
            helper_token_dao: Arc::new(helper_token_dao),
            assembly_dao: Arc::new(MockTestAssemblyDao::new()),
            audit_log_dao: Arc::new(MockTestAuditLogDao::new()),
            permission_service: Arc::new(MockTestPermissionService::new()),
            permission_dao: Arc::new(MockTestPermissionDao::new()),
            session_service: Arc::new(MockTestSessionService::new()),
            uuid_service: Arc::new(StaticUuidService),
            transaction_dao: Arc::new(setup_mock_tx_dao()),
        }
    }

    #[tokio::test]
    async fn test_redeem_rejects_invalid_code_format() {
        // No DAO calls expected -- validation short-circuits.
        let service = build_service_for_redeem(MockTestHelperTokenDao::new());
        let result = service.redeem_helper_token("abc").await;
        match result {
            Err(ServiceError::ValidationError(items)) => {
                assert!(!items.is_empty());
                assert_eq!(&*items[0].field, "code");
            }
            other => panic!("expected ValidationError, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_redeem_returns_410_for_already_used_via_lookup() {
        let mut helper_token_dao = MockTestHelperTokenDao::new();
        let used_at = time::PrimitiveDateTime::new(
            time::Date::from_calendar_date(2026, time::Month::May, 3).unwrap(),
            time::Time::MIDNIGHT,
        );
        helper_token_dao
            .expect_atomic_redeem()
            .times(1)
            .returning(|_, _, _| Ok(None));
        helper_token_dao
            .expect_lookup_status()
            .times(1)
            .returning(move |_, _| Ok(Some((Some(used_at), None))));

        let service = build_service_for_redeem(helper_token_dao);
        let result = service.redeem_helper_token("ABC1234567").await;
        match result {
            Err(ServiceError::Conflict(s)) => {
                assert_eq!(&*s, "already_used");
            }
            other => panic!("expected Conflict(already_used), got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_redeem_returns_403_for_revoked_via_lookup() {
        let mut helper_token_dao = MockTestHelperTokenDao::new();
        let revoked_at = time::PrimitiveDateTime::new(
            time::Date::from_calendar_date(2026, time::Month::May, 3).unwrap(),
            time::Time::MIDNIGHT,
        );
        helper_token_dao
            .expect_atomic_redeem()
            .times(1)
            .returning(|_, _, _| Ok(None));
        helper_token_dao
            .expect_lookup_status()
            .times(1)
            .returning(move |_, _| Ok(Some((None, Some(revoked_at)))));

        let service = build_service_for_redeem(helper_token_dao);
        let result = service.redeem_helper_token("ABC1234567").await;
        match result {
            Err(ServiceError::Conflict(s)) => {
                assert_eq!(&*s, "revoked");
            }
            other => panic!("expected Conflict(revoked), got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_redeem_returns_404_for_unknown_via_lookup() {
        let mut helper_token_dao = MockTestHelperTokenDao::new();
        helper_token_dao
            .expect_atomic_redeem()
            .times(1)
            .returning(|_, _, _| Ok(None));
        helper_token_dao
            .expect_lookup_status()
            .times(1)
            .returning(|_, _| Ok(None));

        let service = build_service_for_redeem(helper_token_dao);
        let result = service.redeem_helper_token("ABC1234567").await;
        match result {
            Err(ServiceError::EntityNotFound(_)) => {}
            other => panic!("expected EntityNotFound, got {:?}", other),
        }
    }
}
