use crate::auth_types::AuthenticatedContext;

/// Check if the user has claims (i.e., is using token-based auth)
pub fn has_claims(context: &AuthenticatedContext) -> bool {
    context.claims.is_some()
}
