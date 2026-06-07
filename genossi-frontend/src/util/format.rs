// Failing-test scaffolding (RED phase). The function body is intentionally
// minimal so the unit tests below fail; the GREEN-phase commit fills in the
// real integer-math implementation.
pub fn format_size(_bytes: u64) -> String {
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_under_kb() {
        assert_eq!(format_size(42), "42 B");
        assert_eq!(format_size(999), "999 B");
    }

    #[test]
    fn kb_range_integer() {
        assert_eq!(format_size(12 * 1024), "12 KB");
        assert_eq!(format_size(1023 * 1024), "1023 KB");
    }

    #[test]
    fn mb_range_one_decimal() {
        assert_eq!(format_size(1_468_006), "1.4 MB");
    }

    #[test]
    fn gb_range_one_decimal() {
        let b: u64 = 12 * 1024 * 1024 * 1024 / 10;
        assert_eq!(format_size(b), "1.2 GB");
    }
}
