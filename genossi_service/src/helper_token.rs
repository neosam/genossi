//! Service-trait für das helper_token-Aggregat (Phase 2).
//!
//! Tests defined first (RED phase) — domain types and trait follow in GREEN.

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use uuid::Uuid;

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
