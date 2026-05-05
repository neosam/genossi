//! Service-trait für das helper_token-Aggregat (Phase 2).
//!
//! Diese Datei definiert nur den Trait-Vertrag und die Domain-Types.
//! Implementation lebt in `genossi_service_impl/src/helper_token.rs` (Plan 05).
//!
//! Wichtige Phase-2-Konventionen:
//!   - `redeem_helper_token` ist PUBLIC (kein Authentication-Argument, D-22)
//!   - `create/list/revoke` erfordern admin-Permission (D-21, im Service-Impl geprüft)
//!   - Domain-Type `HelperToken` enthält KEIN `token_hash` (D-06 parallel — kein
//!     Pre-Image-Leak in den Service-Layer)
//!   - `HelperTokenCreated::code` und `qr_svg` sind ONE-TIME-Output, nirgends
//!     persistiert (D-11)

use async_trait::async_trait;
use mockall::automock;
use std::fmt::Debug;
use std::sync::Arc;
use uuid::Uuid;

use crate::permission::Authentication;
use crate::ServiceError;

/// Domain representation of a helper_token. Mirrors `HelperTokenEntity` but
/// EXCLUDES `token_hash` and `deleted` — neither belongs in the service contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HelperToken {
    pub id: Uuid,
    pub assembly_id: Uuid,
    pub memo: Arc<str>,
    pub used_at: Option<time::PrimitiveDateTime>,
    pub session_id: Option<Arc<str>>,
    pub revoked_at: Option<time::PrimitiveDateTime>,
    pub created: time::PrimitiveDateTime,
    pub version: Uuid,
}

impl From<&genossi_dao::helper_token::HelperTokenEntity> for HelperToken {
    fn from(e: &genossi_dao::helper_token::HelperTokenEntity) -> Self {
        HelperToken {
            id: e.id,
            assembly_id: e.assembly_id,
            memo: e.memo.clone(),
            used_at: e.used_at,
            session_id: e.session_id.clone(),
            revoked_at: e.revoked_at,
            created: e.created,
            version: e.version,
        }
    }
}

/// Submission input for creating a helper_token.
/// Caller (REST handler) provides only the memo; the service generates
/// id, version, created, token_hash, qr_svg.
#[derive(Clone, Debug)]
pub struct HelperTokenSubmission {
    pub memo: Arc<str>,
}

/// One-time output from `create_helper_token`. The `code` (10-char Crockford) and
/// `qr_svg` (SVG string) are returned ONCE in the create-response and never
/// persisted (D-11). Storing only `token_hash = SHA256(code)` in the DB.
#[derive(Clone, Debug)]
pub struct HelperTokenCreated {
    pub token: HelperToken,
    pub code: Arc<str>,
    pub qr_svg: Arc<str>,
}

/// Successful redeem result. The session is bound to `assembly_id`; the cookie
/// will carry `session_id` (REST handler attaches Set-Cookie, D-22).
#[derive(Clone, Debug)]
pub struct HelperRedeemSuccess {
    pub session_id: Arc<str>,
    pub assembly_id: Uuid,
    /// Unix-timestamp when the session expires (24h ab Redeem, D-18).
    pub expires_at: i64,
}

#[automock(type Context=(); type Transaction = genossi_dao::MockTransaction;)]
#[async_trait]
pub trait HelperTokenService: Send + Sync {
    type Context: Clone + Debug + PartialEq + Eq + Send + Sync + 'static;
    type Transaction: genossi_dao::Transaction;

    /// D-21 POST /api/assembly/{assembly_id}/helper-tokens — admin only.
    async fn create_helper_token(
        &self,
        assembly_id: Uuid,
        submission: &HelperTokenSubmission,
        context: Authentication<Self::Context>,
    ) -> Result<HelperTokenCreated, ServiceError>;

    /// D-21 GET /api/assembly/{assembly_id}/helper-tokens — admin only.
    async fn list_for_assembly(
        &self,
        assembly_id: Uuid,
        context: Authentication<Self::Context>,
    ) -> Result<Arc<[HelperToken]>, ServiceError>;

    /// D-21 + D-23 POST /api/assembly/{assembly_id}/helper-tokens/{token_id}/revoke
    /// — admin only; only allowed if `used_at IS NULL` AND
    /// `assembly.status in {Preparation, Open}`.
    async fn revoke_helper_token(
        &self,
        assembly_id: Uuid,
        token_id: Uuid,
        context: Authentication<Self::Context>,
    ) -> Result<HelperToken, ServiceError>;

