use uuid::Uuid;

use crate::claim_utils;

/// Trait for contexts that can provide claim information
pub trait ClaimContext {
    /// Get the inventur_id from claims if present
    fn get_inventur_id(&self) -> Option<Uuid>;

    /// Check if this context has claims
    fn has_claims(&self) -> bool;

    /// Check if the context has access to a specific inventur
    fn has_inventur_access(&self, inventur_id: &Uuid) -> bool {
        match self.get_inventur_id() {
            None => true, // No claims means global access
            Some(claimed_id) => claimed_id == *inventur_id,
        }
    }
}

// Implement for AuthenticatedContext
impl ClaimContext for crate::auth_types::AuthenticatedContext {
    fn get_inventur_id(&self) -> Option<Uuid> {
        claim_utils::extract_inventur_id(self)
    }

    fn has_claims(&self) -> bool {
        claim_utils::has_claims(self)
    }
}

// Implement for MockContext (no claims)
impl ClaimContext for crate::permission::MockContext {
    fn get_inventur_id(&self) -> Option<Uuid> {
        None
    }

    fn has_claims(&self) -> bool {
        false
    }
}

// Implement for () used in automock
impl ClaimContext for () {
    fn get_inventur_id(&self) -> Option<Uuid> {
        None
    }

    fn has_claims(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_authenticated_context_with_claims() {
        let inventur_id = Uuid::new_v4();
        let claims_json = serde_json::json!({
            "inventur_id": inventur_id.to_string(),
            "type": "inventur_token"
        })
        .to_string();

        let context = crate::auth_types::AuthenticatedContext {
            user_id: Arc::from("test_user"),
            claims: Some(Arc::from(claims_json)),
        };

        assert!(context.has_claims());
        assert_eq!(context.get_inventur_id().unwrap(), inventur_id);
        assert!(context.has_inventur_access(&inventur_id));
        assert!(!context.has_inventur_access(&Uuid::new_v4()));
    }

    #[test]
    fn test_authenticated_context_without_claims() {
        let context = crate::auth_types::AuthenticatedContext {
            user_id: Arc::from("test_user"),
            claims: None,
        };

        assert!(!context.has_claims());
        assert!(context.get_inventur_id().is_none());
        // No claims means global access
        assert!(context.has_inventur_access(&Uuid::new_v4()));
    }

    #[test]
    fn test_mock_context() {
        let context = crate::permission::MockContext;

        assert!(!context.has_claims());
        assert!(context.get_inventur_id().is_none());
        // No claims means global access
        assert!(context.has_inventur_access(&Uuid::new_v4()));
    }
}
