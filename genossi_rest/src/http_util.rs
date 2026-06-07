/// Sanitize a string for use as a filename component.
/// Transliterates German umlauts, replaces other non-safe characters with `_`,
/// trims leading/trailing `_`, lowercases the result.
/// Returns `"_"` for empty or all-underscore input.
pub fn sanitize_filename_component(s: &str) -> String {
    let transliterated: String = s
        .chars()
        .flat_map(|c| match c {
            'ä' => vec!['a', 'e'],
            'Ä' => vec!['a', 'e'],
            'ö' => vec!['o', 'e'],
            'Ö' => vec!['o', 'e'],
            'ü' => vec!['u', 'e'],
            'Ü' => vec!['u', 'e'],
            'ß' => vec!['s', 's'],
            _ => vec![c],
        })
        .collect();

    let sanitized: String = transliterated
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();

    let trimmed = sanitized.trim_matches('_');
    if trimmed.is_empty() {
        "_".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Build an RFC 6266 `Content-Disposition: attachment` header value.
///
/// Returns a value with both an ASCII-safe `filename="..."` fallback and
/// a UTF-8 percent-encoded `filename*=UTF-8''...` parameter.
pub fn content_disposition_attachment(filename: &str) -> String {
    let ascii_fallback = sanitize_ascii_filename(filename);
    let utf8_encoded = percent_encode_utf8(filename);
    format!(
        "attachment; filename=\"{}\"; filename*=UTF-8''{}",
        ascii_fallback, utf8_encoded
    )
}

/// Build an RFC 6266 `Content-Disposition: inline` header value.
/// Same filename-encoding rules as `content_disposition_attachment` —
/// the only difference is the disposition kind. Both helpers must share
/// `sanitize_ascii_filename` + `percent_encode_utf8` so the T-02 +
/// T-05 (CR/LF header-injection) guarantees are identical.
pub fn content_disposition_inline(filename: &str) -> String {
    let ascii_fallback = sanitize_ascii_filename(filename);
    let utf8_encoded = percent_encode_utf8(filename);
    format!(
        "inline; filename=\"{}\"; filename*=UTF-8''{}",
        ascii_fallback, utf8_encoded
    )
}

/// Replace non-ASCII, `"`, `\`, `\r`, `\n` with `_` for the ASCII fallback.
fn sanitize_ascii_filename(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii() && c != '"' && c != '\\' && c != '\r' && c != '\n' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Percent-encode a UTF-8 string for RFC 5987 / RFC 6266 `filename*` parameter.
/// Only unreserved characters `[A-Za-z0-9._~-]` pass through unencoded.
fn percent_encode_utf8(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 3);
    for byte in s.as_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'.' | b'_' | b'~' | b'-' => {
                result.push(*byte as char);
            }
            _ => {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_umlaut() {
        assert_eq!(sanitize_filename_component("Müller"), "mueller");
    }

    #[test]
    fn test_sanitize_apostrophe() {
        assert_eq!(sanitize_filename_component("O'Brien"), "o_brien");
    }

    #[test]
    fn test_sanitize_hyphen() {
        assert_eq!(sanitize_filename_component("Anna-Lena"), "anna-lena");
    }

    #[test]
    fn test_sanitize_empty() {
        assert_eq!(sanitize_filename_component(""), "_");
    }

    #[test]
    fn test_sanitize_all_underscores() {
        assert_eq!(sanitize_filename_component("___"), "_");
    }

    #[test]
    fn test_sanitize_eszett() {
        assert_eq!(sanitize_filename_component("Strauß"), "strauss");
    }

    #[test]
    fn test_sanitize_uppercase_umlaut() {
        assert_eq!(sanitize_filename_component("Über"), "ueber");
    }

    #[test]
    fn test_simple_filename() {
        let result = content_disposition_attachment("foo.pdf");
        assert!(result.contains("filename=\"foo.pdf\""));
        assert!(result.contains("filename*=UTF-8''foo.pdf"));
    }

    #[test]
    fn test_umlaut_filename() {
        let result = content_disposition_attachment("Müller.pdf");
        // ASCII fallback replaces ü with _
        assert!(result.contains("filename=\"M_ller.pdf\""));
        // UTF-8 part encodes ü (U+00FC = C3 BC)
        assert!(result.contains("filename*=UTF-8''M%C3%BCller.pdf"));
    }

    #[test]
    fn test_quote_in_filename() {
        let result = content_disposition_attachment("file\"name.pdf");
        // ASCII fallback replaces " with _
        assert!(result.contains("filename=\"file_name.pdf\""));
        // UTF-8 part encodes "
        assert!(result.contains("%22"));
    }

    #[test]
    fn test_newline_in_filename() {
        let result = content_disposition_attachment("file\r\nname.pdf");
        // ASCII fallback replaces \r\n with __
        assert!(result.contains("filename=\"file__name.pdf\""));
        // UTF-8 encodes \r\n
        assert!(result.contains("%0D%0A"));
    }

    #[test]
    fn test_empty_filename() {
        let result = content_disposition_attachment("");
        assert_eq!(result, "attachment; filename=\"\"; filename*=UTF-8''");
    }

    #[test]
    fn test_long_filename() {
        let long_name = "a".repeat(255) + ".pdf";
        let result = content_disposition_attachment(&long_name);
        assert!(result.contains("filename*=UTF-8''"));
        assert!(result.contains(&long_name));
    }

    #[test]
    fn test_space_in_filename() {
        let result = content_disposition_attachment("my file.pdf");
        assert!(result.contains("filename=\"my file.pdf\""));
        assert!(result.contains("filename*=UTF-8''my%20file.pdf"));
    }

    // ── Phase 19 Plan 03 Task 1: inline-disposition unit tests ─────────
    // Mirror naming + assertions of the attachment-tests above. T-02 +
    // T-05 (header injection) explicitly covered.

    #[test]
    fn test_inline_simple_filename() {
        let result = content_disposition_inline("invoice.pdf");
        assert!(result.contains("inline; filename=\"invoice.pdf\""));
        assert!(result.contains("filename*=UTF-8''invoice.pdf"));
    }

    #[test]
    fn test_inline_umlaut_filename() {
        let result = content_disposition_inline("Rückzahlung.pdf");
        // ASCII fallback replaces ü with _
        assert!(result.contains("filename=\"R_ckzahlung.pdf\""));
        // UTF-8 part encodes ü (U+00FC = C3 BC)
        assert!(result.contains("filename*=UTF-8''R%C3%BCckzahlung.pdf"));
    }

    #[test]
    fn test_inline_quote_in_filename() {
        let result = content_disposition_inline("a\"b.pdf");
        // ASCII fallback replaces " with _
        assert!(result.contains("filename=\"a_b.pdf\""));
        // UTF-8 part encodes "
        assert!(result.contains("%22"));
    }

    #[test]
    fn test_inline_newline_in_filename() {
        let result = content_disposition_inline("a\r\nb.pdf");
        // T-05 header-injection guard: CR/LF must NOT appear as literal chars
        assert!(!result.contains('\r'));
        assert!(!result.contains('\n'));
        // ASCII fallback replaces \r\n with __
        assert!(result.contains("filename=\"a__b.pdf\""));
        // UTF-8 part percent-encodes \r\n
        assert!(result.contains("%0D%0A"));
    }
}
