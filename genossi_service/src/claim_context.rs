use uuid::Uuid;

use crate::claim_utils;

/// Trait for contexts that can provide claim information
pub trait ClaimContext {
    /// Check if this context has claims
    fn has_claims(&self) -> bool;

    /// Helper-discrimination (Phase 3, D-17/D-18; resolves Open Question 1 in 03-RESEARCH.md).
    ///
    /// Returns `Some(assembly_id)` if this context represents a redeemed
    /// Helfer-Token (claim `kind == "helper"` AND parseable `assembly_id`); else `None`.
    ///
    /// **The Phase-2 claims schema (genossi_service_impl/src/session.rs:17-30) is**
    /// `{"kind":"helper","assembly_id":"<uuid-string>"}` — there is NO `session_id`
    /// field. The session_id is the SessionEntity row id, available via
    /// `AuthenticatedContext.user_id` (format `helper:<token_id>`); it is NOT
    /// part of the claim JSON. Inside `check_assembly_access` (Plan 05) only
    /// the assembly_id is needed (compare against endpoint aid + check
    /// `assembly.status == Open`). Cascade-side session enumeration in
    /// `close_assembly` reads session ids from `HelperTokenDao::list_session_ids_for_assembly`,
    /// not from this method.
    ///
    /// Default impl: `None` — used by `()` (automock) and `MockContext`
    /// (mock_auth uses cookie-format `helper:<aid>:<tid>` instead of claims;
    /// see Phase 2 D-15 / D-16). Only `AuthenticatedContext` (oidc build)
    /// overrides this method to parse the claims JSON.
    fn as_helper(&self) -> Option<Uuid> {
        None
    }
}

// Implement for AuthenticatedContext
impl ClaimContext for crate::auth_types::AuthenticatedContext {
    fn has_claims(&self) -> bool {
        claim_utils::has_claims(self)
    }

    fn as_helper(&self) -> Option<Uuid> {
        // Defensive parse — malformed JSON or missing fields → None (failure-closed).
        // Claims schema produced by SessionServiceImpl in Phase 2 (D-16),
        // verbatim from genossi_service_impl/src/session.rs:17-30:
        //   #[derive(Deserialize)] struct HelperClaims { kind: String, assembly_id: Uuid }
        //   → JSON: { "kind": "helper", "assembly_id": "<uuid-string>" }
        // NO `session_id` field — see method doc-comment.
        let claims_str = self.claims.as_ref()?;

        // Mirror the Phase-2 HelperClaims struct locally — keeps the Deserialize
        // contract identical to the producer side (single source of truth).
        #[derive(serde::Deserialize)]
        struct HelperClaims {
            kind: String,
            assembly_id: Uuid,
        }

        let parsed: HelperClaims = serde_json::from_str(claims_str.as_ref()).ok()?;
        if parsed.kind != "helper" {
            return None;
        }
        Some(parsed.assembly_id)
    }
}

// Implement for MockContext (no claims)
impl ClaimContext for crate::permission::MockContext {
    fn has_claims(&self) -> bool {
        false
    }
    // as_helper() inherits Default — returns None.
    // mock_auth uses cookie-pattern (helper:<aid>:<tid>), not claims.
}

// Implement for () used in automock
impl ClaimContext for () {
    fn has_claims(&self) -> bool {
        false
    }
    // as_helper() inherits Default — returns None.
}

#[cfg(test)]
mod tests {
    // Test-fixtures use the EXACT JSON shape produced by Phase 2's
    // SessionServiceImpl (see genossi_service_impl/src/session.rs:17-30 for
    // the HelperClaims struct definition and lines 712-713 for the
    // `make_helper_claims` helper). Single source of truth — if Phase 2
    // ever changes the schema, these tests + the override above must follow.

    use super::*;
    use crate::auth_types::AuthenticatedContext;
    use crate::permission::MockContext;
    use std::sync::Arc;
    use uuid::Uuid;

    #[test]
    fn test_as_helper_default_returns_none_for_unit() {
        assert!(ClaimContext::as_helper(&()).is_none());
    }

    #[test]
    fn test_as_helper_for_mock_context_returns_none() {
        // permission::MockContext is a unit-struct (no fields, no Default).
        // mock_auth uses cookie-pattern (helper:<aid>:<tid>), not claims —
        // so this context never carries helper info.
        assert!(MockContext.as_helper().is_none());
    }

    #[test]
    fn test_as_helper_for_authenticated_context_with_helper_claims() {
        let aid = Uuid::new_v4();
        // Verbatim mirror of make_helper_claims (session.rs:712-713):
        let claims = format!(r#"{{"kind":"helper","assembly_id":"{}"}}"#, aid);
        let ctx = AuthenticatedContext {
            user_id: Arc::from("helper:abc"),
            claims: Some(Arc::from(claims.as_str())),
        };
        let helper_aid = ctx.as_helper().expect("expected Some(<aid>)");
        assert_eq!(helper_aid, aid);
    }

    #[test]
    fn test_as_helper_for_authenticated_context_with_oidc_claims_returns_none() {
        let ctx = AuthenticatedContext {
            user_id: Arc::from("alice"),
            claims: Some(Arc::from(r#"{"kind":"oidc"}"#)),
        };
        assert!(ctx.as_helper().is_none());
    }

    #[test]
    fn test_as_helper_for_authenticated_context_without_claims_returns_none() {
        let ctx = AuthenticatedContext {
            user_id: Arc::from("alice"),
            claims: None,
        };
        assert!(ctx.as_helper().is_none());
    }

    #[test]
    fn test_as_helper_for_authenticated_context_with_malformed_claims_returns_none() {
        let ctx = AuthenticatedContext {
            user_id: Arc::from("alice"),
            claims: Some(Arc::from("not-json{")),
        };
        assert!(ctx.as_helper().is_none());
    }

    #[test]
    fn test_as_helper_for_authenticated_context_with_helper_kind_but_invalid_uuid_returns_none() {
        // assembly_id="not-a-uuid" → Deserialize fails (Uuid type) → None.
        let ctx = AuthenticatedContext {
            user_id: Arc::from("helper:abc"),
            claims: Some(Arc::from(
                r#"{"kind":"helper","assembly_id":"not-a-uuid"}"#,
            )),
        };
        assert!(ctx.as_helper().is_none());
    }
}
