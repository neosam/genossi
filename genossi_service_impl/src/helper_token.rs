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

use qrcode::render::svg;
use qrcode::{EcLevel, QrCode};
use rand::{rngs::OsRng, RngCore};
use sha2::{Digest, Sha256};

use genossi_service::{ServiceError, ValidationFailureItem};

/// Process identifier for the audit hash chain (D-07, dot-notation per Phase-1-D-11).
const HELPER_TOKEN_PROCESS_CREATE: &str = "helper_token.create";

/// Required privilege for token-management endpoints (D-21).
const ADMIN_PRIVILEGE: &str = "admin";

/// Auto-register process tag for the synthetic helper user (D-17).
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
    let code = QrCode::with_error_correction_level(payload.as_bytes(), EcLevel::Q).map_err(|e| {
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
// Service-Impl + 4 Methoden -- Task 2 fuegt diese Sektion hinzu
// ============================================================================

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
        // Two consecutive calls should differ with overwhelming probability (50-bit entropy).
        let a = generate_crockford_code();
        let b = generate_crockford_code();
        assert_ne!(a, b, "two consecutive codes equal -- RNG broken? a={}, b={}", a, b);
    }

    #[test]
    fn test_sha256_hex_deterministic() {
        assert_eq!(sha256_hex("ABC1234567"), sha256_hex("ABC1234567"));
        assert_ne!(sha256_hex("ABC1234567"), sha256_hex("ABC1234568"));
        // Length must be 64 hex chars (256 bit).
        assert_eq!(sha256_hex("anything").len(), 64);
        // Lowercase only.
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
        assert!(svg_str.contains("</svg>"), "qr svg must contain closing tag");
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
        // Lowercase
        assert!(matches!(
            validate_code_format("abc1234567"),
            Err(ServiceError::ValidationError(_))
        ));
        // Forbidden chars I, L, O, U (Crockford excludes these)
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
