/// Returns true when the given email value is missing or only whitespace.
///
/// Shared, WASM-independent helper used to decide whether a "Mail senden"
/// button is disabled — both on the member detail page and on the application
/// detail view (Component-First: single source of truth, no duplicated logic).
pub fn is_email_empty(email: Option<&str>) -> bool {
    email.map(|s| s.trim().is_empty()).unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_email_empty_none_is_empty() {
        assert!(is_email_empty(None));
    }

    #[test]
    fn is_email_empty_empty_string_is_empty() {
        assert!(is_email_empty(Some("")));
    }

    #[test]
    fn is_email_empty_whitespace_only_is_empty() {
        assert!(is_email_empty(Some("   ")));
        assert!(is_email_empty(Some("\t\n ")));
    }

    #[test]
    fn is_email_empty_real_address_is_not_empty() {
        assert!(!is_email_empty(Some("member@example.org")));
    }

    #[test]
    fn is_email_empty_address_with_surrounding_whitespace_is_not_empty() {
        assert!(!is_email_empty(Some("  member@example.org  ")));
    }
}
