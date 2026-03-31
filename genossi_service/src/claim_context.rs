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