    /// D-22 POST /api/helper/redeem — PUBLIC (no auth context).
    ///
    /// Expected ServiceError mappings (D-24 — caller in REST-Layer maps to HTTP):
    ///   - `ValidationError`               => 400 Bad Request (code format invalid)
    ///   - `EntityNotFound`                => 404 Not Found (token_hash unknown)
    ///   - `Conflict("already_used")`      => 410 Gone (used_at IS NOT NULL)
    ///   - `Conflict("revoked")`           => 403 Forbidden (revoked_at IS NOT NULL)
    ///   - `Conflict("assembly_not_open")` => 403 Forbidden (assembly.status != Open)
    ///
    /// Plan 05 finalizes the exact error-discriminator strings; Plan 07 adds the
    /// REST-layer mapping. Naming convention: `Conflict(Arc<str>)` payload values
    /// are stable error-codes (lowercase snake_case).
    async fn redeem_helper_token(&self, code: &str) -> Result<HelperRedeemSuccess, ServiceError>;

    /// Phase 4 Plan 01 (D-06): Reverse-lookup for the public Helper-Session
    /// endpoint. Returns `Ok(Some(HelperSessionInfo))` if a helper_token row
    /// carries this `session_id` (i.e. the session was issued by
    /// `redeem_helper_token`), `Ok(None)` if no such row exists. PUBLIC — no
    /// auth context required. Used by `/api/helper/session` and
    /// `/api/helper/logout` to validate that an `app_session` cookie originates
    /// from a helper redeem (and not from an admin/OIDC session). Also returns
    /// `assembly_name` so the REST handler can build `HelperSessionTO` without
    /// going through the admin-only `AssemblyService::get_assembly`.
    async fn find_assembly_for_session(
        &self,
        session_id: &str,
    ) -> Result<Option<HelperSessionInfo>, ServiceError>;
}

/// Phase 4 Plan 01 (D-06) result of `HelperTokenService::find_assembly_for_session`.
/// Carries the bound assembly's id + name (denormalized for the public
/// `/api/helper/session` endpoint, which has no admin context to call
/// `AssemblyService::get_assembly`).
#[derive(Clone, Debug)]
pub struct HelperSessionInfo {
    pub assembly_id: Uuid,
    pub assembly_name: Arc<str>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_helper_token_from_entity_excludes_hash() {
        // The Domain-Type HelperToken intentionally has no token_hash field.
        // We verify via Debug-output that no part of the entity's token_hash
        // payload is reachable through the domain type.
        let now = time::OffsetDateTime::now_utc();
        let now_pdt = time::PrimitiveDateTime::new(now.date(), now.time());
        let entity = genossi_dao::helper_token::HelperTokenEntity {
            id: Uuid::nil(),
            assembly_id: Uuid::nil(),
            memo: Arc::from("Anna"),
            token_hash: Arc::from("not-leaked"),
            created: now_pdt,
            used_at: None,
            session_id: None,
            revoked_at: None,
            deleted: None,
            version: Uuid::nil(),
        };
        let domain = HelperToken::from(&entity);
        let debug_str = format!("{:?}", domain);
        assert!(
            !debug_str.contains("not-leaked"),
            "HelperToken Debug must not contain token_hash; got: {}",
            debug_str
        );
        assert!(
            !debug_str.contains("token_hash"),
            "HelperToken should not have a token_hash field; got: {}",
            debug_str
        );
    }

    #[test]
    fn test_helper_token_created_carries_code_and_qr_svg() {
        let now = time::OffsetDateTime::now_utc();
        let now_pdt = time::PrimitiveDateTime::new(now.date(), now.time());
        let token = HelperToken {
            id: Uuid::nil(),
            assembly_id: Uuid::nil(),
            memo: Arc::from("Anna"),
            used_at: None,
            session_id: None,
            revoked_at: None,
            created: now_pdt,
            version: Uuid::nil(),
        };
        let created = HelperTokenCreated {
            token,
            code: Arc::from("ABC1234567"),
            qr_svg: Arc::from("<svg/>"),
        };
        assert_eq!(created.code.len(), 10);
        assert!(created.qr_svg.starts_with("<svg"));
    }

    #[test]
    fn test_helper_token_submission_constructible() {
        let submission = HelperTokenSubmission {
            memo: Arc::from("Anna"),
        };
        assert_eq!(&*submission.memo, "Anna");
    }

    #[test]
    fn test_helper_redeem_success_carries_session_metadata() {
        let assembly_id = Uuid::new_v4();
        let success = HelperRedeemSuccess {
            session_id: Arc::from("sess-abc"),
            assembly_id,
            expires_at: 1_777_807_000,
        };
        assert_eq!(&*success.session_id, "sess-abc");
        assert_eq!(success.assembly_id, assembly_id);
        assert!(success.expires_at > 0);
    }

    #[test]
    fn test_mock_helper_token_service_compiles() {
        // Compile-only: ensure #[automock] generated MockHelperTokenService.
        let _: MockHelperTokenService = MockHelperTokenService::new();
    }
}
