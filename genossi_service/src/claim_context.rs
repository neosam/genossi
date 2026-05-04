use crate::claim_utils;

/// Trait for contexts that can provide claim information
pub trait ClaimContext {
    /// Check if this context has claims
    fn has_claims(&self) -> bool;
}

// Implement for AuthenticatedContext
impl ClaimContext for crate::auth_types::AuthenticatedContext {
    fn has_claims(&self) -> bool {
        claim_utils::has_claims(self)
    }
}

// Implement for MockContext (no claims)
impl ClaimContext for crate::permission::MockContext {
    fn has_claims(&self) -> bool {
        false
    }
}

// Implement for () used in automock
impl ClaimContext for () {
    fn has_claims(&self) -> bool {
        false
    }
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
        assert!(MockContext::default().as_helper().is_none());
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
