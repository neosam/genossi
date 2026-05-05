//! Helper-Code-Validierung (Phase 4 Plan 02).
//!
//! UX-Validation für `ManualCodeInput` (Plan 05) — schnelles Feedback ohne Roundtrip.
//! KEINE Security-Boundary: Backend D-24 (Phase 2 Plan 02-05) ist authoritative.
//! Diese Konstante MUSS deckungsgleich mit Phase-2-Backend-Crockford-Alphabet sein
//! (siehe RESEARCH.md §"Pitfall 9 — Crockford-Alphabet-Discrepancy").

/// Crockford Base32 alphabet — 32 characters.
/// Excludes I, L, O, U (per Phase 2 D-09).
pub const CROCKFORD_ALPHABET: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// True iff `s` has exactly 10 chars, all from `CROCKFORD_ALPHABET`.
#[allow(dead_code)]
pub fn is_valid_helper_code(s: &str) -> bool {
    s.chars().count() == 10 && s.chars().all(|c| CROCKFORD_ALPHABET.contains(c))
}

/// Filters input to Crockford-Base32, uppercases, truncates to 10 chars.
#[allow(dead_code)]
pub fn sanitize_helper_code_input(raw: &str) -> String {
    raw.to_uppercase()
        .chars()
        .filter(|c| CROCKFORD_ALPHABET.contains(*c))
        .take(10)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alphabet_has_exactly_32_chars() {
        assert_eq!(CROCKFORD_ALPHABET.chars().count(), 32);
    }

    #[test]
    fn alphabet_excludes_i_l_o_u() {
        assert!(!CROCKFORD_ALPHABET.contains('I'));
        assert!(!CROCKFORD_ALPHABET.contains('L'));
        assert!(!CROCKFORD_ALPHABET.contains('O'));
        assert!(!CROCKFORD_ALPHABET.contains('U'));
    }

    #[test]
    fn alphabet_includes_all_digits() {
        for c in '0'..='9' {
            assert!(CROCKFORD_ALPHABET.contains(c), "missing digit {c}");
        }
    }

    #[test]
    fn is_valid_helper_code_accepts_10_char_uppercase() {
        assert!(is_valid_helper_code("ABC1234567"));
        assert!(is_valid_helper_code("0123456789"));
        assert!(is_valid_helper_code("ZYXWVTSRQP"));
    }

    #[test]
    fn is_valid_helper_code_rejects_wrong_length() {
        assert!(!is_valid_helper_code(""));
        assert!(!is_valid_helper_code("ABC123"));
        assert!(!is_valid_helper_code("ABC12345678")); // 11 chars
    }

    #[test]
    fn is_valid_helper_code_rejects_excluded_letters() {
        assert!(!is_valid_helper_code("ABCIDEFGHJ")); // I
        assert!(!is_valid_helper_code("ABCLDEFGHJ")); // L
        assert!(!is_valid_helper_code("ABCODEFGHJ")); // O
        assert!(!is_valid_helper_code("ABCUDEFGHJ")); // U
    }

    #[test]
    fn is_valid_helper_code_rejects_lowercase() {
        assert!(!is_valid_helper_code("abc1234567"));
    }

    #[test]
    fn sanitize_uppercases_and_filters() {
        assert_eq!(sanitize_helper_code_input("abc123!@#"), "ABC123");
    }

    #[test]
    fn sanitize_truncates_to_10_chars() {
        // Input upper: ABCDEFGHIJKLMNOP. Filter drops I, L, O. Take 10:
        // A B C D E F G H J K (skipped I, then J K from the rest)
        assert_eq!(sanitize_helper_code_input("ABCDEFGHIJKLMNOP"), "ABCDEFGHJK");
    }
}
