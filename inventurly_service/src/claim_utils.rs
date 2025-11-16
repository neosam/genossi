use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth_types::AuthenticatedContext;

/// Represents the claims structure for inventur token authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventurClaim {
    pub inventur_id: String,
    #[serde(rename = "type")]
    pub claim_type: String,
}

impl InventurClaim {
    /// Parse the inventur_id as a UUID
    pub fn inventur_uuid(&self) -> Option<Uuid> {
        Uuid::parse_str(&self.inventur_id).ok()
    }

    /// Check if this is an inventur token claim
    pub fn is_inventur_token(&self) -> bool {
        self.claim_type == "inventur_token"
    }
}

/// Extract inventur claims from an AuthenticatedContext
pub fn extract_inventur_claim(context: &AuthenticatedContext) -> Option<InventurClaim> {
    context.claims.as_ref().and_then(|claims_json| {
        serde_json::from_str::<InventurClaim>(claims_json).ok()
    })
}

/// Extract the inventur_id UUID from an AuthenticatedContext
pub fn extract_inventur_id(context: &AuthenticatedContext) -> Option<Uuid> {
    extract_inventur_claim(context).and_then(|claim| claim.inventur_uuid())
}

/// Check if a user has access to a specific inventur based on their claims
///
/// Returns true if:
/// - The user has no claims (meaning they have global access via roles)
/// - The user has claims with an inventur_id matching the requested inventur
pub fn has_inventur_access(context: &AuthenticatedContext, inventur_id: &Uuid) -> bool {
    match extract_inventur_id(context) {
        None => true, // No claims means global access (will be checked via role privileges)
        Some(claimed_id) => claimed_id == *inventur_id,
    }
}

/// Check if the user has claims (i.e., is using inventur token auth)
pub fn has_claims(context: &AuthenticatedContext) -> bool {
    context.claims.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_extract_inventur_claim() {
        let inventur_id = Uuid::new_v4();
        let claims_json = serde_json::json!({
            "inventur_id": inventur_id.to_string(),
            "type": "inventur_token"
        })
        .to_string();

        let context = AuthenticatedContext {
            user_id: Arc::from("test_user"),
            claims: Some(Arc::from(claims_json)),
        };

        let claim = extract_inventur_claim(&context).unwrap();
        assert_eq!(claim.inventur_uuid().unwrap(), inventur_id);
        assert!(claim.is_inventur_token());
    }

    #[test]
    fn test_extract_inventur_claim_none() {
        let context = AuthenticatedContext {
            user_id: Arc::from("test_user"),
            claims: None,
        };

        assert!(extract_inventur_claim(&context).is_none());
    }

    #[test]
    fn test_extract_inventur_id() {
        let inventur_id = Uuid::new_v4();
        let claims_json = serde_json::json!({
            "inventur_id": inventur_id.to_string(),
            "type": "inventur_token"
        })
        .to_string();

        let context = AuthenticatedContext {
            user_id: Arc::from("test_user"),
            claims: Some(Arc::from(claims_json)),
        };

        assert_eq!(extract_inventur_id(&context).unwrap(), inventur_id);
    }

    #[test]
    fn test_has_inventur_access_with_matching_claim() {
        let inventur_id = Uuid::new_v4();
        let claims_json = serde_json::json!({
            "inventur_id": inventur_id.to_string(),
            "type": "inventur_token"
        })
        .to_string();

        let context = AuthenticatedContext {
            user_id: Arc::from("test_user"),
            claims: Some(Arc::from(claims_json)),
        };

        assert!(has_inventur_access(&context, &inventur_id));
    }

    #[test]
    fn test_has_inventur_access_with_non_matching_claim() {
        let inventur_id = Uuid::new_v4();
        let other_inventur_id = Uuid::new_v4();
        let claims_json = serde_json::json!({
            "inventur_id": inventur_id.to_string(),
            "type": "inventur_token"
        })
        .to_string();

        let context = AuthenticatedContext {
            user_id: Arc::from("test_user"),
            claims: Some(Arc::from(claims_json)),
        };

        assert!(!has_inventur_access(&context, &other_inventur_id));
    }

    #[test]
    fn test_has_inventur_access_without_claims() {
        let inventur_id = Uuid::new_v4();
        let context = AuthenticatedContext {
            user_id: Arc::from("test_user"),
            claims: None,
        };

        // No claims means global access
        assert!(has_inventur_access(&context, &inventur_id));
    }

    #[test]
    fn test_has_claims() {
        let context_with_claims = AuthenticatedContext {
            user_id: Arc::from("test_user"),
            claims: Some(Arc::from("{}")),
        };
        let context_without_claims = AuthenticatedContext {
            user_id: Arc::from("test_user"),
            claims: None,
        };

        assert!(has_claims(&context_with_claims));
        assert!(!has_claims(&context_without_claims));
    }
}
